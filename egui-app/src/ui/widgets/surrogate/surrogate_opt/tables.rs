//! 推定最適点・予測パレートフロントの表形式（TrialTable スタイル）レンダリング。

use crate::state::messages::{SurrogateMultiOptUiResult, SurrogateOptUiResult};

/// 推定最適点を TrialTable と同じ表形式（各パラメータ列 + 予測目的値列、1 行）で表示する。
pub(super) fn render_best_point_table(ui: &mut egui::Ui, result: &SurrogateOptUiResult) {
    use egui_extras::{Column, TableBuilder};

    let n_params = result.best_params.len();
    egui::ScrollArea::horizontal()
        .id_salt("surrogate_best_point_scroll")
        .show(ui, |ui| {
            ui.visuals_mut().faint_bg_color = crate::theme::TABLE_STRIPE_BG();
            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .columns(Column::initial(90.0).at_least(50.0), n_params) // 各パラメータ
                .column(Column::initial(110.0).at_least(60.0)) // 予測目的値
                .header(20.0, |mut header| {
                    for (name, _) in &result.best_params {
                        header.col(|ui| {
                            ui.strong(name);
                        });
                    }
                    header.col(|ui| {
                        ui.strong(&result.objective_name);
                    });
                })
                .body(|mut body| {
                    body.row(18.0, |mut row| {
                        for (_, value) in &result.best_params {
                            row.col(|ui| {
                                ui.label(format!("{:.4}", value));
                            });
                        }
                        row.col(|ui| {
                            ui.monospace(format!("{:.6}", result.best_value));
                        });
                    });
                });
        });
}

/// 予測パレートフロントの各点を TrialTable と同じ表形式（目的列 + パラメータ列）で表示する。
pub(super) fn render_front_table(ui: &mut egui::Ui, result: &SurrogateMultiOptUiResult) {
    use egui_extras::{Column, TableBuilder};

    if result.front.is_empty() {
        return;
    }
    let n_obj = result.objective_names.len();
    let n_param = result.param_names.len();

    egui::ScrollArea::both()
        .max_height(200.0)
        .id_salt("surrogate_multi_front_scroll")
        .show(ui, |ui| {
            ui.visuals_mut().faint_bg_color = crate::theme::TABLE_STRIPE_BG();
            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .columns(Column::initial(80.0).at_least(50.0), n_obj) // 各目的
                .columns(Column::initial(80.0).at_least(50.0), n_param) // 各パラメータ
                .header(20.0, |mut header| {
                    for name in &result.objective_names {
                        header.col(|ui| {
                            ui.strong(name);
                        });
                    }
                    for name in &result.param_names {
                        header.col(|ui| {
                            ui.strong(name);
                        });
                    }
                })
                .body(|body| {
                    body.rows(18.0, result.front.len(), |mut row| {
                        let pt = &result.front[row.index()];
                        for v in &pt.values {
                            row.col(|ui| {
                                ui.monospace(format!("{:.6}", v));
                            });
                        }
                        for p in &pt.params {
                            row.col(|ui| {
                                ui.monospace(format!("{:.6}", p));
                            });
                        }
                    });
                });
        });
}
