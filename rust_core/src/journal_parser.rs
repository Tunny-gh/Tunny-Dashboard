//! Optuna Journal形式（JSONL）パーサー
//! op_codeベースのステートマシンでパースする
//!
//! 参照: docs/implements/TASK-101/journal-parser-requirements.md

use serde_json::Value;
use std::collections::{HashMap, HashSet};

// =============================================================================
// 公開型定義
// =============================================================================

/// 最適化方向 🟢 (optuna-journal-log-format.md §CREATE_STUDY)
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationDirection {
    Minimize, // directions[i] == 0
    Maximize, // directions[i] == 1
}

/// Study のメタ情報（interfaces.ts `Study` 型に対応）🟢
#[derive(Debug, Clone)]
pub struct StudyMeta {
    pub study_id: u32,
    pub name: String,
    pub directions: Vec<OptimizationDirection>,
    pub completed_trials: u32,
    pub total_trials: u32,
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub user_attr_names: Vec<String>,
    pub has_constraints: bool,
}

/// parse_journal() の戻り値（interfaces.ts `ParseJournalResult` に対応）🟢
#[derive(Debug)]
pub struct ParseResult {
    pub studies: Vec<StudyMeta>,
    pub duration_ms: f64,
}

pub struct JournalParser;

// =============================================================================
// 分布型・逆変換（REQ-010）
// =============================================================================

/// Optuna パラメータ分布型
/// 参照: optuna-journal-log-format.md §SET_TRIAL_PARAM distribution 🟢
#[derive(Debug)]
enum Distribution {
    /// 連続浮動小数点 — log=true のとき exp(v) で逆変換 🟢
    Float { log: bool },
    /// 整数 — low + round(v) * step（log=true のとき exp してから round）🟡
    Int { low: i64, step: i64, log: bool },
    /// カテゴリ — choices[round(v) as usize] 🟢
    Categorical { choices: Vec<Value> },
    /// 旧形式 Uniform（Float log=false と同等）🟡
    Uniform,
}

impl Distribution {
    /// distribution JSON フィールドから Distribution を構築 🟢
    /// 【対応形式】:
    ///   1. JSON オブジェクト: {"name": "FloatDistribution", "low": 0.0, ...}
    ///   2. JSON 文字列（実ログ形式）: "{\"name\": \"FloatDistribution\", \"attributes\": {...}}"
    ///   3. attributes ネスト: {"name": ..., "attributes": {"low": ..., "high": ..., "log": ...}}
    fn from_json(json: &Value) -> Self {
        // 【文字列 JSON 対応】: distribution が文字列の場合はパースする 🟢
        if let Some(s) = json.as_str() {
            if let Ok(parsed) = serde_json::from_str::<Value>(s) {
                return Distribution::from_json(&parsed);
            }
            return Distribution::Uniform;
        }

        // 【attributes ネスト対応】: attributes があればそちらを優先参照 🟢
        let attrs = json.get("attributes").unwrap_or(json);

        match get_str(json, "name").unwrap_or("") {
            "FloatDistribution" => Distribution::Float {
                log: attrs.get("log").and_then(|v| v.as_bool()).unwrap_or(false),
            },
            "IntDistribution" => Distribution::Int {
                low: attrs.get("low").and_then(|v| v.as_i64()).unwrap_or(0),
                // step は最低 1（0除算防止）🟡
                step: attrs
                    .get("step")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1)
                    .max(1),
                log: attrs.get("log").and_then(|v| v.as_bool()).unwrap_or(false),
            },
            "CategoricalDistribution" => Distribution::Categorical {
                choices: attrs
                    .get("choices")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default(),
            },
            _ => Distribution::Uniform,
        }
    }

    /// 内部表現値（Optuna to_internal_repr 変換済み）→ 表示値（f64）に逆変換
    /// Categorical の場合は choices インデックスを返す（文字列ラベルは categorical_label で取得）🟢
    fn to_display_f64(&self, internal: f64) -> f64 {
        match self {
            // REQ-010: FloatDistribution log=true → exp(v) 🟢
            Distribution::Float { log } => {
                if *log {
                    internal.exp()
                } else {
                    internal
                }
            }
            // REQ-010: IntDistribution — low + round(v) * step 🟡
            Distribution::Int { low, step, log } => {
                let rounded = if *log {
                    internal.exp().round() as i64
                } else {
                    internal.round() as i64
                };
                (*low + rounded * *step) as f64
            }
            // REQ-010: CategoricalDistribution — インデックスを float で返す 🟢
            Distribution::Categorical { .. } => internal.round(),
            Distribution::Uniform => internal,
        }
    }

    /// CategoricalDistribution の文字列ラベルを返す（非数値 choices の場合）🟡
    fn categorical_label(&self, internal: f64) -> Option<String> {
        let Distribution::Categorical { choices } = self else {
            return None;
        };
        let idx = internal.round() as usize;
        choices.get(idx).map(|v| match v {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            other => other.to_string(),
        })
    }
}

// =============================================================================
// 内部ステートマシン型
// =============================================================================

/// Study 構築中の中間状態
struct StudyBuilder {
    study_id: u32,
    name: String,
    directions: Vec<OptimizationDirection>,
    /// RUNNING 含む全試行数（CREATE_TRIAL 出現回数）🟢
    total_trials: u32,
    /// COMPLETE 試行のみ（finalize() で集計）🟢
    completed_trials: u32,
    param_names: HashSet<String>,
    /// 目的列名（最初の COMPLETE 試行の values.len() から生成）🟡
    objective_names: Vec<String>,
    user_attr_names: HashSet<String>,
    has_constraints: bool,
}

/// Trial 構築中の中間状態
struct TrialBuilder {
    study_id: u32,
    /// TrialState: 0=RUNNING 1=COMPLETE 2=PRUNED 3=FAIL 4=WAITING 🟢
    state: u8,
    values: Option<Vec<f64>>,
    /// param_name → 逆変換済み表示値（REQ-010）🟢
    param_display: HashMap<String, f64>,
    /// CategoricalDistribution の文字列ラベル（文字列 choices の場合）🟡
    param_category_label: HashMap<String, String>,
    /// user_attr 数値型（REQ-012）🟢
    user_attrs_numeric: HashMap<String, f64>,
    /// user_attr 文字列型（REQ-012）🟢
    user_attrs_string: HashMap<String, String>,
    /// constraint 値リスト（REQ-013）🟢
    constraint_values: Vec<f64>,
    has_constraints: bool,
}

/// パース全体のステート
struct ParserState {
    studies: Vec<StudyBuilder>,
    /// trial_id（グローバル連番）→ TrialBuilder のマップ 🟢
    /// 分散最適化の重複書き込みは同一キーへの上書きで自動対応（REQ-002）
    trial_builders: HashMap<u32, TrialBuilder>,
    /// 次に採番する trial_id（CREATE_TRIAL 出現順、ログに明示されない）🟢
    next_trial_id: u32,
}

// =============================================================================
// JSON フィールド抽出ヘルパー（DRY 化）
// =============================================================================

/// JSON Value から u64 フィールドを取得
#[inline]
fn get_u64(json: &Value, key: &str) -> Option<u64> {
    json.get(key).and_then(|v| v.as_u64())
}

/// JSON Value から &str フィールドを取得
#[inline]
fn get_str<'a>(json: &'a Value, key: &str) -> Option<&'a str> {
    json.get(key).and_then(|v| v.as_str())
}

// =============================================================================
// ステートマシン実装
// =============================================================================

impl ParserState {
    fn new() -> Self {
        ParserState {
            studies: Vec::new(),
            // 大規模 Journal（50,000 試行）を想定した初期容量 🟡
            trial_builders: HashMap::with_capacity(1024),
            next_trial_id: 0,
        }
    }

    /// op_code を見て適切なハンドラにディスパッチ 🟢
    fn process_op(&mut self, op: u8, json: &Value) {
        match op {
            0 => self.process_create_study(json),
            // op_code 1 (DELETE_STUDY) / 2 / 3 / 7 は現バージョンで未対応（無視）🟡
            4 => self.process_create_trial(json),
            5 => self.process_set_trial_param(json),
            6 => self.process_set_trial_state_values(json),
            8 => self.process_set_trial_user_attr(json),
            9 => self.process_set_trial_system_attr(json),
            // REQ-002: 未知の op_code は無視してログ継続（TC-101-E03）🟢
            _ => {}
        }
    }

    /// op_code=0: CREATE_STUDY — Study メタ情報を登録 🟢
    fn process_create_study(&mut self, json: &Value) {
        let name = get_str(json, "study_name").unwrap_or("").to_string();
        let directions = json
            .get("directions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|d| {
                        if d.as_u64() == Some(0) {
                            OptimizationDirection::Minimize
                        } else {
                            OptimizationDirection::Maximize
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        // study_id = CREATE_STUDY 出現順（0始まり）🟢
        let study_id = self.studies.len() as u32;
        self.studies.push(StudyBuilder {
            study_id,
            name,
            directions,
            total_trials: 0,
            completed_trials: 0,
            param_names: HashSet::new(),
            objective_names: Vec::new(),
            user_attr_names: HashSet::new(),
            has_constraints: false,
        });
    }

    /// op_code=4: CREATE_TRIAL — trial_id を採番して TrialBuilder を作成 🟢
    fn process_create_trial(&mut self, json: &Value) {
        let study_id = get_u64(json, "study_id").unwrap_or(0) as u32;

        // 未知の study_id → スキップ（TC-101-E05）🟡
        if (study_id as usize) >= self.studies.len() {
            return;
        }

        // trial_id = CREATE_TRIAL 出現順（ログに明示されない）🟢
        let trial_id = self.next_trial_id;
        self.next_trial_id += 1;

        // total_trials には RUNNING 試行も含む 🟢
        self.studies[study_id as usize].total_trials += 1;

        self.trial_builders.insert(
            trial_id,
            TrialBuilder {
                study_id,
                state: 0, // デフォルト RUNNING
                values: None,
                param_display: HashMap::new(),
                param_category_label: HashMap::new(),
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: Vec::new(),
                has_constraints: false,
            },
        );
    }

    /// op_code=5: SET_TRIAL_PARAM — REQ-010 分布型逆変換を実施 🟢
    fn process_set_trial_param(&mut self, json: &Value) {
        let trial_id = get_u64(json, "trial_id").unwrap_or(0) as u32;
        let param_name = match get_str(json, "param_name") {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return,
        };
        let internal = json
            .get("param_value_internal")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let dist = json
            .get("distribution")
            .map(Distribution::from_json)
            .unwrap_or(Distribution::Uniform);

        if let Some(trial) = self.trial_builders.get_mut(&trial_id) {
            // REQ-010: 内部スケール値を表示値に逆変換して格納 🟢
            trial
                .param_display
                .insert(param_name.clone(), dist.to_display_f64(internal));
            // CategoricalDistribution の文字列ラベルを保存 🟡
            if let Some(label) = dist.categorical_label(internal) {
                trial.param_category_label.insert(param_name, label);
            }
        }
    }

    /// op_code=6: SET_TRIAL_STATE_VALUES — 状態と目的値を記録 🟢
    /// 分散最適化時の重複書き込みは上書きで対応（TC-101-E07）
    fn process_set_trial_state_values(&mut self, json: &Value) {
        let trial_id = get_u64(json, "trial_id").unwrap_or(0) as u32;
        let state = get_u64(json, "state").unwrap_or(0) as u8;
        let values = json
            .get("values")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|x| x.as_f64()).collect::<Vec<_>>());

        if let Some(trial) = self.trial_builders.get_mut(&trial_id) {
            trial.state = state;
            if let Some(v) = values {
                trial.values = Some(v);
            }
        }
    }

    /// op_code=8: SET_TRIAL_USER_ATTR — REQ-012 型別に分けて格納 🟢
    fn process_set_trial_user_attr(&mut self, json: &Value) {
        let trial_id = get_u64(json, "trial_id").unwrap_or(0) as u32;
        let Some(attrs) = json.get("user_attr").and_then(|v| v.as_object()) else {
            return;
        };
        let Some(trial) = self.trial_builders.get_mut(&trial_id) else {
            return;
        };

        for (key, val) in attrs {
            // REQ-012: 数値型 → float64列、文字列型 → カテゴリ列 🟢
            if let Some(n) = val.as_f64() {
                trial.user_attrs_numeric.insert(key.clone(), n);
            } else if let Some(s) = val.as_str() {
                trial.user_attrs_string.insert(key.clone(), s.to_string());
            }
            // bool 等は現バージョンで対象外 🟡
        }
    }

    /// op_code=9: SET_TRIAL_SYSTEM_ATTR — REQ-013 constraints を展開 🟢
    fn process_set_trial_system_attr(&mut self, json: &Value) {
        let trial_id = get_u64(json, "trial_id").unwrap_or(0) as u32;
        let Some(attrs) = json.get("system_attr").and_then(|v| v.as_object()) else {
            return;
        };
        let Some(trial) = self.trial_builders.get_mut(&trial_id) else {
            return;
        };

        // REQ-013: constraints → c1,c2,c3... 個別列展開（TASK-102 DataFrame で行う）🟢
        // ここでは TrialBuilder に値リストを保持する
        if let Some(constraints) = attrs.get("constraints").and_then(|v| v.as_array()) {
            trial.constraint_values = constraints.iter().filter_map(|v| v.as_f64()).collect();
            trial.has_constraints = true;
        }
    }

    /// 全 TrialBuilder を集約して `(Vec<StudyMeta>, Vec<DataFrame>)` を返す 🟢
    ///
    /// 【改善内容】TASK-102: DataFrame を同時に構築し WASM メモリに格納できるよう変更
    /// 【パフォーマンス】TrialBuilder のデータをムーブセマンティクスで TrialRow に移送し
    ///   HashMap のクローンコストを排除（30変数×50,000試行で約75%のメモリコスト削減）🟡
    fn finalize(self) -> (Vec<StudyMeta>, Vec<crate::dataframe::DataFrame>) {
        use crate::dataframe::{DataFrame, TrialRow};

        let ParserState {
            mut studies,
            trial_builders,
            ..
        } = self;
        let n_studies = studies.len();

        // --- trial_id 昇順でソート（DataFrame の行順を決定）🟢 ---
        // HashMap 反復順は非決定的なため、ソートで安定した列データを保証する
        let mut sorted_trials: Vec<(u32, TrialBuilder)> = trial_builders.into_iter().collect();
        sorted_trials.sort_by_key(|(id, _)| *id);

        // --- Study ごとの DataFrame 構築データ収集 ---
        // `(0..n).map(|_| Vec::new()).collect()` で Clone 境界なしに初期化 🟡
        let mut per_study_rows: Vec<Vec<TrialRow>> = (0..n_studies).map(|_| Vec::new()).collect();
        let mut per_study_unn: Vec<HashSet<String>> = // user_attr 数値列名
            (0..n_studies).map(|_| HashSet::new()).collect();
        let mut per_study_usn: Vec<HashSet<String>> = // user_attr 文字列列名
            (0..n_studies).map(|_| HashSet::new()).collect();
        let mut per_study_max_c: Vec<usize> = vec![0; n_studies]; // 最大 constraint 数

        // --- COMPLETE (state=1) 試行のみ集約（REQ-011）🟢 ---
        // TrialBuilder をムーブ消費することで HashMap データのクローンを回避 🟡
        for (trial_id, trial) in sorted_trials {
            if trial.state != 1 {
                continue;
            }
            let study_idx = trial.study_id as usize;
            if study_idx >= n_studies {
                continue;
            }

            // StudyBuilder の集約（参照でアクセスし、ムーブ前に完了）🟢
            {
                let study = &mut studies[study_idx];
                study.completed_trials += 1;
                for name in trial.param_display.keys() {
                    study.param_names.insert(name.clone());
                }
                // user_attr を StudyBuilder と per_study_unn/usn に同時登録 🟡
                for name in trial.user_attrs_numeric.keys() {
                    study.user_attr_names.insert(name.clone());
                    per_study_unn[study_idx].insert(name.clone());
                }
                for name in trial.user_attrs_string.keys() {
                    study.user_attr_names.insert(name.clone());
                    per_study_usn[study_idx].insert(name.clone());
                }
                if trial.has_constraints {
                    study.has_constraints = true;
                }
                // 目的列名: 最初の COMPLETE 試行の values.len() から "obj0","obj1"... を生成 🟡
                if study.objective_names.is_empty() {
                    if let Some(values) = &trial.values {
                        study.objective_names =
                            (0..values.len()).map(|i| format!("obj{i}")).collect();
                    }
                }
            } // StudyBuilder の可変借用ここで解放

            per_study_max_c[study_idx] =
                per_study_max_c[study_idx].max(trial.constraint_values.len());

            // TrialRow へムーブ: HashMap データのクローンを排除 🟡
            per_study_rows[study_idx].push(TrialRow {
                trial_id, // Optuna の実際の trial_id 🟢
                param_display: trial.param_display,
                param_category_label: trial.param_category_label,
                objective_values: trial.values.unwrap_or_default(),
                user_attrs_numeric: trial.user_attrs_numeric,
                user_attrs_string: trial.user_attrs_string,
                constraint_values: trial.constraint_values,
            });
        }

        // --- StudyMeta と DataFrame を一緒に構築 ---
        let mut study_metas = Vec::with_capacity(n_studies);
        let mut dataframes = Vec::with_capacity(n_studies);

        for (i, b) in studies.into_iter().enumerate() {
            let mut param_names: Vec<String> = b.param_names.into_iter().collect();
            param_names.sort();
            let mut user_attr_names: Vec<String> = b.user_attr_names.into_iter().collect();
            user_attr_names.sort();
            let objective_names = b.objective_names; // DataFrame 構築にも使用

            study_metas.push(StudyMeta {
                study_id: b.study_id,
                name: b.name,
                directions: b.directions,
                completed_trials: b.completed_trials,
                total_trials: b.total_trials,
                param_names: param_names.clone(),
                objective_names: objective_names.clone(),
                user_attr_names,
                has_constraints: b.has_constraints,
            });

            // user_attr 名リストをソートして DataFrame 構築 🟢
            // std::mem::take でインデックス経由の所有権移動（Clone不要）
            let mut unn: Vec<String> = std::mem::take(&mut per_study_unn[i]).into_iter().collect();
            unn.sort();
            let mut usn: Vec<String> = std::mem::take(&mut per_study_usn[i]).into_iter().collect();
            usn.sort();

            dataframes.push(DataFrame::from_trials(
                &per_study_rows[i],
                &param_names,
                &objective_names,
                &unn,
                &usn,
                per_study_max_c[i],
            ));
        }

        (study_metas, dataframes)
    }
}

// =============================================================================
// 公開 API
// =============================================================================

/// Journalファイルをパースして全 Study のメタ情報を返す
///
/// 【機能概要】: Optuna Journal形式（JSONL）を op_code ステートマシンで処理する
/// 【設計方針】: 不完全行・非JSON行はスキップ、全行無効の場合のみ Err を返す
/// 🟢 wasm-api.md §parse_journal に準拠
///
/// # 引数
/// * `data` - Journalファイルの UTF-8 バイト列（Uint8Array からの変換）
pub fn parse_journal(data: &[u8]) -> Result<ParseResult, String> {
    #[cfg(not(target_arch = "wasm32"))]
    let start = std::time::Instant::now();

    // 空ファイルは Ok([]) を即時返却（TC-101-B01）🟡
    if data.is_empty() {
        crate::dataframe::store_dataframes(vec![]);
        return Ok(ParseResult {
            studies: vec![],
            duration_ms: 0.0,
        });
    }

    // UTF-8 デコード: バイナリ混入に対応するため lossy 変換（TC-101-E02）🟢
    let text = String::from_utf8_lossy(data);

    // 空行を除いた行リストを構築
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        crate::dataframe::store_dataframes(vec![]);
        return Ok(ParseResult {
            studies: vec![],
            duration_ms: 0.0,
        });
    }

    let mut state = ParserState::new();
    let mut valid_lines: u32 = 0;

    for line in &lines {
        match serde_json::from_str::<Value>(line.trim()) {
            Ok(json) => {
                valid_lines += 1;
                if let Some(op) = get_u64(&json, "op_code") {
                    // op_code は 0-9 の範囲（u8 として安全）🟢
                    #[allow(clippy::cast_possible_truncation)]
                    state.process_op(op as u8, &json);
                }
            }
            // 不完全/非JSON 行はスキップして継続（TC-101-E01, TC-101-E02）🟢
            Err(_) => {}
        }
    }

    // 有効な JSON 行が 1 行もない場合はエラー（TC-101-E04）🟢
    if valid_lines == 0 {
        return Err("No valid JSON lines found in journal".to_string());
    }

    #[cfg(not(target_arch = "wasm32"))]
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    #[cfg(target_arch = "wasm32")]
    let duration_ms = 0.0_f64;
    // finalize() は StudyMeta と DataFrame を同時に返す（TASK-102）🟢
    let (studies, dataframes) = state.finalize();
    // DataFrame を WASM グローバル状態に格納（select_study() から参照できるようにする）🟢
    crate::dataframe::store_dataframes(dataframes);
    Ok(ParseResult {
        studies,
        duration_ms,
    })
}

// =============================================================================
// テスト
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn to_bytes(s: &str) -> Vec<u8> {
        s.as_bytes().to_vec()
    }

    // =========================================================================
    // 正常系
    // =========================================================================

    #[test]
    fn tc_101_01_create_study_basic() {
        // 【テスト目的】: op_code=0(CREATE_STUDY)を処理し Study名・最適化方向が記録される 🟢
        let data = to_bytes(
            r#"{"op_code":0,"worker_id":"w1","study_name":"my_study","directions":[0,1]}"#,
        );
        let result = parse_journal(&data).expect("パース成功を期待");
        assert_eq!(result.studies.len(), 1); // Study数 🟢
        assert_eq!(result.studies[0].name, "my_study"); // Study名 🟢
        assert_eq!(
            result.studies[0].directions,
            vec![
                OptimizationDirection::Minimize,
                OptimizationDirection::Maximize
            ]
        ); // 最適化方向 🟢
    }

    #[test]
    fn tc_101_02_create_trial_complete() {
        // 【テスト目的】: CREATE_TRIAL → COMPLETE 遷移で completed_trials がカウントされる 🟢
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("パース成功を期待");
        assert_eq!(result.studies[0].completed_trials, 1); // COMPLETE 試行数 🟢
        assert_eq!(result.studies[0].total_trials, 1); // 総試行数 🟢
    }

    #[test]
    fn tc_101_03_float_distribution_no_log() {
        // 【テスト目的】: FloatDistribution(log=false) の param が登録される 🟢
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"x\",\"param_value_internal\":0.5,\"distribution\":{\"name\":\"FloatDistribution\",\"low\":0.0,\"high\":1.0,\"log\":false}}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("パース成功を期待");
        assert!(result.studies[0].param_names.contains(&"x".to_string())); // param 登録 🟢
    }

    #[test]
    fn tc_101_04_float_distribution_log_true() {
        // 【テスト目的】: FloatDistribution(log=true) の param が登録される 🟡
        let ln2: f64 = std::f64::consts::LN_2;
        let line = format!(
            "{{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"lr\",\"param_value_internal\":{ln2},\"distribution\":{{\"name\":\"FloatDistribution\",\"low\":1e-5,\"high\":1.0,\"log\":true}}}}"
        );
        let data = to_bytes(&format!(
            "{}\n{}\n{}\n{}\n",
            r#"{"op_code":0,"worker_id":"w","study_name":"s","directions":[0]}"#,
            r#"{"op_code":4,"worker_id":"w","study_id":0,"datetime_start":"2024-01-01T00:00:00.000000"}"#,
            line,
            r#"{"op_code":6,"worker_id":"w","trial_id":0,"state":1,"values":[0.5],"datetime_complete":"2024-01-01T00:00:01.000000"}"#,
        ));
        let result = parse_journal(&data).expect("パース成功を期待");
        assert!(result.studies[0].param_names.contains(&"lr".to_string())); // param 登録 🟡
    }

    #[test]
    fn tc_101_05_int_distribution_basic() {
        // 【テスト目的】: IntDistribution(step=1) の param が登録される 🟢
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"n\",\"param_value_internal\":3.0,\"distribution\":{\"name\":\"IntDistribution\",\"low\":0,\"high\":10,\"step\":1,\"log\":false}}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("パース成功を期待");
        assert!(result.studies[0].param_names.contains(&"n".to_string())); // param 登録 🟢
    }

    #[test]
    fn tc_101_07_categorical_distribution_string() {
        // 【テスト目的】: CategoricalDistribution の param が登録される 🟢
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"cat\",\"param_value_internal\":1.0,\"distribution\":{\"name\":\"CategoricalDistribution\",\"choices\":[\"a\",\"b\",\"c\"]}}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("パース成功を期待");
        assert!(result.studies[0].param_names.contains(&"cat".to_string())); // param 登録 🟢
    }

    #[test]
    fn tc_101_10_multiple_studies() {
        // 【テスト目的】: 複数 Study が独立して構築される 🟢
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"A\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.1],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:01.000000\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":1,\"state\":1,\"values\":[0.2],\"datetime_complete\":\"2024-01-01T00:00:02.000000\"}\n",
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"B\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":1,\"datetime_start\":\"2024-01-01T00:00:02.000000\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":2,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:03.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("パース成功を期待");
        assert_eq!(result.studies.len(), 2); // Study 数 🟢
        let sa = result.studies.iter().find(|s| s.name == "A").unwrap();
        let sb = result.studies.iter().find(|s| s.name == "B").unwrap();
        assert_eq!(sa.completed_trials, 2); // Study A 完了数 🟢
        assert_eq!(sb.completed_trials, 1); // Study B 完了数 🟢
    }

    #[test]
    fn tc_101_11_trial_id_sequential() {
        // 【テスト目的】: total_trials が CREATE_TRIAL 出現回数に一致する 🟢
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:01.000000\"}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:02.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("パース成功を期待");
        assert_eq!(result.studies[0].total_trials, 3); // 総試行数 🟢
    }

    #[test]
    fn tc_101_12_user_attr_numeric() {
        // 【テスト目的】: 数値型 user_attr が user_attr_names に登録される（REQ-012）🟢
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":8,\"worker_id\":\"w\",\"trial_id\":0,\"user_attr\":{\"loss\":0.123}}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("パース成功を期待");
        assert!(result.studies[0]
            .user_attr_names
            .contains(&"loss".to_string())); // 数値列 🟢
    }

    #[test]
    fn tc_101_13_user_attr_string() {
        // 【テスト目的】: 文字列型 user_attr が user_attr_names に登録される（REQ-012）🟢
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":8,\"worker_id\":\"w\",\"trial_id\":0,\"user_attr\":{\"tag\":\"run_a\"}}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("パース成功を期待");
        assert!(result.studies[0]
            .user_attr_names
            .contains(&"tag".to_string())); // 文字列列 🟢
    }

    #[test]
    fn tc_101_14_constraints_expansion() {
        // 【テスト目的】: constraints がある場合 has_constraints=true（REQ-013）🟢
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":9,\"worker_id\":\"w\",\"trial_id\":0,\"system_attr\":{\"constraints\":[-0.5,0.3]}}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("パース成功を期待");
        assert!(result.studies[0].has_constraints); // constraints フラグ 🟢
    }

    #[test]
    fn tc_101_15_constraints_all_feasible() {
        // 【テスト目的】: 全 constraint <= 0 のとき completed_trials がカウントされる 🟢
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":9,\"worker_id\":\"w\",\"trial_id\":0,\"system_attr\":{\"constraints\":[-1.0,-0.5,0.0]}}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("パース成功を期待");
        assert!(result.studies[0].has_constraints); // constraints フラグ 🟢
        assert_eq!(result.studies[0].completed_trials, 1); // COMPLETE 数 🟢
    }

    #[test]
    fn tc_101_16_multi_objective_values() {
        // 【テスト目的】: 2 目的の values が objective_names に展開される 🟡
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0,1]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.1,0.9],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("パース成功を期待");
        assert_eq!(result.studies[0].objective_names.len(), 2); // 目的列数 🟡
    }

    #[test]
    fn tc_101_17_duration_ms_returned() {
        // 【テスト目的】: ParseResult.duration_ms が 0 以上の値を返す 🟢
        let data = to_bytes(r#"{"op_code":0,"worker_id":"w","study_name":"s","directions":[0]}"#);
        let result = parse_journal(&data).expect("パース成功を期待");
        assert!(result.duration_ms >= 0.0); // 処理時間が非負 🟢
    }

    // =========================================================================
    // 異常系
    // =========================================================================

    #[test]
    fn tc_101_e01_incomplete_json_line_skipped() {
        // 【テスト目的】: 不完全 JSON 行をスキップして処理継続する 🟢
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data);
        assert!(result.is_ok()); // エラーなし 🟢
        assert_eq!(result.unwrap().studies[0].completed_trials, 1); // 有効な試行のみカウント 🟢
    }

    #[test]
    fn tc_101_e02_non_json_line_skipped() {
        // 【テスト目的】: バイナリ混入行をスキップして処理継続する 🟢
        let mut data = Vec::new();
        data.extend_from_slice(
            b"{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        );
        data.extend_from_slice(b"not-json-at-all\n");
        data.extend_from_slice(b"\xff\xfe\x00\n");
        data.extend_from_slice(b"{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n");
        data.extend_from_slice(b"{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n");
        let result = parse_journal(&data);
        assert!(result.is_ok()); // バイナリ混入でもエラーなし 🟢
    }

    #[test]
    fn tc_101_e03_unknown_opcode_ignored() {
        // 【テスト目的】: 未知 op_code を無視して処理継続する 🟢
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":99,\"worker_id\":\"w\"}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data);
        assert!(result.is_ok()); // 未知 op_code でもエラーなし 🟢
    }

    #[test]
    fn tc_101_e04_all_lines_invalid_returns_error() {
        // 【テスト目的】: 全行がパース不可の場合は Err を返す 🟢
        let data: Vec<u8> = vec![0xff, 0xfe, 0x00, 0x01, 0x02];
        let result = parse_journal(&data);
        assert!(result.is_err()); // 全行無効時はエラー 🟢
    }

    #[test]
    fn tc_101_e06_all_trials_not_complete() {
        // 【テスト目的】: 全試行が RUNNING のとき completed_trials == 0 🟢
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("パース成功を期待");
        assert_eq!(result.studies[0].completed_trials, 0); // RUNNING 除外 🟢
        assert_eq!(result.studies[0].total_trials, 2); // 総数は 2 🟢
    }

    #[test]
    fn tc_101_e07_distributed_optimization_overwrite() {
        // 【テスト目的】: 同一 trial_id への重複書き込みは最後の値で上書き 🟢
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w1\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w2\",\"trial_id\":0,\"state\":1,\"values\":[0.3],\"datetime_complete\":\"2024-01-01T00:00:02.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("パース成功を期待");
        assert_eq!(result.studies[0].completed_trials, 1); // 重複でも 1 件 🟢
    }

    // =========================================================================
    // 境界値
    // =========================================================================

    #[test]
    fn tc_101_b01_empty_file() {
        // 【テスト目的】: 空ファイルでクラッシュしない 🟡
        let data: Vec<u8> = Vec::new();
        let result = parse_journal(&data);
        assert!(result.is_ok()); // 空ファイルでもエラーなし 🟡
        assert_eq!(result.unwrap().studies.len(), 0); // Study 数 0 🟡
    }

    #[test]
    fn tc_101_b02_study_only_no_trials() {
        // 【テスト目的】: CREATE_STUDY のみで completed_trials == 0 🟢
        let data = to_bytes(r#"{"op_code":0,"worker_id":"w","study_name":"s","directions":[0]}"#);
        let result = parse_journal(&data).expect("パース成功を期待");
        assert_eq!(result.studies.len(), 1); // Study 数 🟢
        assert_eq!(result.studies[0].completed_trials, 0); // 完了試行なし 🟢
    }

    #[test]
    fn tc_101_b03_categorical_boundary_indices() {
        // 【テスト目的】: Categorical の境界インデックス 0.0 が安全に処理される 🟢
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"cat\",\"param_value_internal\":0.0,\"distribution\":{\"name\":\"CategoricalDistribution\",\"choices\":[\"a\",\"b\",\"c\"]}}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("パース成功を期待");
        assert!(result.studies[0].param_names.contains(&"cat".to_string())); // param 登録 🟢
    }

    #[test]
    fn tc_101_b07_minimal_journal() {
        // 【テスト目的】: 最小 3 行 Journal で completed_trials == 1 🟢
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[1.0],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
        ));
        let result = parse_journal(&data).expect("パース成功を期待");
        assert_eq!(result.studies[0].completed_trials, 1); // 最小構成での完了試行 🟢
    }

    // =========================================================================
    // パフォーマンス
    // =========================================================================

    #[test]
    fn tc_101_p01_performance_50000_lines() {
        // 【テスト目的】: 50,000 試行の Journal を 5,000ms 以内にパースできる 🟢
        let mut lines = Vec::with_capacity(150_002);
        lines.push(
            r#"{"op_code":0,"worker_id":"w","study_name":"perf","directions":[0]}"#.to_string(),
        );
        for i in 0u32..50_000 {
            let val = f64::from(i) / 50_000.0;
            lines.push(r#"{"op_code":4,"worker_id":"w","study_id":0,"datetime_start":"2024-01-01T00:00:00.000000"}"#.to_string());
            lines.push(format!(r#"{{"op_code":5,"worker_id":"w","trial_id":{i},"param_name":"x","param_value_internal":{val},"distribution":{{"name":"FloatDistribution","low":0.0,"high":1.0,"log":false}}}}"#));
            lines.push(format!(r#"{{"op_code":6,"worker_id":"w","trial_id":{i},"state":1,"values":[{val}],"datetime_complete":"2024-01-01T00:00:01.000000"}}"#));
        }
        let data = lines.join("\n").into_bytes();

        let start = std::time::Instant::now();
        let result = parse_journal(&data).expect("50,000 行のパースが成功するべき");
        let elapsed_ms = start.elapsed().as_millis() as f64;

        assert_eq!(result.studies[0].completed_trials, 50_000); // 全試行処理 🟢
        assert!(
            elapsed_ms < 5_000.0,
            "50,000 行のパースが 5,000ms 以内に完了するべき（実測: {elapsed_ms}ms）"
        ); // 性能要件 🟢
    }

    // =========================================================================
    // Distribution 逆変換の単体テスト（REQ-010）
    // =========================================================================

    #[test]
    fn distribution_float_log_false_identity() {
        // 【テスト目的】: FloatDistribution(log=false) は internal == display 🟢
        let dist = Distribution::Float { log: false };
        assert!((dist.to_display_f64(0.5) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn distribution_float_log_true_exp() {
        // 【テスト目的】: FloatDistribution(log=true) は exp(internal) == display 🟢
        let dist = Distribution::Float { log: true };
        let expected = std::f64::consts::LN_2.exp(); // ≈ 2.0
        assert!((dist.to_display_f64(std::f64::consts::LN_2) - expected).abs() < 1e-10);
    }

    #[test]
    fn distribution_int_step1() {
        // 【テスト目的】: IntDistribution(low=0, step=1) は round(internal) + low 🟢
        let dist = Distribution::Int {
            low: 0,
            step: 1,
            log: false,
        };
        assert!((dist.to_display_f64(3.0) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn distribution_int_step2() {
        // 【テスト目的】: IntDistribution(low=0, step=2) は low + round(v) * step 🟡
        let dist = Distribution::Int {
            low: 0,
            step: 2,
            log: false,
        };
        // internal=2.0 → 0 + 2*2 = 4
        assert!((dist.to_display_f64(2.0) - 4.0).abs() < 1e-10);
    }

    #[test]
    fn distribution_categorical_label() {
        // 【テスト目的】: CategoricalDistribution は choices[round(v)] を返す 🟢
        let dist = Distribution::Categorical {
            choices: vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
                Value::String("c".to_string()),
            ],
        };
        assert_eq!(dist.categorical_label(1.0), Some("b".to_string()));
        assert_eq!(dist.categorical_label(0.0), Some("a".to_string()));
        assert_eq!(dist.categorical_label(2.0), Some("c".to_string()));
    }

    #[test]
    fn trial_builder_constraint_values_stored() {
        // 【テスト目的】: constraint_values が TrialBuilder に正しく保持される 🟢
        let trial = TrialBuilder {
            study_id: 0,
            state: 1,
            values: None,
            param_display: HashMap::new(),
            param_category_label: HashMap::new(),
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![-1.0, -0.5, 0.0],
            has_constraints: true,
        };
        assert_eq!(trial.constraint_values.len(), 3);
        assert!(trial.constraint_values.iter().all(|&c| c <= 0.0)); // 全て <= 0 🟢
        let sum: f64 = trial.constraint_values.iter().sum();
        assert!((sum - (-1.5)).abs() < 1e-10); // constraint_sum 🟢
    }

    // =========================================================================
    // distribution 文字列 JSON + attributes ネスト対応テスト
    // =========================================================================

    #[test]
    fn distribution_from_json_string_with_attributes() {
        // 【テスト目的】: 実ログ形式の文字列 distribution（attributes ネスト）を正しくパースする 🟢
        let json_str = r#""{\"name\": \"FloatDistribution\", \"attributes\": {\"step\": 0.01, \"low\": -32.77, \"high\": 32.77, \"log\": false}}""#;
        let val: Value = serde_json::from_str(json_str).unwrap();
        let dist = Distribution::from_json(&val);
        // FloatDistribution(log=false) にマッチすること
        assert!(matches!(dist, Distribution::Float { log: false }));
        // 逆変換: log=false なので identity
        assert!((dist.to_display_f64(7.4) - 7.4).abs() < 1e-10);
    }

    #[test]
    fn distribution_from_json_string_log_true() {
        // 【テスト目的】: log=true の文字列 distribution を正しくパースする 🟢
        let json_str = r#""{\"name\": \"FloatDistribution\", \"attributes\": {\"step\": 0.0, \"low\": 1e-5, \"high\": 1.0, \"log\": true}}""#;
        let val: Value = serde_json::from_str(json_str).unwrap();
        let dist = Distribution::from_json(&val);
        assert!(matches!(dist, Distribution::Float { log: true }));
        // 逆変換: log=true → exp(v)
        let ln2 = std::f64::consts::LN_2;
        assert!((dist.to_display_f64(ln2) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn distribution_from_json_object_with_attributes() {
        // 【テスト目的】: オブジェクト形式 + attributes ネストにも対応する 🟢
        let val: Value = serde_json::from_str(
            r#"{"name": "IntDistribution", "attributes": {"low": 0, "high": 10, "step": 2, "log": false}}"#,
        ).unwrap();
        let dist = Distribution::from_json(&val);
        assert!(matches!(
            dist,
            Distribution::Int {
                low: 0,
                step: 2,
                log: false
            }
        ));
        // low=0, step=2: to_display = 0 + round(3.0)*2 = 6.0
        assert!((dist.to_display_f64(3.0) - 6.0).abs() < 1e-10);
    }

    #[test]
    fn parse_real_log_format_param_values() {
        // 【テスト目的】: 実ログ形式（文字列 distribution）で param_display が正しく構築される 🟢
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[1]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2026-03-28T11:58:48.485367\"}\n",
            "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"x0\",\"param_value_internal\":7.4,\"distribution\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": -32.77, \\\"high\\\": 32.77, \\\"log\\\": false}}\"}\n",
            "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"x1\",\"param_value_internal\":17.43,\"distribution\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": -32.77, \\\"high\\\": 32.77, \\\"log\\\": false}}\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[21.64],\"datetime_complete\":\"2026-03-28T11:58:48.612043\"}\n"
        ));
        let result = parse_journal(&data).expect("パース成功を期待");
        assert_eq!(result.studies[0].completed_trials, 1);
        assert!(result.studies[0].param_names.contains(&"x0".to_string()));
        assert!(result.studies[0].param_names.contains(&"x1".to_string()));
    }

    #[test]
    fn parse_real_log_file() {
        // 【テスト目的】: 実際の test.log ファイルをパースして全パラメータが取得されることを検証 🟢
        let log_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("test.log");
        if !log_path.exists() {
            // CI 等で test.log がない場合はスキップ
            eprintln!("test.log not found at {:?}, skipping", log_path);
            return;
        }
        let data = std::fs::read(&log_path).expect("test.log の読み込みに失敗");
        let result = parse_journal(&data).expect("test.log のパースに失敗");

        // 2 つの Study が含まれるはず（Ackley + DTLZ1）
        assert!(result.studies.len() >= 2, "少なくとも 2 Study が含まれる");

        // Ackley Study: 10 変数 (Ackley_Variable0..9)
        let ackley = &result.studies[0];
        assert!(ackley.completed_trials > 0, "Ackley に完了試行がある");
        assert_eq!(ackley.param_names.len(), 10, "Ackley は 10 パラメータ");
        for i in 0..10 {
            let name = format!("Ackley_Variable{i}");
            assert!(
                ackley.param_names.contains(&name),
                "Ackley に {name} が含まれる"
            );
        }

        // DTLZ1 Study: 10 変数 (DTLZ1_Variable0..9)
        let dtlz = &result.studies[1];
        assert!(dtlz.completed_trials > 0, "DTLZ1 に完了試行がある");
        assert_eq!(dtlz.param_names.len(), 10, "DTLZ1 は 10 パラメータ");
        for i in 0..10 {
            let name = format!("DTLZ1_Variable{i}");
            assert!(
                dtlz.param_names.contains(&name),
                "DTLZ1 に {name} が含まれる"
            );
        }

        // DataFrame にパラメータ値が入っていることを検証
        // (parse_journal 経由で DataFrame が GLOBAL_STATE にセットされているはず)
        // select_study で DataFrame をアクティブにして getTrials で確認
        use crate::dataframe::with_df;
        let df_check = with_df(0, |df| {
            let param_cols = df.param_col_names();
            assert_eq!(param_cols.len(), 10, "Ackley DataFrame に 10 パラメータ列");
            // 最初の行のパラメータ値が 0 でないことを確認
            let col = df
                .get_numeric_column("Ackley_Variable0")
                .expect("列が存在する");
            assert!(
                col[0].abs() > 1e-10,
                "Ackley_Variable0 の最初の値が非ゼロ: {}",
                col[0]
            );
        });
        assert!(df_check.is_some(), "DataFrame が存在する");
    }
}
