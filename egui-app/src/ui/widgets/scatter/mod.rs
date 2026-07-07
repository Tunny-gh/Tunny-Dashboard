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

/// `view` から `features` 列を解決し、全特徴が有限な行のみを採用した学習行列を返す（D-11）。
/// 存在しない列名が 1 つでもあれば空を返す。NaN/Inf を含む行はスキップする
/// （サブサンプルや距離計算などの後処理は呼び出し側の責務）。
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

/// 45°回転ラベルの配置に使う、回転後コーナーのオフセット計算結果。
/// PCP の縦軸ラベルと散布図行列の行・列ラベルで共有する（D-12）。
pub(super) struct RotatedLabelCorners {
    /// 画面上で最も下（最大 ry）になる隅の、回転基準点からの相対オフセット (rx, ry)。
    pub lowest: (f32, f32),
    /// rx が最大となる隅の相対オフセット (rx, ry)。
    pub rightmost: (f32, f32),
    /// 全隅の ry 範囲 (min_ry, max_ry)。
    pub ry_range: (f32, f32),
}

/// サイズ `size` のラベルを角度 `applied`（ラジアン・反時計回りが負）で回転させたときの
/// 4 隅のオフセットを走査し、配置に必要な代表点（最下端・最右端・ry 範囲）を求める。
///
/// - 列/軸ラベル（"/" 形）は最下端（`lowest`）をグリッド上端へ合わせる。
/// - 行ラベルは最右端（`rightmost`）と ry 範囲の中心を使ってグリッド左端へ合わせる。
pub(super) fn rotated_label_corners(size: egui::Vec2, applied: f32) -> RotatedLabelCorners {
    let (sa, ca) = (applied.sin(), applied.cos());
    let corners = [(0.0, 0.0), (size.x, 0.0), (0.0, size.y), (size.x, size.y)];
    let mut lowest = (0.0_f32, f32::MIN); // ry 最大の隅
    let mut rightmost = (f32::MIN, 0.0); // rx 最大の隅
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
