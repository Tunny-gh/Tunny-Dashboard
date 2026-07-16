use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tunny_core::dataframe::DataFrame;

use crate::ui::widgets::range_math::value_range;

// ============================================================
// 基本型定義
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Direction {
    Minimize,
    Maximize,
}

#[derive(Debug, Clone)]
pub struct StudyMeta {
    pub study_id: u32,
    pub name: String,
    pub directions: Vec<Direction>,
    pub completed_trials: usize,
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    /// パラメータごとの宣言レンジ (low, high)（表示単位、数値パラメータのみ）。
    /// log 由来の探索空間範囲。サロゲート最適化の探索箱に使う。空 = 範囲不明。
    pub param_bounds: HashMap<String, (f64, f64)>,
}

/// CSV インポート確認ダイアログの編集状態。
///
/// フラット CSV には最適化方向（最大化/最小化）や変数の宣言レンジが含まれないため、
/// 読み込み直後にこのダイアログでユーザーが確認・修正できるようにする。
/// `AppState::csv_import_settings` が `Some` のときダイアログを表示する。
#[derive(Debug, Clone)]
pub struct CsvImportSettings {
    /// 対象 Study（CSV は単一 Study なので常に 1 件）。
    pub study_id: u32,
    pub study_name: String,
    /// 目的名（`maximize` と同順・同長）。
    pub objective_names: Vec<String>,
    /// 目的ごとの最大化フラグ（true=Maximize, false=Minimize）。
    pub maximize: Vec<bool>,
    /// 数値パラメータのレンジ編集（パラメータ名昇順）。
    pub param_bounds: Vec<ParamBoundEdit>,
}

/// 数値パラメータ 1 件のレンジ編集行。
#[derive(Debug, Clone)]
pub struct ParamBoundEdit {
    pub name: String,
    pub low: f64,
    pub high: f64,
}

impl CsvImportSettings {
    /// パース直後の `StudyMeta`（方向は既定 Minimize、レンジは観測 min/max）から
    /// 編集状態を構築する。
    pub fn from_meta(meta: &StudyMeta) -> Self {
        let maximize: Vec<bool> = meta
            .directions
            .iter()
            .map(|d| matches!(d, Direction::Maximize))
            .collect();
        let mut param_bounds: Vec<ParamBoundEdit> = meta
            .param_bounds
            .iter()
            .map(|(name, &(low, high))| ParamBoundEdit {
                name: name.clone(),
                low,
                high,
            })
            .collect();
        param_bounds.sort_by(|a, b| a.name.cmp(&b.name));
        Self {
            study_id: meta.study_id,
            study_name: meta.name.clone(),
            objective_names: meta.objective_names.clone(),
            maximize,
            param_bounds,
        }
    }

    /// 編集値を `meta` へ反映する（`directions` と `param_bounds` を上書き）。
    pub fn apply_to(&self, meta: &mut StudyMeta) {
        meta.directions = self
            .maximize
            .iter()
            .map(|&m| {
                if m {
                    Direction::Maximize
                } else {
                    Direction::Minimize
                }
            })
            .collect();
        for pb in &self.param_bounds {
            meta.param_bounds.insert(pb.name.clone(), (pb.low, pb.high));
        }
    }

    /// 全レンジが有効（min < max かつ有限）か。無効なら読み込みを抑止する。
    pub fn bounds_valid(&self) -> bool {
        self.param_bounds
            .iter()
            .all(|p| p.low.is_finite() && p.high.is_finite() && p.low < p.high)
    }
}

/// テスト用の行指向フィクスチャ（旧表現、MEM-001 で列指向 `StudyView` に置換済み）。
#[cfg(test)]
#[derive(Debug, Clone, Default)]
pub struct TrialRow {
    pub trial_id: u32,
    /// Study内での0始まり連番（表示用）
    pub trial_number: u32,
    pub params: HashMap<String, f64>,
    pub objectives: Vec<f64>,
    pub pareto_rank: u32,
    pub cluster_id: Option<i32>,
    pub user_attrs: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct StudyContext {
    pub meta: StudyMeta,
    /// 列指向データの軽量ビュー（旧 `trial_rows: Vec<TrialRow>` を置換、MEM-001）。
    pub view: StudyView,
    pub pareto_indices: Vec<u32>,
}

impl StudyContext {
    /// 試行数（旧 `trial_rows.len()` 相当）。列ビューから取得し複製しない。
    pub fn trial_count(&self) -> usize {
        self.view.row_count()
    }

    /// パラメータのデータ範囲 [min, max] を返す（データがない場合は [0.0, 1.0]）
    pub fn param_range(&self, param_name: &str) -> (f64, f64) {
        let Some(values) = self.view.numeric_column(param_name) else {
            return (0.0, 1.0);
        };
        if values.is_empty() {
            return (0.0, 1.0);
        }
        // `values` は直前の空チェックにより非空であることが保証されている。
        let (min, max) = value_range(values.iter().cloned()).unwrap();
        if (max - min).abs() < f64::EPSILON {
            (min - 0.5, max + 0.5)
        } else {
            (min, max)
        }
    }

    /// テスト用: 既存 StudyContext の行データを差し替える（view/pareto_indices を再構築）。
    #[cfg(test)]
    pub(crate) fn set_rows_for_test(&mut self, rows: Vec<TrialRow>) {
        let rebuilt = StudyContext::from_rows_for_test(self.meta.clone(), rows);
        self.view = rebuilt.view;
        self.pareto_indices = rebuilt.pareto_indices;
    }

    /// テスト用: egui `TrialRow` の Vec から StudyContext を構築する。
    /// 列名は行データから導出し、DataFrame→StudyView を組み立てる。
    /// pareto_rank / cluster_id は行から並行配列へ引き継ぐ。
    #[cfg(test)]
    pub(crate) fn from_rows_for_test(meta: StudyMeta, rows: Vec<TrialRow>) -> Self {
        use tunny_core::dataframe::TrialRow as CoreRow;

        let mut param_set = std::collections::BTreeSet::new();
        for r in &rows {
            for k in r.params.keys() {
                param_set.insert(k.clone());
            }
        }
        let param_names: Vec<String> = param_set.into_iter().collect();
        // meta.objective_names を優先して DataFrame 列名と一致させる。
        // meta が空のときは行データから auto 生成する（後方互換）。
        let n_obj = rows.iter().map(|r| r.objectives.len()).max().unwrap_or(0);
        let obj_names: Vec<String> = if !meta.objective_names.is_empty() {
            meta.objective_names.clone()
        } else {
            (0..n_obj).map(|i| format!("obj{i}")).collect()
        };

        let core_rows: Vec<CoreRow> = rows
            .iter()
            .map(|r| CoreRow {
                trial_id: r.trial_id,
                trial_number: r.trial_number,
                param_display: r.params.clone(),
                param_category_label: HashMap::new(),
                objective_values: r.objectives.clone(),
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(&core_rows, &param_names, &obj_names, &[], &[], 0);
        let pareto_rank: Vec<u32> = rows.iter().map(|r| r.pareto_rank).collect();
        let mut view = StudyView::new(Arc::new(df), pareto_rank);
        for (i, r) in rows.iter().enumerate() {
            view.cluster_id[i] = r.cluster_id;
        }
        StudyContext {
            meta,
            view,
            pareto_indices: vec![],
        }
    }
}

// ============================================================
// TASK-2331: StudyView — 列指向 DataFrame スナップショットの軽量ビュー
//
// `Arc<DataFrame>` をラップし、DataFrame にないアプリ層算出値（pareto_rank /
// cluster_id / trial_ids）を並行配列で保持する。行指向 `Vec<TrialRow>`
// と per-row HashMap を永続保持しないため、列データの複製が発生しない（MEM-001）。
// 段階移行のため一時的な互換ヘルパー `row_at` / `to_trial_rows` を提供する
// （最終的に TASK-2342 で除去予定）。
// ============================================================

#[derive(Clone, Debug)]
pub struct StudyView {
    /// 共有ストアから取得した列データの不変スナップショット。
    pub df: Arc<DataFrame>,
    /// 行 index → trial_id。
    pub trial_ids: Vec<u32>,
    /// Pareto ランク（行 index 順、アプリ層算出）。
    pub pareto_rank: Vec<u32>,
    /// クラスタ ID（行 index 順、未割当は None）。
    pub cluster_id: Vec<Option<i32>>,
}

impl StudyView {
    /// `Arc<DataFrame>` と Pareto ランクから StudyView を構築する。
    /// pareto_rank の長さが row_count と不一致の場合は 0 埋めする。
    pub fn new(df: Arc<DataFrame>, pareto_rank: Vec<u32>) -> Self {
        let n = df.row_count();
        let trial_ids: Vec<u32> = (0..n)
            .map(|r| df.get_trial_id(r).unwrap_or(r as u32))
            .collect();
        let pareto_rank = if pareto_rank.len() == n {
            pareto_rank
        } else {
            vec![0; n]
        };
        StudyView {
            df,
            trial_ids,
            pareto_rank,
            cluster_id: vec![None; n],
        }
    }

    /// 行数（= DataFrame.row_count）。
    pub fn row_count(&self) -> usize {
        self.df.row_count()
    }

    /// 数値列の借用スライス（行ごとの HashMap を作らない）。
    pub fn numeric_column(&self, name: &str) -> Option<&[f64]> {
        self.df.get_numeric_column(name)
    }

    /// 実行可能性ビュー。`is_feasible` 列の有無・閾値・
    /// 「列なし = 全行実行可能」のフォールバック判定を一元化する。
    pub fn feasibility(&self) -> tunny_core::dataframe::Feasibility<'_> {
        self.df.feasibility()
    }

    /// 複数の列名をまとめて借用スライスへ解決する（None は欠損列）。
    pub fn numeric_columns(&self, names: &[String]) -> Vec<Option<&[f64]>> {
        names.iter().map(|name| self.numeric_column(name)).collect()
    }

    /// パラメータ列名。
    pub fn param_names(&self) -> &[String] {
        self.df.param_col_names()
    }

    /// 目的列名。
    pub fn objective_names(&self) -> &[String] {
        self.df.objective_col_names()
    }

    /// 互換シム: 列 + 並行配列から一時的に `TrialRow` を組み立てる（テストのみで使用）。
    #[cfg(test)]
    pub(crate) fn row_at(&self, index: usize) -> TrialRow {
        let mut params = HashMap::with_capacity(self.df.param_col_names().len());
        for name in self.df.param_col_names() {
            if let Some(col) = self.df.get_numeric_column(name) {
                if let Some(v) = col.get(index) {
                    params.insert(name.clone(), *v);
                }
            }
        }
        let objectives: Vec<f64> = self
            .df
            .objective_col_names()
            .iter()
            .map(|name| {
                self.df
                    .get_numeric_column(name)
                    .and_then(|c| c.get(index).copied())
                    .unwrap_or(0.0)
            })
            .collect();
        TrialRow {
            trial_id: self.trial_ids.get(index).copied().unwrap_or(index as u32),
            trial_number: index as u32,
            params,
            objectives,
            pareto_rank: self.pareto_rank.get(index).copied().unwrap_or(0),
            cluster_id: self.cluster_id.get(index).copied().flatten(),
            user_attrs: HashMap::new(),
        }
    }

    /// 全行を `TrialRow` として組み立てる（テストのみで使用）。
    #[cfg(test)]
    pub(crate) fn to_trial_rows(&self) -> Vec<TrialRow> {
        (0..self.row_count()).map(|i| self.row_at(i)).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ColormapName {
    Viridis,
    Plasma,
    Jet,
    Turbo,
    Inferno,
    Coolwarm,
    Spectral,
    Cividis,
    BlueYellow,
}

impl ColormapName {
    pub fn label(&self) -> &str {
        match self {
            Self::Viridis => "Viridis",
            Self::Plasma => "Plasma",
            Self::Jet => "Jet",
            Self::Turbo => "Turbo",
            Self::Inferno => "Inferno",
            Self::Coolwarm => "Coolwarm",
            Self::Spectral => "Spectral",
            Self::Cividis => "Cividis",
            Self::BlueYellow => "Blue-Yellow",
        }
    }

    pub fn all() -> &'static [ColormapName] {
        &[
            Self::Viridis,
            Self::Plasma,
            Self::Jet,
            Self::Turbo,
            Self::Inferno,
            Self::Coolwarm,
            Self::Spectral,
            Self::Cividis,
            Self::BlueYellow,
        ]
    }
}

// ============================================================
// .ghx D&D → 最適化設定モーダル / バックグラウンド実行状態
// ============================================================

/// .ghx D&D で開く最適化設定ダイアログの状態。
///
/// `AppState::gh_opt_dialog` が `Some` の間、`ghx_opt_modal` がこの状態を編集しながら
/// 表示する。[`GhxOptAction::Run`](crate::ui::widgets::common::ghx_opt_modal::GhxOptAction)
/// を受け取った側（app.rs）がここから `GhRunConfig` / `ComputeConfig` を組み立てる。
#[derive(Debug, Clone)]
pub struct GhOptDialogState {
    /// D&D / Open で選ばれた .ghx ファイルの絶対パス。
    pub ghx_path: PathBuf,
    /// 元の .ghx XML 全文（Run 時に `build_compute_definition` へそのまま渡す）。
    pub ghx_text: String,
    /// `extract_problem` の抽出結果（変数・目的・警告）。
    pub problem: tunny_core::gh::GhProblem,
    /// 目的ごとの最大化フラグ（`problem.objectives` と同順・同長、既定 false = Minimize）。
    pub maximize: Vec<bool>,
    /// journal 内で一意にするための study 名（既定: "<ghxのstem>-<unix秒下6桁>"）。
    pub study_name: String,
    /// 出力先 journal パス（既定: ghx と同じディレクトリの "<stem>_optuna.log"）。
    pub journal_path: String,
    /// Rhino.Compute サーバー URL（既定 "http://localhost:6500"）。
    pub server_url: String,
    /// Rhino.Compute の API key（空文字なら `ComputeConfig.api_key = None` として扱う）。
    pub api_key: String,
    /// 同時リクエスト数の上限（既定 4、1..=16）。
    pub max_parallel: usize,
    /// true = Random サンプラー、false = NSGA-II（既定）。
    pub sampler_is_random: bool,
    /// Random サンプラーの試行数（既定 50）。
    pub n_trials: usize,
    /// NSGA-II の個体数（既定 16）。
    pub population_size: usize,
    /// NSGA-II の世代数（既定 10）。
    pub generations: usize,
    /// 乱数シード（既定 42）。
    pub seed: u64,
    /// Run 失敗時（`build_compute_definition` / `prepare_gh_run` のエラー）の表示用。
    pub error: Option<String>,
}

impl GhOptDialogState {
    /// .ghx 抽出直後の既定値でダイアログ状態を構築する。
    pub fn new(ghx_path: PathBuf, ghx_text: String, problem: tunny_core::gh::GhProblem) -> Self {
        let stem = ghx_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("gh_opt")
            .to_string();
        let secs_suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() % 1_000_000)
            .unwrap_or(0);
        let study_name = format!("{stem}-{secs_suffix:06}");
        let journal_path = ghx_path
            .parent()
            .map(|dir| dir.join(format!("{stem}_optuna.log")))
            .unwrap_or_else(|| PathBuf::from(format!("{stem}_optuna.log")))
            .to_string_lossy()
            .into_owned();
        let maximize = vec![false; problem.objectives.len()];
        Self {
            ghx_path,
            ghx_text,
            problem,
            maximize,
            study_name,
            journal_path,
            server_url: "http://localhost:6500".to_string(),
            api_key: String::new(),
            max_parallel: 4,
            sampler_is_random: false,
            n_trials: 50,
            population_size: 16,
            generations: 10,
            seed: 42,
            error: None,
        }
    }
}

/// 実行中の .ghx 最適化の状態（非モーダルの進捗オーバーレイ用）。
///
/// `AppState::gh_opt_run` が `Some` の間、進捗オーバーレイ（app.rs）が表示される。
/// `progress` は `run_prepared` を走らせるバックグラウンドスレッドと共有するハンドルで、
/// `snapshot()` で進捗を読み取り `request_cancel()` でキャンセルを要求する。
/// `FitProgress` が `Debug` を実装していないため `#[derive(Debug)]` はできない
/// （`SurrogateOptState::fit_progress` など既存の進捗保持フィールドと同じ制約）。
/// `AppState` は `#[derive(Debug)]` のため、`progress` をプレースホルダに置き換えた
/// 手動実装を用意する。
#[derive(Clone)]
pub struct GhOptRunState {
    pub progress: tunny_core::surrogate_opt::FitProgress,
    pub journal_path: PathBuf,
    pub study_name: String,
    /// `None` = 実行中。`Some` = 終了（成功メッセージ or エラー文字列）。
    pub finished: Option<Result<String, String>>,
}

impl std::fmt::Debug for GhOptRunState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GhOptRunState")
            .field("progress", &"FitProgress { .. }")
            .field("journal_path", &self.journal_path)
            .field("study_name", &self.study_name)
            .field("finished", &self.finished)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colormap_name_all_has_nine_variants() {
        assert_eq!(ColormapName::all().len(), 9);
    }

    #[test]
    fn colormap_name_labels_not_empty() {
        for cmap in ColormapName::all() {
            assert!(!cmap.label().is_empty(), "{:?} has empty label", cmap);
        }
    }

    // ── GhOptDialogState::new のデフォルト導出 ──────────────────
    fn make_gh_problem(n_objectives: usize) -> tunny_core::gh::GhProblem {
        tunny_core::gh::GhProblem {
            variables: vec![],
            objectives: (0..n_objectives)
                .map(|i| tunny_core::gh::GhObjective {
                    source_guid: format!("guid-{i}"),
                    name: format!("f{i}"),
                })
                .collect(),
            tunny_component: "Tunny".to_string(),
            warnings: vec![],
        }
    }

    #[test]
    fn gh_opt_dialog_state_derives_defaults_from_path() {
        let path = PathBuf::from("/tmp/some_dir/model.ghx");
        let state = GhOptDialogState::new(path.clone(), "<xml/>".to_string(), make_gh_problem(2));

        assert_eq!(state.ghx_path, path);
        assert_eq!(state.ghx_text, "<xml/>");
        assert_eq!(state.maximize, vec![false, false]);
        // study_name: "<stem>-<unix秒下6桁>"（6 桁ゼロ埋め）
        assert!(
            state.study_name.starts_with("model-"),
            "study_name: {}",
            state.study_name
        );
        assert_eq!(state.study_name.len(), "model-".len() + 6);
        // journal_path: ghx と同じディレクトリの "<stem>_optuna.log"
        assert_eq!(state.journal_path, "/tmp/some_dir/model_optuna.log");
        assert_eq!(state.server_url, "http://localhost:6500");
        assert_eq!(state.api_key, "");
        assert_eq!(state.max_parallel, 4);
        assert!(!state.sampler_is_random);
        assert_eq!(state.n_trials, 50);
        assert_eq!(state.population_size, 16);
        assert_eq!(state.generations, 10);
        assert_eq!(state.seed, 42);
        assert!(state.error.is_none());
    }

    #[test]
    fn gh_opt_dialog_state_maximize_matches_objective_count() {
        let path = PathBuf::from("model.ghx");
        let state = GhOptDialogState::new(path, String::new(), make_gh_problem(3));
        assert_eq!(state.maximize.len(), 3);
        assert!(state.maximize.iter().all(|&m| !m));
    }

    // ── TASK-2331: StudyView テスト ──────────────────────────────
    fn make_study_view(n: usize) -> StudyView {
        use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};
        let core_rows: Vec<CoreRow> = (0..n)
            .map(|i| CoreRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display: HashMap::from([("x".to_string(), i as f64 * 0.1)]),
                param_category_label: HashMap::new(),
                objective_values: vec![i as f64, i as f64 * 2.0],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(
            &core_rows,
            &["x".to_string()],
            &["obj0".to_string(), "obj1".to_string()],
            &[],
            &[],
            0,
        );
        StudyView::new(std::sync::Arc::new(df), vec![0; n])
    }

    #[test]
    fn study_view_row_count_and_columns() {
        let view = make_study_view(3);
        assert_eq!(view.row_count(), 3);
        assert_eq!(view.numeric_column("x").map(|c| c.len()), Some(3));
        assert!(view.numeric_column("missing").is_none());
        assert_eq!(view.param_names(), &["x".to_string()]);
    }

    #[test]
    fn study_view_row_at_matches_columnar_values() {
        let view = make_study_view(3);
        let row = view.row_at(2);
        assert_eq!(row.trial_id, 2);
        assert_eq!(row.trial_number, 2);
        assert!((row.params["x"] - 0.2).abs() < 1e-9);
        assert_eq!(row.objectives, vec![2.0, 4.0]);
        assert_eq!(row.pareto_rank, 0);
        assert_eq!(row.cluster_id, None);
        assert!(row.user_attrs.is_empty());
    }

    #[test]
    fn study_view_to_trial_rows_roundtrip() {
        let view = make_study_view(4);
        let rows = view.to_trial_rows();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].trial_id, 0);
        assert_eq!(rows[3].objectives, vec![3.0, 6.0]);
    }

    #[test]
    fn study_view_new_pads_mismatched_pareto_rank() {
        use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};
        let core_rows: Vec<CoreRow> = (0..2)
            .map(|i| CoreRow {
                trial_id: i,
                trial_number: i,
                param_display: HashMap::new(),
                param_category_label: HashMap::new(),
                objective_values: vec![i as f64],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(&core_rows, &[], &["obj0".to_string()], &[], &[], 0);
        // pareto_rank の長さ(1) != row_count(2) → 0 埋め
        let view = StudyView::new(std::sync::Arc::new(df), vec![5]);
        assert_eq!(view.pareto_rank, vec![0, 0]);
    }
}
