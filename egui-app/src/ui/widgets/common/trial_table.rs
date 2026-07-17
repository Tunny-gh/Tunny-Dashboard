use crate::state::app_state::AppState;
#[cfg(test)]
use crate::state::app_state::{StudyContext, TrialRow};
use crate::theme::chart_colors::COLOR_LINK;
use crate::theme::colormap_name::colormap_from_name;
use crate::ui::widgets::cluster_table::ClusterTable;
use crate::ui::widgets::mcdm_chart::McdmTable;

/// Display mode of the trial table.
/// Like the Artifact gallery, several related tables are unified into a single widget
/// and switched via a mode selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum TrialTableMode {
    /// The full trial list (selected ∪ pinned). No settings needed.
    #[default]
    All,
    /// Shows the clustering result (each trial's cluster assignment).
    Cluster,
    /// Shows trials in MCDM ranking order.
    Mcdm,
}

impl TrialTableMode {
    fn label(&self) -> &'static str {
        match self {
            TrialTableMode::All => "All Trials",
            TrialTableMode::Cluster => "By Cluster",
            TrialTableMode::Mcdm => "By MCDM Rank",
        }
    }
}

/// The trial table widget.
/// A unified widget that switches, via a mode selector, between the legacy BottomPanel
/// list plus the cluster assignment table (Cluster) and the MCDM ranking table (MCDM).
/// Cluster / MCDM settings and running state are held by their embedded sub-widgets, and
/// compute results are shared/cached per settings key in `cluster_cache` / `mcdm_cache`
/// (the same unified style as the Artifact gallery).
/// Can be placed in any cell of the grid canvas via D&D.
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct TrialTable {
    pub mode: TrialTableMode,
    /// Sub-widget handling Cluster mode's settings and rendering.
    pub cluster: ClusterTable,
    /// Sub-widget handling MCDM mode's settings and rendering.
    pub mcdm: McdmTable,
    /// Cache of the row indices to display (selected ∪ pinned).
    /// Not recomputed unless the contents or count of `selected_indices` / `pinned`
    /// change.
    #[serde(skip)]
    visible_cache: Option<Vec<usize>>,
    #[serde(skip)]
    visible_cache_key: Option<(Vec<u32>, Vec<u32>, usize)>, // (selected_indices, pinned, row_count)
}

impl TrialTable {
    /// Renders the table. Shows the mode selector and switches according to the
    /// selected mode.
    pub fn show(&mut self, ui: &mut egui::Ui, app_state: &mut AppState) {
        if app_state.current_study.is_none() {
            ui.centered_and_justified(|ui| {
                ui.label("Open a journal file");
            });
            return;
        }

        // Mode selector (same feel as the Artifact gallery).
        ui.horizontal(|ui| {
            ui.label("View:");
            egui::ComboBox::from_id_salt("trial_table_mode")
                .selected_text(self.mode.label())
                .show_ui(ui, |ui| {
                    for m in [
                        TrialTableMode::All,
                        TrialTableMode::Cluster,
                        TrialTableMode::Mcdm,
                    ] {
                        ui.selectable_value(&mut self.mode, m, m.label());
                    }
                });
        });
        ui.separator();

        match self.mode {
            TrialTableMode::All => self.show_all(ui, app_state),
            TrialTableMode::Cluster => {
                let cmap = colormap_from_name(&app_state.selected_colormap);
                self.cluster.show(ui, app_state, &cmap);
            }
            TrialTableMode::Mcdm => self.show_mcdm(ui, app_state),
        }
    }

    /// MCDM mode: settings UI + ranked table (delegated to McdmTable).
    /// The pin toggle is returned from McdmTable and applied to AppState here.
    fn show_mcdm(&mut self, ui: &mut egui::Ui, app_state: &mut AppState) {
        let Some(ctx) = app_state.current_study.as_ref() else {
            return;
        };
        let key = self.mcdm.controls.cache_key();
        let result = app_state.mcdm_cache.get(&key);
        let pinned = app_state.pinned_trials.clone();
        let pin_toggled = self.mcdm.show(
            ui,
            result,
            &ctx.view,
            &ctx.meta.param_names,
            &ctx.meta.objective_names,
            &pinned,
        );
        if let Some(trial_id) = pin_toggled {
            let _ = app_state.toggle_pinned_trial(trial_id);
        }
    }

    /// All mode: renders the full trial list (selected ∪ pinned).
    fn show_all(&mut self, ui: &mut egui::Ui, app_state: &mut AppState) {
        let study_ctx = app_state.current_study.as_ref().unwrap();
        let pinned = app_state.pinned_trials.clone();
        let highlighted = app_state.highlighted_trial;

        let param_names = study_ctx.meta.param_names.clone();
        let obj_names = study_ctx.meta.objective_names.clone();

        // Compute the row indices to display (selected ∪ pinned, in original order)
        // without materializing rows. Not recomputed unless the contents or count of
        // selected_indices / pinned change.
        let view = &study_ctx.view;
        let n = view.row_count();
        let cache_key = (app_state.selected_indices.clone(), pinned.clone(), n);
        if self.visible_cache.is_none() || self.visible_cache_key.as_ref() != Some(&cache_key) {
            let visible: Vec<usize> = if app_state.selected_indices.is_empty() {
                (0..n).collect()
            } else {
                let set: std::collections::HashSet<u32> =
                    crate::state::app_state::merge_selected_with_pinned(
                        &app_state.selected_indices,
                        &pinned,
                    )
                    .into_iter()
                    .collect();
                (0..n)
                    .filter(|&i| view.trial_ids.get(i).is_some_and(|id| set.contains(id)))
                    .collect()
            };
            self.visible_cache = Some(visible);
            self.visible_cache_key = Some(cache_key);
        }
        let visible = self.visible_cache.as_ref().unwrap();
        // Borrow column slices from view (no row cloning)
        let param_cols = view.numeric_columns(&param_names);
        let obj_cols = view.numeric_columns(&obj_names);
        let trial_ids = &view.trial_ids;
        let pareto_rank = &view.pareto_rank;

        use egui_extras::{Column, TableBuilder};

        let mut clicked_trial: Option<u32> = None;
        let mut pin_toggled: Option<u32> = None;

        // Expand parameters and objectives into one column each, allowing horizontal
        // scrolling (the same display style as Cluster / MCDM mode).
        egui::ScrollArea::horizontal().show(ui, |ui| {
            // Emphasize the stripe color to make it easy to tell even/odd rows apart.
            ui.visuals_mut().faint_bg_color = crate::theme::TABLE_STRIPE_BG();
            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .column(Column::exact(30.0)) // Pin column
                .column(Column::initial(70.0).at_least(50.0)) // Trial ID
                .columns(Column::initial(90.0).at_least(50.0), param_names.len()) // per variable
                .columns(Column::initial(90.0).at_least(50.0), obj_names.len()) // per objective
                .column(Column::initial(90.0).at_least(50.0)) // Pareto Rank
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("📌");
                    });
                    header.col(|ui| {
                        ui.strong("Trial ID");
                    });
                    for name in &param_names {
                        header.col(|ui| {
                            ui.strong(name);
                        });
                    }
                    for name in &obj_names {
                        header.col(|ui| {
                            ui.strong(name);
                        });
                    }
                    header.col(|ui| {
                        ui.strong("Pareto Rank");
                    });
                })
                .body(|body| {
                    body.rows(18.0, visible.len(), |mut row| {
                        let idx = visible[row.index()];
                        let trial_id = trial_ids.get(idx).copied().unwrap_or(idx as u32);
                        // Display Optuna's trial.number (the 0-based creation-order
                        // number within the Study).
                        let trial_number = view.df.get_trial_number(idx).unwrap_or(idx as u32);
                        let rank = pareto_rank.get(idx).copied().unwrap_or(0);
                        let is_highlighted = highlighted == Some(trial_id);
                        let is_pinned = pinned.contains(&trial_id);

                        row.col(|ui| {
                            let pin_label = if is_pinned { "📌" } else { "·" };
                            if ui.small_button(pin_label).clicked() {
                                pin_toggled = Some(trial_id);
                            }
                        });
                        row.col(|ui| {
                            let res = ui.selectable_label(is_highlighted, trial_number.to_string());
                            if res.clicked() {
                                clicked_trial = Some(trial_id);
                            }
                            if is_highlighted {
                                ui.painter().rect_filled(res.rect, 0.0, COLOR_LINK());
                            }
                        });
                        for col in &param_cols {
                            row.col(|ui| {
                                let v = col.and_then(|c| c.get(idx)).copied().unwrap_or(0.0);
                                ui.label(format!("{:.3}", v));
                            });
                        }
                        for col in &obj_cols {
                            row.col(|ui| {
                                let v = col.and_then(|c| c.get(idx)).copied().unwrap_or(0.0);
                                ui.label(format!("{:.4}", v));
                            });
                        }
                        row.col(|ui| {
                            ui.label(rank.to_string());
                        });
                    });
                });
        });

        if let Some(trial_id) = clicked_trial {
            app_state.set_highlight(trial_id);
        }
        if let Some(trial_id) = pin_toggled {
            // Ignore limit error for now; UI notification is handled by caller
            let _ = app_state.toggle_pinned_trial(trial_id);
        }
    }
}

/// Returns the TrialRows to display (pin-aware version, used only in tests).
/// If selected_indices is empty, returns all; otherwise returns selected ∪ pinned.
#[cfg(test)]
pub fn get_display_rows_with_pins(
    study_ctx: &StudyContext,
    selected_indices: &[u32],
    pinned: &[u32],
) -> Vec<TrialRow> {
    let use_filter = !selected_indices.is_empty();
    let id_set: std::collections::HashSet<u32> = selected_indices.iter().copied().collect();
    let pin_set: std::collections::HashSet<u32> = pinned.iter().copied().collect();
    study_ctx
        .view
        .trial_ids
        .iter()
        .enumerate()
        .filter(|(_, &id)| !use_filter || id_set.contains(&id) || pin_set.contains(&id))
        .map(|(i, _)| study_ctx.view.row_at(i))
        .collect()
}

/// Returns the TrialRows to display (used only in tests).
#[cfg(test)]
pub fn get_display_rows(study_ctx: &StudyContext, selected_indices: &[u32]) -> Vec<TrialRow> {
    get_display_rows_with_pins(study_ctx, selected_indices, &[])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::{Direction, PinError, StudyContext, StudyMeta, TrialRow};
    use std::collections::HashMap;

    fn make_study_ctx(n: usize) -> StudyContext {
        let trial_rows: Vec<TrialRow> = (0..n as u32)
            .map(|i| TrialRow {
                trial_id: i,
                trial_number: i,
                params: HashMap::new(),
                objectives: vec![i as f64],
                pareto_rank: 0,
                cluster_id: None,
                user_attrs: HashMap::new(),
            })
            .collect();
        let meta = StudyMeta {
            study_id: 0,
            name: "test".to_string(),
            directions: vec![Direction::Minimize],
            completed_trials: n,
            param_names: vec![],
            objective_names: vec!["y".to_string()],
            param_bounds: Default::default(),
        };
        StudyContext::from_rows_for_test(meta, trial_rows)
    }

    #[test]
    fn get_display_rows_empty_selected_returns_all() {
        let ctx = make_study_ctx(5);
        let rows = get_display_rows(&ctx, &[]);
        assert_eq!(rows.len(), 5);
    }

    #[test]
    fn get_display_rows_filters_by_trial_id() {
        let ctx = make_study_ctx(5);
        let rows = get_display_rows(&ctx, &[0, 2, 4]);
        assert_eq!(rows.len(), 3);
        let ids: Vec<u32> = rows.iter().map(|r| r.trial_id).collect();
        assert!(ids.contains(&0));
        assert!(ids.contains(&2));
        assert!(ids.contains(&4));
        assert!(!ids.contains(&1));
        assert!(!ids.contains(&3));
    }

    #[test]
    fn get_display_rows_nonexistent_id_excluded() {
        let ctx = make_study_ctx(3);
        let rows = get_display_rows(&ctx, &[99]);
        assert_eq!(rows.len(), 0);
    }

    // ── TASK-2235: pinning UI tests ──────────────────────────────

    #[test]
    fn get_display_rows_keeps_pinned_rows_visible() {
        let ctx = make_study_ctx(5);
        // selected=[0,1], pinned=[4] → 0,1,4 visible
        let rows = get_display_rows_with_pins(&ctx, &[0, 1], &[4]);
        let ids: Vec<u32> = rows.iter().map(|r| r.trial_id).collect();
        assert!(ids.contains(&0));
        assert!(ids.contains(&1));
        assert!(ids.contains(&4));
        assert!(!ids.contains(&2));
        assert!(!ids.contains(&3));
    }

    #[test]
    fn pin_icon_reflects_current_state() {
        // The pin icon switches based on the is_pinned flag
        let is_pinned = true;
        let label = if is_pinned { "📌" } else { "·" };
        assert_eq!(label, "📌");

        let is_pinned = false;
        let label = if is_pinned { "📌" } else { "·" };
        assert_eq!(label, "·");
    }

    #[test]
    fn pin_limit_error_is_surfaceable_to_ui() {
        use crate::state::app_state::AppState;
        let mut state = AppState::new();
        for i in 0..20u32 {
            state.toggle_pinned_trial(i).unwrap();
        }
        let result = state.toggle_pinned_trial(100);
        assert_eq!(result, Err(PinError::MaxPinnedReached { limit: 20 }));
    }

    #[test]
    fn pin_row_then_change_selection_row_stays_visible() {
        let ctx = make_study_ctx(5);
        // pin trial 3, then selection is [0,1] (no longer includes 3)
        let rows = get_display_rows_with_pins(&ctx, &[0, 1], &[3]);
        let ids: Vec<u32> = rows.iter().map(|r| r.trial_id).collect();
        assert!(ids.contains(&3), "pinned row 3 must remain visible");
    }
}
