# Radar Comparison

The Radar Comparison widget overlays **pinned trials** as polygons on a radar (star) chart, one axis per objective (and optionally per parameter). It is the decision-phase companion to the Pareto and MCDM views: once a handful of candidate designs has been narrowed down, pin them and compare their profiles in a single glyph — which candidate is balanced, which one buys its strength in one objective with weakness in another.

## Workflow

1. Pin the candidate trials to compare (📌 in the Trial Table or the trial detail modal; up to 20 pins are kept). The widget shows every pinned trial as one colored polygon with a legend entry.
2. Choose the axes: objectives are always shown; enable **Include parameters** to add the numeric parameters. A radar chart needs at least 3 axes to be readable — with a 2-objective study, enable parameters or pin the widget next to a Pareto view instead.
3. Toggle **Outward = better** (default on) to orient all objective axes so that larger polygons are better (see below). With the toggle off, axes show plain normalized values.

## Normalization

Each axis is min-max normalized over all completed trials of the study, so different units and scales become comparable radii in $[0, 1]$:

$$
u = \frac{v - \min_a}{\max_a - \min_a}
$$

With **Outward = better** enabled, axes of *minimized* objectives are flipped ($u \mapsto 1 - u$) so that "further out" consistently means "better" on every objective axis, regardless of the optimization direction. Parameter axes have no better/worse direction and are never flipped. A degenerate axis (all values equal) is drawn at mid radius.

## Reading the chart

- **Balanced vs. specialized designs**: a regular, round polygon is a balanced candidate; a spiky one excels on some axes at the cost of others.
- **Dominance at a glance**: with *Outward = better*, a polygon that fully contains another is better on every shown objective (the contained trial is dominated within the displayed axes).
- **Grid rings** mark the 25/50/75/100 % levels of the normalized range.

## Caveats

- **The enclosed area is not a score.** Area grows quadratically with radius and depends on how the axes happen to be ordered; adjacent axes also visually correlate. Use the polygon shape for qualitative comparison and the MCDM widgets for a defensible ranking.
- **Axis order is arbitrary** (objectives in study order, then parameters). The same data with a different axis order produces a differently shaped polygon — do not over-interpret shape features that depend on which axes are neighbors.
- Min-max normalization is study-relative: a trial near the middle of a radar axis is average *within this study*, not in any absolute sense, and pinning/unpinning trials does not change the scale (the whole study does).
- For many trials or a continuous view of all trials, use [Parallel coordinates](parallel-coords.md) — the radar is intentionally limited to the pinned few.

## Relation to the trial-detail radar

The trial detail modal also shows a radar chart, but the two answer different questions with different scales:

| | Trial-detail radar (modal) | Radar Comparison (this widget) |
|---|---|---|
| Question | *Where does this one trial sit relative to the Pareto front?* | *How do my shortlisted candidates compare against each other?* |
| Series | the selected trial over all Pareto-front individuals (context cloud) | the pinned trials only |
| Axis scale | Pareto-front envelope (outer ring = front max) | study-wide min-max over all completed trials |
| Direction handling | none (raw position within the front range) | optional *Outward = better* flip for minimized objectives |
| Lifetime | transient (open per trial) | persistent canvas widget, settings saved in sessions |

Both charts share the same rendering core, so grids, spokes, and label conventions look identical.
