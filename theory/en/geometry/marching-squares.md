# Marching Squares Contour Extraction

## Overview

This process extracts iso-lines (contour lines) for a specified value, as line segments, from the
masked value grid (`ObservedSurface`) built by
[Delaunay Triangulation Interpolation](delaunay-interpolation.md). The contour lines drawn by the
"Contours" option on the Observed Contour widget
([Observed Contour](../widgets/observed-contour.md)) are exactly this set of line segments.

Marching squares is a classic algorithm for extracting iso-lines from a scalar field defined on a
grid. It processes each grid cell (its four corner values) independently, finding the crossing
point between the contour level and each cell edge by linear interpolation.

---

## Level selection

The values (levels) at which contour lines are drawn are chosen by dividing the interval between
the grid's minimum value $v_{\min}$ and maximum value $v_{\max}$ into $(n_{\text{levels}}+1)$
equal parts, and using the $n_{\text{levels}}$ interior division points (no line is drawn at
either endpoint $v_{\min}$ or $v_{\max}$ itself):

$$
\text{level}_i = v_{\min} + (v_{\max} - v_{\min}) \cdot \frac{i}{n_{\text{levels}}+1}, \qquad i = 1, \dots, n_{\text{levels}}
$$

If $v_{\max} - v_{\min}$ is effectively zero (a nearly constant grid), no contour lines are drawn
and the result is an empty list of segments.

---

## Cell scanning and edge crossings

The grid is divided into $1 \times 1$ square cells, and each cell's four corners —
$(\text{tl}, \text{tr}, \text{br}, \text{bl})$: top-left, top-right, bottom-right, bottom-left —
are examined. **Any cell containing even one `None` (masked) corner is skipped entirely.** Not
drawing contours across masked cells keeps contour lines from extending into regions with no data,
consistent with the "no extrapolation beyond the convex hull" design of
[Delaunay interpolation](delaunay-interpolation.md).

For cells where all four corners are `Some`, each of the four edges (top, right, bottom, left) is
checked for whether its two endpoints straddle the level. For an edge with endpoints $a, b$
(oriented from $a$ to $b$):

$$
t = \operatorname{clamp}\!\left(\frac{\text{level} - a}{b - a},\ 0,\ 1\right)
$$

gives the crossing ratio. If $a \ge \text{level}$ and $b \ge \text{level}$ have the same truth
value (both endpoints on the same side — no crossing), there is no intersection on that edge.
There is also no intersection when $|b-a|$ is below the floating-point epsilon (avoiding an
unstable division on a near-flat edge).

Each cell yields at most 4 crossing points (one per edge), and the number of crossings determines
the segments produced:

| Crossings | Behavior |
| --- | --- |
| 2 | Emit one segment connecting the two crossing points |
| 4 | Emit two segments: one connecting the "top, right" crossings, one connecting the "bottom, left" crossings (the saddle case, see below) |
| Other (0) | Emit no segment |

---

## Coordinate system

Segment coordinates are returned in the grid's **sample index space**: the top-left corner of cell
$(r, c)$ (row $r$, column $c$) corresponds to $(x, y) = (c, r)$, and an edge crossing is offset by
$t$ from there. Mapping to actual screen coordinates — interpreting cell centers as world-space
sample points — is the renderer's responsibility; this function itself only returns segments on
the dimensionless cell grid.

---

## General theory: saddle-point ambiguity

Marching squares is the 2D counterpart of marching cubes (Lorensen & Cline, 1987), the algorithm
that turns a 3D scalar field into an iso-surface. Each cell's four corners are each either at or
above the level, or below it, giving $2^4 = 16$ possible classifications (corresponding to the
marching cubes classification table).

Among these 16 cases are the **saddle cases**, where diagonal corners are on the same side and
adjacent corners are on the opposite side (for example, top-left and bottom-right both at or above
the level while top-right and bottom-left are below, or the reverse). In this case all four edges
cross the level, and there are two possible ways to pair the crossings into two segments — which
pairing is "correct" cannot be determined from the corner values alone. This is not a defect of
this implementation; it is a well-known algorithmic ambiguity of marching squares in general.
Resolving it exactly requires additional information, such as interpolating a value at the cell
center (e.g., evaluating the diagonal of a bilinear interpolant).

This implementation does not resolve this ambiguity. In the 4-crossing case, it uses a fixed
pairing — the 1st and 2nd crossing found during the edge scan are paired, and the 3rd and 4th are
paired — rather than choosing based on any interior value. Saddle cases occur less often as grid
resolution increases, and the effect on Observed Contour's primary purpose — visualizing the
coarse-grained pattern of where values are high or low — is judged to be small.

---

## References

- Lorensen, W. E., & Cline, H. E. (1987). Marching Cubes: A High Resolution 3D Surface
  Construction Algorithm. _Computer Graphics (SIGGRAPH '87 Proceedings)_, 21(4), 163–169. (Marching
  squares corresponds to the 2D case of this algorithm.)
