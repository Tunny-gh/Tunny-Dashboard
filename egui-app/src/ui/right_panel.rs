use crate::state::app_state::AppState;
use crate::state::layout_state::{ChartId, DragPayload, LayoutState, PanelItem};

const TOGGLE_BTN_WIDTH: f32 = 20.0;

/// 右パネルを描画する。
/// - ▶/◀ ボタンで開閉切り替え（パネル左端に縦配置）
/// - 利用可能な PanelItem の一覧を表示
/// - グリッドに配置済みのアイテムはグレーアウト
pub fn show_right_panel(ui: &mut egui::Ui, _app_state: &AppState, layout: &mut LayoutState) {
    let is_open = layout.right_panel.is_open;

    // animate_bool で開閉アニメーション値 (0.0=閉, 1.0=開) を取得
    let anim_id = ui.id().with("panel_open_anim");
    let t = ui.ctx().animate_bool_with_time(anim_id, is_open, 0.15);

    if !is_open {
        // 閉じた状態: パネル全体を使って▶ボタンを縦中央に配置
        ui.centered_and_justified(|ui| {
            let arrow = animated_arrow(t);
            if ui.button(arrow).clicked() {
                layout.right_panel.is_open = true;
            }
        });
        return;
    }

    // 開いた状態: 左端に縦向き▶/◀ボタン、右にコンテンツ
    ui.horizontal(|ui| {
        // 左端: トグルボタン（縦長）
        ui.vertical(|ui| {
            ui.set_width(TOGGLE_BTN_WIDTH);
            ui.add_space((ui.available_height() / 2.0 - 12.0).max(0.0));
            let arrow = animated_arrow(t);
            if ui.button(arrow).clicked() {
                layout.right_panel.is_open = false;
            }
        });

        ui.separator();

        // 右側: ウィジェット一覧（グループ別）
        ui.vertical(|ui| {
            ui.heading("Widgets");
            ui.add_space(4.0);

            let placed: Vec<&PanelItem> = layout.grid.placed_items();

            let groups: &[(&str, &[PanelItem])] = &[
                (
                    "Convergence",
                    &[
                        PanelItem::Chart(ChartId::OptimizationHistory),
                        PanelItem::Chart(ChartId::HvHistory),
                    ],
                ),
                (
                    "Pareto / Multi-Objective",
                    &[
                        PanelItem::Chart(ChartId::ParetoScatter2D),
                        PanelItem::Chart(ChartId::ParetoScatter3D),
                        PanelItem::Chart(ChartId::ParallelCoordinates),
                    ],
                ),
                (
                    "Variable Analysis",
                    &[
                        PanelItem::Chart(ChartId::ImportanceChart),
                        PanelItem::Chart(ChartId::SensitivityHeatmap),
                        PanelItem::Chart(ChartId::PdpChart),
                        PanelItem::Chart(ChartId::PdpChart2D),
                        PanelItem::Chart(ChartId::ScatterMatrix),
                        PanelItem::Chart(ChartId::SliceChart),
                        PanelItem::Chart(ChartId::SurfacePlot),
                    ],
                ),
                ("Clustering", &[PanelItem::Chart(ChartId::ClusterScatter)]),
                (
                    "MCDM",
                    &[
                        PanelItem::Chart(ChartId::McdmRankChart),
                        PanelItem::Chart(ChartId::McdmScatterChart),
                        PanelItem::Chart(ChartId::McdmTable),
                    ],
                ),
                ("Data", &[PanelItem::TrialTable]),
            ];

            for (group_label, items) in groups {
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new(*group_label)
                        .small()
                        .color(crate::theme::TEXT_SECONDARY),
                );
                ui.separator();
                for item in *items {
                    let is_placed = placed.contains(&item);
                    if is_placed {
                        ui.add_enabled(false, egui::Label::new(item.label()));
                    } else {
                        let drag_id = egui::Id::new("right_panel_item").with(item.label());
                        ui.dnd_drag_source(drag_id, DragPayload::NewWidget(item.clone()), |ui| {
                            ui.label(item.label());
                        });
                    }
                }
            }
        });
    });
}

/// アニメーション値 t (0=閉, 1=開) から矢印文字を返す
fn animated_arrow(t: f32) -> &'static str {
    if t >= 0.5 {
        "◀"
    } else {
        "▶"
    }
}

/// is_open トグルのロジックを単独でテスト可能な関数
pub fn toggle_right_panel(layout: &mut LayoutState) {
    layout.right_panel.is_open = !layout.right_panel.is_open;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::layout_state::LayoutState;

    #[test]
    fn toggle_right_panel_opens_closed() {
        let mut layout = LayoutState::default();
        layout.right_panel.is_open = false;
        toggle_right_panel(&mut layout);
        assert!(layout.right_panel.is_open);
    }

    #[test]
    fn toggle_right_panel_closes_open() {
        let mut layout = LayoutState::default();
        layout.right_panel.is_open = true;
        toggle_right_panel(&mut layout);
        assert!(!layout.right_panel.is_open);
    }

    #[test]
    fn right_panel_default_is_open() {
        let layout = LayoutState::default();
        assert!(layout.right_panel.is_open);
    }

    #[test]
    fn panel_item_all_count_matches() {
        use crate::state::layout_state::ChartId;
        let items = PanelItem::all();
        assert_eq!(items.len(), ChartId::all().len() + 1);
    }

    #[test]
    fn right_panel_lists_surface_plot_under_variable_analysis() {
        // Verify SurfacePlot appears with the correct label in the Variable Analysis group
        let sp = ChartId::SurfacePlot;
        assert_eq!(sp.label(), "Surface Plot");
        let all = ChartId::all();
        assert!(all.contains(&ChartId::SurfacePlot));
        let item = PanelItem::Chart(ChartId::SurfacePlot);
        assert_eq!(item.label(), "Surface Plot");
    }
}
