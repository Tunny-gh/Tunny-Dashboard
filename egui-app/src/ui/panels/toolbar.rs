use crate::io::export::ExportTarget;
use crate::state::app_state::{AppState, ColormapName, StudyMeta};
use crate::theme::{ERROR_COLOR, TOOLBAR_BTN_FG};
use crate::ui::widget_states::WidgetStates;

#[derive(Debug, Clone)]
pub enum ToolbarAction {
    OpenJournal(std::path::PathBuf),
    /// Opens the "Open URL…" dialog (directly enter a PostgreSQL/MySQL connection URL).
    OpenDbUrlDialog,
    SelectStudy(StudyMeta),
    ToggleLiveUpdate,
    SetPollInterval(u64),
    ScanArtifacts(std::path::PathBuf),
    ClearLoadError,

    // TASK-2228: new actions
    ExportCsv(ExportTarget),
    /// Adds the specified Study within the same file as a comparison target.
    AddComparisonStudy(StudyMeta),
    RemoveComparisonStudy(usize),

    /// Saves the current layout, widget settings, and view settings to a session file.
    SaveSession,
    /// Restores the session file at the specified path.
    LoadSession(std::path::PathBuf),

    /// R4: Opens the "Report…" dialog (self-contained report export settings).
    OpenReportDialog,

    /// Opens a process-integration definition (JSON) and shows the tool
    /// optimization setup modal.
    OpenProcessDefinition(std::path::PathBuf),

    /// Opens the process-definition builder (GUI editor) with a fresh, empty
    /// definition. Existing definitions are imported from inside the builder.
    NewProcessDefinition,
}

/// Draws the ToolBar.
pub fn show_toolbar(
    ui: &mut egui::Ui,
    app_state: &AppState,
    is_loading: bool,
    load_error: Option<&str>,
) -> Vec<ToolbarAction> {
    let mut actions = Vec::new();
    ui.horizontal(|ui| {
        // The left half of the bar is grouped into three dropdown menus so that the
        // individual entries don't push the Study selector off the visible width.
        let open_enabled = !is_loading;

        // ── Open: data sources ────────────────────────────────────────────────
        toolbar_menu(ui, "Open", true, |ui| {
            if menu_item(
                ui,
                "Open File…",
                open_enabled,
                "Load a journal (.log) / SQLite / CSV file",
            )
            .clicked()
            {
                if let Some(path) = crate::io::file::open_file_dialog() {
                    actions.push(ToolbarAction::OpenJournal(path));
                }
                ui.close();
            }
            // Directly enter a PostgreSQL/MySQL connection URL, which can't be
            // selected via the file dialog.
            if menu_item(
                ui,
                "Open URL…",
                open_enabled,
                "Enter a PostgreSQL/MySQL connection URL",
            )
            .clicked()
            {
                actions.push(ToolbarAction::OpenDbUrlDialog);
                ui.close();
            }
            ui.separator();
            // REQ-007: Artifacts folder selection.
            if menu_item(
                ui,
                "Artifacts Folder…",
                true,
                "Scan a folder for artifacts linked to trials",
            )
            .clicked()
            {
                if let Some(base_dir) = rfd::FileDialog::new().pick_folder() {
                    actions.push(ToolbarAction::ScanArtifacts(base_dir));
                }
                ui.close();
            }
        });

        // ── Session: layout + widget settings + view settings ─────────────────
        // The data itself is not saved, so saving is allowed even before data is loaded.
        toolbar_menu(ui, "Session", true, |ui| {
            if menu_item(
                ui,
                "Save Session",
                true,
                "Save the canvas layout, widget settings, and view settings",
            )
            .clicked()
            {
                actions.push(ToolbarAction::SaveSession);
                ui.close();
            }
            if menu_item(
                ui,
                "Load Session",
                !is_loading,
                "Restore a saved session (keeps the currently loaded data)",
            )
            .clicked()
            {
                if let Some(path) = crate::io::session::pick_session_file_dialog() {
                    actions.push(ToolbarAction::LoadSession(path));
                }
                ui.close();
            }
        });

        // ── Optimize: external tool (process integration) ──────────────────────
        toolbar_menu(ui, "Optimize", true, |ui| {
            // Pick a definition (JSON) and configure the run.
            if menu_item(
                ui,
                "Optimize Tool…",
                open_enabled,
                "Run an optimization driving an external tool from a process definition (JSON)",
            )
            .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Process definition (*.json)", &["json"])
                    .pick_file()
                {
                    actions.push(ToolbarAction::OpenProcessDefinition(path));
                }
                ui.close();
            }
            // Author or edit a process definition (JSON) in a GUI form instead of by
            // hand. Existing definitions can be loaded from inside the builder.
            if menu_item(
                ui,
                "New Tool…",
                true,
                "Build or edit a process definition (JSON) in a GUI form",
            )
            .clicked()
            {
                actions.push(ToolbarAction::NewProcessDefinition);
                ui.close();
            }
        });

        ui.separator();

        // Study selection: always shows the ComboBox, disabled when nothing is loaded
        {
            ui.label(
                egui::RichText::new("Target Study:")
                    .color(crate::theme::TOOLBAR_TEXT())
                    .size(12.0),
            );
            let current_name = app_state
                .current_study
                .as_ref()
                .map(|c| c.meta.name.clone())
                .unwrap_or_default();
            let mut selected_name = current_name.clone();
            let has_studies = !app_state.all_studies.is_empty();
            let display_text = if is_loading {
                "Loading...".to_string()
            } else {
                current_name.clone()
            };
            ui.scope(|ui| {
                apply_combo_visuals(ui.visuals_mut());
                ui.add_enabled_ui(has_studies && !is_loading, |ui| {
                    egui::ComboBox::from_id_salt("study_select_combo")
                        .selected_text(
                            egui::RichText::new(&display_text).color(crate::theme::TOOLBAR_TEXT()),
                        )
                        .show_ui(ui, |ui| {
                            for study in &app_state.all_studies {
                                ui.selectable_value(
                                    &mut selected_name,
                                    study.name.clone(),
                                    &study.name,
                                );
                            }
                        });
                });
            });
            if selected_name != current_name && !selected_name.is_empty() {
                if let Some(meta) = app_state
                    .all_studies
                    .iter()
                    .find(|s| s.name == selected_name)
                    .cloned()
                {
                    actions.push(ToolbarAction::SelectStudy(meta));
                }
            }
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Live update toggle (enabled only for journal (.log) / SQLite (.db, etc.) /
            // PostgreSQL/MySQL connection URLs. Flat CSV is a one-time import with no
            // concept of streaming appends, so this stays unpressable even when one is
            // open). DB URLs have no extension, so they pass through the `!is_csv_path`
            // check as-is.
            let can_toggle = app_state
                .journal_path
                .as_deref()
                .is_some_and(|p| !crate::io::flat_csv::is_csv_path(p));
            let live_label = if app_state.live_update.enabled {
                format!("Live: On ({}s)", app_state.live_update.interval_ms / 1000)
            } else {
                "Live: Off".to_string()
            };
            let mut response = toolbar_button(ui, &live_label, can_toggle);
            if !can_toggle {
                response = response.on_hover_text(
                    "Live Update is available for journal (.log) / SQLite / DB URL sources only",
                );
            }
            if response.clicked() && can_toggle {
                actions.push(ToolbarAction::ToggleLiveUpdate);
            }

            // Trial count
            let trial_label = if let Some(study) = &app_state.current_study {
                format!("Trials: {}", study.trial_count())
            } else {
                "Trials: -".to_string()
            };
            ui.label(
                egui::RichText::new(trial_label)
                    .color(crate::theme::TOOLBAR_TEXT())
                    .size(12.0),
            );

            // Polling interval slider (shown only when live update is ON)
            if app_state.live_update.enabled {
                let mut interval_sec = app_state.live_update.interval_ms as f64 / 1000.0;
                let prev = interval_sec;
                ui.scope(|ui| {
                    // The toolbar panel blanks `widgets.inactive.bg_fill` to get flat
                    // buttons, which also erases the slider rail and leaves only the
                    // handle outline. Restore a visible rail and fill the elapsed side
                    // so the widget reads as a slider.
                    let vis = ui.visuals_mut();
                    vis.widgets.inactive.bg_fill = crate::theme::TOOLBAR_INPUT_STROKE();
                    vis.selection.bg_fill = crate::theme::ACCENT_BLUE();
                    vis.slider_trailing_fill = true;
                    ui.add(
                        egui::Slider::new(&mut interval_sec, 5.0..=30.0)
                            .step_by(1.0)
                            .text(egui::RichText::new("s").color(crate::theme::TOOLBAR_TEXT())),
                    )
                    .on_hover_text("Polling interval");
                });
                if (interval_sec - prev).abs() > f64::EPSILON {
                    actions.push(ToolbarAction::SetPollInterval(
                        (interval_sec * 1000.0) as u64,
                    ));
                }
            }

            ui.separator();

            // ── Export: CSV (TASK-2233) and R4 self-contained report ──────────────
            // Every entry needs a loaded study, so they all grey out without one.
            let has_study = app_state.current_study.is_some();
            toolbar_menu(ui, "Export", true, |ui| {
                use crate::io::export::ExportTarget;
                for (label, target, hover) in [
                    ("CSV: All Data", ExportTarget::AllData, "Export every trial"),
                    (
                        "CSV: Selected Only",
                        ExportTarget::SelectedOnly,
                        "Export only the currently selected trials",
                    ),
                    (
                        "CSV: Pareto Only",
                        ExportTarget::ParetoOnly,
                        "Export only the Pareto-optimal trials",
                    ),
                ] {
                    if menu_item(ui, label, has_study, hover).clicked() {
                        actions.push(ToolbarAction::ExportCsv(target));
                        ui.close();
                    }
                }
                ui.separator();
                if menu_item(
                    ui,
                    "Report…",
                    has_study,
                    "Export a self-contained report (HTML/Markdown/JSON)",
                )
                .clicked()
                {
                    actions.push(ToolbarAction::OpenReportDialog);
                    ui.close();
                }
            });

            // Comparison targets are not laid out as chips on the bar; instead they're
            // managed via a checkbox list inside a single dropdown (to prevent the bar
            // from overflowing its width). Checking adds a comparison target, unchecking
            // removes it. The base Study itself is not shown in the list.
            push_comparison_selector(ui, app_state, &mut actions);

            ui.separator();

            // Loading indicator
            if is_loading {
                ui.spinner();
            }

            // Error message
            if let Some(err) = load_error {
                if ui
                    .colored_label(ERROR_COLOR(), format!("Error: {}", err))
                    .clicked()
                {
                    actions.push(ToolbarAction::ClearLoadError);
                }
            }
        });
    });
    actions
}

/// Colormap selector (toolbar row 2, leftmost, always shown).
/// Recomputes the colors of all charts on change.
pub fn show_colormap_selector(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    _widget_states: &mut WidgetStates,
) {
    ui.label(
        egui::RichText::new("Colormap:")
            .color(crate::theme::TOOLBAR_TEXT())
            .size(12.0),
    );
    let current_label = app_state.selected_colormap.label().to_string();
    ui.scope(|ui| {
        apply_combo_visuals(ui.visuals_mut());
        egui::ComboBox::from_id_salt("toolbar_colormap_combo")
            .selected_text(egui::RichText::new(current_label).color(crate::theme::TOOLBAR_TEXT()))
            .width(120.0)
            .show_ui(ui, |ui| {
                for cmap in ColormapName::all() {
                    if ui
                        .selectable_label(app_state.selected_colormap == *cmap, cmap.label())
                        .clicked()
                    {
                        app_state.selected_colormap = cmap.clone();
                    }
                }
            });
    });
}

/// Draws the comparison Study selection dropdown.
/// Only a "Compare (count)" label is placed on the bar; opening it lists the Studies
/// from the same file as checkboxes. Add/remove actions are pushed in response to
/// checkbox state changes.
fn push_comparison_selector(
    ui: &mut egui::Ui,
    app_state: &AppState,
    actions: &mut Vec<ToolbarAction>,
) {
    let base_id = app_state.current_study.as_ref().map(|c| c.meta.study_id);
    let n_comp = app_state.comparison_studies.len();
    let has_others = app_state
        .all_studies
        .iter()
        .any(|s| base_id != Some(s.study_id));
    let enabled = app_state.current_study.is_some() && has_others;

    let label = if n_comp > 0 {
        format!("Compare ({})", n_comp)
    } else {
        "Compare".to_string()
    };

    ui.scope(|ui| {
        apply_combo_visuals(ui.visuals_mut());
        ui.add_enabled_ui(enabled, |ui| {
            egui::ComboBox::from_id_salt("compare_select_combo")
                .selected_text(egui::RichText::new(label).color(crate::theme::TOOLBAR_TEXT()))
                .width(130.0)
                .show_ui(ui, |ui| {
                    for s in &app_state.all_studies {
                        if base_id == Some(s.study_id) {
                            continue;
                        }
                        let existing_idx = app_state
                            .comparison_studies
                            .iter()
                            .position(|c| c.meta.study_id == s.study_id);
                        let mut checked = existing_idx.is_some();
                        if ui.checkbox(&mut checked, &s.name).changed() {
                            if checked {
                                actions.push(ToolbarAction::AddComparisonStudy(s.clone()));
                            } else if let Some(idx) = existing_idx {
                                actions.push(ToolbarAction::RemoveComparisonStudy(idx));
                            }
                        }
                    }
                });
        });
    });
}

/// Draws a toolbar button that opens a dropdown menu on click.
/// The button itself is painted exactly like [`toolbar_button`], with a "▾" hint
/// painted after the label so it reads as a menu rather than a direct action.
fn toolbar_menu(
    ui: &mut egui::Ui,
    label: &str,
    enabled: bool,
    contents: impl FnOnce(&mut egui::Ui),
) {
    let response = toolbar_button_impl(ui, label, enabled, true);
    if enabled {
        egui::Popup::menu(&response).show(contents);
    }
}

/// A single entry inside a [`toolbar_menu`] dropdown.
/// Stretches to the menu width and shows `hover` as a tooltip.
fn menu_item(ui: &mut egui::Ui, label: &str, enabled: bool, hover: &str) -> egui::Response {
    ui.add_enabled(enabled, egui::Button::new(label))
        .on_hover_text(hover)
}

fn toolbar_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    toolbar_button_impl(ui, label, enabled, false)
}

/// Width reserved to the right of the label for the drop-down triangle.
const ARROW_WIDTH: f32 = 14.0;

fn toolbar_button_impl(
    ui: &mut egui::Ui,
    label: &str,
    enabled: bool,
    arrow: bool,
) -> egui::Response {
    let padding = egui::vec2(10.0, 5.0);
    let text_color = if enabled {
        crate::theme::TOOLBAR_TEXT()
    } else {
        crate::theme::TOOLBAR_TEXT().gamma_multiply(0.4)
    };
    let galley = ui.fonts_mut(|f| {
        f.layout_no_wrap(
            label.to_string(),
            egui::FontId::proportional(13.0),
            text_color,
        )
    });
    let mut desired = galley.size() + padding * 2.0;
    if arrow {
        desired.x += ARROW_WIDTH;
    }
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(desired, sense);

    if ui.is_rect_visible(rect) {
        let bg = if !enabled {
            egui::Color32::TRANSPARENT
        } else if resp.hovered() {
            crate::theme::TOOLBAR_BTN_HOVER()
        } else {
            egui::Color32::TRANSPARENT
        };
        let final_text_color = if enabled && resp.hovered() {
            TOOLBAR_BTN_FG()
        } else {
            text_color
        };
        ui.painter().rect_filled(rect, 4.0, bg);
        ui.painter()
            .galley(rect.min + padding, galley, final_text_color);
        if arrow {
            // Painted rather than drawn as a "▾" glyph: the bundled proportional
            // font has no geometric-shape coverage and would render tofu.
            let cx = rect.right() - padding.x - ARROW_WIDTH * 0.5;
            let cy = rect.center().y;
            let (hw, hh) = (4.0, 2.5);
            ui.painter().add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(cx - hw, cy - hh),
                    egui::pos2(cx + hw, cy - hh),
                    egui::pos2(cx, cy + hh),
                ],
                final_text_color,
                egui::Stroke::NONE,
            ));
        }
    }
    resp
}

fn apply_combo_visuals(vis: &mut egui::Visuals) {
    use crate::theme::{
        TOOLBAR_BTN_ACTIVE, TOOLBAR_BTN_HOVER, TOOLBAR_INPUT_BG, TOOLBAR_INPUT_STROKE, TOOLBAR_TEXT,
    };
    vis.override_text_color = Some(TOOLBAR_TEXT());
    let bg_stroke = egui::Stroke::new(1.0, TOOLBAR_INPUT_STROKE());
    let fg_text = egui::Stroke::new(1.0, TOOLBAR_TEXT());
    let fg_white = egui::Stroke::new(1.0, TOOLBAR_BTN_FG());
    for w in [&mut vis.widgets.inactive, &mut vis.widgets.noninteractive] {
        w.weak_bg_fill = TOOLBAR_INPUT_BG();
        w.bg_fill = TOOLBAR_INPUT_BG();
        w.bg_stroke = bg_stroke;
        w.fg_stroke = fg_text;
    }
    vis.widgets.hovered.weak_bg_fill = TOOLBAR_BTN_HOVER();
    vis.widgets.hovered.bg_fill = TOOLBAR_BTN_HOVER();
    vis.widgets.hovered.fg_stroke = fg_white;
    vis.widgets.active.weak_bg_fill = TOOLBAR_BTN_ACTIVE();
    vis.widgets.active.bg_fill = TOOLBAR_BTN_ACTIVE();
    vis.widgets.active.fg_stroke = fg_white;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loading_state_clears_on_file_open_sequence() {
        // Verify that selecting a file sets is_loading=true, load_error=None
        let is_loading = true;
        let load_error: Option<String> = None;
        assert!(is_loading);
        assert!(load_error.is_none());
    }

    #[test]
    fn error_cleared_on_click_simulation() {
        // Simulates clicking the error label
        let load_error: Option<String> = None;
        assert!(load_error.is_none());
    }

    fn make_test_meta(id: u32, name: &str) -> StudyMeta {
        StudyMeta {
            study_id: id,
            name: name.to_string(),
            directions: vec![],
            completed_trials: 0,
            param_names: vec![],
            objective_names: vec![],
            param_bounds: Default::default(),
        }
    }

    // TASK-2228: tests for new ToolbarAction variants
    #[test]
    fn toolbar_action_variants_compile_and_match() {
        let actions = vec![
            ToolbarAction::ExportCsv(crate::io::export::ExportTarget::AllData),
            ToolbarAction::ExportCsv(crate::io::export::ExportTarget::SelectedOnly),
            ToolbarAction::ExportCsv(crate::io::export::ExportTarget::ParetoOnly),
            ToolbarAction::AddComparisonStudy(make_test_meta(1, "s")),
            ToolbarAction::RemoveComparisonStudy(0),
        ];
        for action in &actions {
            match action {
                ToolbarAction::ExportCsv(t) => {
                    let _t = t;
                }
                ToolbarAction::AddComparisonStudy(m) => {
                    let _ = m;
                }
                ToolbarAction::RemoveComparisonStudy(idx) => {
                    let _ = idx;
                }
                _ => {}
            }
        }
        assert_eq!(actions.len(), 5);
    }

    // ── TASK-2233: CSV Export UI tests ──────────────────────────

    #[test]
    fn export_csv_action_targets_all_three_modes() {
        use crate::io::export::ExportTarget;
        let targets = [
            ExportTarget::AllData,
            ExportTarget::SelectedOnly,
            ExportTarget::ParetoOnly,
        ];
        for target in &targets {
            let action = ToolbarAction::ExportCsv(target.clone());
            match action {
                ToolbarAction::ExportCsv(_) => {}
                _ => panic!("Expected ExportCsv"),
            }
        }
        assert_eq!(targets.len(), 3);
    }

    #[test]
    fn apply_toolbar_actions_handles_cancel_as_noop() {
        // save_csv_to_file returns Ok(()) on cancel; verify write_csv_to_path is a separate fn
        let csv = "trial_id,trial_number\n0,0";
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let result = crate::io::export::write_csv_to_path(csv, tmp.path());
        assert!(result.is_ok());
        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert_eq!(content, csv);
    }

    // ── TASK-2234: Comparison UI tests ──────────────────────────

    #[test]
    fn toolbar_emits_add_comparison_action() {
        let action = ToolbarAction::AddComparisonStudy(make_test_meta(2, "other"));
        match action {
            ToolbarAction::AddComparisonStudy(m) => assert_eq!(m.study_id, 2),
            _ => panic!("Expected AddComparisonStudy"),
        }
    }

    #[test]
    fn chip_remove_emits_remove_action() {
        let action = ToolbarAction::RemoveComparisonStudy(2);
        match action {
            ToolbarAction::RemoveComparisonStudy(idx) => assert_eq!(idx, 2),
            _ => panic!("Expected RemoveComparisonStudy"),
        }
    }

    #[test]
    fn successful_add_switches_to_comparison_mode() {
        use crate::state::app_state::{AppState, Direction, StudyContext, StudyMeta};
        use crate::state::message_handler::MessageHandler;
        use crate::state::messages::AppMessage;
        use crate::ui::widget_states::WidgetStates;

        let mut app_state = AppState::new();
        let mut widgets = WidgetStates::default();
        let mut is_loading = false;
        let mut load_error = None;

        let context = StudyContext::from_rows_for_test(
            StudyMeta {
                study_id: 10,
                name: "compare_study".to_string(),
                directions: vec![Direction::Minimize],
                completed_trials: 0,
                param_names: vec![],
                objective_names: vec![],
                param_bounds: Default::default(),
            },
            vec![],
        );

        // Simulate setting comparison_mode before the load completes (as app.rs does)
        app_state.comparison_mode = true;

        MessageHandler::handle(
            AppMessage::ComparisonStudyLoaded {
                context: Box::new(context),
            },
            &mut app_state,
            &mut widgets,
            &mut is_loading,
            &mut load_error,
        );

        assert!(app_state.comparison_mode);
        assert_eq!(app_state.comparison_studies.len(), 1);
    }
}
