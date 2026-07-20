//! 3D surface mesh construction and drawing for the 2D PDP widget's 3D view: converts
//! grid values into a depth-sorted quad mesh and paints it (surface cells, uncertainty
//! bands, observed points, and axis line segments) as a single raw `egui::Mesh`.

use crate::theme::chart_colors::COLOR_PDP_CI;
use crate::theme::colormap::ColorMap;
use crate::ui::widgets::scatter_3d::normalize_to_clip;

use super::math::normalize_value;
use super::Grid;

/// Converts grid values into a quad mesh in clip space [-1,1]^3.
/// Coordinate system: x = row index (param1), y = value (vertical axis), z = column
/// index (param2). Returns (clip coordinates of the 4 corners, cell mean value).
/// Ragged rows are skipped.
pub(crate) fn surface_quads(
    values: &[Vec<f64>],
    v_min: f64,
    v_max: f64,
) -> Vec<([[f32; 3]; 4], f64)> {
    let n_row = values.len();
    if n_row < 2 {
        return Vec::new();
    }
    let clip_at = |row: usize, col: usize, n_col: usize| -> [f32; 3] {
        let x = 2.0 * row as f32 / (n_row - 1) as f32 - 1.0;
        let z = 2.0 * col as f32 / (n_col - 1) as f32 - 1.0;
        let y = normalize_to_clip(values[row][col], v_min, v_max);
        [x, y, z]
    };

    let n_col = values[0].len();
    if n_col < 2 {
        return Vec::new();
    }
    let mut quads = Vec::with_capacity((n_row - 1) * (n_col - 1));
    for row in 0..n_row - 1 {
        for col in 0..n_col - 1 {
            if values[row].len() <= col + 1 || values[row + 1].len() <= col + 1 {
                continue;
            }
            let corners = [
                clip_at(row, col, n_col),
                clip_at(row, col + 1, n_col),
                clip_at(row + 1, col + 1, n_col),
                clip_at(row + 1, col, n_col),
            ];
            let mean = (values[row][col]
                + values[row][col + 1]
                + values[row + 1][col]
                + values[row + 1][col + 1])
                / 4.0;
            quads.push((corners, mean));
        }
    }
    quads
}

/// Adds a triangle to the raw mesh
pub(crate) fn push_tri(mesh: &mut egui::Mesh, pts: [egui::Pos2; 3], color: egui::Color32) {
    let base = mesh.vertices.len() as u32;
    for p in pts {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: p,
            uv: egui::epaint::WHITE_UV,
            color,
        });
    }
    mesh.indices.extend([base, base + 1, base + 2]);
}

/// Adds a line segment to the raw mesh as a quad (2 triangles)
pub(crate) fn push_edge(
    mesh: &mut egui::Mesh,
    a: egui::Pos2,
    b: egui::Pos2,
    color: egui::Color32,
    half_width: f32,
) {
    let v = b - a;
    let len = v.length();
    if len < f32::EPSILON {
        return;
    }
    let n = egui::vec2(-v.y, v.x) * (half_width / len);
    push_tri(mesh, [a + n, b + n, b - n], color);
    push_tri(mesh, [a + n, b - n, a - n], color);
}

/// Draws the surface mesh, uncertainty band, and observed points.
///
/// Projects each cell, depth-sorts them, and paints back-to-front (painter's
/// algorithm). Projected quads can become non-convex or extremely thin depending on
/// the viewpoint; egui's tessellator (`Shape::convex_polygon` / stroke) produces
/// diverging miter normals at sharp angles, creating spikes across the screen, so we
/// build a raw `egui::Mesh` directly from the vertex coordinates instead (safe for any
/// degenerate shape since there is no normal computation). Mesh edge lines are also
/// added to the same mesh as thin quads to preserve draw order. Bands (translucent, no
/// mesh lines), observed points, and 3D line segments (axis lines) are mixed into the
/// same depth list so overlap blending and front/back occlusion behind faces come out
/// correctly.
///
/// `pub(crate)`: exposed beyond this module because `response_surface.rs` (the
/// response-surface 3D viewer) reuses the same mesh-drawing routine.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_surface_mesh(
    painter: &egui::Painter,
    project: &impl Fn([f32; 3]) -> (egui::Pos2, f32),
    values: &[Vec<f64>],
    clip_range: (f64, f64),
    color_range: (f64, f64),
    cmap: &ColorMap,
    bands: Option<(&Grid, &Grid)>,
    points: &[([f32; 3], egui::Color32)],
    lines: &[([f32; 3], [f32; 3], egui::Color32)],
) {
    enum Prim {
        Cell {
            corners: [egui::Pos2; 4],
            color: egui::Color32,
            edges: bool,
        },
        Point(egui::Pos2, egui::Color32),
        Line(egui::Pos2, egui::Pos2, egui::Color32),
    }

    let (v_min, v_max) = clip_range;
    let (c_min, c_max) = color_range;
    let mut items: Vec<(f32, Prim)> = Vec::new();

    // Projects grid cells and appends them to the depth list.
    // If `color` is Some, use a fixed color (band); if None, use the colormap (Mean surface).
    let collect_cells =
        |items: &mut Vec<(f32, Prim)>, grid: &[Vec<f64>], flat_color: Option<egui::Color32>| {
            for (corners, mean) in surface_quads(grid, v_min, v_max) {
                let mut pts = [egui::Pos2::ZERO; 4];
                let mut depth = 0.0;
                let mut finite = true;
                for (i, c) in corners.iter().enumerate() {
                    let (p, d) = project(*c);
                    finite &= p.x.is_finite() && p.y.is_finite();
                    pts[i] = p;
                    depth += d;
                }
                // Skip drawing cells that contain non-finite values (e.g. NaN grid)
                if !finite {
                    continue;
                }
                let color = flat_color
                    .unwrap_or_else(|| cmap.interpolate(normalize_value(mean, c_min, c_max)));
                items.push((
                    depth * 0.25,
                    Prim::Cell {
                        corners: pts,
                        color,
                        edges: flat_color.is_none(),
                    },
                ));
            }
        };

    collect_cells(&mut items, values, None);
    if let Some((lower, upper)) = bands {
        collect_cells(&mut items, lower, Some(COLOR_PDP_CI()));
        collect_cells(&mut items, upper, Some(COLOR_PDP_CI()));
    }

    for (p, color) in points {
        let (pos, depth) = project(*p);
        if pos.x.is_finite() && pos.y.is_finite() {
            items.push((depth, Prim::Point(pos, *color)));
        }
    }

    for (a, b, color) in lines {
        let (pos_a, depth_a) = project(*a);
        let (pos_b, depth_b) = project(*b);
        let finite = pos_a.x.is_finite()
            && pos_a.y.is_finite()
            && pos_b.x.is_finite()
            && pos_b.y.is_finite();
        if finite {
            items.push(((depth_a + depth_b) * 0.5, Prim::Line(pos_a, pos_b, *color)));
        }
    }

    items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut mesh = egui::Mesh::default();
    for (_, prim) in &items {
        match prim {
            Prim::Cell {
                corners,
                color,
                edges,
            } => {
                let [p0, p1, p2, p3] = *corners;
                push_tri(&mut mesh, [p0, p1, p2], *color);
                push_tri(&mut mesh, [p0, p2, p3], *color);
                // Mesh lines (cell outline only; no diagonals drawn)
                if *edges {
                    let edge_color = color.gamma_multiply(0.6);
                    for (a, b) in [(p0, p1), (p1, p2), (p2, p3), (p3, p0)] {
                        push_edge(&mut mesh, a, b, edge_color, 0.35);
                    }
                }
            }
            Prim::Point(pos, color) => {
                // Flush the mesh built so far before inserting a circle Shape
                if !mesh.is_empty() {
                    painter.add(egui::Shape::mesh(std::mem::take(&mut mesh)));
                }
                painter.circle_filled(*pos, 3.0, *color);
            }
            Prim::Line(a, b, color) => {
                push_edge(&mut mesh, *a, *b, *color, 0.75);
            }
        }
    }
    if !mesh.is_empty() {
        painter.add(egui::Shape::mesh(mesh));
    }
}
