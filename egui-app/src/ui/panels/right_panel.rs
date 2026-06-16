use crate::state::app_state::AppState;
use crate::state::layout_state::{ChartId, DragPayload, LayoutState, PanelItem};

/// 右パネルを描画する（ウィジェット一覧）。
/// パネルの開閉はホバーで自動制御されるため、トグルボタンは不要。
pub fn show_right_panel(ui: &mut egui::Ui, _app_state: &AppState, layout: &mut LayoutState) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Widgets");
        ui.add_space(4.0);

        let placed: Vec<&PanelItem> = layout.placed_items();

        let groups: &[(&str, &[PanelItem])] = &[
            (
                "Convergence",
                &[
                    PanelItem::Chart(ChartId::OptimizationHistory),
                    PanelItem::Chart(ChartId::ConvergenceIndicators),
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
                    PanelItem::Chart(ChartId::ScatterMatrix),
                    PanelItem::Chart(ChartId::SliceChart),
                    PanelItem::Chart(ChartId::ObservedContour),
                ],
            ),
            // PDP はサロゲートを学習し他変数を周辺化した予測（外挿あり）。データ由来の
            // 分析と取り違えないよう、モデルベースであることを群として明示する。
            (
                "Response Surface (model-based)",
                &[
                    PanelItem::Chart(ChartId::PdpChart),
                    PanelItem::Chart(ChartId::PdpChart2D),
                ],
            ),
            ("Optimization", &[PanelItem::Chart(ChartId::SurrogateOpt)]),
            (
                "Clustering",
                &[
                    PanelItem::Chart(ChartId::ClusterScatter),
                    PanelItem::Chart(ChartId::ClusterScatter3D),
                ],
            ),
            (
                "MCDM",
                &[
                    PanelItem::Chart(ChartId::McdmRankChart),
                    PanelItem::Chart(ChartId::McdmScatterChart),
                    PanelItem::Chart(ChartId::McdmScatterChart3D),
                ],
            ),
            ("Artifacts", &[PanelItem::Chart(ChartId::ArtifactGallery)]),
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
    fn right_panel_default_is_closed() {
        let layout = LayoutState::default();
        assert!(!layout.right_panel.is_open);
    }

    #[test]
    fn panel_item_all_count_matches() {
        use crate::state::layout_state::ChartId;
        let items = PanelItem::all();
        assert_eq!(items.len(), ChartId::all().len() + 1);
    }
}
