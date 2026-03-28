//! WASMメモリ内列指向DataFrame
//!
//! REQ-005: パース完了後、全データを WASM メモリ内 DataFrame として常駐
//! REQ-014: GPU バッファ（positions, positions3d, sizes）を Float32Array で保持
//! REQ-015: フィルタ操作時は colors alpha のみ更新し positions/sizes は不変
//!
//! 参照: docs/implements/TASK-102/dataframe-requirements.md

use std::cell::RefCell;
use std::collections::HashMap;

// =============================================================================
// 試行データ転送型（journal_parser.rs → DataFrame 構築用）
// =============================================================================

/// parse_journal() → DataFrame 構築のための中間データ転送型
/// journal_parser::TrialBuilder からデータを移送して DataFrame を組み立てる 🟢
#[derive(Clone)]
pub struct TrialRow {
    /// Optuna の実際の trial_id（0ベース連番とは異なる場合がある）🟢
    pub trial_id: u32,
    /// パラメータ名 → 逆変換済み表示値（f64）
    pub param_display: HashMap<String, f64>,
    /// CategoricalDistribution の文字列ラベル（数値以外の choices の場合）
    pub param_category_label: HashMap<String, String>,
    /// 目的関数値リスト（obj0, obj1, ...）
    pub objective_values: Vec<f64>,
    /// user_attr 数値型（REQ-012）
    pub user_attrs_numeric: HashMap<String, f64>,
    /// user_attr 文字列型（REQ-012）
    pub user_attrs_string: HashMap<String, String>,
    /// constraints 値リスト（REQ-013）
    pub constraint_values: Vec<f64>,
}

// =============================================================================
// DataFrameInfo（interfaces.ts DataFrameInfo に対応）
// =============================================================================

/// DataFrame の列メタ情報（JS 側が認識するメタデータ）🟢
#[derive(Debug, Clone)]
pub struct DataFrameInfo {
    pub row_count: usize,
    pub column_names: Vec<String>,
    pub param_columns: Vec<String>,
    pub objective_columns: Vec<String>,
    pub user_attr_columns: Vec<String>,
    pub constraint_columns: Vec<String>,
    /// 派生列: is_feasible, constraint_sum（将来: pareto_rank, cluster_id）🟡
    pub derived_columns: Vec<String>,
}

// =============================================================================
// GpuBufferData（wasm-api.md select_study.gpuBufferData に対応）
// =============================================================================

/// WebGL 描画用 GPU バッファ群（Float32Array で保持）🟢
#[derive(Debug)]
pub struct GpuBufferData {
    /// 2D 散布図座標 (N×2): [obj0, obj1] または [norm_idx, obj0] 🟢
    pub positions: Vec<f32>,
    /// 3D Pareto 座標 (N×3): [obj0, obj1, obj2] など 🟢
    pub positions3d: Vec<f32>,
    /// 点サイズ (N×1): 初期値 1.0 🟢
    pub sizes: Vec<f32>,
    pub trial_count: usize,
}

// =============================================================================
// SelectStudyResult（wasm-api.md select_study 戻り値に対応）
// =============================================================================

/// `select_study()` の戻り値 🟢
#[derive(Debug)]
pub struct SelectStudyResult {
    pub data_frame_info: DataFrameInfo,
    pub gpu_buffer_data: GpuBufferData,
}

// =============================================================================
// DataFrame（列指向、WASM メモリ常駐）
// =============================================================================

/// 列指向の DataFrame
/// WASMメモリに常駐し、select_study() / filter_by_ranges() から参照される 🟢
#[derive(Clone)]
pub struct DataFrame {
    row_count: usize,
    /// 各行の実際の Optuna trial_id（get_trial() 用）🟢
    trial_ids: Vec<u32>,
    /// 数値列: Vec<(列名, 値リスト)> — Vec で挿入順を保持 🟡
    numeric_cols: Vec<(String, Vec<f64>)>,
    /// 文字列（カテゴリ）列 🟢
    string_cols: Vec<(String, Vec<String>)>,
    /// 列分類情報（DataFrameInfo 生成用）
    param_col_names: Vec<String>,
    objective_col_names: Vec<String>,
    user_attr_numeric_col_names: Vec<String>,
    user_attr_string_col_names: Vec<String>,
    constraint_col_names: Vec<String>,
    /// 派生列: is_feasible, constraint_sum 🟢
    derived_col_names: Vec<String>,
}

impl DataFrame {
    /// 空の DataFrame を作成（試行なし Study 用）🟢
    pub fn empty() -> Self {
        DataFrame {
            row_count: 0,
            trial_ids: vec![],
            numeric_cols: vec![],
            string_cols: vec![],
            param_col_names: vec![],
            objective_col_names: vec![],
            user_attr_numeric_col_names: vec![],
            user_attr_string_col_names: vec![],
            constraint_col_names: vec![],
            derived_col_names: vec![],
        }
    }

    /// TrialRow のリストから列指向 DataFrame を構築 🟢
    ///
    /// 【設計方針】: 列ファーストで構築し、行方向より列方向アクセスを高速化
    /// 【引数】:
    ///   trial_rows         - COMPLETE 試行データ（trial_id 昇順）
    ///   param_names        - パラメータ名リスト（ソート済み）
    ///   objective_names    - 目的列名リスト（"obj0", "obj1", ...）
    ///   user_attr_numeric  - user_attr 数値列名リスト（ソート済み）
    ///   user_attr_string   - user_attr 文字列列名リスト（ソート済み）
    ///   max_constraints    - 全試行中の最大 constraint 数
    pub fn from_trials(
        trial_rows: &[TrialRow],
        param_names: &[String],
        objective_names: &[String],
        user_attr_numeric_names: &[String],
        user_attr_string_names: &[String],
        max_constraints: usize,
    ) -> Self {
        let n = trial_rows.len();
        if n == 0 {
            return DataFrame::empty();
        }

        // trial_id 列を抽出（get_trial() でのID参照に使用）🟢
        let trial_ids: Vec<u32> = trial_rows.iter().map(|r| r.trial_id).collect();

        let mut numeric_cols: Vec<(String, Vec<f64>)> = Vec::new();
        let mut string_cols: Vec<(String, Vec<String>)> = Vec::new();
        let mut param_col_names = Vec::new();
        let mut objective_col_names = Vec::new();
        let mut user_attr_numeric_col_names = Vec::new();
        let mut user_attr_string_col_names = Vec::new();
        let mut constraint_col_names = Vec::new();
        let mut derived_col_names = Vec::new();

        // --- パラメータ列（param_names はソート済み）---
        // CategoricalDistribution ラベルがある列 → string_cols へ
        // それ以外（FloatDistribution, IntDistribution 等）→ numeric_cols へ 🟢
        for name in param_names {
            let has_label = trial_rows.iter().any(|r| r.param_category_label.contains_key(name));
            if has_label {
                let vals: Vec<String> = trial_rows
                    .iter()
                    .map(|r| r.param_category_label.get(name).cloned().unwrap_or_default())
                    .collect();
                string_cols.push((name.clone(), vals));
            } else {
                // 欠損値は 0.0 で補完（GPU バッファが NaN になるのを防ぐ）🟡
                let vals: Vec<f64> = trial_rows
                    .iter()
                    .map(|r| *r.param_display.get(name).unwrap_or(&0.0))
                    .collect();
                numeric_cols.push((name.clone(), vals));
            }
            param_col_names.push(name.clone());
        }

        // --- 目的列（obj0, obj1, ...）---
        // 欠損値は f64::NAN で補完（その試行に values がない場合）🟡
        for (i, name) in objective_names.iter().enumerate() {
            let vals: Vec<f64> = trial_rows
                .iter()
                .map(|r| r.objective_values.get(i).copied().unwrap_or(f64::NAN))
                .collect();
            numeric_cols.push((name.clone(), vals));
            objective_col_names.push(name.clone());
        }

        // --- user_attr 数値列 ---
        for name in user_attr_numeric_names {
            let vals: Vec<f64> = trial_rows
                .iter()
                .map(|r| *r.user_attrs_numeric.get(name).unwrap_or(&f64::NAN))
                .collect();
            numeric_cols.push((name.clone(), vals));
            user_attr_numeric_col_names.push(name.clone());
        }

        // --- user_attr 文字列列 ---
        for name in user_attr_string_names {
            let vals: Vec<String> = trial_rows
                .iter()
                .map(|r| r.user_attrs_string.get(name).cloned().unwrap_or_default())
                .collect();
            string_cols.push((name.clone(), vals));
            user_attr_string_col_names.push(name.clone());
        }

        // --- constraint 列（c1, c2, ...）+ 派生列（REQ-013）---
        if max_constraints > 0 {
            for ci in 0..max_constraints {
                let col_name = format!("c{}", ci + 1);
                // 試行の constraint 数が max_constraints より少ない場合は 0.0 で補完 🟡
                let vals: Vec<f64> = trial_rows
                    .iter()
                    .map(|r| r.constraint_values.get(ci).copied().unwrap_or(0.0))
                    .collect();
                numeric_cols.push((col_name.clone(), vals));
                constraint_col_names.push(col_name);
            }

            // is_feasible: 全 constraint ≤ 0 のとき 1.0（true），それ以外 0.0 🟢
            let is_feasible_vals: Vec<f64> = trial_rows
                .iter()
                .map(|r| {
                    if r.constraint_values.iter().all(|&c| c <= 0.0) {
                        1.0
                    } else {
                        0.0
                    }
                })
                .collect();
            numeric_cols.push(("is_feasible".to_string(), is_feasible_vals));
            derived_col_names.push("is_feasible".to_string());

            // constraint_sum: 全 constraint の合計 🟢
            let sum_vals: Vec<f64> = trial_rows
                .iter()
                .map(|r| r.constraint_values.iter().sum())
                .collect();
            numeric_cols.push(("constraint_sum".to_string(), sum_vals));
            derived_col_names.push("constraint_sum".to_string());
        }

        DataFrame {
            row_count: n,
            trial_ids,
            numeric_cols,
            string_cols,
            param_col_names,
            objective_col_names,
            user_attr_numeric_col_names,
            user_attr_string_col_names,
            constraint_col_names,
            derived_col_names,
        }
    }

    /// 指定行の trial_id を返す 🟢
    pub fn get_trial_id(&self, row: usize) -> Option<u32> {
        self.trial_ids.get(row).copied()
    }

    /// 列分類情報を外部（filter.rs 等）から参照するためのゲッター群 🟢
    pub fn param_col_names(&self) -> &[String] {
        &self.param_col_names
    }
    pub fn objective_col_names(&self) -> &[String] {
        &self.objective_col_names
    }
    pub fn user_attr_numeric_col_names(&self) -> &[String] {
        &self.user_attr_numeric_col_names
    }
    pub fn user_attr_string_col_names(&self) -> &[String] {
        &self.user_attr_string_col_names
    }
    pub fn constraint_col_names(&self) -> &[String] {
        &self.constraint_col_names
    }

    /// 行数を返す 🟢
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// 全列名リストを返す（数値列→文字列列の順）🟢
    pub fn column_names(&self) -> Vec<String> {
        let mut names: Vec<String> =
            self.numeric_cols.iter().map(|(n, _)| n.clone()).collect();
        names.extend(self.string_cols.iter().map(|(n, _)| n.clone()));
        names
    }

    /// 数値列をスライスで取得 🟢
    pub fn get_numeric_column(&self, name: &str) -> Option<&[f64]> {
        self.numeric_cols
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_slice())
    }

    /// 文字列列をスライスで取得 🟢
    pub fn get_string_column(&self, name: &str) -> Option<&[String]> {
        self.string_cols
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_slice())
    }

    /// DataFrameInfo を生成（JS 側への列メタ情報転送用）🟢
    pub fn info(&self) -> DataFrameInfo {
        // user_attr 列名は数値列 + 文字列列を結合してソート済みリストを返す 🟡
        let mut all_user_attr: Vec<String> = self.user_attr_numeric_col_names.clone();
        all_user_attr.extend(self.user_attr_string_col_names.iter().cloned());

        DataFrameInfo {
            row_count: self.row_count,
            column_names: self.column_names(),
            param_columns: self.param_col_names.clone(),
            objective_columns: self.objective_col_names.clone(),
            user_attr_columns: all_user_attr,
            constraint_columns: self.constraint_col_names.clone(),
            derived_columns: self.derived_col_names.clone(),
        }
    }

    /// GPU バッファ群を生成（positions / positions3d / sizes）🟢
    pub fn gpu_buffers(&self) -> GpuBufferData {
        let n = self.row_count;
        let positions = build_positions(self, n);
        let positions3d = build_positions3d(self, n);
        // sizes: 初期値 1.0（フィルタ操作でも変更しない — REQ-015）🟢
        let sizes = vec![1.0f32; n];
        GpuBufferData { positions, positions3d, sizes, trial_count: n }
    }
}

// =============================================================================
// GPU バッファ構築ヘルパー
// =============================================================================

/// positions (N×2) を構築
///
/// 🟢 目的列数による切り替え:
/// - ≥ 2 目的: [obj0[i], obj1[i]]
/// - 1 目的:   [i/(N-1), obj0[i]]（x 軸を正規化した試行番号）
/// - 0 目的:   zeros
fn build_positions(df: &DataFrame, n: usize) -> Vec<f32> {
    let mut positions = vec![0.0f32; n * 2];
    let obj_names = &df.objective_col_names;

    match obj_names.len() {
        0 => {} // zeros のまま
        1 => {
            // x: 正規化試行番号（[0, 1] にスケール）, y: obj0 🟢
            let obj0 = df.get_numeric_column(&obj_names[0]).unwrap_or(&[]);
            // N=1 のとき分母が 0 になるのを防ぐ 🟡
            let x_scale = if n > 1 { (n - 1) as f32 } else { 1.0 };
            for i in 0..n {
                positions[i * 2] = i as f32 / x_scale;
                positions[i * 2 + 1] =
                    obj0.get(i).copied().unwrap_or(f64::NAN) as f32;
            }
        }
        _ => {
            // x: obj0, y: obj1 🟢
            let obj0 = df.get_numeric_column(&obj_names[0]).unwrap_or(&[]);
            let obj1 = df.get_numeric_column(&obj_names[1]).unwrap_or(&[]);
            for i in 0..n {
                positions[i * 2] = obj0.get(i).copied().unwrap_or(f64::NAN) as f32;
                positions[i * 2 + 1] = obj1.get(i).copied().unwrap_or(f64::NAN) as f32;
            }
        }
    }
    positions
}

/// positions3d (N×3) を構築
///
/// 🟢 目的列数による切り替え:
/// - ≥ 3 目的: [obj0, obj1, obj2]
/// - 2 目的:   [obj0, obj1, 0.0]
/// - 1 目的:   [正規化試行番号, obj0, 0.0]
/// - 0 目的:   zeros
fn build_positions3d(df: &DataFrame, n: usize) -> Vec<f32> {
    let mut positions3d = vec![0.0f32; n * 3];
    let obj_names = &df.objective_col_names;

    if obj_names.is_empty() {
        return positions3d; // zeros
    }

    let obj0 = df.get_numeric_column(&obj_names[0]).unwrap_or(&[]);
    let obj1 = obj_names.get(1).and_then(|name| df.get_numeric_column(name));
    let obj2 = obj_names.get(2).and_then(|name| df.get_numeric_column(name));
    let x_scale = if n > 1 { (n - 1) as f32 } else { 1.0 };

    for i in 0..n {
        let (x, y, z) = match obj_names.len() {
            1 => {
                // x: 正規化試行番号, y: obj0, z: 0.0 🟢
                let x = i as f32 / x_scale;
                let y = obj0.get(i).copied().unwrap_or(f64::NAN) as f32;
                (x, y, 0.0f32)
            }
            2 => {
                // x: obj0, y: obj1, z: 0.0 🟢
                let x = obj0.get(i).copied().unwrap_or(f64::NAN) as f32;
                let y = obj1
                    .and_then(|v| v.get(i))
                    .copied()
                    .unwrap_or(f64::NAN) as f32;
                (x, y, 0.0f32)
            }
            _ => {
                // x: obj0, y: obj1, z: obj2 🟢
                let x = obj0.get(i).copied().unwrap_or(f64::NAN) as f32;
                let y = obj1
                    .and_then(|v| v.get(i))
                    .copied()
                    .unwrap_or(f64::NAN) as f32;
                let z = obj2
                    .and_then(|v| v.get(i))
                    .copied()
                    .unwrap_or(0.0) as f32;
                (x, y, z)
            }
        };
        positions3d[i * 3] = x;
        positions3d[i * 3 + 1] = y;
        positions3d[i * 3 + 2] = z;
    }
    positions3d
}

// =============================================================================
// グローバル状態（WASM メモリ常駐用）
// =============================================================================

struct GlobalState {
    dataframes: Vec<DataFrame>,
    /// select_study() で最後に選択された study_id（filter_by_ranges() が参照）🟢
    active_study_id: Option<u32>,
}

thread_local! {
    /// スレッドローカルな WASM グローバル状態
    /// Rust テストでは各テストスレッドが独立した状態を持つため、テスト間干渉がない 🟡
    static GLOBAL_STATE: RefCell<GlobalState> = RefCell::new(GlobalState {
        dataframes: vec![],
        active_study_id: None,
    });
}

/// parse_journal() 完了時に全 Study の DataFrame を保存（前回分を置き換え）🟢
///
/// 【設計方針】: 毎回全置換することでインクリメンタル更新の複雑さを回避
/// append_journal_diff() は TASK-1501 で対応
pub fn store_dataframes(dfs: Vec<DataFrame>) {
    GLOBAL_STATE.with(|state| {
        let mut s = state.borrow_mut();
        s.dataframes = dfs;
        s.active_study_id = None; // リセット
    });
}

/// アクティブ Study を切り替え、DataFrameInfo と GPU バッファを返す 🟢
///
/// 【変更】: アクティブ study_id を GlobalState に記録する（filter_by_ranges() が参照）
/// 【エラー処理】: 存在しない study_id を指定した場合は Err を返す（TC-102-E01）
pub fn select_study(study_id: u32) -> Result<SelectStudyResult, String> {
    GLOBAL_STATE.with(|state| {
        let mut s = state.borrow_mut();
        let result = s
            .dataframes
            .get(study_id as usize)
            .map(|df| SelectStudyResult {
                data_frame_info: df.info(),
                gpu_buffer_data: df.gpu_buffers(),
            })
            .ok_or_else(|| format!("study_id {} not found (total: {})", study_id, s.dataframes.len()));
        if result.is_ok() {
            s.active_study_id = Some(study_id);
        }
        result
    })
}

/// アクティブ Study の DataFrame を使ってクロージャを実行する 🟢
///
/// filter.rs 等の内部モジュールが DataFrame を参照するための安全なアクセサ
pub fn with_active_df<T, F: FnOnce(&DataFrame) -> T>(f: F) -> Option<T> {
    GLOBAL_STATE.with(|state| {
        let s = state.borrow();
        let idx = s.active_study_id? as usize;
        s.dataframes.get(idx).map(f)
    })
}

/// 指定 study_id の DataFrame を使ってクロージャを実行する（テスト用）🟡
pub fn with_df<T, F: FnOnce(&DataFrame) -> T>(study_id: u32, f: F) -> Option<T> {
    GLOBAL_STATE.with(|state| {
        let s = state.borrow();
        s.dataframes.get(study_id as usize).map(f)
    })
}

// =============================================================================
// テスト
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // テスト共通ヘルパー
    // -------------------------------------------------------------------------

    /// 単純な TrialRow を作成するヘルパー（trial_id はデフォルト 0）
    fn make_trial(params: &[(&str, f64)], objective_values: Vec<f64>) -> TrialRow {
        TrialRow {
            trial_id: 0,
            param_display: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            param_category_label: HashMap::new(),
            objective_values,
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        }
    }

    fn to_bytes(s: &str) -> Vec<u8> {
        s.as_bytes().to_vec()
    }

    // =========================================================================
    // 正常系
    // =========================================================================

    #[test]
    fn tc_102_01_row_count_single_trial() {
        // 【テスト目的】: 1件のTrialRowからDataFrameを構築してrow_count=1になる 🟢
        let rows = vec![make_trial(&[("x", 0.5)], vec![1.0])];
        let df = DataFrame::from_trials(
            &rows,
            &["x".to_string()],
            &["obj0".to_string()],
            &[],
            &[],
            0,
        );
        assert_eq!(df.row_count(), 1); // 【確認】row_count が 1
    }

    #[test]
    fn tc_102_02_param_column_values() {
        // 【テスト目的】: パラメータ列の値がTrialRow.param_displayと一致する 🟢
        let rows = vec![
            make_trial(&[("x", 0.5), ("y", 2.0)], vec![1.0]),
            make_trial(&[("x", 1.5), ("y", 3.0)], vec![2.0]),
        ];
        let param_names = vec!["x".to_string(), "y".to_string()];
        let df =
            DataFrame::from_trials(&rows, &param_names, &["obj0".to_string()], &[], &[], 0);
        let x_col = df.get_numeric_column("x").expect("x列が存在するはず");
        assert!((x_col[0] - 0.5).abs() < 1e-9); // 【確認】x[0] == 0.5
        assert!((x_col[1] - 1.5).abs() < 1e-9); // 【確認】x[1] == 1.5
    }

    #[test]
    fn tc_102_03_objective_column_values() {
        // 【テスト目的】: 目的列の値がTrialRow.objective_valuesと一致する 🟢
        let rows = vec![make_trial(&[], vec![0.1, 0.9])];
        let obj_names = vec!["obj0".to_string(), "obj1".to_string()];
        let df = DataFrame::from_trials(&rows, &[], &obj_names, &[], &[], 0);
        let obj0 = df.get_numeric_column("obj0").expect("obj0が存在するはず");
        let obj1 = df.get_numeric_column("obj1").expect("obj1が存在するはず");
        assert!((obj0[0] - 0.1).abs() < 1e-9); // 【確認】obj0[0] == 0.1
        assert!((obj1[0] - 0.9).abs() < 1e-9); // 【確認】obj1[0] == 0.9
    }

    #[test]
    fn tc_102_04_user_attr_numeric() {
        // 【テスト目的】: user_attr数値列が正確に格納される 🟢
        let mut row = make_trial(&[], vec![1.0]);
        row.user_attrs_numeric.insert("loss".to_string(), 0.123);
        let df = DataFrame::from_trials(
            &[row],
            &[],
            &["obj0".to_string()],
            &["loss".to_string()],
            &[],
            0,
        );
        let loss = df.get_numeric_column("loss").expect("loss列が存在するはず");
        assert!((loss[0] - 0.123).abs() < 1e-9); // 【確認】loss == 0.123
    }

    #[test]
    fn tc_102_05_user_attr_string() {
        // 【テスト目的】: user_attr文字列列が正確に格納される 🟢
        let mut row = make_trial(&[], vec![1.0]);
        row.user_attrs_string.insert("tag".to_string(), "run_a".to_string());
        let df = DataFrame::from_trials(
            &[row],
            &[],
            &["obj0".to_string()],
            &[],
            &["tag".to_string()],
            0,
        );
        let tag = df.get_string_column("tag").expect("tag列が存在するはず");
        assert_eq!(tag[0], "run_a"); // 【確認】tag == "run_a"
    }

    #[test]
    fn tc_102_06_constraint_columns() {
        // 【テスト目的】: constraint列（c1, c2）と派生列（is_feasible, constraint_sum）が正確 🟢
        let mut row = make_trial(&[], vec![1.0]);
        row.constraint_values = vec![-0.5, 0.3];
        let df = DataFrame::from_trials(&[row], &[], &["obj0".to_string()], &[], &[], 2);
        let c1 = df.get_numeric_column("c1").expect("c1が存在するはず");
        let c2 = df.get_numeric_column("c2").expect("c2が存在するはず");
        let is_feas = df.get_numeric_column("is_feasible").expect("is_feasibleが存在するはず");
        let csum = df.get_numeric_column("constraint_sum").expect("constraint_sumが存在するはず");
        assert!((c1[0] - (-0.5)).abs() < 1e-9); // 【確認】c1 == -0.5
        assert!((c2[0] - 0.3).abs() < 1e-9);    // 【確認】c2 == 0.3
        assert!((is_feas[0] - 0.0).abs() < 1e-9); // 【確認】0.3>0 → feasible=0（false）
        assert!((csum[0] - (-0.2)).abs() < 1e-6); // 【確認】sum == -0.2
    }

    #[test]
    fn tc_102_07_positions_buffer_size() {
        // 【テスト目的】: positionsバッファサイズがN×2になる 🟢
        let rows = vec![
            make_trial(&[], vec![1.0, 2.0]),
            make_trial(&[], vec![3.0, 4.0]),
            make_trial(&[], vec![5.0, 6.0]),
        ];
        let obj_names = vec!["obj0".to_string(), "obj1".to_string()];
        let df = DataFrame::from_trials(&rows, &[], &obj_names, &[], &[], 0);
        let gpu = df.gpu_buffers();
        assert_eq!(gpu.positions.len(), 6); // N=3, 3×2=6 🟢
        assert_eq!(gpu.trial_count, 3);     // trial_count=3 🟢
    }

    #[test]
    fn tc_102_08_positions3d_buffer_size() {
        // 【テスト目的】: positions3dバッファサイズがN×3になる 🟢
        let rows = vec![
            make_trial(&[], vec![1.0, 2.0]),
            make_trial(&[], vec![3.0, 4.0]),
            make_trial(&[], vec![5.0, 6.0]),
        ];
        let obj_names = vec!["obj0".to_string(), "obj1".to_string()];
        let df = DataFrame::from_trials(&rows, &[], &obj_names, &[], &[], 0);
        let gpu = df.gpu_buffers();
        assert_eq!(gpu.positions3d.len(), 9); // N=3, 3×3=9 🟢
    }

    #[test]
    fn tc_102_09_sizes_buffer() {
        // 【テスト目的】: sizesバッファサイズがN、全値1.0 🟢
        let rows = vec![
            make_trial(&[], vec![1.0]),
            make_trial(&[], vec![2.0]),
            make_trial(&[], vec![3.0]),
        ];
        let df = DataFrame::from_trials(&rows, &[], &["obj0".to_string()], &[], &[], 0);
        let gpu = df.gpu_buffers();
        assert_eq!(gpu.sizes.len(), 3); // N=3 🟢
        assert!(gpu.sizes.iter().all(|&s| (s - 1.0f32).abs() < 1e-6)); // 全値1.0 🟢
    }

    #[test]
    fn tc_102_10_positions_two_objectives() {
        // 【テスト目的】: 2目的StudyのpositionsはObj0×Obj1 🟢
        let rows = vec![make_trial(&[], vec![1.0, 2.0])];
        let obj_names = vec!["obj0".to_string(), "obj1".to_string()];
        let df = DataFrame::from_trials(&rows, &[], &obj_names, &[], &[], 0);
        let gpu = df.gpu_buffers();
        // positions[0]=obj0[0]=1.0, positions[1]=obj1[0]=2.0 🟢
        assert!((gpu.positions[0] - 1.0f32).abs() < 1e-6);
        assert!((gpu.positions[1] - 2.0f32).abs() < 1e-6);
    }

    #[test]
    fn tc_102_11_positions_single_objective() {
        // 【テスト目的】: 1目的のpositionsは[正規化インデックス, obj0] 🟢
        // N=3, x_scale=2.0 → idx/2.0 =[0.0, 0.5, 1.0]
        let rows = vec![
            make_trial(&[], vec![1.0]),
            make_trial(&[], vec![2.0]),
            make_trial(&[], vec![3.0]),
        ];
        let df = DataFrame::from_trials(&rows, &[], &["obj0".to_string()], &[], &[], 0);
        let gpu = df.gpu_buffers();
        // trial 0: [0.0, 1.0], trial 1: [0.5, 2.0], trial 2: [1.0, 3.0]
        assert!((gpu.positions[0] - 0.0f32).abs() < 1e-6); // idx=0 🟢
        assert!((gpu.positions[1] - 1.0f32).abs() < 1e-6); // obj=1.0 🟢
        assert!((gpu.positions[2] - 0.5f32).abs() < 1e-6); // idx=1/2 🟢
        assert!((gpu.positions[3] - 2.0f32).abs() < 1e-6); // obj=2.0 🟢
        assert!((gpu.positions[4] - 1.0f32).abs() < 1e-6); // idx=2/2 🟢
        assert!((gpu.positions[5] - 3.0f32).abs() < 1e-6); // obj=3.0 🟢
    }

    #[test]
    fn tc_102_12_dataframe_info_column_classification() {
        // 【テスト目的】: DataFrameInfoの列分類が正確 🟢
        let mut row = make_trial(&[("x", 0.5)], vec![1.0]);
        row.user_attrs_numeric.insert("loss".to_string(), 0.1);
        row.constraint_values = vec![-0.5];
        let df = DataFrame::from_trials(
            &[row],
            &["x".to_string()],
            &["obj0".to_string()],
            &["loss".to_string()],
            &[],
            1,
        );
        let info = df.info();
        assert_eq!(info.param_columns, vec!["x"]);           // 【確認】param列
        assert_eq!(info.objective_columns, vec!["obj0"]);    // 【確認】obj列
        assert_eq!(info.user_attr_columns, vec!["loss"]);    // 【確認】user_attr列
        assert_eq!(info.constraint_columns, vec!["c1"]);     // 【確認】constraint列
        assert!(info.derived_columns.contains(&"is_feasible".to_string())); // 【確認】派生列
        assert!(info.derived_columns.contains(&"constraint_sum".to_string())); // 【確認】派生列
    }

    #[test]
    fn tc_102_13_select_study_returns_result() {
        // 【テスト目的】: parse_journal後にselect_study(0)がSelectStudyResultを返す 🟢
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
            "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"x\",",
            "\"param_value_internal\":0.5,",
            "\"distribution\":{\"name\":\"FloatDistribution\",\"low\":0.0,\"high\":1.0,\"log\":false}}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5]}\n"
        ));
        crate::journal_parser::parse_journal(&data).expect("パース成功を期待");
        let result = select_study(0).expect("select_study(0)が成功するはず");
        assert!(result.data_frame_info.row_count >= 1); // 【確認】row_count >= 1
    }

    #[test]
    fn tc_102_14_select_study_multiple_studies() {
        // 【テスト目的】: 複数Study環境でselect_study(1)がStudyBを返す 🟢
        // StudyA: 3試行, StudyB: 2試行
        let data = to_bytes(concat!(
            // Study A
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"A\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[1.0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":1,\"state\":1,\"values\":[2.0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":2,\"state\":1,\"values\":[3.0]}\n",
            // Study B
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"B\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":1,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":3,\"state\":1,\"values\":[4.0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":1,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":4,\"state\":1,\"values\":[5.0]}\n"
        ));
        crate::journal_parser::parse_journal(&data).expect("パース成功を期待");
        let result_b = select_study(1).expect("StudyB取得");
        assert_eq!(result_b.data_frame_info.row_count, 2); // StudyB は2試行 🟢
    }

    // =========================================================================
    // 異常系
    // =========================================================================

    #[test]
    fn tc_102_e01_invalid_study_id_returns_err() {
        // 【テスト目的】: 存在しないstudy_idでErrが返る 🟢
        let data = to_bytes(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        );
        crate::journal_parser::parse_journal(&data).expect("パース成功を期待");
        let result = select_study(99);
        assert!(result.is_err()); // 【確認】Err が返ること
    }

    #[test]
    fn tc_102_e02_all_running_returns_empty() {
        // 【テスト目的】: 全試行RUNNINGは空のDataFrame 🟢
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":0,\"values\":null}\n"
        ));
        crate::journal_parser::parse_journal(&data).expect("パース成功を期待");
        let result = select_study(0).expect("空でも Ok を返すはず");
        assert_eq!(result.data_frame_info.row_count, 0); // 【確認】空DataFrame 🟢
    }

    // =========================================================================
    // 境界値
    // =========================================================================

    #[test]
    fn tc_102_b01_three_objectives_positions() {
        // 【テスト目的】: 3目的StudyのpositionsはObj0×Obj1、positions3dはObj0×Obj1×Obj2 🟢
        let rows = vec![make_trial(&[], vec![0.1, 0.2, 0.3])];
        let obj_names =
            vec!["obj0".to_string(), "obj1".to_string(), "obj2".to_string()];
        let df = DataFrame::from_trials(&rows, &[], &obj_names, &[], &[], 0);
        let gpu = df.gpu_buffers();
        // positions: [obj0, obj1] = [0.1, 0.2] 🟢
        assert!((gpu.positions[0] - 0.1f32).abs() < 1e-6);
        assert!((gpu.positions[1] - 0.2f32).abs() < 1e-6);
        // positions3d: [obj0, obj1, obj2] = [0.1, 0.2, 0.3] 🟢
        assert!((gpu.positions3d[0] - 0.1f32).abs() < 1e-6);
        assert!((gpu.positions3d[1] - 0.2f32).abs() < 1e-6);
        assert!((gpu.positions3d[2] - 0.3f32).abs() < 1e-6);
    }

    #[test]
    fn tc_102_b02_study_with_no_complete_trials() {
        // 【テスト目的】: CREATE_STUDYのみ（試行なし）は空DataFrameでエラーなし 🟢
        let data = to_bytes(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        );
        crate::journal_parser::parse_journal(&data).expect("パース成功を期待");
        let result = select_study(0).expect("空Studyでも Ok を返すはず");
        assert_eq!(result.data_frame_info.row_count, 0); // 【確認】0行 🟢
        assert_eq!(result.gpu_buffer_data.trial_count, 0); // 【確認】trial_count=0 🟢
    }

    // =========================================================================
    // パフォーマンス
    // =========================================================================

    #[test]
    fn tc_102_p01_performance_50000_trials() {
        // 【テスト目的】: 50,000試行でselect_study < 100ms 🟢
        use std::time::Instant;

        // 50,000試行のJournal生成
        let mut lines = Vec::with_capacity(100_001);
        lines.push(
            r#"{"op_code":0,"worker_id":"w","study_name":"perf","directions":[0]}"#.to_string(),
        );
        for i in 0u32..50_000 {
            lines.push(
                "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}".to_string()
            );
            lines.push(format!(
                "{{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":{i},\"state\":1,\"values\":[{v}]}}",
                v = (i as f64) * 0.001
            ));
        }
        let data = lines.join("\n").into_bytes();

        crate::journal_parser::parse_journal(&data).expect("パース成功を期待");

        let start = Instant::now();
        let result = select_study(0).expect("select_study 成功を期待");
        let elapsed_ms = start.elapsed().as_millis();

        assert_eq!(result.data_frame_info.row_count, 50_000); // 【確認】50000行
        assert!(elapsed_ms < 100, "select_study が 100ms を超過: {}ms", elapsed_ms); // 🟢
    }
}
