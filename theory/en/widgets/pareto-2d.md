# Pareto Scatter 2D

## Overview

The Pareto Scatter 2D chart displays optimization trials as a 2D scatter plot of two objective functions, highlighting the **Pareto front** — the set of non-dominated solutions where no trial is strictly better than another on all objectives simultaneously.

Use this chart to explore trade-offs between two objectives and identify the best-compromise trials on the Pareto front.

## Operations

- **Zoom**: Scroll the mouse wheel inside the chart area to zoom in and out.
- **Pan**: Click and drag to pan across the plot.
- **Hover**: Hover over a point to see the trial number and objective values in a tooltip.
- **Select**: Click a point to highlight that trial across all widgets.
- **Objective axes**: The X and Y axes are configured from the study's objective settings.

## How to Read

- **Pareto front points** (highlighted): non-dominated trials — no other trial is better in both objectives simultaneously. These are your best candidates.
- **Dominated points** (dimmed): at least one other trial is better or equal in every objective.
- **Trade-off shape**: a smooth Pareto curve indicates a continuous trade-off. A scattered front may indicate insufficient trials or conflicting objectives.
- **Gaps in the front**: regions where no solution has been found — consider running more trials in those areas.
- **Outliers far from the front**: dominated trials that performed poorly on both objectives.
