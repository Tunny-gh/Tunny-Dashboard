//! Shared navigation settings for 2D charts.
//!
//! Unifies the interaction scheme across all 2D charts (egui_plot-based):
//! - Left drag: box zoom (zoom in by selecting a range)
//! - Right drag: pan
//! - Left double-click: reset to the default view (auto range)
//! - Scroll wheel: zoom centered on the cursor position
//!   (egui_plot's default "wheel = pan" is disabled and reassigned to zoom via [`apply_wheel_zoom`])

/// Extension trait that applies the unified navigation settings to `egui_plot::Plot`.
///
/// When adding a new 2D chart, always chain `.unified_nav()` into the `Plot::new(..)` builder,
/// and call [`apply_wheel_zoom`] at the top of the `show` closure.
pub trait UnifiedNav {
    fn unified_nav(self) -> Self;
}

impl UnifiedNav for egui_plot::Plot<'_> {
    fn unified_nav(self) -> Self {
        self.boxed_zoom_pointer_button(egui::PointerButton::Primary)
            .pan_pointer_button(egui::PointerButton::Secondary)
            .allow_double_click_reset(true)
            .allow_scroll(false)
    }
}

/// Applies a wheel action as a zoom centered on the cursor position.
///
/// In egui, the wheel alone maps to `smooth_scroll_delta` (for panning); it only becomes
/// `zoom_delta` for Ctrl+wheel / pinch. Since we want wheel = zoom uniformly on charts, while
/// hovering over the plot we consume the scroll input and convert it into a zoom
/// (this also suppresses the parent container's scrolling).
pub fn apply_wheel_zoom(plot_ui: &mut egui_plot::PlotUi<'_>) {
    if !plot_ui.response().hovered() {
        return;
    }
    let scroll_y = plot_ui.ctx().input_mut(|i| {
        let y = i.smooth_scroll_delta.y;
        i.smooth_scroll_delta = egui::Vec2::ZERO;
        y
    });
    if scroll_y != 0.0 {
        // Same sensitivity as egui's Ctrl+wheel zoom (2^(delta/200)).
        let factor = 2.0_f32.powf(scroll_y / 200.0);
        plot_ui.zoom_bounds_around_hovered(egui::Vec2::splat(factor));
    }
}
