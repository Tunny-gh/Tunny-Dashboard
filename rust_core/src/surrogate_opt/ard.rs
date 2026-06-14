//! ARD（自動関連度決定）パラメータ重要度を DataFrame から直接算出する。
//!
//! GP-FITC サロゲートを学習し、その ARD 長さスケールから相対パラメータ重要度を
//! 取り出す。感度分析（Importance ウィジェット）の 1 手法として使う。Sobol の
//! `sensitivity::compute_sobol_from_df` と同じく「DataFrame → 1 目的の重要度」の
//! 入口を提供する。パラメータは数値列・カテゴリ列とも `get_param_numeric_values`
//! で数値化して GP に渡す（カテゴリは Sobol 同様ラベル符号化）。

use super::{fit_surrogate_with_validation, SurrogateFitRequest, SurrogateModelKind};
use crate::dataframe::DataFrame;
use crate::sensitivity::get_param_numeric_values;

/// ARD パラメータ重要度の結果（1 目的分）。
pub struct ArdImportanceResult {
    /// パラメータ名（`importances` と同順）。
    pub param_names: Vec<String>,
    /// 各パラメータの相対重要度（合計 1.0、`param_names` と同順）。
    pub importances: Vec<f64>,
    /// 学習した GP の交差検証 R²（重要度の信頼度の目安）。
    pub r_squared: f64,
}

/// 指定目的（`obj_idx`）について GP-FITC を学習し、ARD 由来のパラメータ重要度を返す。
///
/// trial 数が学習に不足する／GP が ARD を露出しない（学習失敗など）場合は `None`。
pub fn compute_ard_importance_from_df(
    df: &DataFrame,
    obj_idx: usize,
) -> Option<ArdImportanceResult> {
    let param_names = df.param_col_names().to_vec();
    let n = df.row_count();
    let n_params = param_names.len();
    if n_params == 0 {
        return None;
    }
    let objective_name = df.objective_col_names().get(obj_idx)?.clone();

    // パラメータ列（数値 or ラベル符号化）→ 行優先 X 行列。
    let param_columns: Vec<Vec<f64>> = param_names
        .iter()
        .map(|name| get_param_numeric_values(df, name, n).unwrap_or_else(|| vec![0.0; n]))
        .collect();
    let x_matrix: Vec<Vec<f64>> = (0..n)
        .map(|i| param_columns.iter().map(|col| col[i]).collect())
        .collect();
    let y: Vec<f64> = df
        .get_numeric_column(&objective_name)?
        .iter()
        .take(n)
        .copied()
        .collect();

    let req = SurrogateFitRequest {
        x_matrix,
        y,
        param_names,
        objective_name,
        model: SurrogateModelKind::GpFitc,
        auto_select: false,
        constraints: vec![],
        priority_rows: vec![],
    };
    // 入力検証（最小 trial 数等）は fit_surrogate_with_validation が行う。
    let trained = fit_surrogate_with_validation(&req).ok()?;
    let importances = trained.param_importance?;
    Some(ArdImportanceResult {
        param_names: trained.param_names,
        importances,
        r_squared: trained.validation.cv_r2_mean,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{DataFrame, TrialRow};
    use std::collections::HashMap;

    fn make_row(trial_id: u32, params: &[(&str, f64)], objectives: Vec<f64>) -> TrialRow {
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

    /// 結線確認: GP-FITC を学習して ARD 重要度が得られ、param_names と整合し、
    /// 合計が 1.0 になること。egobox GP の数値品質そのものは検証しない。
    #[test]
    fn ard_importance_from_df_wires_through_gp() {
        // x0 が応答を強く動かし、x1 はほぼ無関係になるよう構成する。
        let rows: Vec<TrialRow> = (0..30)
            .map(|i| {
                let x0 = i as f64 / 30.0;
                let x1 = ((i * 7) % 30) as f64 / 30.0;
                make_row(i, &[("x0", x0), ("x1", x1)], vec![3.0 * x0 + 0.01 * x1])
            })
            .collect();
        let df = DataFrame::from_trials(
            &rows,
            &["x0".to_string(), "x1".to_string()],
            &["obj".to_string()],
            &[],
            &[],
            0,
        );

        let result =
            compute_ard_importance_from_df(&df, 0).expect("GP-FITC should expose ARD importance");
        assert_eq!(result.param_names, vec!["x0".to_string(), "x1".to_string()]);
        assert_eq!(result.importances.len(), 2);
        let sum: f64 = result.importances.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-6,
            "importances should sum to 1.0, got {sum}"
        );
        // x0 が応答を ~300 倍強く動かすため、列順が保たれていれば x0 の重要度が大きい。
        // （合計 1.0・長さ一致だけでは param↔importance の入れ替わりを検出できない）。
        assert!(
            result.importances[0] > result.importances[1],
            "x0 drives the response far more than x1; ARD importance must rank it higher: {:?}",
            result.importances
        );
        assert!(result.r_squared.is_finite());
    }

    /// 範囲外の目的インデックスは None。
    #[test]
    fn ard_importance_from_df_out_of_range_objective() {
        let rows: Vec<TrialRow> = (0..12)
            .map(|i| make_row(i, &[("x0", i as f64)], vec![i as f64]))
            .collect();
        let df = DataFrame::from_trials(
            &rows,
            &["x0".to_string()],
            &["obj".to_string()],
            &[],
            &[],
            0,
        );
        assert!(compute_ard_importance_from_df(&df, 5).is_none());
    }
}
