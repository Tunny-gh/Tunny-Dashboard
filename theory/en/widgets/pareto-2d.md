# Pareto Scatter 2D

## Overview

The Pareto Scatter 2D chart displays optimization trials as a 2D scatter plot of two objective functions, highlighting the **Pareto front** — the set of non-dominated solutions where no trial is strictly better than another on all objectives simultaneously.

Use this chart to explore trade-offs between two objectives and identify the best-compromise trials on the Pareto front.

## Controls

- **X / Y axis**: Use the X and Y dropdowns to select which objective is plotted on each axis.
- **Surrogate front**: When a surrogate optimization result is available, check "Surrogate front" to overlay the predicted Pareto front as gold diamond markers.
- **Infeasible trials** (constrained studies only): always shown in a distinct colour when the study has constraints — there is no control to hide them.

## Operations

- **Zoom**: Scroll the mouse wheel, or drag with the left button to zoom into the selected rectangle.
- **Pan**: Drag with the right mouse button to pan across the plot.
- **Reset view**: Double-click with the left button to restore the default view.
- **Hover**: Hover over a point to see the trial number and objective values in a tooltip.
- **Select (single)**: Click a point to open the trial detail modal for that trial.
- **Select (region)**: Hold Shift and drag with the left button to draw a rectangular selection region and highlight all enclosed points.
- **Clear selection**: Click blank space to clear the current selection.

## How to Read

- **Pareto front points** (red): non-dominated trials — no other trial is better in both objectives simultaneously. These are your best candidates.
- **Non-Pareto points** (blue): dominated trials — another trial is at least as good in every objective and strictly better in at least one.
- **Infeasible points**: trials that violate constraints, shown in a distinct colour.
- **Unselected points** (greyed): points outside the current brush/selection region.
- **Trade-off shape**: a smooth Pareto curve indicates a continuous trade-off. A scattered front may indicate insufficient trials or conflicting objectives.
- **Gaps in the front**: regions where no solution has been found — consider running more trials in those areas.
- **Outliers far from the front**: dominated trials that performed poorly on both objectives.

> Pareto front (non-dominated) classification is computed with fast non-dominated sort (Deb et al. 2002). See [NSGA-II — Fast Non-dominated Sort](../optimization/nsga2.md) for the algorithm.
