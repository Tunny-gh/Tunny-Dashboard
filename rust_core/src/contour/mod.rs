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

    // 5. 正規化格子で点位置 + 重心補間（三角形 bbox のバケット索引で高速化）。
    let index = TriGridIndex::new(tris);
    let gxs = linspace(0.0, 1.0, n_grid);
    let gys = linspace(0.0, 1.0, n_grid);
    let mut z = vec![vec![None; n_grid]; n_grid];
    for (i, &gx) in gxs.iter().enumerate() {
        for (j, &gy) in gys.iter().enumerate() {
            z[i][j] = index.interpolate(gx, gy);
        }
    }

    ObservedSurface {
        x_values,
        y_values,
        z,
    }
}

/// 三角形 bbox によるグリッドバケット索引（正規化空間 [0,1]^2 前提）。
///
/// 格子セルごとの全三角形線形走査（O(grid² × tris)）を避けるため、
/// 一様な nb×nb のバケット格子に「bbox が重なる三角形の index」を昇順で
/// 積んでおき、点位置判定は点の属するバケット内の候補のみ調べる。
/// バケット内の候補は三角形 index 昇順のまま保持されるため、
/// 「最初に点を含む三角形が勝つ」という線形走査の結果と完全に一致する。
struct TriGridIndex {
    tris: Vec<Tri>,
    /// バケット格子の一辺の数。
    nb: usize,
    /// `buckets[iy * nb + ix]` = そのセルに bbox が重なる三角形 index（昇順）。
    buckets: Vec<Vec<u32>>,
}

impl TriGridIndex {
    fn new(tris: Vec<Tri>) -> Self {
        // 三角形数の平方根程度のバケット数で、セルあたり候補数を O(√tris) に抑える。
        let nb = ((tris.len() as f64).sqrt().ceil() as usize).clamp(1, 64);
        let mut buckets = vec![Vec::new(); nb * nb];
        // 重心座標判定は EPS(1e-9) だけ三角形外の点も受理するため、bbox を
        // それより十分大きいマージンで拡げて候補の取り漏らしを防ぐ。
        const MARGIN: f64 = 1e-6;
        for (ti, t) in tris.iter().enumerate() {
            let xmin = t.ax.min(t.bx).min(t.cx) - MARGIN;
            let xmax = t.ax.max(t.bx).max(t.cx) + MARGIN;
            let ymin = t.ay.min(t.by).min(t.cy) - MARGIN;
            let ymax = t.ay.max(t.by).max(t.cy) + MARGIN;
            let (ix0, ix1) = (Self::cell(xmin, nb), Self::cell(xmax, nb));
            let (iy0, iy1) = (Self::cell(ymin, nb), Self::cell(ymax, nb));
            for iy in iy0..=iy1 {
                for ix in ix0..=ix1 {
                    buckets[iy * nb + ix].push(ti as u32);
                }
            }
        }
        Self { tris, nb, buckets }
    }

    /// 座標（ほぼ [0,1]）→ バケット index。負値は 0 に飽和（`as usize` の仕様）。
    fn cell(v: f64, nb: usize) -> usize {
        ((v * nb as f64) as usize).min(nb - 1)
    }

    /// 点 `(px, py)` を含む最初（index 最小）の三角形で重心補間する。
    /// どの三角形にも含まれなければ `None`（凸包外 / マスク領域）。
    fn interpolate(&self, px: f64, py: f64) -> Option<f64> {
        let bucket = &self.buckets[Self::cell(py, self.nb) * self.nb + Self::cell(px, self.nb)];
        for &ti in bucket {
            if let Some(v) = barycentric_value(&self.tris[ti as usize], px, py) {
                return Some(v);
            }
        }
        None
    }
}

/// 点 `(px, py)` が三角形 `t` 内（境界の数値誤差込み）なら重心座標で補間値を返す。
fn barycentric_value(t: &Tri, px: f64, py: f64) -> Option<f64> {
    const EPS: f64 = 1e-9;
    let denom = (t.by - t.cy) * (t.ax - t.cx) + (t.cx - t.bx) * (t.ay - t.cy);
    if denom.abs() < 1e-15 {
        return None; // 退化三角形。
    }
    let la = ((t.by - t.cy) * (px - t.cx) + (t.cx - t.bx) * (py - t.cy)) / denom;
    let lb = ((t.cy - t.ay) * (px - t.cx) + (t.ax - t.cx) * (py - t.cy)) / denom;
    let lc = 1.0 - la - lb;
    if la >= -EPS && lb >= -EPS && lc >= -EPS {
        Some(la * t.va + lb * t.vb + lc * t.vc)
    } else {
        None
    }
}

/// 正規化空間のユークリッド距離。
fn dist(a: &Point, b: &Point) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
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

// ============================================================
// 観測点の密度グリッド（散布オーバーレイのシェーディング用）
// ============================================================

/// 観測点を (nx-1)×(ny-1) のセルにビニングし、半径 `blur_radius` で局所平滑化したうえで
/// 最大値で割った正規化密度 (0..1) を返す。セル (i,j) は x∈[x_i,x_{i+1}]、y∈[y_j,y_{j+1}]。
/// 1 セル単位ではほぼ 0/1 でノイズが多いため、平滑化して領域の濃淡を表す。
pub fn cell_density_grid(
    points: &[[f64; 3]],
    (x_min, x_max): (f64, f64),
    (y_min, y_max): (f64, f64),
    nx: usize,
    ny: usize,
    blur_radius: usize,
) -> Vec<Vec<f32>> {
    let (cx, cy) = (nx.saturating_sub(1).max(1), ny.saturating_sub(1).max(1));
    let mut counts = vec![vec![0f32; cy]; cx];
    if x_max > x_min && y_max > y_min {
        for p in points {
            let fx = ((p[0] - x_min) / (x_max - x_min)).clamp(0.0, 1.0);
            let fy = ((p[1] - y_min) / (y_max - y_min)).clamp(0.0, 1.0);
            let i = ((fx * cx as f64) as usize).min(cx - 1);
            let j = ((fy * cy as f64) as usize).min(cy - 1);
            counts[i][j] += 1.0;
        }
    }
    let smoothed = box_blur_2d(&counts, blur_radius);
    let max = smoothed.iter().flatten().copied().fold(0.0_f32, f32::max);
    let denom = if max > 0.0 { max } else { 1.0 };
    smoothed
        .iter()
        .map(|col| col.iter().map(|&c| c / denom).collect())
        .collect()
}

/// 2D グリッドに半径 `r` の分離型箱平滑化（近傍平均）を適用する。`r == 0` は恒等。
pub fn box_blur_2d(grid: &[Vec<f32>], r: usize) -> Vec<Vec<f32>> {
    if r == 0 || grid.is_empty() {
        return grid.to_vec();
    }
    let nx = grid.len();
    let ny = grid[0].len();
    // 横方向の移動平均。
    let mut tmp = vec![vec![0f32; ny]; nx];
    for (i, col) in tmp.iter_mut().enumerate() {
        for (j, slot) in col.iter_mut().enumerate() {
            let lo = i.saturating_sub(r);
            let hi = (i + r).min(nx - 1);
            let mut sum = 0.0;
            for row in grid.iter().take(hi + 1).skip(lo) {
                sum += row[j];
            }
            *slot = sum / (hi - lo + 1) as f32;
        }
    }
    // 縦方向の移動平均。
    let mut out = vec![vec![0f32; ny]; nx];
    for (out_row, src_row) in out.iter_mut().zip(tmp.iter()) {
        for (j, slot) in out_row.iter_mut().enumerate() {
            let lo = j.saturating_sub(r);
            let hi = (j + r).min(ny - 1);
            let sum: f32 = src_row[lo..=hi].iter().sum();
            *slot = sum / (hi - lo + 1) as f32;
        }
    }
    out
}

// ============================================================
// マスク対応の等高線セグメント抽出（marching squares）
// ============================================================

/// マスク付き値グリッドから等高線の線分を抽出する（marching squares）。
/// 4 隅とも `Some` のセルのみ対象。`n_levels` 本の等値線を
/// `v_min..v_max` を等分した内部レベルに引く。
///
/// 返す座標はグリッドのサンプル index 空間（`display[r][c]` のサンプルが
/// `[c as f64, r as f64]`）。描画側はセル中心をサンプル位置として
/// スクリーン座標へ写像する。
pub fn contour_line_segments(
    display: &[Vec<Option<f64>>],
    v_min: f64,
    v_max: f64,
    n_levels: usize,
) -> Vec<([f64; 2], [f64; 2])> {
    let mut segments = Vec::new();
    let ny = display.len();
    if ny < 2 {
        return segments;
    }
    let nx = display[0].len();
    if nx < 2 || (v_max - v_min).abs() < f64::EPSILON {
        return segments;
    }

    for li in 1..=n_levels {
        let level = v_min + (v_max - v_min) * li as f64 / (n_levels + 1) as f64;
        for r in 0..ny - 1 {
            for c in 0..nx - 1 {
                let (Some(tl), Some(tr), Some(br), Some(bl)) = (
                    display[r][c],
                    display[r][c + 1],
                    display[r + 1][c + 1],
                    display[r + 1][c],
                ) else {
                    continue; // 不完全セルは等高線を描かない。
                };
                // 4 辺（上・右・下・左）の交点を集める。
                let mut pts: Vec<[f64; 2]> = Vec::with_capacity(4);
                let (x0, x1) = (c as f64, (c + 1) as f64);
                let (y0, y1) = (r as f64, (r + 1) as f64);
                if let Some(t) = edge_cross(tl, tr, level) {
                    pts.push([x0 + t, y0]);
                }
                if let Some(t) = edge_cross(tr, br, level) {
                    pts.push([x1, y0 + t]);
                }
                if let Some(t) = edge_cross(bl, br, level) {
                    pts.push([x0 + t, y1]);
                }
                if let Some(t) = edge_cross(tl, bl, level) {
                    pts.push([x0, y0 + t]);
                }
                match pts.len() {
                    2 => segments.push((pts[0], pts[1])),
                    4 => {
                        segments.push((pts[0], pts[1]));
                        segments.push((pts[2], pts[3]));
                    }
                    _ => {}
                }
            }
        }
    }
    segments
}

/// 辺の 2 端点 `a`,`b` が `level` を挟むなら、a→b 上の交点比率 `t`(0..1) を返す。
fn edge_cross(a: f64, b: f64, level: f64) -> Option<f64> {
    let above_a = a >= level;
    let above_b = b >= level;
    if above_a == above_b {
        return None;
    }
    let denom = b - a;
    if denom.abs() < f64::EPSILON {
        return None;
    }
    Some(((level - a) / denom).clamp(0.0, 1.0))
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
    fn bucketed_index_matches_linear_scan_exactly() {
        // バケット索引は「index 最小の含有三角形が勝つ」線形走査と完全一致する。
        let pts: Vec<[f64; 3]> = (0..40)
            .map(|i| {
                // 決定論的な擬似ランダム配置。
                let x = ((i * 37 + 11) % 97) as f64 / 97.0;
                let y = ((i * 53 + 29) % 89) as f64 / 89.0;
                [x, y, x * 3.0 - y * 2.0 + (x * y).sin()]
            })
            .collect();
        let s = observed_surface(&pts, 31, 0.0);
        // 同一入力から三角形リストを再構築し、線形走査で照合する。
        let cleaned = clean_points(&pts);
        let (mut xmin, mut xmax) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut ymin, mut ymax) = (f64::INFINITY, f64::NEG_INFINITY);
        for p in &cleaned {
            xmin = xmin.min(p[0]);
            xmax = xmax.max(p[0]);
            ymin = ymin.min(p[1]);
            ymax = ymax.max(p[1]);
        }
        let norm: Vec<Point> = cleaned
            .iter()
            .map(|p| Point {
                x: (p[0] - xmin) / (xmax - xmin),
                y: (p[1] - ymin) / (ymax - ymin),
            })
            .collect();
        let tri = triangulate(&norm);
        let tris: Vec<Tri> = tri
            .triangles
            .chunks_exact(3)
            .map(|t| Tri {
                ax: norm[t[0]].x,
                ay: norm[t[0]].y,
                bx: norm[t[1]].x,
                by: norm[t[1]].y,
                cx: norm[t[2]].x,
                cy: norm[t[2]].y,
                va: cleaned[t[0]][2],
                vb: cleaned[t[1]][2],
                vc: cleaned[t[2]][2],
            })
            .collect();
        let gxs = linspace(0.0, 1.0, 31);
        let gys = linspace(0.0, 1.0, 31);
        for (i, &gx) in gxs.iter().enumerate() {
            for (j, &gy) in gys.iter().enumerate() {
                let brute = tris.iter().find_map(|t| barycentric_value(t, gx, gy));
                assert_eq!(s.z[i][j], brute, "mismatch at grid ({i},{j})");
            }
        }
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

    // ── 密度グリッド / 等高線セグメント（Observed Contour オーバーレイ用） ──

    #[test]
    fn edge_cross_detects_straddle() {
        // 0 と 2 が level=1 を挟む → 中点 t=0.5。
        assert_eq!(edge_cross(0.0, 2.0, 1.0), Some(0.5));
        // 同符号は None。
        assert!(edge_cross(0.0, 0.5, 1.0).is_none());
        assert!(edge_cross(2.0, 3.0, 1.0).is_none());
    }

    #[test]
    fn cell_density_grid_bins_and_normalizes() {
        // 3x3 グリッド → 2x2 セル。左下セルに 2 点、右上セルに 2 点。
        let pts = vec![
            [0.1, 0.1, 0.0],
            [0.2, 0.2, 0.0],
            [0.9, 0.9, 0.0],
            [1.0, 1.0, 0.0], // 端は最終セルにクランプ
        ];
        // blur=0 はビニングそのまま（正規化のみ）。
        let d = cell_density_grid(&pts, (0.0, 1.0), (0.0, 1.0), 3, 3, 0);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].len(), 2);
        // 左下 (i=0,j=0) が最大カウント 2 → 1.0。
        assert!((d[0][0] - 1.0).abs() < 1e-6);
        // 右上 (i=1,j=1) はカウント 2（0.9 と 1.0）→ 1.0。
        assert!((d[1][1] - 1.0).abs() < 1e-6);
        // 空セルは 0。
        assert!(d[0][1].abs() < 1e-6);
        assert!(d[1][0].abs() < 1e-6);
    }

    #[test]
    fn box_blur_spreads_into_neighbors() {
        // 中央だけ値を持つ 3x3。半径1の平滑化で隣接セルが非ゼロになる。
        let mut g = vec![vec![0.0_f32; 3]; 3];
        g[1][1] = 9.0;
        let b = box_blur_2d(&g, 1);
        assert!(b[1][1] > 0.0);
        assert!(b[0][1] > 0.0); // 縦横の隣接に滲む
        assert!(b[1][0] > 0.0);
        // 総和は発散しない（端クランプの平均なので完全保存ではない）。
        let before: f32 = g.iter().flatten().sum();
        let after: f32 = b.iter().flatten().sum();
        assert!((before - after).abs() < before);
    }

    #[test]
    fn box_blur_zero_radius_is_identity() {
        let g = vec![vec![1.0_f32, 2.0], vec![3.0, 4.0]];
        assert_eq!(box_blur_2d(&g, 0), g);
    }

    #[test]
    fn contour_segments_cross_simple_gradient() {
        // 2x2 グリッド、値 0..3。level は内部等分なので必ず横切る線分が出る。
        let g = vec![vec![Some(0.0), Some(1.0)], vec![Some(2.0), Some(3.0)]];
        let segs = contour_line_segments(&g, 0.0, 3.0, 2);
        assert!(!segs.is_empty());
        // 座標はサンプル index 空間 [0,1]x[0,1] に収まる。
        for (a, b) in &segs {
            for p in [a, b] {
                assert!(p[0] >= 0.0 && p[0] <= 1.0);
                assert!(p[1] >= 0.0 && p[1] <= 1.0);
            }
        }
    }

    #[test]
    fn contour_segments_skip_masked_cells() {
        // 1 隅が None のセルは線分を出さない。
        let g = vec![vec![Some(0.0), None], vec![Some(2.0), Some(3.0)]];
        assert!(contour_line_segments(&g, 0.0, 3.0, 3).is_empty());
    }
}
