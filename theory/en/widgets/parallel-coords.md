# Parallel Coordinates

## Overview

Parallel Coordinates displays each trial as a polyline crossing vertical axes — one axis per parameter or objective. This allows simultaneous visualization of high-dimensional data, making it easy to see patterns, correlations, and clusters across many variables at once.

## Controls

- **Color by**: Use the "Color by" dropdown to select which axis value is used to colour the polylines, according to the active colormap.
- **Axes**: Use the "Axes" dropdown to show or hide individual axes. "All" and "None" buttons toggle all axes at once.
- **Show Infeasible** (constrained studies only): Toggle display of infeasible trials (shown in a distinct colour).

## Operations

- **Brush / filter**: Click and drag on an axis to create a range filter — only trials passing through the selected range are highlighted.
- **Move brush**: Click and drag an existing brush range to slide it along the axis.
- **Multi-brush**: Create brushes on multiple axes simultaneously to narrow down trials.
- **Clear all brushes**: Double-click or right-click anywhere on the chart to clear all active brushes.

## How to Read

- **Converging lines**: lines that converge to a narrow band on an axis indicate that good trials share a similar value for that variable.
- **Parallel bands**: multiple lines forming a consistent band across several axes suggest a cluster of similar trials.
- **Crossing lines**: lines that cross each other between two axes indicate a negative correlation between those two variables — when one goes up, the other tends to go down.
- **Parallel lines**: lines that do not cross between two axes indicate a positive correlation.
- **Outlier lines**: lines that deviate sharply from the main bundle are outlier trials — they may represent unusual solutions worth investigating.
- **Use brushing**: drag on the objective axis to select only top-performing trials, then observe which parameter values they share.
