pub mod cluster_scatter;
pub mod cluster_scatter_3d;
pub mod dendrogram;
pub mod observed_contour;
pub mod parallel_coords;
pub mod pca_biplot;
pub mod rank_plot;
pub mod rank_plot_3d;
pub mod scatter_3d;
pub mod scatter_matrix;
pub mod som_map;

#[cfg(test)]
use crate::state::types::StudyView;

/// Resolves finite feature rows and their source row indices.
/// Returns empty if any column does not exist.
///
/// Callers that map a per-row result of a clustering routine back onto trials need this:
/// dropping the NaN rows shifts every later row's position, so the returned matrix can no
/// longer be indexed with a `StudyView` row index.
#[cfg(test)]
pub(super) fn feature_matrix_with_rows(
    view: &StudyView,
    features: &[String],
) -> (Vec<usize>, Vec<Vec<f64>>) {
    let data = tdashboard_analysis::preparation::feature_rows(&view.df, features, false);
    (data.rows, data.matrix)
}

/// Re-exported so the charts in this module keep reaching for the rotated-label math
/// through `super::`; it now lives with the rest of the axis-label helpers because the
/// box plot needs it too (D-12).
pub(super) use crate::ui::widgets::common::axis_labels::rotated_label_corners;
