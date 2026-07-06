//! trial state ごとの表示色・凡例（Intermediate Values / Timeline 共通）。
//!
//! 両ウィジェットとも trial state（RUNNING/COMPLETE/PRUNED/FAIL/WAITING）で
//! 色分けするため、色決定と凡例描画をここへ切り出して重複を避ける。

use tunny_core::extras::TrialState;

use crate::theme::chart_colors::{
    COLOR_EMPTY_STATE, COLOR_STATE_COMPLETE, COLOR_STATE_FAIL, COLOR_STATE_PRUNED,
    COLOR_STATE_RUNNING, COLOR_STATE_WAITING,
};

/// state の出現順（重複なし）リストを作る。Intermediate Values / Timeline の
/// 凡例に載せる state 集合を作る共通ヘルパー（D-12）。
pub fn distinct_states_in_order<I: IntoIterator<Item = TrialState>>(states: I) -> Vec<TrialState> {
    let mut present: Vec<TrialState> = Vec::new();
    for s in states {
        if !present.contains(&s) {
            present.push(s);
        }
    }
    present
}

/// hover 中でない曲線・バーを薄く見せる（アルファのみ落とす）。
/// Intermediate Values / Timeline 共通（D-12）。
pub fn dim(color: egui::Color32) -> egui::Color32 {
    let [r, g, b, _] = color.to_array();
    egui::Color32::from_rgba_unmultiplied(r, g, b, 90)
}

/// 空状態メッセージを中央に表示する（Intermediate Values / Timeline 共通・D-12）。
pub fn empty_state(ui: &mut egui::Ui, message: &str) {
    ui.centered_and_justified(|ui| {
        ui.colored_label(COLOR_EMPTY_STATE(), message);
    });
}

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
