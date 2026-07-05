# Pareto Scatter 3D

## Overview

The Pareto Scatter 3D chart extends the 2D Pareto view to three objective functions simultaneously, displaying non-dominated trials as a 3D Pareto surface. This is useful for multi-objective optimization with three competing objectives.

## Controls

- **Show Infeasible** (constrained studies only): Toggle display of infeasible trials.

## Operations

- **Rotate**: Right-click and drag to rotate the 3D view (arcball rotation).
- **Pan**: Middle-click drag, or Shift + right-click drag, to pan the view.
- **Zoom**: Scroll the mouse wheel to zoom in and out.
- **Hover**: Hover over a point to show a tooltip with its objective values and Pareto rank.
- **Trial details**: Left-click a point to open its detail modal (objective/variable values, artifacts).

## How to Read

- **Pareto surface points** (highlighted): non-dominated trials — no single trial dominates all three objectives simultaneously.
- **Dominated points**: trials for which another trial is at least as good on all three objectives and strictly better on at least one. Pareto-front and dominated points are distinguished by colour, not by dimming; dimming (reduced opacity) instead marks points that fall outside the current selection filter, regardless of Pareto rank.
- **3D trade-off surface**: the Pareto front forms a surface (rather than a curve in 2D). Points on this surface represent optimal trade-offs among all three objectives.
- **Perspective matters**: rotate to different angles to fully understand the shape of the Pareto surface. Some regions may only be visible from certain viewpoints.
- **Sparse surfaces**: fewer trials means larger gaps on the front — add more trials to fill in the trade-off space.

> The Pareto rank shown in the tooltip (rank 0 = Pareto front, rank 1 = non-dominated among the rest, and so on) is computed with fast non-dominated sort (Deb et al. 2002). See [NSGA-II — Fast Non-dominated Sort](../optimization/nsga2.md) for the algorithm.
