pub mod cluster_scatter;
pub mod cluster_scatter_3d;
pub mod dendrogram;
pub mod observed_contour;
pub mod parallel_coords;
pub mod pca_biplot;
pub mod rank_plot;
pub mod scatter_3d;
pub mod scatter_matrix;
pub mod som_map;

use crate::state::types::StudyView;

/// Resolves the `features` columns from `view` and returns a training matrix containing
/// only rows where every feature is finite (D-11). Returns empty if even one column
/// name doesn't exist. Rows containing NaN/Inf are skipped (post-processing such as
/// subsampling or distance computation is the caller's responsibility).
pub(super) fn feature_matrix(view: &StudyView, features: &[String]) -> Vec<Vec<f64>> {
    let Some(cols): Option<Vec<&[f64]>> = features.iter().map(|f| view.numeric_column(f)).collect()
    else {
        return Vec::new();
    };
    (0..view.row_count())
        .filter_map(|r| {
            cols.iter()
                .map(|c| c.get(r).copied().filter(|v| v.is_finite()))
                .collect::<Option<Vec<f64>>>()
        })
        .collect()
}

/// The result of computing rotated-corner offsets, used to position 45°-rotated labels.
/// Shared between the PCP's vertical axis labels and the scatter matrix's row/column
/// labels (D-12).
pub(super) struct RotatedLabelCorners {
    /// The corner that ends up lowest on screen (max ry), as a relative offset (rx, ry)
    /// from the rotation origin.
    pub lowest: (f32, f32),
    /// The relative offset (rx, ry) of the corner with the maximum rx.
    pub rightmost: (f32, f32),
    /// The ry range across all corners (min_ry, max_ry).
    pub ry_range: (f32, f32),
}

/// Scans the offsets of the 4 corners when a label of size `size` is rotated by angle
/// `applied` (radians, counterclockwise is negative), and computes the representative
/// points needed for placement (lowest end, rightmost end, ry range).
///
/// - Column/axis labels (a "/" shape) align their lowest end (`lowest`) to the top of
///   the grid.
/// - Row labels use the rightmost end (`rightmost`) and the center of the ry range to
///   align to the left of the grid.
pub(super) fn rotated_label_corners(size: egui::Vec2, applied: f32) -> RotatedLabelCorners {
    let (sa, ca) = (applied.sin(), applied.cos());
    let corners = [(0.0, 0.0), (size.x, 0.0), (0.0, size.y), (size.x, size.y)];
    let mut lowest = (0.0_f32, f32::MIN); // corner with max ry
    let mut rightmost = (f32::MIN, 0.0); // corner with max rx
    let (mut min_ry, mut max_ry) = (f32::MAX, f32::MIN);
    for (px, py) in corners {
        let rx = px * ca - py * sa;
        let ry = px * sa + py * ca;
        if ry > lowest.1 {
            lowest = (rx, ry);
        }
        if rx > rightmost.0 {
            rightmost = (rx, ry);
        }
        min_ry = min_ry.min(ry);
        max_ry = max_ry.max(ry);
    }
    RotatedLabelCorners {
        lowest,
        rightmost,
        ry_range: (min_ry, max_ry),
    }
}
