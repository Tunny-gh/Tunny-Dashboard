//! Key Findings（まとめ）の決定論的生成。
//!
//! 各 [`FindingKind`] は文書化された発火条件を満たす場合のみ生成される。
//! 生成順は固定（BestSingle/ParetoSummary → ConvergenceStatus → TopImportance
//! → TradeOff → Feasibility → PruningEfficiency → DataQuality）で、同一入力に対して
//! バイト同一の出力を保証する。文章化はレンダラのテンプレートが担当し、ここでは
//! 数値・文字列のファクトのみを詰める。

use std::collections::BTreeMap;

use super::model::{ConvergenceStatus, FindingKind, KeyFinding};

/// COMPLETE がこの数未満なら収束判定を [`ConvergenceStatus::Insufficient`] とする。
pub(super) const MIN_TRIALS_FOR_CONVERGENCE: usize = 10;
/// best 更新がこの割合以降にあれば「なお改善中」とみなす閾値。
pub(super) const STILL_IMPROVING_FRACTION: f64 = 0.8;
/// トレードオフ finding を発火させる Spearman の上限（これ未満で発火）。
pub(super) const TRADEOFF_RHO_THRESHOLD: f64 = -0.3;

/// 収束状態を判定する。
///
/// - COMPLETE が [`MIN_TRIALS_FOR_CONVERGENCE`] 未満 → `Insufficient`
/// - best の最終更新位置 `last_improve_frac`（0.0..=1.0）が
///   [`STILL_IMPROVING_FRACTION`] 以上 → `StillImproving`
/// - それ以外 → `Converged`
pub(super) fn convergence_status(
    complete_count: usize,
    last_improve_frac: f64,
) -> ConvergenceStatus {
    if complete_count < MIN_TRIALS_FOR_CONVERGENCE {
        ConvergenceStatus::Insufficient
    } else if last_improve_frac >= STILL_IMPROVING_FRACTION {
        ConvergenceStatus::StillImproving
    } else {
        ConvergenceStatus::Converged
    }
}

/// findings 生成に必要な、builder が事前計算したファクト。
pub(super) struct FindingInputs {
    /// 多目的か。
    pub is_multi: bool,
    /// 単目的の `(best 値, trial.number, 発見時点%)`。
    pub best_single: Option<(f64, u32, f64)>,
    /// 多目的の `(front サイズ, COMPLETE 数)`。
    pub pareto: Option<(usize, usize)>,
    /// 収束判定。
    pub convergence_status: ConvergenceStatus,
    /// 重要度上位 `(param, score)`（最大3件、降順）。
    pub top_importance: Vec<(String, f64)>,
    /// 重要度の手法名。
    pub importance_method: Option<String>,
    /// 最も負の目的間 Spearman `(obj_a, obj_b, rho)`（rho < 閾値のときのみ Some）。
    pub trade_off: Option<(String, String, f64)>,
    /// 制約充足 `(feasible 率, feasible 数, 総数, 最良 feasible trial.number)`。
    pub feasibility: Option<(f64, usize, usize, Option<u32>)>,
    /// 枝刈り `(prune 率, PRUNED 数, 中央値 step)`。
    pub pruning: Option<(f64, usize, Option<f64>)>,
    /// データ品質 `(NaN 目的値の件数, FAIL 件数)`。
    pub data_quality: Option<(usize, usize)>,
}

/// [`FindingInputs`] から Key Findings を固定順で組み立てる。
pub(super) fn generate_findings(inputs: &FindingInputs) -> Vec<KeyFinding> {
    let mut out = Vec::new();

    // 1. BestSingle（単目的） / ParetoSummary（多目的）
    if inputs.is_multi {
        if let Some((front, complete)) = inputs.pareto {
            out.push(pareto_summary(front, complete));
        }
    } else if let Some((best, trial, pct)) = inputs.best_single {
        out.push(best_single(best, trial, pct));
    }

    // 2. ConvergenceStatus
    out.push(convergence_finding(inputs.convergence_status));

    // 3. TopImportance
    if !inputs.top_importance.is_empty() {
        if let Some(method) = &inputs.importance_method {
            out.push(top_importance(method, &inputs.top_importance));
        }
    }

    // 4. TradeOff
    if let Some((a, b, rho)) = &inputs.trade_off {
        out.push(trade_off(a, b, *rho));
    }

    // 5. Feasibility
    if let Some((rate, feasible, total, best_trial)) = inputs.feasibility {
        out.push(feasibility(rate, feasible, total, best_trial));
    }

    // 6. PruningEfficiency
    if let Some((rate, pruned, median_step)) = inputs.pruning {
        if pruned > 0 {
            out.push(pruning_efficiency(rate, pruned, median_step));
        }
    }

    // 7. DataQuality
    if let Some((nan_count, fail_count)) = inputs.data_quality {
        if nan_count > 0 || fail_count > 0 {
            out.push(data_quality(nan_count, fail_count));
        }
    }

    out
}

fn best_single(best: f64, trial: u32, found_pct: f64) -> KeyFinding {
    let mut metrics = BTreeMap::new();
    metrics.insert("best".to_string(), best);
    metrics.insert("trial".to_string(), trial as f64);
    metrics.insert("found_pct".to_string(), found_pct);
    KeyFinding {
        kind: FindingKind::BestSingle,
        metrics,
        labels: BTreeMap::new(),
    }
}

fn pareto_summary(front_size: usize, complete: usize) -> KeyFinding {
    let mut metrics = BTreeMap::new();
    metrics.insert("front_size".to_string(), front_size as f64);
    metrics.insert("complete".to_string(), complete as f64);
    KeyFinding {
        kind: FindingKind::ParetoSummary,
        metrics,
        labels: BTreeMap::new(),
    }
}

fn convergence_finding(status: ConvergenceStatus) -> KeyFinding {
    let mut labels = BTreeMap::new();
    let key = match status {
        ConvergenceStatus::Converged => "converged",
        ConvergenceStatus::StillImproving => "still_improving",
        ConvergenceStatus::Insufficient => "insufficient",
    };
    labels.insert("status".to_string(), key.to_string());
    KeyFinding {
        kind: FindingKind::ConvergenceStatus,
        metrics: BTreeMap::new(),
        labels,
    }
}

fn top_importance(method: &str, top: &[(String, f64)]) -> KeyFinding {
    let mut metrics = BTreeMap::new();
    let mut labels = BTreeMap::new();
    labels.insert("method".to_string(), method.to_string());
    for (rank, (name, score)) in top.iter().take(3).enumerate() {
        labels.insert(format!("param{}", rank + 1), name.clone());
        metrics.insert(format!("score{}", rank + 1), *score);
    }
    KeyFinding {
        kind: FindingKind::TopImportance,
        metrics,
        labels,
    }
}

fn trade_off(obj_a: &str, obj_b: &str, rho: f64) -> KeyFinding {
    let mut metrics = BTreeMap::new();
    let mut labels = BTreeMap::new();
    metrics.insert("rho".to_string(), rho);
    labels.insert("obj_a".to_string(), obj_a.to_string());
    labels.insert("obj_b".to_string(), obj_b.to_string());
    KeyFinding {
        kind: FindingKind::TradeOff,
        metrics,
        labels,
    }
}

fn feasibility(rate: f64, feasible: usize, total: usize, best_trial: Option<u32>) -> KeyFinding {
    let mut metrics = BTreeMap::new();
    let mut labels = BTreeMap::new();
    metrics.insert("rate".to_string(), rate);
    metrics.insert("feasible".to_string(), feasible as f64);
    metrics.insert("total".to_string(), total as f64);
    if let Some(t) = best_trial {
        metrics.insert("best_trial".to_string(), t as f64);
        labels.insert("has_best".to_string(), "true".to_string());
    }
    KeyFinding {
        kind: FindingKind::Feasibility,
        metrics,
        labels,
    }
}

fn pruning_efficiency(rate: f64, pruned: usize, median_step: Option<f64>) -> KeyFinding {
    let mut metrics = BTreeMap::new();
    let mut labels = BTreeMap::new();
    metrics.insert("rate".to_string(), rate);
    metrics.insert("pruned".to_string(), pruned as f64);
    if let Some(step) = median_step {
        metrics.insert("median_step".to_string(), step);
        labels.insert("has_step".to_string(), "true".to_string());
    }
    KeyFinding {
        kind: FindingKind::PruningEfficiency,
        metrics,
        labels,
    }
}

fn data_quality(nan_count: usize, fail_count: usize) -> KeyFinding {
    let mut metrics = BTreeMap::new();
    metrics.insert("nan_count".to_string(), nan_count as f64);
    metrics.insert("fail_count".to_string(), fail_count as f64);
    KeyFinding {
        kind: FindingKind::DataQuality,
        metrics,
        labels: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convergence_insufficient_below_threshold() {
        // COMPLETE < 10 は常に Insufficient（改善位置に依らない）。
        assert_eq!(convergence_status(9, 1.0), ConvergenceStatus::Insufficient);
        assert_eq!(convergence_status(0, 0.0), ConvergenceStatus::Insufficient);
    }

    #[test]
    fn convergence_still_improving_when_last_update_in_tail() {
        // 後半20%（>=0.8）で best 更新 → StillImproving。
        assert_eq!(
            convergence_status(100, 0.8),
            ConvergenceStatus::StillImproving
        );
        assert_eq!(
            convergence_status(100, 0.95),
            ConvergenceStatus::StillImproving
        );
    }

    #[test]
    fn convergence_converged_when_last_update_early() {
        // 後半20%より前で最後の更新 → Converged。
        assert_eq!(convergence_status(100, 0.79), ConvergenceStatus::Converged);
        assert_eq!(convergence_status(50, 0.5), ConvergenceStatus::Converged);
    }

    #[test]
    fn pruning_finding_only_when_pruned_positive() {
        let base = FindingInputs {
            is_multi: false,
            best_single: Some((1.0, 0, 10.0)),
            pareto: None,
            convergence_status: ConvergenceStatus::Converged,
            top_importance: vec![],
            importance_method: None,
            trade_off: None,
            feasibility: None,
            pruning: Some((0.0, 0, None)),
            data_quality: None,
        };
        let findings = generate_findings(&base);
        assert!(
            !findings
                .iter()
                .any(|f| f.kind == FindingKind::PruningEfficiency),
            "PRUNED=0 のとき PruningEfficiency は出さない"
        );

        let with_prune = FindingInputs {
            pruning: Some((0.3, 6, Some(4.0))),
            ..base
        };
        let findings = generate_findings(&with_prune);
        assert!(findings
            .iter()
            .any(|f| f.kind == FindingKind::PruningEfficiency));
    }

    #[test]
    fn data_quality_only_when_nan_or_fail() {
        let base = FindingInputs {
            is_multi: false,
            best_single: Some((1.0, 0, 10.0)),
            pareto: None,
            convergence_status: ConvergenceStatus::Converged,
            top_importance: vec![],
            importance_method: None,
            trade_off: None,
            feasibility: None,
            pruning: None,
            data_quality: Some((0, 0)),
        };
        assert!(
            !generate_findings(&base)
                .iter()
                .any(|f| f.kind == FindingKind::DataQuality),
            "NaN=0 かつ FAIL=0 では DataQuality を出さない"
        );

        let with_nan = FindingInputs {
            data_quality: Some((2, 0)),
            ..base
        };
        assert!(generate_findings(&with_nan)
            .iter()
            .any(|f| f.kind == FindingKind::DataQuality));
    }

    #[test]
    fn feasibility_finding_present_only_with_constraints() {
        let base = FindingInputs {
            is_multi: false,
            best_single: Some((1.0, 0, 10.0)),
            pareto: None,
            convergence_status: ConvergenceStatus::Converged,
            top_importance: vec![],
            importance_method: None,
            trade_off: None,
            feasibility: None,
            pruning: None,
            data_quality: None,
        };
        assert!(!generate_findings(&base)
            .iter()
            .any(|f| f.kind == FindingKind::Feasibility));

        let with_constraints = FindingInputs {
            feasibility: Some((0.5, 10, 20, Some(3))),
            ..base
        };
        assert!(generate_findings(&with_constraints)
            .iter()
            .any(|f| f.kind == FindingKind::Feasibility));
    }

    #[test]
    fn multi_objective_emits_pareto_summary_not_best_single() {
        let inputs = FindingInputs {
            is_multi: true,
            best_single: None,
            pareto: Some((5, 30)),
            convergence_status: ConvergenceStatus::Converged,
            top_importance: vec![],
            importance_method: None,
            trade_off: Some(("obj0".to_string(), "obj1".to_string(), -0.7)),
            feasibility: None,
            pruning: None,
            data_quality: None,
        };
        let findings = generate_findings(&inputs);
        assert!(findings
            .iter()
            .any(|f| f.kind == FindingKind::ParetoSummary));
        assert!(!findings.iter().any(|f| f.kind == FindingKind::BestSingle));
        assert!(findings.iter().any(|f| f.kind == FindingKind::TradeOff));
    }
}
