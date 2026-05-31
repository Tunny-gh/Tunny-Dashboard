//! TASK-2259 Redフェーズ: SpearmanMetric・RidgeMetric の SensitivityMetric トレイト実装テスト
//!
//! テストケース定義: docs/implements/rust-core-refactoring/TASK-2259/spearman-ridge-metric-impl-testcases.md
//! 要件定義: docs/implements/rust-core-refactoring/TASK-2259/spearman-ridge-metric-impl-requirements.md

use super::compute_sensitivity_single_obj;
use super::metric_trait::SensitivityMetric;
use super::ridge::RidgeMetric;
use super::spearman::SpearmanMetric;
use crate::dataframe::{select_study, store_dataframes, DataFrame, TrialRow};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// テストユーティリティ
// ---------------------------------------------------------------------------

fn make_row_multi(trial_id: u32, params: &[(&str, f64)], objectives: Vec<f64>) -> TrialRow {
    TrialRow {
        trial_id,
        param_display: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        param_category_label: HashMap::new(),
        objective_values: objectives,
        user_attrs_numeric: HashMap::new(),
        user_attrs_string: HashMap::new(),
        constraint_values: vec![],
    }
}

fn setup_df(rows: Vec<TrialRow>, params: &[&str], objs: &[&str]) -> DataFrame {
    let param_names: Vec<String> = params.iter().map(|s| s.to_string()).collect();
    let obj_names: Vec<String> = objs.iter().map(|s| s.to_string()).collect();
    let df = DataFrame::from_trials(&rows, &param_names, &obj_names, &[], &[], 0);
    store_dataframes(vec![df.clone()]);
    select_study(0).expect("study 0 exists");
    df
}

// ===========================================================================
// 正常系テストケース
// ===========================================================================

#[test]
fn tc_2259_01_spearman_metric_name() {
    // 【テスト目的】: SpearmanMetric::name() が正しい文字列 "Spearman" を返すことを確認
    // 【テスト内容】: SpearmanMetric インスタンスを生成し name() を呼び出す
    // 【期待される動作】: "Spearman" という &'static str が返る
    // 🔵 信頼性レベル: 青（要件定義書セクション2、interfaces.rs で明記）

    // 【テストデータ準備】: ゼロサイズ構造体のインスタンス化確認
    // 【初期条件設定】: 特別な前提条件なし
    let metric = SpearmanMetric;

    // 【実際の処理実行】: name() メソッドの呼び出し
    // 【処理内容】: トレイトメソッド name() の戻り値確認
    let name = metric.name();

    // 【結果検証】: 戻り値が "Spearman" と完全一致すること
    // 【期待値確認】: 要件定義書セクション2 name() 戻り値テーブル
    assert_eq!(name, "Spearman"); // 【確認内容】: 正確な文字列が返る 🔵
}

#[test]
fn tc_2259_02_ridge_metric_name() {
    // 【テスト目的】: RidgeMetric::name() が正しい文字列 "Ridge" を返すことを確認
    // 【テスト内容】: RidgeMetric インスタンスを生成し name() を呼び出す
    // 【期待される動作】: "Ridge" という &'static str が返る
    // 🔵 信頼性レベル: 青（要件定義書セクション2、interfaces.rs で明記）

    // 【テストデータ準備】: ゼロサイズ構造体のインスタンス化確認
    // 【初期条件設定】: 特別な前提条件なし
    let metric = RidgeMetric;

    // 【実際の処理実行】: name() メソッドの呼び出し
    // 【処理内容】: トレイトメソッド name() の戻り値確認
    let name = metric.name();

    // 【結果検証】: 戻り値が "Ridge" と完全一致すること
    // 【期待値確認】: 要件定義書セクション2 name() 戻り値テーブル
    assert_eq!(name, "Ridge"); // 【確認内容】: 正確な文字列が返る 🔵
}

#[test]
fn tc_2259_03_spearman_positive_correlation() {
    // 【テスト目的】: SpearmanMetric::compute() が正の相関データで正しい SensitivityResult を返す
    // 【テスト内容】: 20行・2パラメータの DataFrame で obj_idx=0 のとき Spearman 感度値が正しく計算される
    // 【期待される動作】: x1-obj0 は正相関(>0.99)、x2-obj0 は負相関(<-0.99)、他フィールドは空/None
    // 🔵 信頼性レベル: 青（要件定義書セクション2 SensitivityResult 定義、full.rs L57-76）

    // 【テストデータ準備】: tc_801_11 と同一パターン。x1=i, x2=20-i, y=i の完全相関データ
    // 【初期条件設定】: 20行・2パラメータ・1目的関数の DataFrame
    let rows: Vec<TrialRow> = (0..20)
        .map(|i| {
            make_row_multi(
                i,
                &[("x1", i as f64), ("x2", (20 - i) as f64)],
                vec![i as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["x1", "x2"], &["obj0"]);

    // 【実際の処理実行】: SpearmanMetric::compute() を呼び出す
    // 【処理内容】: トレイトメソッド compute() による Spearman 感度計算
    let metric = SpearmanMetric;
    let result = metric.compute(&df, 0);

    // 【結果検証】: SensitivityResult の全フィールドを検証
    // 【期待値確認】: 要件定義書セクション2「SensitivityResult の内容（SpearmanMetric）」
    assert!(
        result.is_some(),
        "SpearmanMetric::compute() should return Some"
    ); // 【確認内容】: 計算が正常に完了し Some が返る 🔵
    let r = result.unwrap();
    assert_eq!(r.param_names, vec!["x1", "x2"]); // 【確認内容】: パラメータ名が正しく設定される 🔵
    assert_eq!(r.objective_names, vec!["obj0"]); // 【確認内容】: 目的関数名が1要素のみ 🔵
    assert_eq!(r.spearman.len(), 2); // 【確認内容】: パラメータ数分の感度値が設定される 🔵
    assert!(
        r.spearman[0][0] > 0.99,
        "x1-obj0 should be positively correlated: {}",
        r.spearman[0][0]
    ); // 【確認内容】: x1-obj0 正相関 🔵
    assert!(
        r.spearman[1][0] < -0.99,
        "x2-obj0 should be negatively correlated: {}",
        r.spearman[1][0]
    ); // 【確認内容】: x2-obj0 負相関 🔵
    assert!(r.ridge.is_empty()); // 【確認内容】: Ridge フィールドは空 🔵
    assert!(r.rf_anova.is_none()); // 【確認内容】: rf_anova は None 🔵
    assert!(r.mdi.is_none()); // 【確認内容】: mdi は None 🔵
    assert!(r.shap.is_none()); // 【確認内容】: shap は None 🔵
    assert!(r.permutation.is_none()); // 【確認内容】: permutation は None 🔵
}

#[test]
fn tc_2259_04_ridge_linear_data() {
    // 【テスト目的】: RidgeMetric::compute() が線形関係データで正しい SensitivityResult を返す
    // 【テスト内容】: 50行・1パラメータの完全線形データで R^2 > 0.99 を確認
    // 【期待される動作】: R^2 が 0.99 以上、beta が正の符号を持つ
    // 🔵 信頼性レベル: 青（要件定義書セクション2、full.rs L77-89）

    // 【テストデータ準備】: tc_801_06 と同一パターン。x1=i, y=2*i+1 の完全線形関係
    // 【初期条件設定】: 50行・1パラメータ・1目的関数の DataFrame
    let rows: Vec<TrialRow> = (0..50)
        .map(|i| make_row_multi(i, &[("x1", i as f64)], vec![2.0 * i as f64 + 1.0]))
        .collect();
    let df = setup_df(rows, &["x1"], &["obj0"]);

    // 【実際の処理実行】: RidgeMetric::compute() を呼び出す
    // 【処理内容】: トレイトメソッド compute() による Ridge 感度計算
    let metric = RidgeMetric;
    let result = metric.compute(&df, 0);

    // 【結果検証】: SensitivityResult の全フィールドを検証
    // 【期待値確認】: 要件定義書セクション2「SensitivityResult の内容（RidgeMetric）」
    assert!(
        result.is_some(),
        "RidgeMetric::compute() should return Some"
    ); // 【確認内容】: 計算が正常に完了 🔵
    let r = result.unwrap();
    assert_eq!(r.param_names, vec!["x1"]); // 【確認内容】: パラメータ名 🔵
    assert_eq!(r.objective_names, vec!["obj0"]); // 【確認内容】: 目的関数名1要素 🔵
    assert!(r.spearman.is_empty()); // 【確認内容】: Spearman フィールドは空 🔵
    assert_eq!(r.ridge.len(), 1); // 【確認内容】: ridge フィールドに1要素 🔵
    assert_eq!(r.ridge[0].beta.len(), 1); // 【確認内容】: beta がパラメータ数分 🔵
    assert!(
        r.ridge[0].r_squared > 0.99,
        "R² should be close to 1.0: {}",
        r.ridge[0].r_squared
    ); // 【確認内容】: R^2 が高い値 🔵
    assert!(
        r.ridge[0].beta[0] > 0.0,
        "beta should be positive: {}",
        r.ridge[0].beta[0]
    ); // 【確認内容】: beta の符号が正 🔵
    assert!(r.rf_anova.is_none()); // 【確認内容】: rf_anova は None 🔵
    assert!(r.mdi.is_none()); // 【確認内容】: mdi は None 🔵
    assert!(r.shap.is_none()); // 【確認内容】: shap は None 🔵
    assert!(r.permutation.is_none()); // 【確認内容】: permutation は None 🔵
}

#[test]
fn tc_2259_05_spearman_matches_legacy() {
    // 【テスト目的】: SpearmanMetric::compute() が compute_sensitivity_single_obj(Spearman) と同一結果を返す
    // 【テスト内容】: 20行・3パラメータの DataFrame で両方の計算結果を浮動小数点許容誤差 1e-10 で比較
    // 【期待される動作】: 全パラメータの spearman 値が差 < 1e-10 で一致
    // 🔵 信頼性レベル: 青（NFR-102、full.rs L57-76 と直接的に比較）

    // 【テストデータ準備】: 3パラメータの多様なデータパターン
    // 【初期条件設定】: 20行・3パラメータ・1目的関数の DataFrame
    let rows: Vec<TrialRow> = (0..20)
        .map(|i| {
            make_row_multi(
                i,
                &[
                    ("p0", i as f64),
                    ("p1", (i as f64 * 0.5).sin()),
                    ("p2", (i % 7) as f64),
                ],
                vec![i as f64 * 2.0 + 1.0],
            )
        })
        .collect();
    let df = setup_df(rows, &["p0", "p1", "p2"], &["obj0"]);

    // 【実際の処理実行】: トレイト実装と新 API の両方で計算して比較
    let metric = SpearmanMetric;
    let metric_result = metric.compute(&df, 0);
    let api_results = compute_sensitivity_single_obj(&df, vec![Box::new(SpearmanMetric)], 0);

    // 【結果検証】: 両者の計算結果が同一であることを確認
    assert!(metric_result.is_some()); // 【確認内容】: トレイト実装が Some を返す 🔵
    assert!(!api_results.is_empty()); // 【確認内容】: 新 API も結果を返す 🔵
    let mr = metric_result.unwrap();
    let ar = &api_results[0];
    for i in 0..mr.spearman.len() {
        let diff = (mr.spearman[i][0] - ar.spearman[i][0]).abs();
        assert!(
            diff < 1e-10,
            "Spearman mismatch at param {}: metric={}, api={}, diff={}",
            i,
            mr.spearman[i][0],
            ar.spearman[i][0],
            diff
        ); // 【確認内容】: 各パラメータの感度値が差 < 1e-10 で一致 🔵
    }
}

#[test]
fn tc_2259_06_ridge_matches_legacy() {
    // 【テスト目的】: RidgeMetric::compute() が compute_sensitivity_single_obj(Ridge) と同一結果を返す
    // 【テスト内容】: 30行・3パラメータの DataFrame で beta と r_squared を比較
    // 【期待される動作】: beta 全要素と r_squared が差 < 1e-10 で一致
    // 🔵 信頼性レベル: 青（NFR-102、full.rs L77-89 と直接的に比較）

    // 【テストデータ準備】: 3パラメータの多様なデータパターン
    // 【初期条件設定】: 30行・3パラメータ・1目的関数の DataFrame
    let rows: Vec<TrialRow> = (0..30)
        .map(|i| {
            make_row_multi(
                i,
                &[
                    ("p0", i as f64),
                    ("p1", (i as f64 * 0.3).cos()),
                    ("p2", (i % 5) as f64),
                ],
                vec![i as f64 * 1.5 + 3.0],
            )
        })
        .collect();
    let df = setup_df(rows, &["p0", "p1", "p2"], &["obj0"]);

    // 【実際の処理実行】: トレイト実装と新 API の両方で計算して比較
    let metric = RidgeMetric;
    let metric_result = metric.compute(&df, 0);
    let api_results = compute_sensitivity_single_obj(&df, vec![Box::new(RidgeMetric)], 0);

    // 【結果検証】: 両者の計算結果が同一であることを確認
    assert!(metric_result.is_some()); // 【確認内容】: トレイト実装が Some を返す 🔵
    assert!(!api_results.is_empty()); // 【確認内容】: 新 API も結果を返す 🔵
    let mr = metric_result.unwrap();
    let ar = &api_results[0];
    for i in 0..mr.ridge[0].beta.len() {
        let diff = (mr.ridge[0].beta[i] - ar.ridge[0].beta[i]).abs();
        assert!(
            diff < 1e-10,
            "Ridge beta mismatch at param {}: metric={}, api={}, diff={}",
            i,
            mr.ridge[0].beta[i],
            ar.ridge[0].beta[i],
            diff
        ); // 【確認内容】: beta 各要素が差 < 1e-10 で一致 🔵
    }
    let r2_diff = (mr.ridge[0].r_squared - ar.ridge[0].r_squared).abs();
    assert!(
        r2_diff < 1e-10,
        "Ridge R² mismatch: metric={}, api={}, diff={}",
        mr.ridge[0].r_squared,
        ar.ridge[0].r_squared,
        r2_diff
    ); // 【確認内容】: R^2 が差 < 1e-10 で一致 🔵
}

#[test]
fn tc_2259_09_spearman_as_trait_object() {
    // 【テスト目的】: SpearmanMetric を Box<dyn SensitivityMetric> として利用できることを確認
    // 【テスト内容】: トレイトオブジェクト経由で compute() と name() を呼び出す
    // 【期待される動作】: トレイトオブジェクト経由でも正常に計算が行われる
    // 🔵 信頼性レベル: 青（設計文書 architecture.md「ディスパッチ」、metric_trait.rs の Send + Sync 制約）

    // 【テストデータ準備】: 基本的な DataFrame
    // 【初期条件設定】: 20行・2パラメータ・1目的関数
    let rows: Vec<TrialRow> = (0..20)
        .map(|i| {
            make_row_multi(
                i,
                &[("x1", i as f64), ("x2", (20 - i) as f64)],
                vec![i as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["x1", "x2"], &["obj0"]);

    // 【実際の処理実行】: トレイトオブジェクトとして格納してメソッド呼び出し
    // 【処理内容】: ポリモーフィックな利用パターンの検証
    let metric: Box<dyn SensitivityMetric> = Box::new(SpearmanMetric);

    // 【結果検証】: トレイトオブジェクト経由の動的ディスパッチが正常に機能
    // 【期待値確認】: compute() が Some を返し、name() が正しい値を返す
    assert_eq!(metric.name(), "Spearman"); // 【確認内容】: name() が正しい文字列を返す 🔵
    let result = metric.compute(&df, 0);
    assert!(
        result.is_some(),
        "trait object compute() should return Some"
    ); // 【確認内容】: 計算結果が返る 🔵
}

#[test]
fn tc_2259_10_ridge_as_trait_object() {
    // 【テスト目的】: RidgeMetric を Box<dyn SensitivityMetric> として利用できることを確認
    // 【テスト内容】: トレイトオブジェクト経由で compute() と name() を呼び出す
    // 【期待される動作】: トレイトオブジェクト経由でも正常に計算が行われる
    // 🔵 信頼性レベル: 青（設計文書 architecture.md「ディスパッチ」）

    // 【テストデータ準備】: 基本的な DataFrame
    // 【初期条件設定】: 20行・2パラメータ・1目的関数
    let rows: Vec<TrialRow> = (0..20)
        .map(|i| {
            make_row_multi(
                i,
                &[("x1", i as f64), ("x2", (20 - i) as f64)],
                vec![i as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["x1", "x2"], &["obj0"]);

    // 【実際の処理実行】: トレイトオブジェクトとして格納してメソッド呼び出し
    // 【処理内容】: ポリモーフィックな利用パターンの検証
    let metric: Box<dyn SensitivityMetric> = Box::new(RidgeMetric);

    // 【結果検証】: トレイトオブジェクト経由の動的ディスパッチが正常に機能
    // 【期待値確認】: compute() が Some を返し、name() が正しい値を返す
    assert_eq!(metric.name(), "Ridge"); // 【確認内容】: name() が正しい文字列を返す 🔵
    let result = metric.compute(&df, 0);
    assert!(
        result.is_some(),
        "trait object compute() should return Some"
    ); // 【確認内容】: 計算結果が返る 🔵
}

#[test]
fn tc_2259_11_multiple_metrics_vector_dispatch() {
    // 【テスト目的】: 複数メトリックを Vec<Box<dyn SensitivityMetric>> に格納し統一的に処理できることを確認
    // 【テスト内容】: SpearmanMetric と RidgeMetric を同一 Vec に格納しイテレーションで compute() を呼び出す
    // 【期待される動作】: 各メトリックが独立して正しい結果を返す
    // 🔵 信頼性レベル: 青（要件定義書セクション1「何をする機能か」、REQ-A03）

    // 【テストデータ準備】: 20行・2パラメータの DataFrame
    // 【初期条件設定】: 両メトリックで計算可能なデータ
    let rows: Vec<TrialRow> = (0..20)
        .map(|i| {
            make_row_multi(
                i,
                &[("x1", i as f64), ("x2", (20 - i) as f64)],
                vec![i as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["x1", "x2"], &["obj0"]);

    // 【実際の処理実行】: Vec に格納してイテレーション
    // 【処理内容】: 統一的なディスパッチパターン
    let metrics: Vec<Box<dyn SensitivityMetric>> =
        vec![Box::new(SpearmanMetric), Box::new(RidgeMetric)];

    // 【結果検証】: 各メトリックの name() と compute() が正しく動作
    // 【期待値確認】: REQ-A03 新規指標追加時にディスパッチ側の変更なし
    assert_eq!(metrics[0].name(), "Spearman"); // 【確認内容】: 1つ目のメトリック名 🔵
    assert_eq!(metrics[1].name(), "Ridge"); // 【確認内容】: 2つ目のメトリック名 🔵

    let spearman_result = metrics[0].compute(&df, 0);
    assert!(
        spearman_result.is_some(),
        "SpearmanMetric should return Some"
    ); // 【確認内容】: Spearman 計算成功 🔵
    assert!(!spearman_result.unwrap().spearman.is_empty()); // 【確認内容】: spearman フィールドが空でない 🔵

    let ridge_result = metrics[1].compute(&df, 0);
    assert!(ridge_result.is_some(), "RidgeMetric should return Some"); // 【確認内容】: Ridge 計算成功 🔵
    assert!(!ridge_result.unwrap().ridge.is_empty()); // 【確認内容】: ridge フィールドが空でない 🔵
}

#[test]
fn tc_2259_12_spearman_obj_idx_1() {
    // 【テスト目的】: SpearmanMetric::compute() が obj_idx=1（2番目の目的関数）で正しく計算すること
    // 【テスト内容】: 20行・2パラメータ・2目的関数の DataFrame で obj_idx=1 を指定
    // 【期待される動作】: 2番目の目的関数に対する Spearman 感度値が正しく計算される
    // 🔵 信頼性レベル: 青（要件定義書セクション2、full.rs の obj_idx ロジック）

    // 【テストデータ準備】: 2目的関数のデータ。obj0=i, obj1=20-i
    // 【初期条件設定】: 20行・2パラメータ・2目的関数
    let rows: Vec<TrialRow> = (0..20)
        .map(|i| {
            make_row_multi(
                i,
                &[("x1", i as f64), ("x2", (20 - i) as f64)],
                vec![i as f64, (20 - i) as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["x1", "x2"], &["obj0", "obj1"]);

    // 【実際の処理実行】: obj_idx=1 で SpearmanMetric::compute() を呼び出す
    // 【処理内容】: 2番目の目的関数に対する感度計算
    let metric = SpearmanMetric;
    let result = metric.compute(&df, 1);

    // 【結果検証】: objective_names が正しく1要素であること
    // 【期待値確認】: obj_idx=1 の場合、objective_names == ["obj1"]
    assert!(result.is_some()); // 【確認内容】: 計算成功 🔵
    let r = result.unwrap();
    assert_eq!(r.objective_names, vec!["obj1"]); // 【確認内容】: 2番目の目的関数名のみ 🔵
    assert_eq!(r.spearman.len(), 2); // 【確認内容】: パラメータ数分の感度値 🔵
}

#[test]
fn tc_2259_13_ridge_obj_idx_1() {
    // 【テスト目的】: RidgeMetric::compute() が obj_idx=1（2番目の目的関数）で正しく計算すること
    // 【テスト内容】: 30行・2パラメータ・2目的関数の DataFrame で obj_idx=1 を指定
    // 【期待される動作】: 2番目の目的関数に対する RidgeResult が正しく計算される
    // 🔵 信頼性レベル: 青（要件定義書セクション2、full.rs L77-89）

    // 【テストデータ準備】: 2目的関数のデータ
    // 【初期条件設定】: 30行・2パラメータ・2目的関数
    let rows: Vec<TrialRow> = (0..30)
        .map(|i| {
            make_row_multi(
                i,
                &[("x1", i as f64), ("x2", (30 - i) as f64)],
                vec![i as f64 * 2.0, (30 - i) as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["x1", "x2"], &["obj0", "obj1"]);

    // 【実際の処理実行】: obj_idx=1 で RidgeMetric::compute() を呼び出す
    // 【処理内容】: 2番目の目的関数に対する Ridge 回帰
    let metric = RidgeMetric;
    let result = metric.compute(&df, 1);

    // 【結果検証】: objective_names と ridge の構造を確認
    // 【期待値確認】: obj_idx=1 の場合、objective_names == ["obj1"]
    assert!(result.is_some()); // 【確認内容】: 計算成功 🔵
    let r = result.unwrap();
    assert_eq!(r.objective_names, vec!["obj1"]); // 【確認内容】: 2番目の目的関数名のみ 🔵
    assert_eq!(r.ridge.len(), 1); // 【確認内容】: ridge に1要素 🔵
    assert_eq!(r.ridge[0].beta.len(), 2); // 【確認内容】: beta がパラメータ数分 🔵
}

// ===========================================================================
// 異常系テストケース
// ===========================================================================

#[test]
fn tc_2259_14_spearman_insufficient_data_n1() {
    // 【テスト目的】: SpearmanMetric::compute() がデータ不足（n=1）で None を返すことを確認
    // 【テスト内容】: 1行の DataFrame で compute() を呼び出し None が返ることを確認
    // 【期待される動作】: None が返りパニックしない
    // 🔵 信頼性レベル: 青（EDGE-2259-01、完了条件「データ不足時に None を返しパニックしない」）

    // 【テストデータ準備】: 1行のみの DataFrame
    // 【初期条件設定】: 1行・2パラメータ・1目的関数
    let rows = vec![make_row_multi(0, &[("x1", 1.0), ("x2", 2.0)], vec![3.0])];
    let df = setup_df(rows, &["x1", "x2"], &["obj0"]);

    // 【実際の処理実行】: SpearmanMetric::compute() を呼び出す
    // 【処理内容】: データ不足時のエラーハンドリング
    let metric = SpearmanMetric;
    let result = metric.compute(&df, 0);

    // 【結果検証】: None が返ること（パニックしないこと）
    // 【期待値確認】: n < 2 の場合は None
    assert!(
        result.is_none(),
        "SpearmanMetric should return None when n < 2"
    ); // 【確認内容】: データ不足で None 🔵
}

#[test]
fn tc_2259_15_ridge_insufficient_data_n1() {
    // 【テスト目的】: RidgeMetric::compute() がデータ不足（n=1）で None を返すことを確認
    // 【テスト内容】: 1行の DataFrame で compute() を呼び出し None が返ることを確認
    // 【期待される動作】: None が返りパニックしない
    // 🔵 信頼性レベル: 青（EDGE-2259-01）

    // 【テストデータ準備】: 1行のみの DataFrame
    // 【初期条件設定】: 1行・2パラメータ・1目的関数
    let rows = vec![make_row_multi(0, &[("x1", 1.0), ("x2", 2.0)], vec![3.0])];
    let df = setup_df(rows, &["x1", "x2"], &["obj0"]);

    // 【実際の処理実行】: RidgeMetric::compute() を呼び出す
    // 【処理内容】: データ不足時のエラーハンドリング
    let metric = RidgeMetric;
    let result = metric.compute(&df, 0);

    // 【結果検証】: None が返ること（パニックしないこと）
    // 【期待値確認】: n < 2 の場合は None
    assert!(
        result.is_none(),
        "RidgeMetric should return None when n < 2"
    ); // 【確認内容】: データ不足で None 🔵
}

#[test]
fn tc_2259_16_spearman_empty_data_n0() {
    // 【テスト目的】: SpearmanMetric::compute() が空データ（n=0）で None を返すことを確認
    // 【テスト内容】: 0行の DataFrame で compute() を呼び出し None が返ることを確認
    // 【期待される動作】: None が返りパニックしない
    // 🔵 信頼性レベル: 青（EDGE-2259-01、full.rs L29 の n < 2 チェック）

    // 【テストデータ準備】: 0行の DataFrame
    // 【初期条件設定】: 空の DataFrame（パラメータ名と目的関数名は設定済み）
    let rows: Vec<TrialRow> = vec![];
    let df = setup_df(rows, &["x1", "x2"], &["obj0"]);

    // 【実際の処理実行】: SpearmanMetric::compute() を呼び出す
    // 【処理内容】: 空データのエラーハンドリング
    let metric = SpearmanMetric;
    let result = metric.compute(&df, 0);

    // 【結果検証】: None が返ること（パニックしないこと）
    // 【期待値確認】: n = 0 < 2 の場合は None
    assert!(
        result.is_none(),
        "SpearmanMetric should return None when n = 0"
    ); // 【確認内容】: 空データで None 🔵
}

#[test]
fn tc_2259_18_spearman_invalid_obj_idx() {
    // 【テスト目的】: SpearmanMetric::compute() が無効な obj_idx（範囲外）で None を返すことを確認
    // 【テスト内容】: 2目的関数の DataFrame で obj_idx=5 を指定
    // 【期待される動作】: None が返りパニックしない
    // 🔵 信頼性レベル: 青（EDGE-2259-03、full.rs L25-27 の get(obj_idx) チェック）

    // 【テストデータ準備】: 10行・2パラメータ・2目的関数の DataFrame
    // 【初期条件設定】: objective_col_names.len() == 2 の状態で obj_idx=5 を指定
    let rows: Vec<TrialRow> = (0..10)
        .map(|i| {
            make_row_multi(
                i,
                &[("x1", i as f64), ("x2", (10 - i) as f64)],
                vec![i as f64, (10 - i) as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["x1", "x2"], &["obj0", "obj1"]);

    // 【実際の処理実行】: 範囲外の obj_idx=5 で compute() を呼び出す
    // 【処理内容】: インデックス境界チェックの検証
    let metric = SpearmanMetric;
    let result = metric.compute(&df, 5);

    // 【結果検証】: None が返ること（パニックしないこと）
    // 【期待値確認】: obj_idx >= objective_names.len() の場合は None
    assert!(
        result.is_none(),
        "SpearmanMetric should return None for out-of-range obj_idx"
    ); // 【確認内容】: 範囲外で None 🔵
}

#[test]
fn tc_2259_19_ridge_invalid_obj_idx() {
    // 【テスト目的】: RidgeMetric::compute() が無効な obj_idx（範囲外）で None を返すことを確認
    // 【テスト内容】: 2目的関数の DataFrame で obj_idx=100 を指定
    // 【期待される動作】: None が返りパニックしない
    // 🔵 信頼性レベル: 青（EDGE-2259-03）

    // 【テストデータ準備】: 10行・2パラメータ・2目的関数の DataFrame
    // 【初期条件設定】: objective_col_names.len() == 2 の状態で obj_idx=100 を指定
    let rows: Vec<TrialRow> = (0..10)
        .map(|i| {
            make_row_multi(
                i,
                &[("x1", i as f64), ("x2", (10 - i) as f64)],
                vec![i as f64, (10 - i) as f64],
            )
        })
        .collect();
    let df = setup_df(rows, &["x1", "x2"], &["obj0", "obj1"]);

    // 【実際の処理実行】: 範囲外の obj_idx=100 で compute() を呼び出す
    // 【処理内容】: インデックス境界チェックの検証
    let metric = RidgeMetric;
    let result = metric.compute(&df, 100);

    // 【結果検証】: None が返ること（パニックしないこと）
    // 【期待値確認】: obj_idx >= objective_names.len() の場合は None
    assert!(
        result.is_none(),
        "RidgeMetric should return None for out-of-range obj_idx"
    ); // 【確認内容】: 範囲外で None 🔵
}

// ===========================================================================
// 境界値テストケース
// ===========================================================================

#[test]
fn tc_2259_20_spearman_empty_params() {
    // 【テスト目的】: SpearmanMetric::compute() がパラメータなし（param_names 空）で None を返すことを確認
    // 【テスト内容】: 0パラメータの DataFrame で compute() を呼び出す
    // 【期待される動作】: None が返りパニックしない
    // 🔵 信頼性レベル: 青（EDGE-2259-02、full.rs L29 param_names.is_empty() チェック）

    // 【テストデータ準備】: パラメータなしの DataFrame
    // 【初期条件設定】: 10行・0パラメータ・1目的関数
    let rows: Vec<TrialRow> = (0..10)
        .map(|i| make_row_multi(i, &[], vec![i as f64]))
        .collect();
    let df = setup_df(rows, &[], &["obj0"]);

    // 【実際の処理実行】: SpearmanMetric::compute() を呼び出す
    // 【処理内容】: 空パラメータリストのエラーハンドリング
    let metric = SpearmanMetric;
    let result = metric.compute(&df, 0);

    // 【結果検証】: None が返ること
    // 【期待値確認】: param_names.is_empty() の場合は None
    assert!(
        result.is_none(),
        "SpearmanMetric should return None when param_names is empty"
    ); // 【確認内容】: 空パラメータで None 🔵
}

#[test]
fn tc_2259_22_spearman_min_rows_n2() {
    // 【テスト目的】: SpearmanMetric::compute() が最小行数（n=2）で正しい計算結果を返すことを確認
    // 【テスト内容】: 2行・1パラメータの完全正相関データで Spearman = 1.0 を確認
    // 【期待される動作】: n=2 で Some が返り、spearman 値が正確
    // 🔵 信頼性レベル: 青（spearman.rs L77-79 n < 2 チェック、full.rs L29）

    // 【テストデータ準備】: 最小行数の完全正相関データ
    // 【初期条件設定】: 2行・1パラメータ・1目的関数、x1=[1,2], y=[1,2]
    let rows = vec![
        make_row_multi(0, &[("x1", 1.0)], vec![1.0]),
        make_row_multi(1, &[("x1", 2.0)], vec![2.0]),
    ];
    let df = setup_df(rows, &["x1"], &["obj0"]);

    // 【実際の処理実行】: SpearmanMetric::compute() を呼び出す
    // 【処理内容】: 最小行数での計算
    let metric = SpearmanMetric;
    let result = metric.compute(&df, 0);

    // 【結果検証】: Some が返り、完全正相関が検出される
    // 【期待値確認】: n=2 で正常計算、spearman = 1.0
    assert!(result.is_some()); // 【確認内容】: n=2 で Some が返る 🔵
    let r = result.unwrap();
    assert_eq!(r.spearman.len(), 1); // 【確認内容】: パラメータ1つ分 🔵
    assert!(
        (r.spearman[0][0] - 1.0).abs() < 1e-9,
        "n=2 perfect positive: {}",
        r.spearman[0][0]
    ); // 【確認内容】: 完全正相関 🔵
}

#[test]
fn tc_2259_23_ridge_min_rows_n2() {
    // 【テスト目的】: RidgeMetric::compute() が最小行数（n=2）で正しい RidgeResult を返すことを確認
    // 【テスト内容】: 2行・1パラメータのデータで Ridge 計算が正常に行われる
    // 【期待される動作】: n=2 で Some が返り、ridge フィールドに結果が設定される
    // 🔵 信頼性レベル: 青（ridge.rs L136 n < 2 チェック）

    // 【テストデータ準備】: 最小行数の線形データ
    // 【初期条件設定】: 2行・1パラメータ・1目的関数、x1=[1,2], y=[3,5]
    let rows = vec![
        make_row_multi(0, &[("x1", 1.0)], vec![3.0]),
        make_row_multi(1, &[("x1", 2.0)], vec![5.0]),
    ];
    let df = setup_df(rows, &["x1"], &["obj0"]);

    // 【実際の処理実行】: RidgeMetric::compute() を呼び出す
    // 【処理内容】: 最小行数での Ridge 計算
    let metric = RidgeMetric;
    let result = metric.compute(&df, 0);

    // 【結果検証】: Some が返り、RidgeResult が正しく構築される
    // 【期待値確認】: n=2 で正常計算
    assert!(result.is_some()); // 【確認内容】: n=2 で Some が返る 🔵
    let r = result.unwrap();
    assert_eq!(r.ridge.len(), 1); // 【確認内容】: ridge に1要素 🔵
    assert_eq!(r.ridge[0].beta.len(), 1); // 【確認内容】: beta がパラメータ数分 🔵
}

#[test]
fn tc_2259_27_spearman_large_data() {
    // 【テスト目的】: SpearmanMetric::compute() が大規模データ（1000行）で正しい結果を返すことを確認
    // 【テスト内容】: 1000行・5パラメータの DataFrame で計算が正常に完了する
    // 【期待される動作】: 全 spearman 値が [-1.0, 1.0] の範囲内
    // 🔵 信頼性レベル: 青（compute_spearman は既存関数、正確性は担保済み）

    // 【テストデータ準備】: 1000行の多パラメータデータ
    // 【初期条件設定】: 1000行・5パラメータ・1目的関数
    let rows: Vec<TrialRow> = (0..1000)
        .map(|i| {
            make_row_multi(
                i,
                &[
                    ("p0", i as f64),
                    ("p1", (i as f64 * 0.01).sin()),
                    ("p2", (i % 10) as f64),
                    ("p3", (i as f64).ln()),
                    ("p4", (i as f64 * 0.5).cos()),
                ],
                vec![i as f64 * 2.0],
            )
        })
        .collect();
    let df = setup_df(rows, &["p0", "p1", "p2", "p3", "p4"], &["obj0"]);

    // 【実際の処理実行】: SpearmanMetric::compute() を呼び出す
    // 【処理内容】: 大規模データでの計算
    let metric = SpearmanMetric;
    let result = metric.compute(&df, 0);

    // 【結果検証】: 計算が正常に完了し、値が範囲内
    // 【期待値確認】: 大規模データでも正確な SensitivityResult が返る
    assert!(result.is_some()); // 【確認内容】: 大規模データで Some 🔵
    let r = result.unwrap();
    assert_eq!(r.spearman.len(), 5); // 【確認内容】: 5パラメータ分 🔵
    for (i, param_vals) in r.spearman.iter().enumerate() {
        for (j, val) in param_vals.iter().enumerate() {
            assert!(
                *val >= -1.0 && *val <= 1.0,
                "spearman[{}][{}] = {} out of [-1, 1]",
                i,
                j,
                val
            ); // 【確認内容】: 値が [-1, 1] の範囲内 🔵
        }
    }
}

#[test]
fn tc_2259_29_spearman_send_sync() {
    // 【テスト目的】: SpearmanMetric が Send + Sync トレイトを自動実装することをコンパイル時に確認
    // 【テスト内容】: コンパイル時型チェック関数で Send + Sync 制約を検証
    // 【期待される動作】: コンパイルが成功する（ゼロサイズ構造体は自動的に Send + Sync）
    // 🔵 信頼性レベル: 青（metric_trait.rs Send + Sync 制約、Rust 型システムによる自動導出）

    // 【テストデータ準備】: 不要（コンパイル時チェック）
    // 【初期条件設定】: 型レベルの確認
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<SpearmanMetric>(); // 【確認内容】: SpearmanMetric が Send + Sync を満たす 🔵
}

#[test]
fn tc_2259_30_ridge_send_sync() {
    // 【テスト目的】: RidgeMetric が Send + Sync トレイトを自動実装することをコンパイル時に確認
    // 【テスト内容】: コンパイル時型チェック関数で Send + Sync 制約を検証
    // 【期待される動作】: コンパイルが成功する
    // 🔵 信頼性レベル: 青（metric_trait.rs Send + Sync 制約）

    // 【テストデータ準備】: 不要（コンパイル時チェック）
    // 【初期条件設定】: 型レベルの確認
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<RidgeMetric>(); // 【確認内容】: RidgeMetric が Send + Sync を満たす 🔵
}
