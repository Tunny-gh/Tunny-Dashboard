use super::importance_chart::{
    compute_sorted_importance, compute_sorted_sobol, ImportanceChart, ImportanceMetric,
};
use crate::state::app_state::{AppState, SensitivityResult, SobolResult};

pub struct ObjectiveImportance {
    pub index: usize,
    pub name: String,
    pub scores: Vec<(String, f64)>,
    pub unsupported: Vec<String>,
    pub r_squared: Option<f64>,
    pub error: Option<String>,
}

impl ObjectiveImportance {
    pub fn new(
        index: usize,
        name: String,
        metric: ImportanceMetric,
        sensitivity: Option<&SensitivityResult>,
        sobol: Option<&SobolResult>,
        error: Option<String>,
    ) -> Self {
        let scores = if error.is_some() {
            vec![]
        } else if metric.is_sobol() {
            sobol
                .map(|r| compute_sorted_sobol(r, index, &metric))
                .unwrap_or_default()
        } else {
            sensitivity
                .map(|r| compute_sorted_importance(r, &metric, 0))
                .unwrap_or_default()
        };
        let r_squared = match metric {
            ImportanceMetric::Spearman => None,
            ImportanceMetric::SobolFirst | ImportanceMetric::SobolTotal => {
                sobol.and_then(|r| r.r_squared.get(index)).copied()
            }
            ImportanceMetric::Ridge => sensitivity
                .and_then(|r| r.ridge.first())
                .map(|r| r.r_squared),
            ImportanceMetric::Ard => sensitivity
                .and_then(|r| r.ard.as_ref())
                .map(|r| r.r_squared),
            _ => sensitivity
                .and_then(|r| super::importance_chart::extract_tree_r_squared(r, &metric)),
        }
        .filter(|r| r.is_finite() && error.is_none());
        let unsupported = sensitivity
            .filter(|_| metric.is_signed())
            .map(|r| r.unsupported_categorical.clone())
            .unwrap_or_default();
        Self {
            index,
            name,
            scores,
            unsupported,
            r_squared,
            error,
        }
    }

    pub fn is_partial(&self) -> bool {
        self.error.is_some()
            || self.scores.is_empty()
            || !self.unsupported.is_empty()
            || self.scores.iter().any(|(_, s)| !s.is_finite())
    }
}

impl ImportanceChart {
    pub fn results(&self, state: &AppState) -> Vec<ObjectiveImportance> {
        let Some(study) = &state.current_study else {
            return vec![];
        };
        self.objective_indices(study.meta.objective_names.len())
            .into_iter()
            .map(|i| {
                let key = (self.metric.cache_id(), i, self.feasible_only);
                ObjectiveImportance::new(
                    i,
                    study.meta.objective_names[i].clone(),
                    self.metric,
                    state.importance_cache.get(&key),
                    state.sobol_cache.get(&(i, self.feasible_only)),
                    self.errors.get(&key).cloned(),
                )
            })
            .collect()
    }
}

pub fn parameter_groups(results: &[ObjectiveImportance]) -> Vec<String> {
    let mut groups: Vec<String> = results
        .iter()
        .flat_map(|r| r.scores.iter().map(|(n, _)| n.clone()))
        .collect();
    groups.sort();
    groups.dedup();
    let max = |name: &str| {
        results
            .iter()
            .flat_map(|r| &r.scores)
            .filter(|(n, s)| n == name && s.is_finite())
            .map(|(_, s)| *s)
            .fold(0.0_f64, f64::max)
    };
    groups.sort_by(|a, b| max(b).total_cmp(&max(a)).then_with(|| a.cmp(b)));
    groups
}

pub fn axis_max(results: &[ObjectiveImportance]) -> f64 {
    let max = results
        .iter()
        .flat_map(|r| &r.scores)
        .map(|(_, s)| *s)
        .filter(|s| s.is_finite())
        .fold(0.0_f64, f64::max);
    if max <= 0.0 {
        return 1.0;
    }
    let step = 10.0_f64.powf(max.log10().floor());
    let fraction = max / step;
    let nice = if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };
    let bound = nice * step;
    if bound.is_finite() && bound >= max {
        bound
    } else {
        max
    }
}

pub fn paint_scores(ui: &mut egui::Ui, results: &[ObjectiveImportance], all: bool) {
    use crate::theme::chart_colors::{COLOR_GRID_STROKE, COLOR_IMPORTANCE_BAR};
    let groups = if all {
        parameter_groups(results)
    } else {
        results
            .first()
            .map(|r| r.scores.iter().map(|(n, _)| n.clone()).collect())
            .unwrap_or_default()
    };
    if groups.is_empty() {
        return;
    }
    let max = if all {
        axis_max(results)
    } else {
        let max = results
            .iter()
            .flat_map(|r| &r.scores)
            .map(|(_, s)| *s)
            .filter(|s| s.is_finite())
            .fold(0.0_f64, f64::max);
        if max > 0.0 {
            max
        } else {
            1.0
        }
    };
    let label_width = 150.0;
    let row_height = 22.0;
    let group_height = if all {
        results.len() as f32 * row_height + 8.0
    } else {
        24.0
    };
    egui::ScrollArea::vertical().show(ui, |ui| {
        let width = ui.available_width().max(300.0);
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(width, 26.0 + groups.len() as f32 * group_height),
            egui::Sense::hover(),
        );
        let painter = ui.painter();
        let left = rect.left() + label_width;
        let plot_width = (width - label_width - 100.0).max(50.0);
        let top = rect.top() + 26.0;
        let font = egui::TextStyle::Body.resolve(ui.style());
        let text = ui.visuals().text_color();
        let grid = egui::Stroke::new(0.6, COLOR_GRID_STROKE().gamma_multiply(0.55));
        let intervals = (plot_width / 65.0).floor().clamp(1.0, 4.0) as usize;
        for tick in 0..=intervals {
            let x = left + plot_width * tick as f32 / intervals as f32;
            painter.line_segment([egui::pos2(x, top), egui::pos2(x, rect.bottom())], grid);
            let value = max * (tick as f64 / intervals as f64);
            let label = if max < 0.01 || max >= 10000.0 {
                format!("{value:.1e}")
            } else {
                format!("{value:.3}")
            };
            painter.text(
                egui::pos2(x, rect.top()),
                egui::Align2::CENTER_TOP,
                label,
                font.clone(),
                text,
            );
        }
        for (g, name) in groups.iter().enumerate() {
            let y = top + g as f32 * group_height;
            painter
                .with_clip_rect(egui::Rect::from_min_max(
                    rect.min,
                    egui::pos2(left - 8.0, rect.bottom()),
                ))
                .text(
                    egui::pos2(rect.left(), y + row_height / 2.0),
                    egui::Align2::LEFT_CENTER,
                    name,
                    font.clone(),
                    text,
                );
            for (j, result) in results.iter().enumerate() {
                let y = y + j as f32 * row_height;
                let score = result
                    .scores
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, s)| *s)
                    .filter(|s| s.is_finite());
                if let Some(score) = score {
                    let color = if all {
                        objective_color(result.index)
                    } else {
                        COLOR_IMPORTANCE_BAR()
                    };
                    let bar = egui::Rect::from_min_size(
                        egui::pos2(left, y + 2.0),
                        egui::vec2((score / max * plot_width as f64) as f32, row_height - 4.0),
                    );
                    painter.rect_filled(bar, 2.0, color);
                }
                let value = if result.error.is_some() {
                    "Failed".into()
                } else {
                    score
                        .map(|s| {
                            if all {
                                score_label(s)
                            } else {
                                format!("{s:.3}")
                            }
                        })
                        .unwrap_or_else(|| "Unavailable".into())
                };
                painter.text(
                    egui::pos2(left + plot_width + 8.0, y + row_height / 2.0),
                    egui::Align2::LEFT_CENTER,
                    value,
                    font.clone(),
                    text,
                );
            }
            let bottom = y + group_height - 4.0;
            painter.line_segment(
                [
                    egui::pos2(rect.left(), bottom),
                    egui::pos2(rect.right(), bottom),
                ],
                grid,
            );
        }
    });
}

pub fn objective_color(index: usize) -> egui::Color32 {
    // A continuous hue sequence avoids repeating a fixed small palette for many objectives.
    egui::ecolor::Hsva::new((index as f32 * 0.618_034 + 0.58).fract(), 0.65, 0.8, 1.0).into()
}

fn score_label(score: f64) -> String {
    if score != 0.0 && (score.abs() < 0.0005 || score.abs() >= 10000.0) {
        format!("{score:.3e}")
    } else {
        format!("{score:.3}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(index: usize, scores: &[(&str, f64)]) -> ObjectiveImportance {
        ObjectiveImportance {
            index,
            name: format!("objective {index}"),
            scores: scores.iter().map(|(n, s)| (n.to_string(), *s)).collect(),
            unsupported: vec![],
            r_squared: Some(0.7 + index as f64 * 0.1),
            error: None,
        }
    }

    #[test]
    fn grouping_uses_maximum_with_shared_scale_and_preserves_zero_and_missing() {
        let results = [
            result(0, &[("a", 0.1), ("b", 0.8), ("zero", 0.0)]),
            result(1, &[("a", 1.7), ("b", f64::NAN)]),
        ];
        assert_eq!(parameter_groups(&results), ["a", "b", "zero"]);
        assert!(axis_max(&results) >= 1.7);
        assert!(axis_max(&results).is_finite());
        assert_eq!(
            axis_max(&[result(0, &[("zero", 0.0), ("missing", f64::NAN)])]),
            1.0
        );
        assert_eq!(score_label(0.0), "0.000");
        assert_ne!(score_label(1e-8), "0.000");
    }

    #[test]
    fn sobol_scores_and_fit_use_the_actual_objective_index() {
        let sobol = SobolResult {
            param_names: vec!["a".into(), "b".into()],
            first_order: vec![vec![0.1, 0.6], vec![0.2, 0.4]],
            total_effect: vec![vec![0.3, 0.8], vec![0.4, 0.7]],
            r_squared: vec![0.5, 0.9],
        };
        let selected = ObjectiveImportance::new(
            1,
            "second".into(),
            ImportanceMetric::SobolTotal,
            None,
            Some(&sobol),
            None,
        );
        assert_eq!(selected.scores, [("a".into(), 0.8), ("b".into(), 0.7)]);
        assert_eq!(selected.r_squared, Some(0.9));
        let failed = ObjectiveImportance::new(
            1,
            "second".into(),
            ImportanceMetric::SobolTotal,
            None,
            Some(&sobol),
            Some("failed".into()),
        );
        assert!(failed.scores.is_empty());
        assert!(failed.r_squared.is_none());
    }

    #[test]
    fn axis_labels_fit_narrow_and_wide_charts_in_both_themes() {
        for dark in [false, true] {
            for width in [320.0, 480.0, 800.0] {
                let ctx = egui::Context::default();
                let visuals = crate::theme::tunny_visuals(dark);
                let background = visuals.panel_fill;
                ctx.set_visuals(visuals);
                let results = [result(0, &[("parameter", 0.9)])];
                let output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 400.0),
                        )),
                        ..Default::default()
                    },
                    |ui| paint_scores(ui, &results, true),
                );
                let top = output
                    .shapes
                    .iter()
                    .find_map(|s| {
                        if let egui::Shape::LineSegment { points, .. } = &s.shape {
                            (points[0].x == points[1].x).then_some(points[0].y)
                        } else {
                            None
                        }
                    })
                    .unwrap();
                let labels = output
                    .shapes
                    .iter()
                    .filter_map(|s| {
                        if let egui::Shape::Text(t) = &s.shape {
                            (t.pos.y < top)
                                .then_some(egui::Rect::from_min_size(t.pos, t.galley.size()))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                assert!(labels.len() >= 2);
                for pair in labels.windows(2) {
                    assert!(
                        pair[0].right() < pair[1].left(),
                        "Ticks must not overlap at width {width}"
                    );
                }
                for shape in &output.shapes {
                    if let egui::Shape::LineSegment { stroke, .. } = &shape.shape {
                        assert!(stroke.width < 1.0, "Guides must stay secondary to bars");
                        assert_ne!(stroke.color, background);
                    }
                }
            }
        }
    }

    #[test]
    fn all_paints_legend_fits_guides_group_separators_and_explicit_partial_status_in_both_themes() {
        for dark in [false, true] {
            let ctx = egui::Context::default();
            ctx.set_visuals(crate::theme::tunny_visuals(dark));
            let mut chart = ImportanceChart::default();
            let mut results = vec![
                result(0, &[("a", 0.8), ("b", 0.0)]),
                result(1, &[("a", 0.4), ("b", f64::NAN)]),
                result(2, &[]),
            ];
            results[2].error = Some("Insufficient finite rows".into());
            results[2].r_squared = None;
            let names = results.iter().map(|r| r.name.clone()).collect::<Vec<_>>();
            let output = ctx.run_ui(egui::RawInput::default(), |ui| {
                chart.show_results(ui, &results, &names, false)
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|s| {
                    if let egui::Shape::Text(t) = &s.shape {
                        Some(t.galley.text())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            for expected in [
                "ALL",
                "objective 0",
                "objective 1",
                "objective 2",
                "0.800",
                "0.400",
                "0.000",
                "Unavailable",
                "Failed",
                "Partial CSV export",
                "R² = 0.700",
                "R² = 0.800",
            ] {
                assert!(text.contains(expected), "{expected}: {text}");
            }
            let guides = output
                .shapes
                .iter()
                .filter_map(|s| {
                    if let egui::Shape::LineSegment { points, stroke } = &s.shape {
                        (stroke.width == 0.6).then_some(points)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            assert_eq!(guides.iter().filter(|p| p[0].x == p[1].x).count(), 5);
            assert_eq!(guides.iter().filter(|p| p[0].y == p[1].y).count(), 2);
            let bars = output
                .shapes
                .iter()
                .filter_map(|s| {
                    if let egui::Shape::Rect(r) = &s.shape {
                        (r.fill == objective_color(0) || r.fill == objective_color(1)).then_some(r)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            let first = bars
                .iter()
                .find(|r| r.fill == objective_color(0) && r.rect.width() > 0.0)
                .unwrap();
            let second = bars.iter().find(|r| r.fill == objective_color(1)).unwrap();
            assert!((first.rect.width() / second.rect.width() - 2.0).abs() < 0.001);
            assert_ne!(objective_color(0), objective_color(1));
            assert!(guides.iter().all(|p| p.iter().all(|v| v.is_finite())));
        }
    }
}
