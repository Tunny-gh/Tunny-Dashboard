//! trial state ごとの表示色・凡例（Intermediate Values / Timeline 共通）。
//!
//! 両ウィジェットとも trial state（RUNNING/COMPLETE/PRUNED/FAIL/WAITING）で
//! 色分けするため、色決定と凡例描画をここへ切り出して重複を避ける。

use tunny_core::extras::TrialState;

use crate::theme::chart_colors::{
    COLOR_STATE_COMPLETE, COLOR_STATE_FAIL, COLOR_STATE_PRUNED, COLOR_STATE_RUNNING,
    COLOR_STATE_WAITING,
};

/// trial state に対応する表示色を返す。
pub fn state_color(state: TrialState) -> egui::Color32 {
    match state {
        TrialState::Complete => COLOR_STATE_COMPLETE(),
        TrialState::Pruned => COLOR_STATE_PRUNED(),
        TrialState::Running => COLOR_STATE_RUNNING(),
        TrialState::Fail => COLOR_STATE_FAIL(),
        TrialState::Waiting => COLOR_STATE_WAITING(),
    }
}

/// `present` に含まれる state のみ、固定順（Complete/Pruned/Running/Fail/Waiting）で
/// 色スウォッチ + ラベルの凡例を描画する。
pub fn show_state_legend(ui: &mut egui::Ui, present: &[TrialState]) {
    const ORDER: [TrialState; 5] = [
        TrialState::Complete,
        TrialState::Pruned,
        TrialState::Running,
        TrialState::Fail,
        TrialState::Waiting,
    ];
    ui.horizontal(|ui| {
        for state in ORDER {
            if !present.contains(&state) {
                continue;
            }
            let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 2.0, state_color(state));
            ui.label(
                egui::RichText::new(state.label())
                    .small()
                    .color(crate::theme::TEXT_SECONDARY()),
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_color_is_distinct_per_state() {
        let colors = [
            state_color(TrialState::Complete),
            state_color(TrialState::Pruned),
            state_color(TrialState::Running),
            state_color(TrialState::Fail),
            state_color(TrialState::Waiting),
        ];
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(colors[i], colors[j], "colors[{i}] == colors[{j}]");
            }
        }
    }
}
