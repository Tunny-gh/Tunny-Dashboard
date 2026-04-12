use crate::state::app_state::AppState;
use crate::state::layout_state::{LayoutState, PanelItem};

/// 右パネルを描画する。
/// - ≡ ボタンで開閉切り替え
/// - 利用可能な PanelItem の一覧を表示
/// - グリッドに配置済みのアイテムはグレーアウト
pub fn show_right_panel(ui: &mut egui::Ui, _app_state: &AppState, layout: &mut LayoutState) {
    // ≡ ハンバーガーボタンで開閉
    if ui.button("≡").clicked() {
        layout.right_panel.is_open = !layout.right_panel.is_open;
    }

    if !layout.right_panel.is_open {
        return;
    }

    ui.separator();
    ui.heading("Widgets");
    ui.add_space(4.0);

    // グリッドに配置済みのアイテムリスト（グレーアウト判定用）
    let placed: Vec<&PanelItem> = layout.grid.placed_items();

    for item in PanelItem::all() {
        let is_placed = placed.contains(&&item);
        if is_placed {
            // 配置済みはグレーアウトで表示（ドラッグ不可）
            ui.add_enabled(false, egui::Label::new(item.label()));
        } else {
            // 未配置はドラッグソースとして登録
            let drag_id = egui::Id::new("right_panel_item").with(item.label());
            ui.dnd_drag_source(drag_id, item.clone(), |ui| {
                ui.label(item.label());
            });
        }
    }
}

/// is_open トグルのロジックを単独でテスト可能な関数
pub fn toggle_right_panel(layout: &mut LayoutState) {
    layout.right_panel.is_open = !layout.right_panel.is_open;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::layout_state::{GridLayout, LayoutState, RightPanelState};

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
        // PanelItem::all() は ChartId::all().len() + 1 件
        use crate::state::layout_state::ChartId;
        let items = PanelItem::all();
        assert_eq!(items.len(), ChartId::all().len() + 1);
    }
}
