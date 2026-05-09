# Pareto Scatter 3D

## Overview

The Pareto Scatter 3D chart extends the 2D Pareto view to three objective functions simultaneously, displaying non-dominated trials as a 3D Pareto surface. This is useful for multi-objective optimization with three competing objectives.

## Operations

- **Rotate**: Click and drag to rotate the 3D view (arcball rotation).
- **Zoom**: Scroll the mouse wheel to zoom in and out.
- **Pan**: Hold Shift and drag to pan.
- **Hover**: Hover over a point to see trial number and all three objective values.
- **Select**: Click a point to highlight that trial across all widgets.
- **Reset view**: Double-click to reset the camera to the default perspective.

## How to Read

- **Pareto surface points** (highlighted): non-dominated trials — no single trial dominates all three objectives simultaneously.
- **Dominated points** (dimmed): at least one other trial is strictly better on all three objectives.
- **3D trade-off surface**: the Pareto front forms a surface (rather than a curve in 2D). Points on this surface represent optimal trade-offs among all three objectives.
- **Perspective matters**: rotate to different angles to fully understand the shape of the Pareto surface. Some regions may only be visible from certain viewpoints.
- **Sparse surfaces**: fewer trials means larger gaps on the front — add more trials to fill in the trade-off space.
