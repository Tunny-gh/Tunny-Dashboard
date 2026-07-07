//! 獲得関数（単目的）・EHVI（多目的）による候補提案の結果テーブル描画。
//!
//! いずれも結果テーブルと "Copy enqueue JSON" ボタン（Optuna の
//! `study.enqueue_trial(params)` に渡せる JSON 配列を生成）を表示する。

/// 獲得関数による候補提案の結果テーブルと "Copy enqueue JSON" ボタンを描画する。
pub(super) fn render_suggest_result(
    ui: &mut egui::Ui,
    result: &crate::state::messages::SurrogateSuggestUiResult,
) {
    if result.candidates.is_empty() {
        return;
    }

    ui.add_space(4.0);
    ui.strong(format!(
        "Suggested candidates for '{}':",
        result.objective_name
    ));

    egui::ScrollArea::vertical()
        .max_height(200.0)
        .id_salt("surrogate_suggest_scroll")
        .show(ui, |ui| {
            egui::Grid::new("surrogate_suggest_table")
                .striped(true)
                .min_col_width(60.0)
                .show(ui, |ui| {
                    // ── ヘッダ行 ──────────────────────────────────────
                    let has_feas = result
                        .candidates
                        .first()
                        .map(|c| c.feasibility_probability.is_some())
                        .unwrap_or(false);
                    for name in &result.param_names {
                        ui.strong(name);
                    }
                    ui.strong("Predicted");
                    ui.strong("Std");
                    if has_feas {
                        ui.strong("P(feas)");
                    }
                    ui.strong("Acq. score");
                    ui.end_row();

                    // ── データ行 ──────────────────────────────────────
                    for c in &result.candidates {
                        for v in &c.params {
                            ui.monospace(format!("{:.6}", v));
                        }
                        ui.monospace(format!("{:.6}", c.predicted_value));
                        match c.predicted_std {
                            Some(std) => ui.monospace(format!("±{:.6}", std)),
                            None => ui.label("—"),
                        };
                        if has_feas {
                            match c.feasibility_probability {
                                Some(p) => {
                                    let pct = (p * 100.0).round() as u32;
                                    let color = if p >= 0.8 {
                                        egui::Color32::from_rgb(22, 163, 74)
                                    } else if p >= 0.5 {
                                        egui::Color32::from_rgb(202, 138, 4)
                                    } else {
                                        egui::Color32::RED
                                    };
                                    ui.colored_label(color, format!("{}%", pct));
                                }
                                None => {
                                    ui.label("—");
                                }
                            };
                        }
                        ui.monospace(format!("{:.4e}", c.acq_score));
                        ui.end_row();
                    }
                });
        });

    ui.add_space(4.0);

    // ── "Copy enqueue JSON" ボタン ──────────────────────────────
    // Optuna の study.enqueue_trial(params) に渡せる JSON 配列を生成する。
    if ui
        .button("Copy enqueue JSON")
        .on_hover_text(
            "Optuna の study.enqueue_trial(params) に渡せる形式でクリップボードへコピーします。",
        )
        .clicked()
    {
        let json_items: Vec<serde_json::Value> = result
            .candidates
            .iter()
            .map(|c| {
                let obj: serde_json::Map<String, serde_json::Value> = result
                    .param_names
                    .iter()
                    .zip(c.params.iter())
                    .map(|(name, &val)| (name.clone(), serde_json::Value::from(val)))
                    .collect();
                serde_json::Value::Object(obj)
            })
            .collect();
        let json_str = serde_json::to_string_pretty(&json_items).unwrap_or_default();
        ui.ctx().copy_text(json_str);
    }
}

/// EHVI による多目的候補提案の結果テーブルと "Copy enqueue JSON" ボタンを描画する。
pub(super) fn render_multi_suggest_result(
    ui: &mut egui::Ui,
    result: &crate::state::messages::SurrogateMultiSuggestUiResult,
) {
    if result.candidates.is_empty() {
        return;
    }

    ui.add_space(4.0);
    ui.strong("Suggested candidates (EHVI):");

    egui::ScrollArea::vertical()
        .max_height(200.0)
        .id_salt("surrogate_multi_suggest_scroll")
        .show(ui, |ui| {
            egui::Grid::new("surrogate_multi_suggest_table")
                .striped(true)
                .min_col_width(60.0)
                .show(ui, |ui| {
                    // ── ヘッダ行 ──────────────────────────────────────
                    for name in &result.param_names {
                        ui.strong(name);
                    }
                    // 目的ごとに「予測値 ± std」列を 1 つにまとめる。
                    for name in &result.objective_names {
                        ui.strong(name);
                    }
                    ui.strong("EHVI");
                    ui.end_row();

                    // ── データ行 ──────────────────────────────────────
                    for c in &result.candidates {
                        for v in &c.params {
                            ui.monospace(format!("{:.6}", v));
                        }
                        for (k, val) in c.predicted_values.iter().enumerate() {
                            match c.predicted_stds.get(k).and_then(|s| *s) {
                                Some(std) => ui.monospace(format!("{:.4} ± {:.4}", val, std)),
                                None => ui.monospace(format!("{:.4}", val)),
                            };
                        }
                        ui.monospace(format!("{:.4e}", c.ehvi_score));
                        ui.end_row();
                    }
                });
        });

    ui.add_space(4.0);

    // ── "Copy enqueue JSON" ボタン（params のみのオブジェクト配列） ──
    if ui
        .button("Copy enqueue JSON")
        .on_hover_text(
            "Optuna の study.enqueue_trial(params) に渡せる形式でクリップボードへコピーします。",
        )
        .clicked()
    {
        let json_items: Vec<serde_json::Value> = result
            .candidates
            .iter()
            .map(|c| {
                let obj: serde_json::Map<String, serde_json::Value> = result
                    .param_names
                    .iter()
                    .zip(c.params.iter())
                    .map(|(name, &val)| (name.clone(), serde_json::Value::from(val)))
                    .collect();
                serde_json::Value::Object(obj)
            })
            .collect();
        let json_str = serde_json::to_string_pretty(&json_items).unwrap_or_default();
        ui.ctx().copy_text(json_str);
    }
}
