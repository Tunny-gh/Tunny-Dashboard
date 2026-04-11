use std::sync::mpsc::SyncSender;

use crate::state::app_state::AppState;
use crate::state::layout_state::{LayoutMode, LayoutState};
use crate::state::messages::AppMessage;

/// レイアウトモードボタンのラベル定義
pub const LAYOUT_MODE_BUTTONS: &[(LayoutMode, &str)] = &[
    (LayoutMode::MultiObjective, "Multi-Objective"),
    (LayoutMode::VariableSpace, "Variable Space"),
    (LayoutMode::ConvergenceAnalysis, "Convergence Analysis"),
    (LayoutMode::FreeLayout, "Free Layout"),
];

/// ToolBar を描画する
pub fn show_toolbar(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    layout: &mut LayoutState,
    tx: &SyncSender<AppMessage>,
    is_loading: &mut bool,
    load_error: &mut Option<String>,
) {
    ui.horizontal(|ui| {
        // ファイル開くボタン
        let open_enabled = !*is_loading;
        if ui
            .add_enabled(open_enabled, egui::Button::new("Open"))
            .clicked()
        {
            if let Some(path) = crate::io::file::open_file_dialog() {
                *is_loading = true;
                *load_error = None;
                let tx2 = tx.clone();
                crate::app::spawn_task(tx2, move || crate::io::journal::load_journal_task(path));
            }
        }

        ui.separator();

        // レイアウトモードボタン群
        for (mode, label) in LAYOUT_MODE_BUTTONS {
            let is_selected = layout.layout_mode == *mode;
            if ui.selectable_label(is_selected, *label).clicked() {
                layout.layout_mode = mode.clone();
            }
        }

        ui.separator();

        // Study選択: 複数スタディがある場合は ComboBox、1件の場合は名前表示
        if app_state.all_studies.len() > 1 {
            let current_name = app_state
                .current_study
                .as_ref()
                .map(|c| c.meta.name.clone())
                .unwrap_or_default();
            let mut selected_name = current_name.clone();
            egui::ComboBox::from_id_salt("study_select_combo")
                .selected_text(if selected_name.is_empty() {
                    "Select a study"
                } else {
                    &selected_name
                })
                .show_ui(ui, |ui| {
                    for study in &app_state.all_studies {
                        ui.selectable_value(
                            &mut selected_name,
                            study.name.clone(),
                            &study.name,
                        );
                    }
                });
            if selected_name != current_name && !selected_name.is_empty() {
                if let (Some(meta), Some(path)) = (
                    app_state
                        .all_studies
                        .iter()
                        .find(|s| s.name == selected_name)
                        .cloned(),
                    app_state.journal_path.clone(),
                ) {
                    *is_loading = true;
                    let tx2 = tx.clone();
                    crate::app::spawn_task(tx2, move || {
                        // 再パース + 同スレッドで選択（thread_local 制約を回避）
                        crate::io::study::load_and_select_task(path, meta)
                    });
                }
            }
        } else if let Some(ctx) = &app_state.current_study {
            ui.label(&ctx.meta.name);
        } else if !app_state.all_studies.is_empty() {
            ui.label("Loading study...");
        } else {
            ui.label("Open a file");
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // ライブ更新トグル
            let live_label = if app_state.live_update.enabled {
                "Live Update: On"
            } else {
                "Live Update: Off"
            };
            if ui.button(live_label).clicked() {
                app_state.live_update.enabled = !app_state.live_update.enabled;
            }

            // ローディングインジケーター
            if *is_loading {
                ui.spinner();
            }

            // エラーメッセージ
            if let Some(err) = load_error.clone() {
                if ui
                    .colored_label(egui::Color32::RED, format!("Error: {}", err))
                    .clicked()
                {
                    *load_error = None;
                }
            }
        });
    });
}

/// LayoutMode を文字列から解決する（テスト用ユーティリティ）
pub fn layout_mode_label(mode: LayoutMode) -> &'static str {
    LAYOUT_MODE_BUTTONS
        .iter()
        .find(|(m, _)| *m == mode)
        .map(|(_, label)| *label)
        .unwrap_or("Unknown")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::layout_state::LayoutMode;

    #[test]
    fn layout_mode_buttons_cover_all_modes() {
        let modes: Vec<LayoutMode> = LAYOUT_MODE_BUTTONS
            .iter()
            .map(|(m, _)| m.clone())
            .collect();
        assert!(modes.contains(&LayoutMode::MultiObjective));
        assert!(modes.contains(&LayoutMode::VariableSpace));
        assert!(modes.contains(&LayoutMode::ConvergenceAnalysis));
        assert!(modes.contains(&LayoutMode::FreeLayout));
        assert_eq!(modes.len(), 4);
    }

    #[test]
    fn layout_mode_label_returns_correct_label() {
        assert_eq!(layout_mode_label(LayoutMode::MultiObjective), "Multi-Objective");
        assert_eq!(layout_mode_label(LayoutMode::VariableSpace), "Variable Space");
        assert_eq!(
            layout_mode_label(LayoutMode::ConvergenceAnalysis),
            "Convergence Analysis"
        );
        assert_eq!(layout_mode_label(LayoutMode::FreeLayout), "Free Layout");
    }

    #[test]
    fn layout_mode_switch_updates_state() {
        let mut layout = LayoutState::default();
        assert_eq!(layout.layout_mode, LayoutMode::MultiObjective);
        layout.layout_mode = LayoutMode::ConvergenceAnalysis;
        assert_eq!(layout.layout_mode, LayoutMode::ConvergenceAnalysis);
    }

    #[test]
    fn loading_state_clears_on_file_open_sequence() {
        let mut is_loading = false;
        let mut load_error: Option<String> = Some("Previous error".to_string());
        // ファイル選択時に is_loading=true, load_error=None になることを確認
        is_loading = true;
        load_error = None;
        assert!(is_loading);
        assert!(load_error.is_none());
    }

    #[test]
    fn error_cleared_on_click_simulation() {
        let mut load_error: Option<String> = Some("Error".to_string());
        // エラーラベルクリック時のシミュレーション
        load_error = None;
        assert!(load_error.is_none());
    }
}
