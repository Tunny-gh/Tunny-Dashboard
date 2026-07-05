# Delaunay Triangulation Interpolation

## Overview

Unlike PDP or surrogate response surfaces, the Observed Contour widget
([Observed Contour](../widgets/observed-contour.md)) fits no surrogate model at all. Instead, it
runs a Delaunay triangulation with the **observed trial points themselves** as vertices, then
fills each triangle's interior with piecewise-linear interpolation via barycentric coordinates to
build a value grid.

A triangulation only covers the point set's **convex hull**, so there is no basis for
interpolating outside it. Observed Contour does not extrapolate there — it masks those cells with
`None`. This is not an implementation limitation; it is a deliberate design choice to leave
unobserved regions blank rather than paint them.

Used by Tunny Dashboard in:

- The 2D/3D value grid generation for the Observed Contour widget (`observed_surface`)

---

## Preprocessing

Before triangulating, the array of observed points `(x, y, value)` goes through the following
preprocessing steps.

1. **Finite-value filter** — points where any of `x`, `y`, or `value` is non-finite (NaN or Inf)
   are dropped.
2. **Merging near-duplicate points** — the remaining points are normalized by their X/Y range and
   quantized to a grid at resolution $10^6$; points that land in the same grid cell are merged
   into one (values are arithmetic-averaged). This prevents "almost the same point" caused by
   floating-point rounding from being passed to the triangulation as separate vertices, which
   would otherwise produce degenerate triangles or coverage gaps.
3. **Count / range guard** — if fewer than 3 points remain after merging, or either the X or Y
   range is zero (degenerate), an empty grid with no axes and no data is returned.

---

## Triangulation in normalized space

Observed points can differ substantially in unit and scale between the X and Y axes (for example,
a parameter versus an objective). Feeding that scale difference straight into the triangulation
tends to produce triangles that are extremely thin along one axis, degrading interpolation
quality. To avoid this, both triangulation and point-location are done in coordinates normalized
to $[0,1]$ per axis over the observed range,
$\left(\dfrac{x - x_{\min}}{x_{\max}-x_{\min}}, \dfrac{y - y_{\min}}{y_{\max}-y_{\min}}\right)$.
The interpolated value (`value`) itself keeps its original scale.

### Properties of Delaunay triangulation

A Delaunay triangulation is the canonical triangulation defined over a planar point set. It
satisfies the **empty circumcircle property**: for every triangle, no other point lies inside its
circumcircle. A consequence of this property is that, among all triangulations of the same point
set, the Delaunay triangulation **maximizes the minimum angle** (it avoids extremely thin,
sliver-shaped triangles). Since thin triangles tend to amplify interpolation error in
piecewise-linear interpolation, this property is also desirable from a purely practical
interpolation-quality standpoint.

This implementation uses the `delaunator` crate (by Mapbox, based on the sweep-hull method).
Sweep-hull sweeps through the points while incrementally building the hull, running in
$O(n \log n)$.

### Sparsity guard

In sparsely observed regions, a Delaunay triangulation can end up bridging two distant clusters
with a long, thin triangle. Using that triangle for interpolation would create a fake surface that
looks continuous across a region with no actual data.

To prevent this, the longest edge of each triangle in normalized space,

$$
\ell = \max\bigl(\lVert a-b \rVert,\ \lVert b-c \rVert,\ \lVert c-a \rVert\bigr)
$$

is computed, and any triangle whose longest edge exceeds `max_edge_ratio` is dropped from
interpolation. `max_edge_ratio` is typically set to somewhere around 0.1–0.3; passing `0.0`
disables the sparsity guard entirely (every triangle is used).

---

## Barycentric interpolation

Given a triangle $(a, b, c)$ (vertices in normalized coordinates) and the values $v_a, v_b, v_c$
at those vertices, the barycentric coordinates $(\lambda_a, \lambda_b, \lambda_c)$ of a point
$p=(p_x, p_y)$ are given by the closed form:

$$
\begin{aligned}
D &= (b_y - c_y)(a_x - c_x) + (c_x - b_x)(a_y - c_y) \\
\lambda_a &= \frac{(b_y - c_y)(p_x - c_x) + (c_x - b_x)(p_y - c_y)}{D} \\
\lambda_b &= \frac{(c_y - a_y)(p_x - c_x) + (a_x - c_x)(p_y - c_y)}{D} \\
\lambda_c &= 1 - \lambda_a - \lambda_b
\end{aligned}
$$

$D$ is proportional to the (signed) area of the triangle, and it approaches zero when the three
points are nearly collinear (a degenerate triangle). This implementation skips any triangle with
$|D| < 10^{-15}$.

Whether point $p$ lies inside the triangle (boundary included) is determined by checking that all
three barycentric coordinates are non-negative. To tolerate rounding error, the test uses
$\lambda \ge -\varepsilon$ (with $\varepsilon = 10^{-9}$) rather than a strict $\ge 0$. When the
point is judged to be inside, the interpolated value is the convex combination

$$
v(p) = \lambda_a v_a + \lambda_b v_b + \lambda_c v_c
$$

If a grid point falls inside no triangle at all (outside the convex hull, or inside a region whose
triangle was dropped by the sparsity guard), the result is `None`. This explicitly signals that no
interpolation is possible; the value is not clamped or defaulted to zero — it is masked as "no
data."

---

## Implementation notes

- Triangle lookup is a simple linear scan that stops at the first hit; there is no spatial index
  (such as a k-d tree) accelerating it.
- The constants used for normalization, quantization, and degeneracy checks — quantization
  resolution $10^6$, containment tolerance $\varepsilon=10^{-9}$, and the degeneracy threshold
  $10^{-15}$ — all match the values in the implementation (`rust_core/src/contour/mod.rs`).
- If the triangulation itself comes back empty (for example, all points are collinear, so
  `delaunator` returns no triangles), the result has axes but every cell is `None` (it does not
  panic).
- The masked value grid produced here is the input to contour-line extraction
  ([Marching Squares](marching-squares.md)).

---

## References

- de Berg, M., Cheong, O., van Kreveld, M., & Overmars, M. (2008). _Computational Geometry:
  Algorithms and Applications_ (3rd ed.). Springer. (A standard textbook treatment of Delaunay
  triangulation and the empty circumcircle property.)
- delaunator (Mapbox): a fast 2D Delaunay triangulation implementation based on the sweep-hull
  method. https://github.com/mapbox/delaunator
