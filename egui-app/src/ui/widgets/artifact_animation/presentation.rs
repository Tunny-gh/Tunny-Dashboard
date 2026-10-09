use super::{data::Data, ReadyFrame};
use crate::theme::chart_colors::{
    COLOR_HIGHLIGHT_PT, COLOR_INFEASIBLE, COLOR_NON_PARETO, COLOR_OPT_PRUNED, COLOR_OPT_TRIAL,
    COLOR_PARETO, COLOR_UNVERIFIED,
};
use tunny_core::dataframe::FeasibilityState;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Presentation {
    pub trial_number: bool,
    pub objectives: bool,
    pub parameters: bool,
    pub objective_names: Vec<String>,
    pub parameter_names: Vec<String>,
    pub pareto: bool,
    pub history: bool,
    pub x: usize,
    pub y: usize,
    pub history_objective: usize,
    #[serde(skip)]
    pub(super) palette: Option<[egui::Color32; 7]>,
}
impl Default for Presentation {
    fn default() -> Self {
        Self {
            trial_number: true,
            objectives: false,
            parameters: false,
            objective_names: Vec::new(),
            parameter_names: Vec::new(),
            pareto: false,
            history: false,
            x: 0,
            y: 1,
            history_objective: 0,
            palette: None,
        }
    }
}
impl Presentation {
    fn colors(&self) -> [egui::Color32; 7] {
        self.palette.unwrap_or_else(|| {
            [
                COLOR_PARETO(),
                COLOR_NON_PARETO(),
                COLOR_INFEASIBLE(),
                COLOR_UNVERIFIED(),
                COLOR_HIGHLIGHT_PT(),
                COLOR_OPT_TRIAL(),
                COLOR_OPT_PRUNED(),
            ]
        })
    }
    pub(super) fn snapshot(&self) -> Self {
        let mut snapshot = self.clone();
        snapshot.palette = Some(self.colors());
        snapshot
    }
    pub(super) fn controls(
        &mut self,
        ui: &mut egui::Ui,
        objectives: &[String],
        parameters: &[String],
    ) {
        self.x = self.x.min(objectives.len().saturating_sub(1));
        self.y = self.y.min(objectives.len().saturating_sub(1));
        self.history_objective = self
            .history_objective
            .min(objectives.len().saturating_sub(1));
        self.objective_names
            .retain(|name| objectives.contains(name));
        self.parameter_names
            .retain(|name| parameters.contains(name));
        ui.collapsing("Presentation", |ui| {
            ui.checkbox(&mut self.trial_number, "Trial number");
            names(
                ui,
                "Objective values",
                &mut self.objectives,
                &mut self.objective_names,
                objectives,
            );
            names(
                ui,
                "Parameter values",
                &mut self.parameters,
                &mut self.parameter_names,
                parameters,
            );
            ui.add_enabled(
                objectives.len() >= 2,
                egui::Checkbox::new(&mut self.pareto, "Pareto 2D"),
            );
            if self.pareto && objectives.len() >= 2 {
                axis(ui, "X objective", &mut self.x, objectives);
                axis(ui, "Y objective", &mut self.y, objectives);
            }
            ui.checkbox(&mut self.history, "Optimization History");
            if self.history {
                axis(
                    ui,
                    "History objective",
                    &mut self.history_objective,
                    objectives,
                );
            }
        });
    }
}
fn names(
    ui: &mut egui::Ui,
    label: &str,
    enabled: &mut bool,
    selected: &mut Vec<String>,
    names: &[String],
) {
    selected.retain(|n| names.contains(n));
    if ui.checkbox(enabled, label).changed() && *enabled && selected.is_empty() {
        selected.extend_from_slice(names);
    }
    if *enabled {
        ui.menu_button(format!("Select {label}"), |ui| {
            for name in names {
                let mut checked = selected.contains(name);
                if ui.checkbox(&mut checked, name).changed() {
                    if checked {
                        selected.push(name.clone());
                    } else {
                        selected.retain(|n| n != name);
                    }
                }
            }
        });
    }
}
fn axis(ui: &mut egui::Ui, label: &str, index: &mut usize, names: &[String]) {
    egui::ComboBox::from_id_salt(label)
        .selected_text(
            names
                .get(*index)
                .map(String::as_str)
                .unwrap_or("No objectives"),
        )
        .show_ui(ui, |ui| {
            for (i, name) in names.iter().enumerate() {
                ui.selectable_value(index, i, name);
            }
        });
}

/// The only content-rendering path, used by both live playback and snapshot capture.
pub(super) fn content(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    data: &Data,
    ready: &ReadyFrame,
    p: &Presentation,
) -> bool {
    ui.painter().rect_filled(rect, 0.0, ui.visuals().panel_fill);
    if rect.width() <= 24.0 || rect.height() <= 24.0 {
        ui.put(
            rect,
            egui::Label::new("Enlarge the widget to show animation content."),
        );
        return false;
    }
    let rect = rect.shrink(6.0);
    let charts = (p.pareto && data.bounds.len() >= 2) || (p.history && !data.bounds.is_empty());
    let values = p.objectives || p.parameters;
    let narrow = charts && rect.width() < 560.0;
    let image_group = if charts {
        if narrow {
            egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), rect.height() * 0.52))
        } else {
            egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * 0.55, rect.height()))
        }
    } else {
        rect
    };
    let image_rect = if values {
        egui::Rect::from_min_size(
            image_group.min,
            egui::vec2(image_group.width() * 0.68, image_group.height()),
        )
    } else {
        image_group
    };
    // egui's Image maintains the original aspect ratio inside this fixed image box.
    ui.scope_builder(egui::UiBuilder::new().max_rect(image_rect), |ui| {
        ui.set_clip_rect(ui.clip_rect().intersect(image_rect));
        ui.put(
            image_rect,
            egui::Image::new(&ready.texture)
                .fit_to_exact_size(image_rect.size())
                .maintain_aspect_ratio(true),
        );
        if p.trial_number {
            let original = ready.texture.size_vec2();
            let scale = (image_rect.width() / original.x).min(image_rect.height() / original.y);
            let fitted = egui::Rect::from_center_size(image_rect.center(), original * scale);
            let position = fitted.min + egui::vec2(8.0, 8.0);
            let text = format!("Trial {}", data.frames[ready.index].number);
            let galley = ui.painter().layout_no_wrap(
                text,
                egui::FontId::proportional(16.0),
                ui.visuals().text_color(),
            );
            ui.painter().rect_filled(
                egui::Rect::from_min_size(position, galley.size()).expand(4.0),
                2.0,
                ui.visuals().panel_fill,
            );
            ui.painter()
                .galley(position, galley, ui.visuals().text_color());
        }
    });
    if values {
        let values_rect = egui::Rect::from_min_max(
            egui::pos2(image_rect.right() + 6.0, image_group.top()),
            image_group.max,
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(values_rect), |ui| {
            ui.set_clip_rect(ui.clip_rect().intersect(values_rect));
            let row = data.frames[ready.index].row;
            for (enabled, objective, names) in [
                (p.objectives, true, &p.objective_names),
                (p.parameters, false, &p.parameter_names),
            ] {
                if !enabled {
                    continue;
                }
                ui.strong(if objective {
                    "Objectives"
                } else {
                    "Parameters"
                });
                for name in names {
                    let column = if objective {
                        data.study.view.df.objective_column(name)
                    } else {
                        data.study.view.df.numeric_parameter_column(name)
                    };
                    let value = if let Some(v) = column.and_then(|c| c.get(row)) {
                        if v.is_finite() {
                            format!("{v:.4}")
                        } else {
                            "—".into()
                        }
                    } else if objective {
                        "—".into()
                    } else if let Some(categories) = data.study.view.df.category_values(name) {
                        categories
                            .get(row)
                            .and_then(Option::as_ref)
                            .map(|value| match value.as_str() {
                                Some("") => "\"\"".into(),
                                Some(text) => text.into(),
                                None => value.to_string(),
                            })
                            .unwrap_or_else(|| "—".into())
                    } else {
                        data.study
                            .view
                            .string_column(name)
                            .and_then(|c| c.get(row))
                            .cloned()
                            .map(|v| if v.is_empty() { "\"\"".into() } else { v })
                            .unwrap_or_else(|| "—".into())
                    };
                    ui.label(format!("{name}: {value}"));
                }
            }
        });
    }
    if charts {
        let area = if narrow {
            egui::Rect::from_min_max(
                egui::pos2(rect.left(), image_group.bottom() + 6.0),
                rect.max,
            )
        } else {
            egui::Rect::from_min_max(egui::pos2(image_group.right() + 6.0, rect.top()), rect.max)
        };
        let pareto = p.pareto && data.bounds.len() >= 2;
        let history = p.history && !data.bounds.is_empty();
        let height = if pareto && history {
            (area.height() - 6.0) / 2.0
        } else {
            area.height()
        };
        if pareto {
            plot(
                ui,
                egui::Rect::from_min_size(area.min, egui::vec2(area.width(), height)),
                data,
                ready,
                p,
                false,
            );
        }
        if history {
            let top = if pareto {
                area.top() + height + 6.0
            } else {
                area.top()
            };
            plot(
                ui,
                egui::Rect::from_min_max(egui::pos2(area.left(), top), area.max),
                data,
                ready,
                p,
                true,
            );
        }
    }
    true
}

fn plot(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    data: &Data,
    ready: &ReadyFrame,
    p: &Presentation,
    history: bool,
) {
    let Series {
        groups,
        highlighted,
        best_points,
    } = series(data, ready, p, history);
    let names = &data.study.meta.objective_names;
    let [front, other, infeasible, unverified, current, trial, best] = p.colors();
    let (xb, yb) = if history {
        (data.trial_bounds, data.bounds[p.history_objective])
    } else {
        (data.bounds[p.x], data.bounds[p.y])
    };
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.set_clip_rect(ui.clip_rect().intersect(rect));
        ui.label(if history {
            "Optimization History"
        } else {
            "Pareto 2D (all objectives)"
        });
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            let labels = if history {
                [("Trials", trial), ("Best", best)]
            } else {
                [("Front", front), ("Other", other)]
            };
            for (label, color) in labels {
                ui.label(egui::RichText::new(label).size(10.0).color(color));
            }
            if data.study.view.feasibility().has_constraints() {
                ui.label(
                    egui::RichText::new("Infeasible")
                        .size(10.0)
                        .color(infeasible),
                );
                ui.label(
                    egui::RichText::new("Feasibility unverified")
                        .size(10.0)
                        .color(unverified),
                );
            }
            ui.label(
                egui::RichText::new("Current trial")
                    .size(10.0)
                    .color(current),
            );
        });
        egui_plot::Plot::new(if history {
            "animation_history"
        } else {
            "animation_pareto"
        })
        .width(rect.width().max(1.0))
        .height(ui.available_height().max(1.0))
        .allow_zoom(false)
        .allow_scroll(false)
        .allow_drag(false)
        .allow_boxed_zoom(false)
        .allow_axis_zoom_drag(false)
        .allow_double_click_reset(false)
        .auto_bounds(false)
        .show_grid(true)
        .show_x(false)
        .show_y(false)
        .show_crosshair(false)
        .x_axis_label(if history {
            "Study trial number"
        } else {
            &names[p.x]
        })
        .y_axis_label(if history {
            &names[p.history_objective]
        } else {
            &names[p.y]
        })
        .show(ui, |plot| {
            plot.set_plot_bounds(egui_plot::PlotBounds::from_min_max(
                [xb.0, yb.0],
                [xb.1, yb.1],
            ));
            let labels = if history {
                ["", "All Trials", "Infeasible", "Feasibility unverified"]
            } else {
                [
                    "Pareto front",
                    "Non-Pareto",
                    "Infeasible",
                    "Feasibility unverified",
                ]
            };
            let colors = [
                front,
                if history { trial } else { other },
                infeasible,
                unverified,
            ];
            for ((points, label), color) in groups.into_iter().zip(labels).zip(colors) {
                if !points.is_empty() {
                    plot.points(
                        egui_plot::Points::new(label, points)
                            .color(color)
                            .radius(2.5)
                            .allow_hover(false),
                    );
                }
            }
            if history && !best_points.is_empty() {
                plot.line(
                    egui_plot::Line::new("Best Value", best_points)
                        .color(best)
                        .allow_hover(false),
                );
            }
            if !highlighted.is_empty() {
                plot.points(
                    egui_plot::Points::new("Current trial", highlighted)
                        .color(current)
                        .radius(5.0)
                        .allow_hover(false),
                );
            }
        });
    });
}

pub(super) struct Series {
    pub groups: [Vec<[f64; 2]>; 4],
    pub highlighted: Vec<[f64; 2]>,
    pub best_points: Vec<[f64; 2]>,
}

pub(super) fn series(data: &Data, ready: &ReadyFrame, p: &Presentation, history: bool) -> Series {
    let mut groups: [Vec<[f64; 2]>; 4] = Default::default();
    let mut highlighted = Vec::new();
    let mut best_points = Vec::new();
    let mut best = if data.minimize[p.history_objective] {
        f64::INFINITY
    } else {
        f64::NEG_INFINITY
    };
    let feas = data.study.view.feasibility();
    for &row in &ready.prefix.rows {
        let values = &data.objectives[row];
        let point = if history {
            let value = values[p.history_objective];
            if !value.is_finite() {
                continue;
            }
            // Match the existing history's objective-only cumulative best semantics.
            if data.minimize[p.history_objective] {
                best = best.min(value);
            } else {
                best = best.max(value);
            }
            best_points.push([data.number(row) as f64, best]);
            [data.number(row) as f64, value]
        } else {
            if !ready.prefix.ranks.contains_key(&row) {
                continue;
            }
            [values[p.x], values[p.y]]
        };
        let group = match feas.state(row) {
            FeasibilityState::Infeasible => 2,
            FeasibilityState::Unverified => 3,
            FeasibilityState::Feasible if !history && ready.prefix.ranks.get(&row) == Some(&0) => 0,
            FeasibilityState::Feasible => 1,
        };
        groups[group].push(point);
        if row == data.frames[ready.index].row {
            highlighted.push(point);
        }
    }
    Series {
        groups,
        highlighted,
        best_points,
    }
}
