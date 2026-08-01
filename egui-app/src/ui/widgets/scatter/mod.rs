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
    feature_matrix_with_rows(view, features).1
}

/// Same as [`feature_matrix`], but also returns the source row index of every kept row.
///
/// Callers that map a per-row result of a clustering routine back onto trials need this:
/// dropping the NaN rows shifts every later row's position, so the returned matrix can no
/// longer be indexed with a `StudyView` row index.
pub(super) fn feature_matrix_with_rows(
    view: &StudyView,
    features: &[String],
) -> (Vec<usize>, Vec<Vec<f64>>) {
    let Some(cols): Option<Vec<&[f64]>> = features.iter().map(|f| view.numeric_column(f)).collect()
    else {
        return (Vec::new(), Vec::new());
    };
    (0..view.row_count())
        .filter_map(|r| {
            cols.iter()
                .map(|c| c.get(r).copied().filter(|v| v.is_finite()))
                .collect::<Option<Vec<f64>>>()
                .map(|row| (r, row))
        })
        .unzip()
}

/// Re-exported so the charts in this module keep reaching for the rotated-label math
/// through `super::`; it now lives with the rest of the axis-label helpers because the
/// box plot needs it too (D-12).
pub(super) use crate::ui::widgets::common::axis_labels::rotated_label_corners;
