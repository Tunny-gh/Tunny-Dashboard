use crate::state::app_state::{AppState, ColorMode, ColormapName};
use crate::state::layout_state::LayoutState;
use crate::state::messages::AppMessage;

/// LeftPanel を描画する（フィルター専用、チャート選択は右パネルへ移動）
pub fn show_left_panel(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    _layout: &mut LayoutState,
    tx: &std::sync::mpsc::SyncSender<AppMessage>,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        show_study_info(ui, app_state);
        ui.separator();
        show_filter_sliders(ui, app_state);
        ui.separator();
        show_color_mode(ui, app_state);
        show_colormap_selector(ui, app_state);

        // REQ-001: Trade-off Navigator（多目的 Study 時のみ）
        let (obj_names, is_minimize) = if let Some(ctx) = &app_state.current_study {
            let names = ctx.meta.objective_names.clone();
            let minimize: Vec<bool> = ctx
                .meta
                .directions
                .iter()
                .map(|d| matches!(d, crate::state::types::Direction::Minimize))
                .collect();
            (names, minimize)
        } else {
            (vec![], vec![])
        };
        if obj_names.len() >= 2 {
            show_tradeoff_navigator(ui, app_state, &obj_names, &is_minimize, tx);
        } else if obj_names.len() == 1 {
            // REQ-008: Convergence Card（単目的）
            show_convergence_card(ui, app_state);
        }
    });
}

/// Study情報セクション
fn show_study_info(ui: &mut egui::Ui, app_state: &AppState) {
    ui.heading("Study Info");
    if let Some(ctx) = &app_state.current_study {
        ui.label(format!("Study: {}", ctx.meta.name));
        ui.label(format!("Trials: {}", ctx.meta.completed_trials));
        ui.label(format!("Objectives: {}", ctx.meta.objective_names.len()));
        ui.label(format!("Parameters: {}", ctx.meta.param_names.len()));
        ui.label(format!(
            "Selected: {} / {}",
            app_state.selected_indices.len(),
            ctx.trial_rows.len()
        ));
    } else {
        ui.label("Open a file");
    }
}

/// 変数フィルタースライダー
fn show_filter_sliders(ui: &mut egui::Ui, app_state: &mut AppState) {
    let param_names: Vec<String> = app_state
        .current_study
        .as_ref()
        .map(|ctx| ctx.meta.param_names.clone())
        .unwrap_or_default();

    if param_names.is_empty() {
        return;
    }

    ui.collapsing("Filters", |ui| {
        for param_name in &param_names {
            let (data_min, data_max) = app_state
                .current_study
                .as_ref()
                .map(|ctx| ctx.param_range(param_name))
                .unwrap_or((0.0, 1.0));

            let (mut filter_min, mut filter_max) = app_state
                .filter_ranges
                .get(param_name)
                .copied()
                .unwrap_or((data_min, data_max));

            ui.label(param_name);
            let mut changed = false;
            changed |= ui
                .add(
                    egui::Slider::new(&mut filter_min, data_min..=data_max)
                        .text("min")
                        .clamping(egui::SliderClamping::Always),
                )
                .changed();
            changed |= ui
                .add(
                    egui::Slider::new(&mut filter_max, data_min..=data_max)
                        .text("max")
                        .clamping(egui::SliderClamping::Always),
                )
                .changed();

            // min <= max を保証
            if filter_min > filter_max {
                filter_min = filter_max;
            }

            if changed {
                app_state.set_filter(param_name, filter_min, filter_max);
            }
        }
    });
}

/// カラーモード選択
fn show_color_mode(ui: &mut egui::Ui, app_state: &mut AppState) {
    ui.label("Color Mode:");
    let current_label = app_state.color_mode.label().to_string();
    let mut changed = false;
    egui::ComboBox::from_id_salt("left_panel_color_mode_combo")
        .selected_text(current_label)
        .show_ui(ui, |ui| {
            changed |= ui
                .selectable_value(
                    &mut app_state.color_mode,
                    ColorMode::ParetoRank,
                    "Pareto Rank",
                )
                .changed();
            changed |= ui
                .selectable_value(
                    &mut app_state.color_mode,
                    ColorMode::TrialNumber,
                    "Trial Number",
                )
                .changed();
            if let Some(ctx) = &app_state.current_study {
                for obj_name in &ctx.meta.objective_names {
                    let mode = ColorMode::ObjectiveValue(obj_name.clone());
                    changed |= ui
                        .selectable_value(
                            &mut app_state.color_mode,
                            mode,
                            format!("Objective: {}", obj_name),
                        )
                        .changed();
                }
            }
            if app_state.cluster_result.is_some() {
                changed |= ui
                    .selectable_value(
                        &mut app_state.color_mode,
                        ColorMode::ClusterId,
                        "Cluster ID",
                    )
                    .changed();
            }
            if app_state.mcdm_result.is_some() {
                changed |= ui
                    .selectable_value(
                        &mut app_state.color_mode,
                        ColorMode::McdmScore,
                        "MCDM Score",
                    )
                    .changed();
            }
        });
    if changed {
        app_state.update_chart_colors();
    }
}

/// カラーマップ選択セレクタ
fn show_colormap_selector(ui: &mut egui::Ui, app_state: &mut AppState) {
    ui.label("Colormap:");
    let current_label = app_state.selected_colormap.label().to_string();
    let mut changed = false;
    egui::ComboBox::from_id_salt("left_panel_colormap_combo")
        .selected_text(current_label)
        .show_ui(ui, |ui| {
            for cmap in ColormapName::all() {
                if ui
                    .selectable_label(app_state.selected_colormap == *cmap, cmap.label())
                    .clicked()
                {
                    app_state.selected_colormap = cmap.clone();
                    changed = true;
                }
            }
        });
    if changed {
        app_state.update_chart_colors();
    }
}

// ============================================================
// TASK-2119: Trade-off Navigator UI
// ============================================================

/// 重みベクタの合計が 1.0 になるよう正規化する（合計が 0 なら均等分割）
pub fn normalize_weights(weights: &mut [f64]) {
    let sum: f64 = weights.iter().sum();
    if sum > f64::EPSILON {
        for w in weights.iter_mut() {
            *w /= sum;
        }
    } else if !weights.is_empty() {
        let uniform = 1.0 / weights.len() as f64;
        for w in weights.iter_mut() {
            *w = uniform;
        }
    }
}

/// Trade-off Navigator セクション（多目的 Study 時のみ表示）
pub fn show_tradeoff_navigator(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    objective_names: &[String],
    is_minimize: &[bool],
    tx: &std::sync::mpsc::SyncSender<AppMessage>,
) {
    if objective_names.len() < 2 {
        return;
    }

    ui.collapsing("[*] Trade-off Navigator", |ui| {
        // 重みをリサイズ（目的数に合わせる）
        if app_state.tradeoff_weights.len() != objective_names.len() {
            let n = objective_names.len();
            app_state.tradeoff_weights = vec![1.0 / n as f64; n];
        }

        let mut changed = false;

        // REQ-001-B: 各目的のスライダー
        for (i, name) in objective_names.iter().enumerate() {
            let mut val = app_state.tradeoff_weights[i] as f32;
            if ui
                .add(egui::Slider::new(&mut val, 0.0_f32..=1.0_f32).text(name))
                .changed()
            {
                app_state.tradeoff_weights[i] = val as f64;
                changed = true;
            }
        }

        // REQ-001-C: 正規化 + REQ-001-D: 非同期スコアリング
        if changed {
            normalize_weights(&mut app_state.tradeoff_weights);
            crate::state::message_handler::MessageHandler::trigger_tradeoff_computation(
                app_state.tradeoff_weights.clone(),
                is_minimize.to_vec(),
                tx.clone(),
            );
        }

        // REQ-001-E: 最良解の表示
        if let Some(indices) = &app_state.tradeoff_sorted_indices {
            if let Some(&best_id) = indices.first() {
                ui.label(format!("[*] Best Trial: #{best_id}"));
            }
        }
    });
}

// ============================================================
// TASK-2120: Convergence Card UI
// ============================================================

/// 直近 last_n 試行で Best 値が改善された割合を計算する
pub fn compute_improvement_rate(history: &[(u32, f64)], last_n: usize) -> f64 {
    let window: Vec<_> = history.iter().rev().take(last_n).collect();
    if window.len() < 2 {
        return 0.0;
    }
    let mut best_so_far = f64::INFINITY;
    let mut improved_count = 0usize;
    for &(_, val) in window.iter().rev() {
        if *val < best_so_far {
            best_so_far = *val;
            improved_count += 1;
        }
    }
    (improved_count as f64) / (window.len() as f64)
}

/// 単目的 Study の Best 値推移を計算して返す（minimization 前提）
pub fn build_best_trial_history(
    trials: &[crate::state::types::TrialRow],
    objective_idx: usize,
    is_minimize: bool,
) -> Vec<(u32, f64)> {
    let mut history = Vec::with_capacity(trials.len());
    let mut best = if is_minimize {
        f64::INFINITY
    } else {
        f64::NEG_INFINITY
    };
    for trial in trials {
        if let Some(&obj_val) = trial.objectives.get(objective_idx) {
            let improved = if is_minimize {
                obj_val < best
            } else {
                obj_val > best
            };
            if improved {
                best = obj_val;
            }
            history.push((trial.trial_id, best));
        }
    }
    history
}

/// 収束診断カード（単目的 Study 時のみ表示）
pub fn show_convergence_card(ui: &mut egui::Ui, app_state: &AppState) {
    ui.collapsing("[+] Convergence", |ui| {
        match &app_state.best_trial_history {
            None => {
                ui.label("No data");
            }
            Some(history) if history.is_empty() => {
                ui.label("No trials");
            }
            Some(history) => {
                let (best_trial_id, best_value) = history
                    .iter()
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .copied()
                    .unwrap();
                ui.label(format!("Best: {:.6}", best_value));
                ui.label(format!("Best Trial: #{best_trial_id}"));
                let rate = compute_improvement_rate(history, 100);
                ui.label(format!("Improvement (last 100): {:.1}%", rate * 100.0));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::{AppState, ColorMode};
    use crate::state::layout_state::ChartId;

    #[test]
    fn set_filter_updates_filter_ranges() {
        let mut state = AppState::new();
        state.set_filter("x", 0.0, 0.5);
        let range = state.filter_ranges.get("x").copied();
        assert_eq!(range, Some((0.0, 0.5)));
    }

    #[test]
    fn set_filter_overwrites_existing() {
        let mut state = AppState::new();
        state.set_filter("x", 0.0, 1.0);
        state.set_filter("x", 0.2, 0.8);
        let range = state.filter_ranges.get("x").copied();
        assert_eq!(range, Some((0.2, 0.8)));
    }

    #[test]
    fn color_mode_label_variants() {
        assert_eq!(ColorMode::ParetoRank.label(), "Pareto Rank");
        assert_eq!(ColorMode::TrialNumber.label(), "Trial Number");
        assert_eq!(ColorMode::ClusterId.label(), "Cluster ID");
        assert_eq!(ColorMode::McdmScore.label(), "MCDM Score");
        assert_eq!(
            ColorMode::ObjectiveValue("y".to_string()).label(),
            "Objective"
        );
    }

    #[test]
    fn color_mode_switch_updates_state() {
        let mut state = AppState::new();
        assert_eq!(state.color_mode, ColorMode::ParetoRank);
        state.color_mode = ColorMode::TrialNumber;
        assert_eq!(state.color_mode, ColorMode::TrialNumber);
    }

    #[test]
    fn chart_id_all_returns_all_variants() {
        let all = ChartId::all();
        assert_eq!(all.len(), 13);
        assert!(all.contains(&ChartId::ParetoScatter2D));
        assert!(all.contains(&ChartId::ClusterScatter));
        assert!(all.contains(&ChartId::McdmRankChart));
        assert!(all.contains(&ChartId::McdmTable));
    }

    #[test]
    fn chart_id_label_not_empty() {
        for chart_id in ChartId::all() {
            assert!(!chart_id.label().is_empty());
        }
    }

    // TASK-2120 tests
    #[test]
    fn improvement_rate_all_improving() {
        let history = vec![(0u32, 1.0_f64), (1, 0.8), (2, 0.5)];
        let rate = compute_improvement_rate(&history, 100);
        assert!(rate > 0.0);
    }

    #[test]
    fn improvement_rate_empty_returns_zero() {
        let rate = compute_improvement_rate(&[], 100);
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn build_best_trial_history_minimize() {
        use crate::state::types::TrialRow;
        let mut t0 = TrialRow::default();
        t0.trial_id = 0;
        t0.objectives = vec![1.0];
        let mut t1 = TrialRow::default();
        t1.trial_id = 1;
        t1.objectives = vec![0.5];
        let mut t2 = TrialRow::default();
        t2.trial_id = 2;
        t2.objectives = vec![0.8];
        let history = build_best_trial_history(&[t0, t1, t2], 0, true);
        assert_eq!(history.len(), 3);
        assert_eq!(history[0], (0, 1.0));
        assert_eq!(history[1], (1, 0.5));
        assert_eq!(history[2], (2, 0.5));
    }

    #[test]
    fn normalize_weights_sum_to_one() {
        let mut weights = vec![1.0, 2.0, 1.0];
        normalize_weights(&mut weights);
        let sum: f64 = weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }
}
