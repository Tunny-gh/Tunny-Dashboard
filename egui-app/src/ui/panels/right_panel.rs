use crate::state::app_state::AppState;
use crate::state::layout_state::{ChartId, DragPayload, LayoutState, PanelItem};

/// アイコンタイルの幅（キャプションの折り返し幅でもある）。
/// 長いウィジェット名が途中で切れないよう、複数行に折り返せる幅を確保する。
const TILE_W: f32 = 78.0;
/// アイコンタイルの基準の高さ（アイコン + キャプション2行を想定）。
/// 余白を詰めるため2行ぶんに切り詰めている。名前が3行以上の場合は伸びる。
const TILE_H: f32 = 50.0;
/// SVG アイコンの描画サイズ。
const ICON_SIZE: f32 = 24.0;

/// 各 PanelItem に対応する SVG アイコンを返す。
/// アイコンは白単色で作成しており、描画時にテーマ色で乗算ティントする。
fn item_icon(item: &PanelItem) -> egui::ImageSource<'static> {
    match item {
        PanelItem::TrialTable => {
            egui::include_image!("../../../assets/widget_icons/trial_table.svg")
        }
        PanelItem::Chart(id) => chart_icon(id),
    }
}

fn chart_icon(id: &ChartId) -> egui::ImageSource<'static> {
    match id {
        ChartId::OptimizationHistory => {
            egui::include_image!("../../../assets/widget_icons/optimization_history.svg")
        }
        ChartId::ConvergenceIndicators => {
            egui::include_image!("../../../assets/widget_icons/convergence_indicators.svg")
        }
        ChartId::ParetoScatter2D => {
            egui::include_image!("../../../assets/widget_icons/pareto_scatter_2d.svg")
        }
        ChartId::ParetoScatter3D => {
            egui::include_image!("../../../assets/widget_icons/pareto_scatter_3d.svg")
        }
        ChartId::ParallelCoordinates => {
            egui::include_image!("../../../assets/widget_icons/parallel_coordinates.svg")
        }
        ChartId::ImportanceChart => {
            egui::include_image!("../../../assets/widget_icons/importance_chart.svg")
        }
        ChartId::SensitivityHeatmap => {
            egui::include_image!("../../../assets/widget_icons/sensitivity_heatmap.svg")
        }
        ChartId::ScatterMatrix => {
            egui::include_image!("../../../assets/widget_icons/scatter_matrix.svg")
        }
        ChartId::SliceChart => {
            egui::include_image!("../../../assets/widget_icons/slice_chart.svg")
        }
        ChartId::ObservedContour => {
            egui::include_image!("../../../assets/widget_icons/observed_contour.svg")
        }
        ChartId::PdpChart => egui::include_image!("../../../assets/widget_icons/pdp_chart.svg"),
        ChartId::PdpChart2D => {
            egui::include_image!("../../../assets/widget_icons/pdp_chart_2d.svg")
        }
        ChartId::SurrogateOpt => {
            egui::include_image!("../../../assets/widget_icons/surrogate_opt.svg")
        }
        ChartId::ClusterScatter => {
            egui::include_image!("../../../assets/widget_icons/cluster_scatter.svg")
        }
        ChartId::ClusterScatter3D => {
            egui::include_image!("../../../assets/widget_icons/cluster_scatter_3d.svg")
        }
        ChartId::McdmRankChart => {
            egui::include_image!("../../../assets/widget_icons/mcdm_rank_chart.svg")
        }
        ChartId::McdmScatterChart => {
            egui::include_image!("../../../assets/widget_icons/mcdm_scatter_chart.svg")
        }
        ChartId::McdmScatterChart3D => {
            egui::include_image!("../../../assets/widget_icons/mcdm_scatter_chart_3d.svg")
        }
        ChartId::ArtifactGallery => {
            egui::include_image!("../../../assets/widget_icons/artifact_gallery.svg")
        }
    }
}

/// 1 ウィジェット分のアイコンタイル（アイコン + キャプション）を描画する。
/// `enabled` が false（配置済み）の場合はティントを薄くし、操作不可にする。
fn tile_contents(ui: &mut egui::Ui, item: &PanelItem, enabled: bool) {
    let tint = if enabled {
        crate::theme::TEXT_SECONDARY
    } else {
        egui::Color32::from_gray(190)
    };
    ui.allocate_ui_with_layout(
        egui::vec2(TILE_W, TILE_H),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            ui.set_min_size(egui::vec2(TILE_W, TILE_H));
            // キャプションは TILE_W 幅で折り返す（途中で切らない）。
            ui.set_max_width(TILE_W);
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.add(
                egui::Image::new(item_icon(item))
                    .fit_to_exact_size(egui::vec2(ICON_SIZE, ICON_SIZE))
                    .tint(tint),
            );
            ui.add(
                egui::Label::new(egui::RichText::new(item.label()).size(9.0).color(tint)).wrap(),
            );
        },
    );
}

/// アイコンタイルを枠付きで描画する。未配置ならドラッグ元にする。
fn widget_tile(ui: &mut egui::Ui, item: &PanelItem, is_placed: bool) {
    let frame = egui::Frame::default()
        .fill(crate::theme::WIDGET_BG)
        .corner_radius(6)
        .inner_margin(egui::Margin::same(2));

    if is_placed {
        frame.show(ui, |ui| tile_contents(ui, item, false));
    } else {
        let drag_id = egui::Id::new("right_panel_item").with(item.label());
        let resp = ui
            .dnd_drag_source(drag_id, DragPayload::NewWidget(item.clone()), |ui| {
                frame.show(ui, |ui| tile_contents(ui, item, true));
            })
            .response;
        resp.on_hover_text(item.label());
    }
}

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
            (
                "Artifacts / Data",
                &[
                    PanelItem::Chart(ChartId::ArtifactGallery),
                    PanelItem::TrialTable,
                ],
            ),
        ];

        for (group_label, items) in groups {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(*group_label)
                    .small()
                    .color(crate::theme::TEXT_SECONDARY),
            );
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 3.0);
                for item in *items {
                    let is_placed = placed.contains(&item);
                    widget_tile(ui, item, is_placed);
                }
            });
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
