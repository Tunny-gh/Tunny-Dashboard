use crate::state::app_state::AppState;
use egui::Color32;

/// state 層の RGBA 表現を UI 描画用 Color32 へ変換する。
fn to_color32(rgba: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
}

/// ビュー切り替えタブの状態
#[derive(Default, PartialEq, Clone, Copy)]
pub enum ComparisonView {
    #[default]
    Stats,
    HvHistory,
    ParetoFront,
    KdeDistribution,
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
    });
    ui.separator();

    match active_view {
        ComparisonView::Stats => show_stats_summary(ui, app_state),
        ComparisonView::HvHistory => show_hv_history(ui, app_state),
        ComparisonView::ParetoFront => show_pareto_overlay(ui, app_state),
        ComparisonView::KdeDistribution => show_kde_distribution(ui, app_state),
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
                    .map(to_color32)
                    .unwrap_or(Color32::GRAY);

                let obj_vals: Vec<f64> = study
                    .trial_rows
                    .iter()
                    .filter_map(|t| t.objectives.first().copied())
                    .collect();

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
                .map(to_color32)
                .unwrap_or(Color32::GRAY);

            // Best 値の遷移を折れ線として表示（HV の代替）
            let mut best_so_far = f64::INFINITY;
            let points: PlotPoints = study
                .trial_rows
                .iter()
                .enumerate()
                .filter_map(|(i, t)| {
                    let v = t.objectives.first().copied()?;
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
        .and_then(|s| s.trial_rows.first())
        .map(|t| t.objectives.len() >= 2)
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
                .map(to_color32)
                .unwrap_or(Color32::GRAY);

            let pareto_set: std::collections::HashSet<u32> =
                study.pareto_indices.iter().copied().collect();

            let pts: Vec<[f64; 2]> = study
                .trial_rows
                .iter()
                .filter(|t| pareto_set.contains(&t.trial_id))
                .filter_map(|t| {
                    if t.objectives.len() >= 2 {
                        Some([t.objectives[0], t.objectives[1]])
                    } else {
                        None
                    }
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
                .map(to_color32)
                .unwrap_or(Color32::GRAY);

            let vals: Vec<f64> = study
                .trial_rows
                .iter()
                .filter_map(|t| t.objectives.first().copied())
                .collect();

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
}
