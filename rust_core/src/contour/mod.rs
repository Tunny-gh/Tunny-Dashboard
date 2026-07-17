//! Build a contour grid interpolated only from observed trial points (surrogate-independent).
//!
//! For the Observed Contour widget. Unlike PDP or a surrogate response surface, this
//! trains no model at all — it builds the value grid using only **Delaunay linear
//! interpolation of observed points**. Regions with no data (outside the convex hull,
//! or triangle regions dropped by the sparsity guard) are masked with `None`, and
//! **no extrapolation** is performed.
//!
//! Coordinates are triangulated and point-location tests are done in normalized
//! space `[0,1]^2` to absorb the scale difference between X/Y (e.g. parameter vs.
//! objective), while the interpolated value uses the original `value`.

use delaunator::{triangulate, Point};

use crate::math::grid::linspace;

/// Grid interpolated only from observed points. `None` cells mean no data (masked = no extrapolation).
#[derive(Debug, Clone)]
pub struct ObservedSurface {
    /// X-axis grid (linspace over the observed X range, original units).
    pub x_values: Vec<f64>,
    /// Y-axis grid (linspace over the observed Y range, original units).
    pub y_values: Vec<f64>,
    /// Grid of interpolated values. `z[i][j]` is the value at (x_values[i], y_values[j]).
    /// `None` = outside the convex hull / triangle region dropped by the sparsity guard.
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

/// Triangle in normalized space (vertex coordinates and each vertex's value).
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

/// Returns a grid built by Delaunay linear interpolation over observed points `(x, y, value)`, limited to the convex hull interior.
///
/// - `n_grid`: number of grid points per side (e.g. 60).
/// - `max_edge_ratio`: sparsity guard. If a triangle's longest edge in normalized
///   space exceeds `max_edge_ratio`, the triangle is dropped (so distant clusters
///   aren't bridged by a spurious surface). `0.0` disables it; typical range 0.1-0.3.
///
/// Returns an empty grid when there are fewer than 3 points or the range is degenerate (never panics).
/// When points are collinear and no triangle can be formed, returns a grid with axes but all cells `None`.
pub fn observed_surface(pts: &[[f64; 3]], n_grid: usize, max_edge_ratio: f64) -> ObservedSurface {
    if n_grid == 0 {
        return ObservedSurface::empty();
    }

    // 1. Keep only finite points and merge near-duplicate (x,y) points (averaging the value).
    let cleaned = clean_points(pts);
    if cleaned.len() < 3 {
        return ObservedSurface::empty();
    }

    // 2. Bounding box (original units).
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

    // 3. Triangulate in normalized coordinates [0,1]^2 (absorbs the X/Y scale difference).
    let norm: Vec<Point> = cleaned
        .iter()
        .map(|p| Point {
            x: (p[0] - xmin) / xr,
            y: (p[1] - ymin) / yr,
        })
        .collect();
    let tri = triangulate(&norm);
    if tri.triangles.is_empty() {
        // Collinear points, etc. Axes exist but interpolation is impossible, so mask everything.
        return ObservedSurface {
            x_values,
            y_values,
            z: vec![vec![None; n_grid]; n_grid],
        };
    }

    // 4. Triangle list (with sparsity guard applied).
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

    // 5. Point-location + barycentric interpolation on the normalized grid (sped up via a triangle-bbox bucket index).
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

/// Grid bucket index keyed by triangle bounding boxes (assumes normalized space [0,1]^2).
///
/// To avoid a linear scan over all triangles per grid cell (O(grid² × tris)),
/// this stacks the indices of triangles whose bbox overlaps each cell of a
/// uniform nb×nb bucket grid, in ascending order; point-location then only
/// examines the candidates in the point's own bucket. Since candidates within
/// a bucket stay in ascending triangle-index order, this exactly matches the
/// result of a linear scan where "the first triangle containing the point wins."
struct TriGridIndex {
    tris: Vec<Tri>,
    /// Number of buckets per side of the bucket grid.
    nb: usize,
    /// `buckets[iy * nb + ix]` = indices of triangles whose bbox overlaps this cell (ascending).
    buckets: Vec<Vec<u32>>,
}

impl TriGridIndex {
    fn new(tris: Vec<Tri>) -> Self {
        // Use roughly sqrt(triangle count) buckets so candidates per cell stay O(√tris).
        let nb = ((tris.len() as f64).sqrt().ceil() as usize).clamp(1, 64);
        let mut buckets = vec![Vec::new(); nb * nb];
        // The barycentric-coordinate test accepts points up to EPS(1e-9) outside the
        // triangle, so expand the bbox by a margin comfortably larger than that to
        // avoid missing candidates.
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

    /// Coordinate (approximately in [0,1]) → bucket index. Negative values saturate to 0 (per `as usize` semantics).
    fn cell(v: f64, nb: usize) -> usize {
        ((v * nb as f64) as usize).min(nb - 1)
    }

    /// Barycentric-interpolates using the first (lowest-index) triangle containing
    /// point `(px, py)`. Returns `None` if no triangle contains it (outside the
    /// convex hull / masked region).
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

/// Returns the barycentric-interpolated value if point `(px, py)` lies inside triangle `t` (within numerical tolerance at the boundary).
fn barycentric_value(t: &Tri, px: f64, py: f64) -> Option<f64> {
    const EPS: f64 = 1e-9;
    let denom = (t.by - t.cy) * (t.ax - t.cx) + (t.cx - t.bx) * (t.ay - t.cy);
    if denom.abs() < 1e-15 {
        return None; // Degenerate triangle.
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

/// Euclidean distance in normalized space.
fn dist(a: &Point, b: &Point) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

/// Extracts only finite points and merges points whose (x,y) are close together
/// into one (averaging the value). Duplicates are detected by quantizing onto a
/// normalized grid (resolution 1e6), which is robust to floating-point equality issues.
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
        // Degenerate (rejected later). Skip duplicate merging.
        return finite;
    }

    const Q: f64 = 1.0e6;
    // key -> (sum_x, sum_y, sum_v, count).
    // HashMap iteration order is randomized per instance, so we separately track
    // insertion order (= original data order) to make the output order deterministic.
    // If the order drifted, the triangulation order would change, causing the
    // interpolated values at grid points on boundaries to jitter in the last digit
    // from run to run.
    let mut acc: HashMap<(i64, i64), (f64, f64, f64, u32)> = HashMap::new();
    let mut order: Vec<(i64, i64)> = Vec::new();
    for p in &finite {
        let kx = ((p[0] - xmin) / xr * Q).round() as i64;
        let ky = ((p[1] - ymin) / yr * Q).round() as i64;
        let e = acc.entry((kx, ky)).or_insert_with(|| {
            order.push((kx, ky));
            (0.0, 0.0, 0.0, 0)
        });
        e.0 += p[0];
        e.1 += p[1];
        e.2 += p[2];
        e.3 += 1;
    }
    order
        .iter()
        .map(|k| {
            let (sx, sy, sv, c) = acc[k];
            let c = c as f64;
            [sx / c, sy / c, sv / c]
        })
        .collect()
}

// ============================================================
// Density grid of observed points (for scatter-overlay shading)
// ============================================================

/// Bins observed points into (nx-1)×(ny-1) cells, applies local smoothing with
/// radius `blur_radius`, and returns the normalized density (0..1) divided by the
/// max value. Cell (i,j) covers x∈[x_i,x_{i+1}], y∈[y_j,y_{j+1}]. A single cell is
/// mostly 0/1 and noisy, so smoothing is applied to express regional shading.
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

/// Applies a separable box blur (neighborhood average) with radius `r` to a 2D grid. `r == 0` is the identity.
pub fn box_blur_2d(grid: &[Vec<f32>], r: usize) -> Vec<Vec<f32>> {
    if r == 0 || grid.is_empty() {
        return grid.to_vec();
    }
    let nx = grid.len();
    let ny = grid[0].len();
    // Horizontal moving average.
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
    // Vertical moving average.
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
// Mask-aware contour segment extraction (marching squares)
// ============================================================

/// Extracts contour-line segments from a masked value grid (marching squares).
/// Only cells whose 4 corners are all `Some` are considered. Draws `n_levels`
/// contour lines at internal levels obtained by evenly dividing `v_min..v_max`.
///
/// The returned coordinates are in the grid's sample-index space (the sample at
/// `display[r][c]` is `[c as f64, r as f64]`). The rendering side maps cell
/// centers as sample positions to screen coordinates.
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
                    continue; // Skip incomplete cells; no contour is drawn there.
                };
                // Collect intersection points on the 4 edges (top, right, bottom, left).
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

/// If the two edge endpoints `a`,`b` straddle `level`, returns the intersection ratio `t`(0..1) along a→b.
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

    /// Counts the number of `Some` entries in the value grid.
    fn count_some(s: &ObservedSurface) -> usize {
        s.z.iter().flatten().filter(|v| v.is_some()).count()
    }

    #[test]
    fn interpolates_plane_inside_hull_and_masks_outside() {
        // Triangle spanning the plane value = x + 2y from 3 points (region x+y<=1).
        let pts = [[0.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 1.0, 2.0]];
        let s = observed_surface(&pts, 11, 0.0);
        assert_eq!(s.x_values.len(), 11);
        assert_eq!(s.z.len(), 11);

        // Inside the triangle (x+y<=1), matches the plane value. i=2 → x=0.2, j=2 → y=0.2.
        let inside = s.z[2][2].expect("inside hull should be Some");
        assert!((inside - (0.2 + 2.0 * 0.2)).abs() < 1e-9, "got {inside}");

        // Outside the triangle (x+y>1, e.g. i=9,j=9 → x=0.9,y=0.9) is masked.
        assert!(s.z[9][9].is_none(), "outside hull should be masked");
    }

    #[test]
    fn sparsity_guard_drops_bridging_triangle() {
        // Two separated clusters. The bridging triangle between them is dropped by max_edge_ratio.
        let pts = [
            [0.0, 0.0, 0.0],
            [0.05, 0.0, 0.0],
            [0.0, 0.05, 0.0],
            [1.0, 1.0, 10.0],
            [0.95, 1.0, 10.0],
            [1.0, 0.95, 10.0],
        ];
        // Strict guard: the center (0.5,0.5) is masked because the bridging triangle is dropped.
        let strict = observed_surface(&pts, 21, 0.2);
        assert!(
            strict.z[10][10].is_none(),
            "midpoint should be masked under strict guard"
        );

        // Guard disabled: the center is interpolated via the bridging triangle (more `Some` entries).
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
        // All points share the same X (a vertical line) → zero X range.
        let pts = [[1.0, 0.0, 0.0], [1.0, 1.0, 1.0], [1.0, 2.0, 2.0]];
        let s = observed_surface(&pts, 10, 0.0);
        assert!(s.x_values.is_empty());
    }

    #[test]
    fn bucketed_index_matches_linear_scan_exactly() {
        // The bucket index exactly matches a linear scan where "the containing triangle with the lowest index wins."
        let pts: Vec<[f64; 3]> = (0..40)
            .map(|i| {
                // Deterministic pseudo-random placement.
                let x = ((i * 37 + 11) % 97) as f64 / 97.0;
                let y = ((i * 53 + 29) % 89) as f64 / 89.0;
                [x, y, x * 3.0 - y * 2.0 + (x * y).sin()]
            })
            .collect();
        let s = observed_surface(&pts, 31, 0.0);
        // Rebuild the triangle list from the same input and cross-check via linear scan.
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
        // Even with many duplicates, they are merged and a surface is returned as long as triangles can be formed.
        let mut pts = vec![[0.0, 0.0, 0.0]; 20];
        pts.extend_from_slice(&[[1.0, 0.0, 1.0], [0.0, 1.0, 2.0], [1.0, 1.0, 3.0]]);
        let s = observed_surface(&pts, 9, 0.0);
        assert_eq!(s.x_values.len(), 9);
        assert!(count_some(&s) > 0);
    }

    // ── Density grid / contour segments (for the Observed Contour overlay) ──

    #[test]
    fn edge_cross_detects_straddle() {
        // 0 and 2 straddle level=1 → midpoint t=0.5.
        assert_eq!(edge_cross(0.0, 2.0, 1.0), Some(0.5));
        // Same sign yields None.
        assert!(edge_cross(0.0, 0.5, 1.0).is_none());
        assert!(edge_cross(2.0, 3.0, 1.0).is_none());
    }

    #[test]
    fn cell_density_grid_bins_and_normalizes() {
        // 3x3 grid → 2x2 cells. 2 points in the bottom-left cell, 2 in the top-right cell.
        let pts = vec![
            [0.1, 0.1, 0.0],
            [0.2, 0.2, 0.0],
            [0.9, 0.9, 0.0],
            [1.0, 1.0, 0.0], // Edge clamps to the last cell
        ];
        // blur=0 leaves the binning as-is (only normalization).
        let d = cell_density_grid(&pts, (0.0, 1.0), (0.0, 1.0), 3, 3, 0);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].len(), 2);
        // Bottom-left (i=0,j=0) has the max count of 2 → 1.0.
        assert!((d[0][0] - 1.0).abs() < 1e-6);
        // Top-right (i=1,j=1) has count 2 (0.9 and 1.0) → 1.0.
        assert!((d[1][1] - 1.0).abs() < 1e-6);
        // Empty cells are 0.
        assert!(d[0][1].abs() < 1e-6);
        assert!(d[1][0].abs() < 1e-6);
    }

    #[test]
    fn box_blur_spreads_into_neighbors() {
        // 3x3 with only the center holding a value. Radius-1 smoothing makes neighboring cells nonzero.
        let mut g = vec![vec![0.0_f32; 3]; 3];
        g[1][1] = 9.0;
        let b = box_blur_2d(&g, 1);
        assert!(b[1][1] > 0.0);
        assert!(b[0][1] > 0.0); // Bleeds into vertical/horizontal neighbors
        assert!(b[1][0] > 0.0);
        // The total sum doesn't diverge (not perfectly conserved since edges clamp the average).
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
        // 2x2 grid, values 0..3. Since levels are evenly spaced internally, a crossing segment is always produced.
        let g = vec![vec![Some(0.0), Some(1.0)], vec![Some(2.0), Some(3.0)]];
        let segs = contour_line_segments(&g, 0.0, 3.0, 2);
        assert!(!segs.is_empty());
        // Coordinates fall within the sample-index space [0,1]x[0,1].
        for (a, b) in &segs {
            for p in [a, b] {
                assert!(p[0] >= 0.0 && p[0] <= 1.0);
                assert!(p[1] >= 0.0 && p[1] <= 1.0);
            }
        }
    }

    #[test]
    fn contour_segments_skip_masked_cells() {
        // A cell with one corner as None produces no segment.
        let g = vec![vec![Some(0.0), None], vec![Some(2.0), Some(3.0)]];
        assert!(contour_line_segments(&g, 0.0, 3.0, 3).is_empty());
    }
}
