use crate::state::app_state::AppState;
use crate::theme::color_compute::rgba_to_color32;
use egui::Color32;

/// ビュー切り替えタブの状態
#[derive(Default, PartialEq, Clone, Copy)]
pub enum ComparisonView {
    #[default]
    Stats,
    HvHistory,
    ParetoFront,
    KdeDistribution,
    Diff,
}

/// 1 比較 Study 分の差分行
#[derive(Debug, Clone)]
pub struct ComparisonDiffRow {
    pub study_name: String,
    pub trial_count_delta: i64,
    pub best_value_delta: Option<f64>,
    pub pareto_dominance_ratio: Option<f64>,
    pub incompatible_reason: Option<String>,
}

/// 単一目的列の最小値を返す（NaN/Inf 除外）。
fn col_min(col: &[f64]) -> Option<f64> {
    col.iter()
        .copied()
        .filter(|v| v.is_finite())
        .reduce(f64::min)
}

/// base Study と comparison studies の差分行を生成する pure function。
/// objective 名不一致の Study には `incompatible_reason` を設定する。
pub fn build_comparison_diff_rows(
    base: &crate::state::app_state::StudyContext,
    comparison_studies: &[crate::state::app_state::StudyContext],
) -> Vec<ComparisonDiffRow> {
    let base_best = base
        .meta
        .objective_names
        .first()
        .and_then(|name| base.view.numeric_column(name))
        .and_then(col_min);

    comparison_studies
        .iter()
        .map(|comp| {
            if base.meta.objective_names != comp.meta.objective_names {
                return ComparisonDiffRow {
                    study_name: comp.meta.name.clone(),
                    trial_count_delta: 0,
                    best_value_delta: None,
                    pareto_dominance_ratio: None,
                    incompatible_reason: Some(format!(
                        "Objective mismatch: {:?} vs {:?}",
                        base.meta.objective_names, comp.meta.objective_names
                    )),
                };
            }

            let trial_count_delta = comp.trial_count() as i64 - base.trial_count() as i64;

            let best_value_delta = base_best.and_then(|b| {
                comp.meta
                    .objective_names
                    .first()
                    .and_then(|name| comp.view.numeric_column(name))
                    .and_then(col_min)
                    .map(|c| c - b)
            });

            let n = comp.trial_count();
            let pareto_dominance_ratio = if n > 0 {
                Some(comp.pareto_indices.len() as f64 / n as f64)
            } else {
                None
            };

            ComparisonDiffRow {
                study_name: comp.meta.name.clone(),
                trial_count_delta,
                best_value_delta,
                pareto_dominance_ratio,
                incompatible_reason: None,
            }
        })
        .collect()
}

/// マルチスタディ比較パネルを表示する
pub fn show_comparison_panel(
    ui: &mut egui::Ui,
    app_state: &AppState,
    active_view: &mut ComparisonView,
) {
    if app_state.comparison_studies.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label("Add comparison studies via toolbar");
        });
        return;
    }

    ui.horizontal(|ui| {
        ui.selectable_value(active_view, ComparisonView::Stats, "Stats");
        ui.selectable_value(active_view, ComparisonView::HvHistory, "HV History");
        ui.selectable_value(active_view, ComparisonView::ParetoFront, "Pareto");
        ui.selectable_value(active_view, ComparisonView::KdeDistribution, "KDE");
        ui.selectable_value(active_view, ComparisonView::Diff, "Diff");
    });
    ui.separator();

    match active_view {
        ComparisonView::Stats => show_stats_summary(ui, app_state),
        ComparisonView::HvHistory => show_hv_history(ui, app_state),
        ComparisonView::ParetoFront => show_pareto_overlay(ui, app_state),
        ComparisonView::KdeDistribution => show_kde_distribution(ui, app_state),
        ComparisonView::Diff => show_diff_tab(ui, app_state),
    }
}

// ============================================================
// Stats Summary
// ============================================================

fn compute_median(vals: &[f64]) -> Option<f64> {
    if vals.is_empty() {
        return None;
    }
    let mut sorted = vals.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        Some((sorted[mid - 1] + sorted[mid]) / 2.0)
    } else {
        Some(sorted[mid])
    }
}

fn compute_std_dev(vals: &[f64]) -> Option<f64> {
    if vals.len() < 2 {
        return None;
    }
    let mean = vals.iter().sum::<f64>() / vals.len() as f64;
    let variance = vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / vals.len() as f64;
    Some(variance.sqrt())
}

fn show_stats_summary(ui: &mut egui::Ui, app_state: &AppState) {
    egui::Grid::new("comparison_stats_grid")
        .striped(true)
        .show(ui, |ui| {
            ui.strong("Study");
            ui.strong("Trials");
            ui.strong("Best");
            ui.strong("Median");
            ui.strong("StdDev");
            ui.end_row();

            for (idx, study) in app_state.comparison_studies.iter().enumerate() {
                let color = app_state
                    .comparison_colors
                    .get(idx)
                    .copied()
                    .map(rgba_to_color32)
                    .unwrap_or(Color32::GRAY);

                let obj_vals: Vec<f64> = study
                    .meta
                    .objective_names
                    .first()
                    .and_then(|name| study.view.numeric_column(name))
                    .map(|col| col.iter().copied().filter(|v| v.is_finite()).collect())
                    .unwrap_or_default();

                let n = obj_vals.len();
                let best = obj_vals.iter().cloned().fold(f64::INFINITY, f64::min);
                let median = compute_median(&obj_vals);
                let std_dev = compute_std_dev(&obj_vals);

                ui.colored_label(color, &study.meta.name);
                ui.label(n.to_string());
                ui.label(if best.is_finite() {
                    format!("{:.6}", best)
                } else {
                    "-".to_string()
                });
                ui.label(median.map_or("-".to_string(), |v| format!("{:.6}", v)));
                ui.label(std_dev.map_or("-".to_string(), |v| format!("{:.6}", v)));
                ui.end_row();
            }
        });
}

// ============================================================
// HV History Overlay
// ============================================================

fn show_hv_history(ui: &mut egui::Ui, app_state: &AppState) {
    use egui_plot::{Line, Plot, PlotPoints};

    Plot::new("hv_history_plot").show(ui, |plot_ui| {
        for (idx, study) in app_state.comparison_studies.iter().enumerate() {
            let color = app_state
                .comparison_colors
                .get(idx)
                .copied()
                .map(rgba_to_color32)
                .unwrap_or(Color32::GRAY);

            // Best 値の遷移を折れ線として表示（HV の代替）
            let obj_col = study
                .meta
                .objective_names
                .first()
                .and_then(|name| study.view.numeric_column(name));
            let mut best_so_far = f64::INFINITY;
            let points: PlotPoints = (0..study.view.row_count())
                .filter_map(|i| {
                    let v = obj_col?.get(i).copied()?;
                    if !v.is_finite() {
                        return None;
                    }
                    if v < best_so_far {
                        best_so_far = v;
                    }
                    Some([i as f64, best_so_far])
                })
                .collect();

            plot_ui.line(Line::new(points).color(color).name(&study.meta.name));
        }
    });
}

// ============================================================
// Pareto Front Overlay
// ============================================================

fn show_pareto_overlay(ui: &mut egui::Ui, app_state: &AppState) {
    use egui_plot::{Plot, PlotPoints, Points};

    let is_2d = app_state
        .comparison_studies
        .first()
        .map(|s| s.meta.objective_names.len() >= 2)
        .unwrap_or(false);

    if !is_2d {
        ui.label("Pareto Front overlay requires 2+ objectives");
        return;
    }

    Plot::new("pareto_overlay_plot").show(ui, |plot_ui| {
        for (idx, study) in app_state.comparison_studies.iter().enumerate() {
            let color = app_state
                .comparison_colors
                .get(idx)
                .copied()
                .map(rgba_to_color32)
                .unwrap_or(Color32::GRAY);

            let pareto_set: std::collections::HashSet<u32> =
                study.pareto_indices.iter().copied().collect();

            let obj_names = &study.meta.objective_names;
            let col0 = obj_names.first().and_then(|n| study.view.numeric_column(n));
            let col1 = obj_names.get(1).and_then(|n| study.view.numeric_column(n));

            let pts: Vec<[f64; 2]> = (0..study.view.row_count())
                .filter_map(|i| {
                    let tid = study.view.trial_ids.get(i).copied()?;
                    if !pareto_set.contains(&tid) {
                        return None;
                    }
                    let x = col0?.get(i).copied()?;
                    let y = col1?.get(i).copied()?;
                    Some([x, y])
                })
                .collect();

            plot_ui.points(
                Points::new(PlotPoints::new(pts))
                    .color(color)
                    .radius(4.0)
                    .name(&study.meta.name),
            );
        }
    });
}

// ============================================================
// KDE Distribution
// ============================================================

/// シンプルな KDE 近似（Gaussian kernel）
fn kde_points(vals: &[f64], bandwidth: f64, n_points: usize) -> Vec<[f64; 2]> {
    if vals.is_empty() || bandwidth <= 0.0 {
        return vec![];
    }
    let min = vals.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if min >= max {
        return vec![];
    }
    let step = (max - min) / n_points as f64;
    (0..=n_points)
        .map(|i| {
            let x = min + i as f64 * step;
            let density: f64 = vals
                .iter()
                .map(|&v| {
                    let u = (x - v) / bandwidth;
                    (-0.5 * u * u).exp() / (bandwidth * (2.0 * std::f64::consts::PI).sqrt())
                })
                .sum::<f64>()
                / vals.len() as f64;
            [x, density]
        })
        .collect()
}

fn show_kde_distribution(ui: &mut egui::Ui, app_state: &AppState) {
    use egui_plot::{Line, Plot, PlotPoints};

    Plot::new("kde_distribution_plot").show(ui, |plot_ui| {
        for (idx, study) in app_state.comparison_studies.iter().enumerate() {
            let color = app_state
                .comparison_colors
                .get(idx)
                .copied()
                .map(rgba_to_color32)
                .unwrap_or(Color32::GRAY);

            let vals: Vec<f64> = study
                .meta
                .objective_names
                .first()
                .and_then(|name| study.view.numeric_column(name))
                .map(|col| col.iter().copied().filter(|v| v.is_finite()).collect())
                .unwrap_or_default();

            if vals.is_empty() {
                continue;
            }

            // Scott's rule: h = 1.06 * sigma * n^(-1/5)
            let std_dev = compute_std_dev(&vals).unwrap_or(1.0);
            let bandwidth = 1.06 * std_dev * (vals.len() as f64).powf(-0.2);

            let pts = kde_points(&vals, bandwidth, 100);
            let plot_points: PlotPoints = pts.into_iter().collect();
            plot_ui.line(Line::new(plot_points).color(color).name(&study.meta.name));
        }
    });
}

// ============================================================
// Diff タブ描画
// ============================================================

fn show_diff_tab(ui: &mut egui::Ui, app_state: &AppState) {
    let Some(base) = &app_state.current_study else {
        ui.label("No base study loaded.");
        return;
    };
    if app_state.comparison_studies.is_empty() {
        ui.label("No comparison studies added.");
        return;
    }

    let rows = build_comparison_diff_rows(base, &app_state.comparison_studies);

    use egui_extras::{Column, TableBuilder};
    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .column(Column::auto().at_least(120.0))
        .column(Column::auto().at_least(80.0))
        .column(Column::auto().at_least(100.0))
        .column(Column::auto().at_least(100.0))
        .column(Column::remainder())
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.strong("Study");
            });
            header.col(|ui| {
                ui.strong("Δ Trials");
            });
            header.col(|ui| {
                ui.strong("Δ Best Value");
            });
            header.col(|ui| {
                ui.strong("Pareto Ratio");
            });
            header.col(|ui| {
                ui.strong("Note");
            });
        })
        .body(|body| {
            body.rows(18.0, rows.len(), |mut row| {
                let r = &rows[row.index()];
                row.col(|ui| {
                    ui.label(&r.study_name);
                });
                row.col(|ui| {
                    let s = format!("{:+}", r.trial_count_delta);
                    ui.label(s);
                });
                row.col(|ui| {
                    let s = r
                        .best_value_delta
                        .map(|v| format!("{:+.4}", v))
                        .unwrap_or_else(|| "—".to_string());
                    ui.label(s);
                });
                row.col(|ui| {
                    let s = r
                        .pareto_dominance_ratio
                        .map(|v| format!("{:.1}%", v * 100.0))
                        .unwrap_or_else(|| "—".to_string());
                    ui.label(s);
                });
                row.col(|ui| {
                    if let Some(reason) = &r.incompatible_reason {
                        ui.label(reason);
                    }
                });
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_median_odd() {
        let vals = vec![3.0, 1.0, 2.0];
        assert_eq!(compute_median(&vals), Some(2.0));
    }

    #[test]
    fn compute_median_even() {
        let vals = vec![1.0, 2.0, 3.0, 4.0];
        assert_eq!(compute_median(&vals), Some(2.5));
    }

    #[test]
    fn compute_median_empty() {
        assert_eq!(compute_median(&[]), None);
    }

    #[test]
    fn compute_std_dev_basic() {
        let vals = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let sd = compute_std_dev(&vals).unwrap();
        assert!((sd - 2.0).abs() < 0.01);
    }

    #[test]
    fn compute_std_dev_single() {
        assert_eq!(compute_std_dev(&[1.0]), None);
    }

    #[test]
    fn kde_points_empty() {
        let pts = kde_points(&[], 0.5, 100);
        assert!(pts.is_empty());
    }

    #[test]
    fn kde_points_basic() {
        let vals = vec![1.0, 2.0, 3.0];
        let pts = kde_points(&vals, 0.5, 50);
        assert!(!pts.is_empty());
        // all densities positive
        for [_, d] in &pts {
            assert!(*d >= 0.0);
        }
    }

    #[test]
    fn comparison_view_default_is_stats() {
        let view = ComparisonView::default();
        assert!(matches!(view, ComparisonView::Stats));
    }

    // ── TASK-2236: Comparison Diff テスト ──────────────────────

    use crate::state::app_state::{Direction, StudyContext, StudyMeta, TrialRow, TrialState};
    use std::collections::HashMap;

    fn make_ctx(
        name: &str,
        obj_names: Vec<String>,
        objectives: Vec<Vec<f64>>,
        pareto: Vec<u32>,
    ) -> StudyContext {
        let trial_rows = objectives
            .iter()
            .enumerate()
            .map(|(i, objs)| TrialRow {
                trial_id: i as u32,
                trial_number: i as u32,
                params: HashMap::new(),
                objectives: objs.clone(),
                pareto_rank: 0,
                cluster_id: None,
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            })
            .collect();
        let meta = StudyMeta {
            study_id: 0,
            name: name.to_string(),
            directions: vec![Direction::Minimize],
            completed_trials: objectives.len(),
            total_trials: objectives.len(),
            param_names: vec![],
            objective_names: obj_names,
            user_attr_names: vec![],
            has_constraints: false,
        };
        let mut ctx = StudyContext::from_rows_for_test(meta, trial_rows);
        ctx.pareto_indices = pareto;
        ctx
    }

    #[test]
    fn diff_metrics_compute_expected_delta_values() {
        let base = make_ctx(
            "base",
            vec!["f".to_string()],
            vec![vec![1.0], vec![2.0], vec![0.5]],
            vec![2],
        );
        let comp = make_ctx(
            "comp",
            vec!["f".to_string()],
            vec![vec![0.3], vec![1.5], vec![0.3], vec![0.3], vec![0.3]],
            vec![0],
        );
        let rows = build_comparison_diff_rows(&base, &[comp]);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].incompatible_reason.is_none());
        // trial_count_delta = 5 - 3 = 2
        assert_eq!(rows[0].trial_count_delta, 2);
        // best_value_delta = 0.3 - 0.5 = -0.2
        let delta = rows[0].best_value_delta.unwrap();
        assert!((delta - (-0.2)).abs() < 1e-9);
    }

    #[test]
    fn diff_metrics_compute_pareto_dominance_ratio() {
        let base = make_ctx("base", vec!["f".to_string()], vec![vec![1.0]], vec![0]);
        let comp = make_ctx(
            "comp",
            vec!["f".to_string()],
            vec![vec![0.5], vec![1.0], vec![1.5], vec![2.0]],
            vec![0],
        );
        let rows = build_comparison_diff_rows(&base, &[comp]);
        let ratio = rows[0].pareto_dominance_ratio.unwrap();
        assert!(
            (ratio - 0.25).abs() < 1e-9,
            "expected 1/4 = 0.25, got {}",
            ratio
        );
    }

    #[test]
    fn diff_metrics_detect_objective_mismatch() {
        let base = make_ctx("base", vec!["f1".to_string()], vec![vec![1.0]], vec![]);
        let comp = make_ctx("comp", vec!["f2".to_string()], vec![vec![1.0]], vec![]);
        let rows = build_comparison_diff_rows(&base, &[comp]);
        assert!(rows[0].incompatible_reason.is_some());
    }

    #[test]
    fn diff_tab_hidden_without_comparison_studies() {
        // When comparison_studies is empty, build_comparison_diff_rows returns empty vec
        let base = make_ctx("base", vec!["f".to_string()], vec![vec![1.0]], vec![]);
        let rows = build_comparison_diff_rows(&base, &[]);
        assert!(rows.is_empty());
    }
}
