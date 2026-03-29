//! 範囲クエリフィルタエンジン
//!
//! REQ-042: filter_by_ranges() は 50,000件で5ms以内
//! REQ-043: フィルタ結果として行インデックスのみ返す（positions/sizes は変更しない）
//!
//! 参照: docs/implements/TASK-103/filter-requirements.md

use std::collections::HashMap;

// =============================================================================
// 公開型定義
// =============================================================================

/// 範囲条件（min/max どちらも省略可能 = 開区間）🟢
#[derive(Debug, Clone)]
pub struct Range {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// 1試行分の再構築データ（get_trial() の戻り値）🟢
#[derive(Debug, Clone)]
pub struct TrialData {
    /// Optuna の実際の trial_id 🟢
    pub trial_id: u32,
    /// 数値パラメータ（FloatDistribution, IntDistribution）
    pub params_numeric: Vec<(String, f64)>,
    /// カテゴリパラメータ（CategoricalDistribution）
    pub params_categorical: Vec<(String, String)>,
    /// 目的関数値リスト（obj0, obj1, ...）
    pub values: Vec<f64>,
    /// フィージビリティ（constraints がない場合は None）🟡
    pub is_feasible: Option<bool>,
    /// user_attr 数値型
    pub user_attrs_numeric: Vec<(String, f64)>,
    /// user_attr 文字列型
    pub user_attrs_string: Vec<(String, String)>,
}

// =============================================================================
// 内部ロジック（テスト可能な純粋関数）
// =============================================================================

/// JSON文字列を HashMap<String, Range> にパース 🟢
///
/// 【設計】: serde_json を使って直接パースし、エラー時は空の HashMap を返す
pub fn parse_ranges(ranges_json: &str) -> HashMap<String, Range> {
    // 【JSON構造】: {"col": {"min": 2.0, "max": 8.0}, "col2": {"min": null, "max": 0.5}}
    let parsed: serde_json::Value = match serde_json::from_str(ranges_json) {
        Ok(v) => v,
        Err(_) => return HashMap::new(),
    };
    let obj = match parsed.as_object() {
        Some(o) => o,
        None => return HashMap::new(),
    };

    let mut result = HashMap::new();
    for (col_name, range_val) in obj {
        // 【各エントリ】: {"min": number_or_null, "max": number_or_null}
        let min = range_val.get("min").and_then(|v| v.as_f64());
        let max = range_val.get("max").and_then(|v| v.as_f64());
        result.insert(col_name.clone(), Range { min, max });
    }
    result
}

/// DataFrame に対して範囲条件を適用し、一致する行インデックスを返す（純粋関数）🟢
///
/// 【設計】: グローバル状態に依存しないため単体テストが容易
/// 【複雑度】: O(N×K)（N=行数, K=条件数）— REQ-042 の性能要件を満たす
pub fn filter_rows(df: &crate::dataframe::DataFrame, ranges: &HashMap<String, Range>) -> Vec<u32> {
    let n = df.row_count();
    // 【早期リターン①】: DataFrame が空 → 空配列
    if n == 0 {
        return vec![];
    }
    // 【早期リターン②】: 条件なし → 全行を返す
    if ranges.is_empty() {
        return (0..n as u32).collect();
    }

    // 【事前チェック】: 全ての列名が DataFrame に存在するか確認
    // 存在しない列名があれば空配列を返す（エラーにしない — TC-103-E01）
    for (col_name, _) in ranges {
        if df.get_numeric_column(col_name).is_none() {
            return vec![]; // 【不明な列】: 空結果
        }
    }

    // 【事前チェック】: min > max の条件があれば空配列を返す（TC-103-E02）
    for range in ranges.values() {
        if let (Some(min), Some(max)) = (range.min, range.max) {
            if min > max {
                return vec![]; // 【矛盾した範囲】: 空結果
            }
        }
    }

    // 【事前キャッシュ】: 列スライスを事前取得しループ内の Vec スキャンを排除
    // Vec::iter().find() の O(C) コストをループ外に出すことで O(N×C) → O(C) + O(N×K) に改善
    let col_ranges: Vec<(&[f64], &Range)> = ranges
        .iter()
        .map(|(name, range)| (df.get_numeric_column(name).unwrap(), range))
        .collect();

    // 【メインフィルタ】: 各行に対してすべての条件を AND 評価
    let mut result = Vec::with_capacity(n / 4); // 典型的な選択率を想定した事前確保
    'outer: for row in 0..n {
        for (col, range) in &col_ranges {
            let val = col[row];
            // 【NaN値】: 欠損値は条件を満たさない（除外）🟡
            if val.is_nan() {
                continue 'outer;
            }
            // 【下限チェック】: min が指定されている場合、val >= min であること
            if let Some(min) = range.min {
                if val < min {
                    continue 'outer;
                }
            }
            // 【上限チェック】: max が指定されている場合、val <= max であること
            if let Some(max) = range.max {
                if val > max {
                    continue 'outer;
                }
            }
        }
        result.push(row as u32);
    }
    result
}

/// DataFrame の行からTrialDataを構築する（純粋関数）🟢
pub fn build_trial_data(df: &crate::dataframe::DataFrame, row: usize) -> Option<TrialData> {
    if row >= df.row_count() {
        return None; // 【範囲外】: None を返す（TC-103-E03）
    }

    let trial_id = df.get_trial_id(row)?;

    // 【数値パラメータ】: param_col_names のうち numeric_cols に存在するもの
    let params_numeric: Vec<(String, f64)> = df
        .param_col_names()
        .iter()
        .filter_map(|name| {
            df.get_numeric_column(name)
                .map(|col| (name.clone(), col[row]))
        })
        .collect();

    // 【カテゴリパラメータ】: param_col_names のうち string_cols に存在するもの
    let params_categorical: Vec<(String, String)> = df
        .param_col_names()
        .iter()
        .filter_map(|name| {
            df.get_string_column(name)
                .map(|col| (name.clone(), col[row].clone()))
        })
        .collect();

    // 【目的値】: objective_col_names 順に収集
    let values: Vec<f64> = df
        .objective_col_names()
        .iter()
        .filter_map(|name| df.get_numeric_column(name).map(|col| col[row]))
        .collect();

    // 【フィージビリティ】: is_feasible 列が存在する場合のみ Some
    let is_feasible = df
        .get_numeric_column("is_feasible")
        .map(|col| col[row] == 1.0);

    // 【user_attr 数値】: user_attr_numeric_col_names から収集
    let user_attrs_numeric: Vec<(String, f64)> = df
        .user_attr_numeric_col_names()
        .iter()
        .filter_map(|name| {
            df.get_numeric_column(name)
                .map(|col| (name.clone(), col[row]))
        })
        .collect();

    // 【user_attr 文字列】: user_attr_string_col_names から収集
    let user_attrs_string: Vec<(String, String)> = df
        .user_attr_string_col_names()
        .iter()
        .filter_map(|name| {
            df.get_string_column(name)
                .map(|col| (name.clone(), col[row].clone()))
        })
        .collect();

    Some(TrialData {
        trial_id,
        params_numeric,
        params_categorical,
        values,
        is_feasible,
        user_attrs_numeric,
        user_attrs_string,
    })
}

// =============================================================================
// 公開 API（グローバル状態アクセス版）
// =============================================================================

/// アクティブ Study の DataFrame に対して範囲条件を適用し、一致する行インデックスを返す 🟢
///
/// 【入力】: `{"col": {"min": 2.0, "max": 8.0}, "col2": {"min": null, "max": 0.5}}`
/// 【出力】: 条件を全て満たす行の0ベースインデックス（昇順）
/// 【エラー処理】: 存在しない列名 → 空配列（パニックなし）、min>max → 空配列
pub fn filter_by_ranges(ranges_json: &str) -> Vec<u32> {
    let ranges = parse_ranges(ranges_json);
    crate::dataframe::with_active_df(|df| filter_rows(df, &ranges)).unwrap_or_default()
}

/// アクティブ Study の DataFrame から1試行分のデータを返す 🟢
///
/// 【引数】: `index` — DataFrame 内の0ベース行インデックス
/// 【戻り値】: 存在しないインデックスの場合は None
pub fn get_trial(index: u32) -> Option<TrialData> {
    crate::dataframe::with_active_df(|df| build_trial_data(df, index as usize)).flatten()
}

/// アクティブ Study の DataFrame から複数試行のデータを返す 🟢
///
/// 【引数】: `indices` — 行インデックスの配列（存在しないインデックスはスキップ）
pub fn get_trials_batch(indices: &[u32]) -> Vec<TrialData> {
    crate::dataframe::with_active_df(|df| {
        indices
            .iter()
            .filter_map(|&idx| build_trial_data(df, idx as usize))
            .collect()
    })
    .unwrap_or_default()
}

// =============================================================================
// テスト
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{select_study, store_dataframes, DataFrame, TrialRow};
    use std::collections::HashMap;

    // -------------------------------------------------------------------------
    // テスト共通ヘルパー
    // -------------------------------------------------------------------------

    /// trial_id 付きの TrialRow を生成するヘルパー
    fn make_row(trial_id: u32, params: &[(&str, f64)], obj: Vec<f64>) -> TrialRow {
        TrialRow {
            trial_id,
            param_display: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            param_category_label: HashMap::new(),
            objective_values: obj,
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        }
    }

    /// DataFrame を作成して store_dataframes + select_study するヘルパー
    fn setup_df(rows: Vec<TrialRow>, params: &[&str], objs: &[&str]) -> DataFrame {
        let param_names: Vec<String> = params.iter().map(|s| s.to_string()).collect();
        let obj_names: Vec<String> = objs.iter().map(|s| s.to_string()).collect();
        // 【設計】: DataFrame::Clone を使って1回の構築でストア用・返却用を両立
        let df = DataFrame::from_trials(&rows, &param_names, &obj_names, &[], &[], 0);
        store_dataframes(vec![df.clone()]);
        select_study(0).expect("study 0 should exist");
        df
    }

    // =========================================================================
    // 正常系
    // =========================================================================

    #[test]
    fn tc_103_01_single_range_filter() {
        // 【テスト目的】: 単変数範囲フィルタが正確な行インデックスを返す 🟢
        let rows = vec![
            make_row(0, &[("x", 1.0)], vec![0.0]),
            make_row(1, &[("x", 3.0)], vec![0.0]),
            make_row(2, &[("x", 5.0)], vec![0.0]),
            make_row(3, &[("x", 7.0)], vec![0.0]),
            make_row(4, &[("x", 9.0)], vec![0.0]),
        ];
        let df = setup_df(rows, &["x"], &["obj0"]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: Some(3.0),
                max: Some(7.0),
            },
        );

        let result = filter_rows(&df, &ranges);

        // 【確認】インデックス1,2,3（x=3,5,7）が返る
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn tc_103_02_multi_column_and_filter() {
        // 【テスト目的】: 複合ANDフィルタが積集合を返す 🟢
        let rows = vec![
            make_row(0, &[("x", 1.0)], vec![10.0]),
            make_row(1, &[("x", 3.0)], vec![20.0]),
            make_row(2, &[("x", 5.0)], vec![30.0]),
            make_row(3, &[("x", 7.0)], vec![40.0]),
        ];
        let df = setup_df(rows, &["x"], &["obj0"]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: Some(3.0),
                max: Some(7.0),
            },
        );
        ranges.insert(
            "obj0".to_string(),
            Range {
                min: Some(20.0),
                max: Some(30.0),
            },
        );

        let result = filter_rows(&df, &ranges);

        // 【確認】x=[3,5]かつobj0=[20,30] → インデックス[1,2]
        assert_eq!(result, vec![1, 2]);
    }

    #[test]
    fn tc_103_03_null_min_open_lower_bound() {
        // 【テスト目的】: min=nullで下限なし（全値が上限以下なら通過）🟢
        let rows = vec![
            make_row(0, &[("x", 1.0)], vec![]),
            make_row(1, &[("x", 5.0)], vec![]),
            make_row(2, &[("x", 9.0)], vec![]),
        ];
        let df = setup_df(rows, &["x"], &[]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: None,
                max: Some(6.0),
            },
        );

        let result = filter_rows(&df, &ranges);

        // 【確認】x=1,5が≤6.0 → インデックス[0,1]
        assert_eq!(result, vec![0, 1]);
    }

    #[test]
    fn tc_103_04_null_max_open_upper_bound() {
        // 【テスト目的】: max=nullで上限なし（全値が下限以上なら通過）🟢
        let rows = vec![
            make_row(0, &[("x", 1.0)], vec![]),
            make_row(1, &[("x", 5.0)], vec![]),
            make_row(2, &[("x", 9.0)], vec![]),
        ];
        let df = setup_df(rows, &["x"], &[]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: Some(4.0),
                max: None,
            },
        );

        let result = filter_rows(&df, &ranges);

        // 【確認】x=5,9が≥4.0 → インデックス[1,2]
        assert_eq!(result, vec![1, 2]);
    }

    #[test]
    fn tc_103_05_both_null_all_pass() {
        // 【テスト目的】: min/maxともにnullで全行通過 🟢
        let rows = vec![
            make_row(0, &[("x", 1.0)], vec![]),
            make_row(1, &[("x", 5.0)], vec![]),
            make_row(2, &[("x", 9.0)], vec![]),
        ];
        let df = setup_df(rows, &["x"], &[]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: None,
                max: None,
            },
        );

        let result = filter_rows(&df, &ranges);

        // 【確認】全行通過 → [0,1,2]
        assert_eq!(result, vec![0, 1, 2]);
    }

    #[test]
    fn tc_103_06_boundary_min_inclusive() {
        // 【テスト目的】: min境界値（値==min）の行が含まれる（閉区間）🟢
        let rows = vec![
            make_row(0, &[("x", 2.0)], vec![]),
            make_row(1, &[("x", 4.0)], vec![]),
            make_row(2, &[("x", 6.0)], vec![]),
        ];
        let df = setup_df(rows, &["x"], &[]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: Some(4.0),
                max: Some(8.0),
            },
        );

        let result = filter_rows(&df, &ranges);

        // 【確認】x=4.0はmin境界を含む → [1,2]
        assert_eq!(result, vec![1, 2]);
    }

    #[test]
    fn tc_103_07_boundary_max_inclusive() {
        // 【テスト目的】: max境界値（値==max）の行が含まれる（閉区間）🟢
        let rows = vec![
            make_row(0, &[("x", 2.0)], vec![]),
            make_row(1, &[("x", 4.0)], vec![]),
            make_row(2, &[("x", 6.0)], vec![]),
        ];
        let df = setup_df(rows, &["x"], &[]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: Some(0.0),
                max: Some(4.0),
            },
        );

        let result = filter_rows(&df, &ranges);

        // 【確認】x=4.0はmax境界を含む → [0,1]
        assert_eq!(result, vec![0, 1]);
    }

    #[test]
    fn tc_103_08_objective_column_filter() {
        // 【テスト目的】: 目的列（obj0）へのフィルタが動作する 🟢
        let rows = vec![
            make_row(0, &[], vec![0.1]),
            make_row(1, &[], vec![0.5]),
            make_row(2, &[], vec![0.9]),
        ];
        let df = setup_df(rows, &[], &["obj0"]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "obj0".to_string(),
            Range {
                min: Some(0.3),
                max: Some(0.7),
            },
        );

        let result = filter_rows(&df, &ranges);

        // 【確認】obj0=0.5のみ通過 → [1]
        assert_eq!(result, vec![1]);
    }

    #[test]
    fn tc_103_09_get_trial_returns_correct_data() {
        // 【テスト目的】: get_trial(0)が正確なtrial_idとparamsを返す 🟢
        let rows = vec![
            make_row(10, &[("x", 0.5)], vec![2.0]),
            make_row(20, &[("x", 1.5)], vec![3.0]),
        ];
        setup_df(rows, &["x"], &["obj0"]);

        let trial = get_trial(0).expect("インデックス0は存在するはず");

        // 【確認】trial_id, params_numeric, values が正確
        assert_eq!(trial.trial_id, 10); // 【確認】Optunaのtrial_id=10
        assert_eq!(trial.values, vec![2.0]); // 【確認】obj0=2.0
        let x_val = trial
            .params_numeric
            .iter()
            .find(|(k, _)| k == "x")
            .map(|(_, v)| *v)
            .expect("xパラメータが存在するはず");
        assert!((x_val - 0.5).abs() < 1e-9); // 【確認】x=0.5
    }

    #[test]
    fn tc_103_10_get_trials_batch_returns_two_trials() {
        // 【テスト目的】: get_trials_batch([0,1])が2件を返す 🟢
        let rows = vec![
            make_row(10, &[("x", 0.5)], vec![2.0]),
            make_row(20, &[("x", 1.5)], vec![3.0]),
        ];
        setup_df(rows, &["x"], &["obj0"]);

        let trials = get_trials_batch(&[0, 1]);

        // 【確認】2件返る、かつtrial_idが正確
        assert_eq!(trials.len(), 2); // 【確認】2件
        assert_eq!(trials[0].trial_id, 10); // 【確認】最初の試行
        assert_eq!(trials[1].trial_id, 20); // 【確認】2番目の試行
    }

    // =========================================================================
    // 異常系
    // =========================================================================

    #[test]
    fn tc_103_e01_unknown_column_returns_empty() {
        // 【テスト目的】: 存在しない列名で空結果（エラーなし）🟢
        let rows = vec![
            make_row(0, &[("x", 1.0)], vec![]),
            make_row(1, &[("x", 5.0)], vec![]),
        ];
        let df = setup_df(rows, &["x"], &[]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "nonexistent".to_string(),
            Range {
                min: Some(0.0),
                max: Some(10.0),
            },
        );

        let result = filter_rows(&df, &ranges);

        // 【確認】存在しない列 → 空配列（panicなし）
        assert_eq!(result, Vec::<u32>::new()); // 【確認】空結果
    }

    #[test]
    fn tc_103_e02_min_greater_than_max_returns_empty() {
        // 【テスト目的】: min > max で空結果 🟢
        let rows = vec![
            make_row(0, &[("x", 1.0)], vec![]),
            make_row(1, &[("x", 5.0)], vec![]),
            make_row(2, &[("x", 9.0)], vec![]),
        ];
        let df = setup_df(rows, &["x"], &[]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: Some(8.0),
                max: Some(2.0),
            },
        );

        let result = filter_rows(&df, &ranges);

        // 【確認】min(8.0) > max(2.0) → 空配列
        assert_eq!(result, Vec::<u32>::new()); // 【確認】空結果
    }

    #[test]
    fn tc_103_e03_get_trial_out_of_range_returns_none() {
        // 【テスト目的】: get_trial で範囲外インデックス → None 🟢
        let rows = vec![make_row(0, &[("x", 1.0)], vec![])];
        setup_df(rows, &["x"], &[]);

        let result = get_trial(99);

        // 【確認】存在しないインデックス → None（panicなし）
        assert!(result.is_none()); // 【確認】None を返す
    }

    // =========================================================================
    // 境界値
    // =========================================================================

    #[test]
    fn tc_103_b01_empty_dataframe_returns_empty() {
        // 【テスト目的】: 空DataFrameに対してフィルタ → 空結果 🟢
        let df = DataFrame::empty();
        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: Some(0.0),
                max: Some(1.0),
            },
        );

        let result = filter_rows(&df, &ranges);

        // 【確認】行数0 → 空配列
        assert_eq!(result, Vec::<u32>::new()); // 【確認】空結果
    }

    #[test]
    fn tc_103_b02_all_rows_match_returns_all() {
        // 【テスト目的】: 全行一致で全インデックスが返る 🟢
        let rows = vec![
            make_row(0, &[("x", 1.0)], vec![]),
            make_row(1, &[("x", 5.0)], vec![]),
            make_row(2, &[("x", 9.0)], vec![]),
        ];
        let df = setup_df(rows, &["x"], &[]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: Some(0.0),
                max: Some(100.0),
            },
        );

        let result = filter_rows(&df, &ranges);

        // 【確認】全3行が条件を満たす → [0,1,2]
        assert_eq!(result, vec![0, 1, 2]); // 【確認】全インデックス
    }

    #[test]
    fn tc_103_b03_no_rows_match_returns_empty() {
        // 【テスト目的】: 全行除外で空配列 🟢
        let rows = vec![
            make_row(0, &[("x", 1.0)], vec![]),
            make_row(1, &[("x", 5.0)], vec![]),
            make_row(2, &[("x", 9.0)], vec![]),
        ];
        let df = setup_df(rows, &["x"], &[]);
        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: Some(20.0),
                max: Some(30.0),
            },
        );

        let result = filter_rows(&df, &ranges);

        // 【確認】全行が範囲外 → []
        assert_eq!(result, Vec::<u32>::new()); // 【確認】空結果
    }

    // =========================================================================
    // パフォーマンス
    // =========================================================================

    #[test]
    fn tc_103_p01_performance_50000_rows_under_5ms() {
        // 【テスト目的】: 50,000行×3列フィルタが5ms以内 🟢
        let n = 50_000usize;
        let rows: Vec<TrialRow> = (0..n)
            .map(|i| {
                let x = (i % 100) as f64 / 100.0;
                let y = (i % 50) as f64 / 50.0;
                TrialRow {
                    trial_id: i as u32,
                    param_display: [("x".to_string(), x), ("y".to_string(), y)]
                        .into_iter()
                        .collect(),
                    param_category_label: HashMap::new(),
                    objective_values: vec![x + y],
                    user_attrs_numeric: HashMap::new(),
                    user_attrs_string: HashMap::new(),
                    constraint_values: vec![],
                }
            })
            .collect();
        let df = setup_df(rows, &["x", "y"], &["obj0"]);

        let mut ranges = HashMap::new();
        ranges.insert(
            "x".to_string(),
            Range {
                min: Some(0.2),
                max: Some(0.8),
            },
        );
        ranges.insert(
            "y".to_string(),
            Range {
                min: Some(0.1),
                max: Some(0.9),
            },
        );
        ranges.insert(
            "obj0".to_string(),
            Range {
                min: Some(0.3),
                max: Some(1.5),
            },
        );

        let start = std::time::Instant::now();
        let result = filter_rows(&df, &ranges);
        let elapsed = start.elapsed();

        // 【確認】5ms以内で完了
        assert!(
            elapsed.as_millis() <= 5,
            "filter_rows が {}ms かかった（期待: ≤5ms）",
            elapsed.as_millis()
        );
        assert!(
            !result.is_empty(),
            "フィルタ結果が空 — 何かの条件が間違っている可能性"
        );
    }
}
