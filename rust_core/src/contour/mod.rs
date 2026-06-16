//! 観測トライアル点だけから補間した等高線格子を作る（サロゲート非依存）。
//!
//! Observed Contour ウィジェット用。PDP やサロゲート応答曲面と異なり、モデルを一切
//! 学習せず、**観測点の Delaunay 線形補間**のみで値の格子を作る。データの無い領域
//! （凸包の外、および疎ガードで落ちた三角形領域）は `None` でマスクし、**外挿しない**。
//!
//! 座標は X/Y のスケール差（例: パラメータ vs 目的関数）を吸収するため正規化空間
//! `[0,1]^2` で三角形分割・点位置判定を行い、補間値は元の `value` を使う。

use delaunator::{triangulate, Point};

use crate::math::grid::linspace;

/// 観測点だけから補間した格子。`None` のセルはデータなし（マスク＝外挿しない）。
#[derive(Debug, Clone)]
pub struct ObservedSurface {
    /// X 軸格子（観測 X 範囲の linspace、元の単位）。
    pub x_values: Vec<f64>,
    /// Y 軸格子（観測 Y 範囲の linspace、元の単位）。
    pub y_values: Vec<f64>,
    /// 補間値の格子。`z[i][j]` は (x_values[i], y_values[j]) の値。
    /// `None` = 凸包外 / 疎ガードで落ちた三角形領域。
    pub z: Vec<Vec<Option<f64>>>,
}

impl ObservedSurface {
    fn empty() -> Self {
        Self {
            x_values: vec![],
            y_values: vec![],
            z: vec![],
        }
    }
}

/// 正規化空間の三角形（頂点座標と各頂点の値）。
struct Tri {
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
    cx: f64,
    cy: f64,
    va: f64,
    vb: f64,
    vc: f64,
}

/// 観測点 `(x, y, value)` から、凸包内のみを Delaunay 線形補間した格子を返す。
///
/// - `n_grid`: 一辺の格子点数（例 60）。
/// - `max_edge_ratio`: 疎ガード。正規化空間で三角形の最長辺が `max_edge_ratio` を超えると
///   その三角形を捨てる（離れたクラスタを偽の面で繋がない）。`0.0` で無効、典型 0.1〜0.3。
///
/// 点が 3 未満 / 範囲が退化しているときは空の格子を返す（panic しない）。
/// 共線で三角形が作れないときは、軸はあるが全セル `None` の格子を返す。
pub fn observed_surface(pts: &[[f64; 3]], n_grid: usize, max_edge_ratio: f64) -> ObservedSurface {
    if n_grid == 0 {
        return ObservedSurface::empty();
    }

    // 1. 有限点のみ + (x,y) 近重複を統合（値は平均）。
    let cleaned = clean_points(pts);
    if cleaned.len() < 3 {
        return ObservedSurface::empty();
    }

    // 2. bbox（元の単位）。
    let (mut xmin, mut xmax) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut ymin, mut ymax) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in &cleaned {
        xmin = xmin.min(p[0]);
        xmax = xmax.max(p[0]);
        ymin = ymin.min(p[1]);
        ymax = ymax.max(p[1]);
    }
    let xr = xmax - xmin;
    let yr = ymax - ymin;
    if xr <= 0.0 || yr <= 0.0 {
        return ObservedSurface::empty();
    }

    let x_values = linspace(xmin, xmax, n_grid);
    let y_values = linspace(ymin, ymax, n_grid);

    // 3. 正規化座標 [0,1]^2 で三角形分割（X/Y のスケール差を吸収）。
    let norm: Vec<Point> = cleaned
        .iter()
        .map(|p| Point {
            x: (p[0] - xmin) / xr,
            y: (p[1] - ymin) / yr,
        })
        .collect();
    let tri = triangulate(&norm);
    if tri.triangles.is_empty() {
        // 共線など。軸はあるが補間できないので全マスク。
        return ObservedSurface {
            x_values,
            y_values,
            z: vec![vec![None; n_grid]; n_grid],
        };
    }

    // 4. 三角形リスト（疎ガード適用）。
    let guard = max_edge_ratio > 0.0;
    let mut tris: Vec<Tri> = Vec::with_capacity(tri.triangles.len() / 3);
    for t in tri.triangles.chunks_exact(3) {
        let (ia, ib, ic) = (t[0], t[1], t[2]);
        let (na, nb, nc) = (&norm[ia], &norm[ib], &norm[ic]);
        if guard {
            let longest = dist(na, nb).max(dist(nb, nc)).max(dist(nc, na));
            if longest > max_edge_ratio {
                continue;
            }
        }
        tris.push(Tri {
            ax: na.x,
            ay: na.y,
            bx: nb.x,
            by: nb.y,
            cx: nc.x,
            cy: nc.y,
            va: cleaned[ia][2],
            vb: cleaned[ib][2],
            vc: cleaned[ic][2],
        });
    }

    // 5. 正規化格子で点位置 + 重心補間。
    let gxs = linspace(0.0, 1.0, n_grid);
    let gys = linspace(0.0, 1.0, n_grid);
    let mut z = vec![vec![None; n_grid]; n_grid];
    for (i, &gx) in gxs.iter().enumerate() {
        for (j, &gy) in gys.iter().enumerate() {
            z[i][j] = interpolate(&tris, gx, gy);
        }
    }

    ObservedSurface {
        x_values,
        y_values,
        z,
    }
}

/// 正規化空間のユークリッド距離。
fn dist(a: &Point, b: &Point) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

/// 点 `(px, py)` を含む最初の三角形を見つけ、重心座標で値を補間する。
/// どの三角形にも含まれなければ `None`（凸包外 / マスク領域）。
fn interpolate(tris: &[Tri], px: f64, py: f64) -> Option<f64> {
    const EPS: f64 = 1e-9;
    for t in tris {
        let denom = (t.by - t.cy) * (t.ax - t.cx) + (t.cx - t.bx) * (t.ay - t.cy);
        if denom.abs() < 1e-15 {
            continue; // 退化三角形。
        }
        let la = ((t.by - t.cy) * (px - t.cx) + (t.cx - t.bx) * (py - t.cy)) / denom;
        let lb = ((t.cy - t.ay) * (px - t.cx) + (t.ax - t.cx) * (py - t.cy)) / denom;
        let lc = 1.0 - la - lb;
        if la >= -EPS && lb >= -EPS && lc >= -EPS {
            return Some(la * t.va + lb * t.vb + lc * t.vc);
        }
    }
    None
}

/// 有限点のみ抽出し、(x,y) が近接する点を 1 つに統合（値は平均）する。
/// 正規化グリッド（解像度 1e6）に量子化して重複を判定するため、浮動小数の同値問題に強い。
fn clean_points(pts: &[[f64; 3]]) -> Vec<[f64; 3]> {
    use std::collections::HashMap;

    let finite: Vec<[f64; 3]> = pts
        .iter()
        .copied()
        .filter(|p| p[0].is_finite() && p[1].is_finite() && p[2].is_finite())
        .collect();
    if finite.len() < 2 {
        return finite;
    }

    let (mut xmin, mut xmax) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut ymin, mut ymax) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in &finite {
        xmin = xmin.min(p[0]);
        xmax = xmax.max(p[0]);
        ymin = ymin.min(p[1]);
        ymax = ymax.max(p[1]);
    }
    let xr = xmax - xmin;
    let yr = ymax - ymin;
    if xr <= 0.0 || yr <= 0.0 {
        // 退化（後段で弾く）。重複統合はしない。
        return finite;
    }

    const Q: f64 = 1.0e6;
    // key -> (sum_x, sum_y, sum_v, count)
    let mut acc: HashMap<(i64, i64), (f64, f64, f64, u32)> = HashMap::new();
    for p in &finite {
        let kx = ((p[0] - xmin) / xr * Q).round() as i64;
        let ky = ((p[1] - ymin) / yr * Q).round() as i64;
        let e = acc.entry((kx, ky)).or_insert((0.0, 0.0, 0.0, 0));
        e.0 += p[0];
        e.1 += p[1];
        e.2 += p[2];
        e.3 += 1;
    }
    acc.values()
        .map(|(sx, sy, sv, c)| {
            let c = *c as f64;
            [sx / c, sy / c, sv / c]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 値の格子から Some の個数を数える。
    fn count_some(s: &ObservedSurface) -> usize {
        s.z.iter().flatten().filter(|v| v.is_some()).count()
    }

    #[test]
    fn interpolates_plane_inside_hull_and_masks_outside() {
        // 3 点で平面 value = x + 2y を張る三角形（領域 x+y<=1）。
        let pts = [[0.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 1.0, 2.0]];
        let s = observed_surface(&pts, 11, 0.0);
        assert_eq!(s.x_values.len(), 11);
        assert_eq!(s.z.len(), 11);

        // 三角形内部（x+y<=1）は平面値に一致。i=2 → x=0.2, j=2 → y=0.2。
        let inside = s.z[2][2].expect("inside hull should be Some");
        assert!((inside - (0.2 + 2.0 * 0.2)).abs() < 1e-9, "got {inside}");

        // 三角形外（x+y>1、例 i=9,j=9 → x=0.9,y=0.9）はマスク。
        assert!(s.z[9][9].is_none(), "outside hull should be masked");
    }

    #[test]
    fn sparsity_guard_drops_bridging_triangle() {
        // 2 つの離れたクラスタ。間を橋渡しする三角形は max_edge_ratio で落ちる。
        let pts = [
            [0.0, 0.0, 0.0],
            [0.05, 0.0, 0.0],
            [0.0, 0.05, 0.0],
            [1.0, 1.0, 10.0],
            [0.95, 1.0, 10.0],
            [1.0, 0.95, 10.0],
        ];
        // 厳しいガード: 中央 (0.5,0.5) は橋渡し三角形が落ちてマスクされる。
        let strict = observed_surface(&pts, 21, 0.2);
        assert!(
            strict.z[10][10].is_none(),
            "midpoint should be masked under strict guard"
        );

        // ガード無効: 中央は橋渡し三角形で補間される（Some が増える）。
        let loose = observed_surface(&pts, 21, 0.0);
        assert!(
            count_some(&loose) > count_some(&strict),
            "no guard should fill more cells"
        );
    }

    #[test]
    fn too_few_points_returns_empty() {
        let pts = [[0.0, 0.0, 1.0], [1.0, 1.0, 2.0]];
        let s = observed_surface(&pts, 10, 0.0);
        assert!(s.x_values.is_empty());
        assert!(s.z.is_empty());
    }

    #[test]
    fn degenerate_range_returns_empty() {
        // 全点が同じ X（縦一列）→ X 範囲ゼロ。
        let pts = [[1.0, 0.0, 0.0], [1.0, 1.0, 1.0], [1.0, 2.0, 2.0]];
        let s = observed_surface(&pts, 10, 0.0);
        assert!(s.x_values.is_empty());
    }

    #[test]
    fn duplicate_points_do_not_panic() {
        // 重複だらけでも統合され、三角形が作れれば surface が返る。
        let mut pts = vec![[0.0, 0.0, 0.0]; 20];
        pts.extend_from_slice(&[[1.0, 0.0, 1.0], [0.0, 1.0, 2.0], [1.0, 1.0, 3.0]]);
        let s = observed_surface(&pts, 9, 0.0);
        assert_eq!(s.x_values.len(), 9);
        assert!(count_some(&s) > 0);
    }
}
