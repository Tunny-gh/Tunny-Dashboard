# Response Surface 3D

The Response Surface 3D widget renders the **raw predicted surface of a trained surrogate** over two chosen parameters, sliced through an anchor design point. It answers "what does the model believe the objective looks like *around this design*?" — the direct visual counterpart to the numbers produced by the Surrogate Optimizer and the Robustness widget.

## Slice, not marginal

The [2D PDP widget](../sensitivity-analysis/pdp.md) shows a *marginal* effect: at each grid point of the two selected parameters, the remaining parameters are averaged out. This widget shows a *conditional slice*: the remaining parameters are **fixed at the anchor point's values**, so the surface is exactly the model's prediction along a 2-parameter plane through that design:

$$
z(x_1, x_2) = \hat{f}(x_1, x_2, \mathbf{x}_{\text{anchor}, \setminus \{1,2\}})
$$

The two answer different questions. The PDP is the better tool for "how does this parameter act *in general*"; the slice is the better tool for "what is the local landscape around *my candidate*" — ridges, plateaus, and the direction in which the candidate could still be improved.

## Workflow

1. Select the **objective** and **surrogate model**, then **Fit Surrogate** (asynchronous; same training pipeline and ≥10-trial requirement as the Surrogate Optimizer and Robustness widgets).
2. Choose the two **axis parameters** and the **anchor** — best trial (direction-aware) or a pinned trial. The grid spans each axis parameter's declared range; all other parameters are frozen at the anchor's values.
3. Set the **grid resolution** (20/30/50 per axis). Evaluation is instant on the trained model; only the fit is a long-running step.
4. Rotate/zoom with the 3D camera (same controls as the other 3D charts). **Observed points** can be overlaid; with a GP model, **uncertainty** display is available. Hovering an overlaid observed point shows a tooltip with its parameter and objective values, and clicking it opens the trial-detail modal — the same interaction as the other 3D scatter charts.

## Reading the surface

- The anchor's own prediction sits on the surface at the anchor's coordinates of the two axes — a candidate on a steep flank rather than a flat basin is sensitive to those two parameters (compare with the [robustness analysis](../optimization/robustness-analysis.md), which quantifies the same intuition with input-noise statistics).
- Overlaid observed points that lie far from the surface are *not* errors: observed trials generally have different values of the frozen parameters, so they need not lie on this slice. Dense nearby observations mean the slice is well supported; a slice through a data-sparse region is mostly extrapolation — check the GP uncertainty.
- With Ridge the surface is always a plane; visible curvature requires a GP or LightGBM model.

## Caveats

- The surface is a model belief, bounded by surrogate quality — validate fit quality (e.g. via the Surrogate Optimizer's validation plot) before trusting local features.
- Different anchors give different slices. When comparing two candidates' neighborhoods, keep the axis parameters identical and switch only the anchor.
- Axis parameters must be numeric; categorical parameters cannot form the slice plane (they are frozen at the anchor's value like all other non-axis parameters).
