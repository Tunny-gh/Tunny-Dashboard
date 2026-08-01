//! Shared helpers for placing categorical axis labels, including the 45°-rotated
//! layout used when horizontal labels would overlap.
//!
//! Two things live here:
//! - [`rotated_label_corners`], the corner math shared by the PCP's vertical axis
//!   labels, the scatter matrix's row/column labels, and the correlation matrix's
//!   column headers (D-12).
//! - [`plot_x_label_band`] / [`draw_plot_x_labels`], which draw the category names of
//!   an `egui_plot` chart in a band reserved below the plot. `egui_plot` picks its own
//!   tick spacing from the available width and cannot rotate tick labels, so a chart
//!   with many categories has to hide its own x axis and paint the names here instead.

/// The result of computing rotated-corner offsets, used to position 45°-rotated labels.
pub struct RotatedLabelCorners {
    /// The corner that ends up lowest on screen (max ry), as a relative offset (rx, ry)
    /// from the rotation origin.
    pub lowest: (f32, f32),
    /// The corner that ends up highest on screen (min ry), as a relative offset (rx, ry)
    /// from the rotation origin.
    pub highest: (f32, f32),
    /// The relative offset (rx, ry) of the corner with the maximum rx.
    pub rightmost: (f32, f32),
    /// The ry range across all corners (min_ry, max_ry).
    pub ry_range: (f32, f32),
}

/// Scans the offsets of the 4 corners when a label of size `size` is rotated by angle
/// `applied` (radians, counterclockwise is negative), and computes the representative
/// points needed for placement (lowest end, highest end, rightmost end, ry range).
///
/// - Labels above a grid (a "/" shape) align their lowest end (`lowest`) to the top of
///   the grid.
/// - Labels below an axis align their highest end (`highest`) to the axis, so the text
///   trails away down and to the left.
/// - Row labels use the rightmost end (`rightmost`) and the center of the ry range to
///   align to the left of the grid.
pub fn rotated_label_corners(size: egui::Vec2, applied: f32) -> RotatedLabelCorners {
    let (sa, ca) = (applied.sin(), applied.cos());
    let corners = [(0.0, 0.0), (size.x, 0.0), (0.0, size.y), (size.x, size.y)];
    let mut lowest = (0.0_f32, f32::MIN); // corner with max ry
    let mut highest = (0.0_f32, f32::MAX); // corner with min ry
    let mut rightmost = (f32::MIN, 0.0); // corner with max rx
    let (mut min_ry, mut max_ry) = (f32::MAX, f32::MIN);
    for (px, py) in corners {
        let rx = px * ca - py * sa;
        let ry = px * sa + py * ca;
        if ry > lowest.1 {
            lowest = (rx, ry);
        }
        if ry < highest.1 {
            highest = (rx, ry);
        }
        if rx > rightmost.0 {
            rightmost = (rx, ry);
        }
        min_ry = min_ry.min(ry);
        max_ry = max_ry.max(ry);
    }
    RotatedLabelCorners {
        lowest,
        highest,
        rightmost,
        ry_range: (min_ry, max_ry),
    }
}

/// Rotation applied to category labels that do not fit horizontally (counterclockwise,
/// so the text reads as a "/" leading up to its tick).
const LABEL_ANGLE: f32 = -std::f32::consts::FRAC_PI_4;

/// Font size of the hand-drawn category labels (matches the correlation matrix).
const LABEL_FONT_SIZE: f32 = 10.0;

/// Upper bound on the height of the label band, so a single very long name cannot
/// squeeze the plot itself out of the widget.
const MAX_BAND_HEIGHT: f32 = 110.0;

/// Padding between the plot frame and the labels drawn beneath it.
const BAND_PADDING: f32 = 4.0;

/// How the category labels of one chart are laid out.
pub struct PlotXLabelBand {
    /// Height to reserve below the plot for the labels.
    pub height: f32,
    /// Whether the labels are rotated by 45° instead of drawn horizontally.
    pub rotated: bool,
    font: egui::FontId,
    /// Width of the widest label, in points.
    max_label_w: f32,
    /// Height of a single line of label text, in points.
    label_h: f32,
}

/// Plans the label band for `labels` spread across a plot `plot_width` points wide.
///
/// Labels stay horizontal while the widest one fits within a category's share of the
/// width, and switch to 45° otherwise — the same test the correlation matrix uses for
/// its column headers, so the two charts agree on when names start to slant.
pub fn plot_x_label_band(ui: &egui::Ui, labels: &[String], plot_width: f32) -> PlotXLabelBand {
    let font = egui::FontId::proportional(LABEL_FONT_SIZE);
    let (mut max_label_w, mut label_h) = (0.0_f32, ui.text_style_height(&egui::TextStyle::Small));
    for label in labels {
        let galley =
            ui.painter()
                .layout_no_wrap(label.clone(), font.clone(), egui::Color32::PLACEHOLDER);
        max_label_w = max_label_w.max(galley.size().x);
        label_h = label_h.max(galley.size().y);
    }

    let slot_w = plot_width / labels.len().max(1) as f32;
    let rotated = max_label_w > slot_w - 4.0;
    let height = if rotated {
        let (sa, ca) = (LABEL_ANGLE.abs().sin(), LABEL_ANGLE.abs().cos());
        (max_label_w * sa + label_h * ca).min(MAX_BAND_HEIGHT)
    } else {
        label_h
    } + BAND_PADDING;

    PlotXLabelBand {
        height,
        rotated,
        font,
        max_label_w,
        label_h,
    }
}

impl PlotXLabelBand {
    /// The minimum horizontal distance between two label anchors that still keeps the
    /// text legible. Rotated labels only need to clear their own line height along the
    /// perpendicular, which is why they fit far more categories than horizontal ones.
    fn min_spacing(&self) -> f32 {
        if self.rotated {
            self.label_h / LABEL_ANGLE.abs().sin()
        } else {
            self.max_label_w + 6.0
        }
    }

    /// The stride at which labels have to be dropped to avoid overlap, given the
    /// on-screen distance between two adjacent categories: 1 draws every label, 2 every
    /// other one, and so on.
    ///
    /// `spacing` comes from the live plot transform rather than from the frame width, so
    /// zooming in spreads the categories out and brings dropped names back.
    pub fn stride(&self, spacing: f32, n_labels: usize) -> usize {
        if n_labels <= 1 {
            return 1;
        }
        // A collapsed or non-finite transform must not divide into `step_by(0)`.
        if spacing.is_nan() || spacing <= 0.0 {
            return n_labels;
        }
        (self.min_spacing() / spacing).ceil().max(1.0) as usize
    }
}

/// Draws the category labels of an `egui_plot` chart into `band` (the strip reserved
/// below the plot), positioning each one under plot x coordinate `i` via `transform`
/// so the names follow panning and zooming.
///
/// Labels whose category has been scrolled outside the plot frame are skipped, and when
/// the categories are packed closer than the text can be read, only every
/// [`PlotXLabelBand::stride`]-th label is drawn rather than letting them overlap.
pub fn draw_plot_x_labels(
    ui: &egui::Ui,
    band: egui::Rect,
    transform: &egui_plot::PlotTransform,
    labels: &[String],
    plan: &PlotXLabelBand,
) {
    if labels.is_empty() {
        return;
    }
    let frame = *transform.frame();
    let color = ui.visuals().text_color();
    // Rotated labels trail down and to the left of their anchor, past the plot's left
    // edge, so the clip covers the full band rather than just the frame's x range.
    let painter = ui.painter().with_clip_rect(band);
    // One category step in screen points, read off the live transform so the thinning
    // relaxes as the user zooms in.
    let spacing =
        (transform.position_from_point_x(1.0) - transform.position_from_point_x(0.0)).abs();
    let stride = plan.stride(spacing, labels.len());

    for (i, label) in labels.iter().enumerate().step_by(stride.max(1)) {
        let x = transform.position_from_point_x(i as f64);
        if !frame.x_range().contains(x) {
            continue;
        }
        let galley = ui
            .painter()
            .layout_no_wrap(label.clone(), plan.font.clone(), color);
        if plan.rotated {
            // Pin the end of the "/"-shaped label just below the axis at the category's
            // center; the rest of the text trails away down and to the left.
            let highest = rotated_label_corners(galley.size(), LABEL_ANGLE).highest;
            let anchor = egui::pos2(x, band.top() + BAND_PADDING);
            painter.add(
                egui::epaint::TextShape::new(
                    anchor - egui::vec2(highest.0, highest.1),
                    galley,
                    color,
                )
                .with_angle(LABEL_ANGLE),
            );
        } else {
            let size = galley.size();
            painter.galley(
                egui::pos2(x - size.x * 0.5, band.top() + BAND_PADDING),
                galley,
                color,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sizes chosen so the rotated corners land on round numbers: at -45° a 100x10
    /// label spans 0.707*(100+10) vertically.
    const LABEL_SIZE: egui::Vec2 = egui::Vec2 { x: 100.0, y: 10.0 };

    #[test]
    fn rotated_corners_put_text_end_highest_and_start_lowest() {
        let c = rotated_label_corners(LABEL_SIZE, -std::f32::consts::FRAC_PI_4);
        // The (w, 0) corner — the end of the text — rises above the rotation origin.
        assert!(c.highest.1 < 0.0);
        // The (0, h) corner — the start of the text — drops below it.
        assert!(c.lowest.1 > 0.0);
        assert!(c.highest.1 < c.lowest.1);
    }

    #[test]
    fn rotated_corners_ry_range_matches_highest_and_lowest() {
        let c = rotated_label_corners(LABEL_SIZE, -std::f32::consts::FRAC_PI_4);
        assert!((c.ry_range.0 - c.highest.1).abs() < 1e-4);
        assert!((c.ry_range.1 - c.lowest.1).abs() < 1e-4);
    }

    fn band(rotated: bool, max_label_w: f32, label_h: f32) -> PlotXLabelBand {
        PlotXLabelBand {
            height: 0.0,
            rotated,
            font: egui::FontId::proportional(LABEL_FONT_SIZE),
            max_label_w,
            label_h,
        }
    }

    #[test]
    fn stride_is_one_when_labels_have_room() {
        // 100px between categories, far beyond the ~17px a 12px-tall rotated label needs.
        assert_eq!(band(true, 100.0, 12.0).stride(100.0, 10), 1);
    }

    #[test]
    fn stride_thins_out_when_rotated_labels_would_still_overlap() {
        // 2px between categories; a rotated 12px-tall label needs ~17px.
        assert!(band(true, 100.0, 12.0).stride(2.0, 200) > 1);
    }

    #[test]
    fn stride_relaxes_as_the_spacing_grows() {
        // Zooming in widens the gap between categories, which must bring dropped
        // labels back rather than leaving the initial thinning in place.
        let plan = band(true, 100.0, 12.0);
        assert!(plan.stride(8.0, 40) > plan.stride(20.0, 40));
        assert_eq!(plan.stride(20.0, 40), 1);
    }

    #[test]
    fn horizontal_labels_need_their_full_width_of_spacing() {
        // The same geometry needs a much larger stride while horizontal, because the
        // whole label width has to fit instead of just its height.
        let rotated = band(true, 100.0, 12.0).stride(10.0, 40);
        let horizontal = band(false, 100.0, 12.0).stride(10.0, 40);
        assert!(horizontal > rotated);
    }

    #[test]
    fn stride_of_single_label_is_one() {
        assert_eq!(band(false, 100.0, 12.0).stride(400.0, 1), 1);
    }

    #[test]
    fn degenerate_spacing_falls_back_to_a_single_label() {
        // A collapsed or non-finite transform must not produce `step_by(0)`.
        assert_eq!(band(true, 100.0, 12.0).stride(0.0, 40), 40);
        assert_eq!(band(true, 100.0, 12.0).stride(f32::NAN, 40), 40);
    }
}
