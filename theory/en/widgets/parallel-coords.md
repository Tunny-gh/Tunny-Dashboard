# Parallel Coordinates

## Overview

Parallel Coordinates displays each trial as a polyline crossing vertical axes — one axis per parameter or objective. This allows simultaneous visualization of high-dimensional data, making it easy to see patterns, correlations, and clusters across many variables at once.

## Operations

- **Axis highlight**: Click on an axis label to highlight it and see which trials span that axis range.
- **Brush / filter**: Click and drag on an axis to create a range filter — only trials passing through the selected range are highlighted.
- **Multi-brush**: Create brushes on multiple axes simultaneously to narrow down trials.
- **Clear brush**: Click outside the brushed region on an axis to remove that filter.
- **Reorder axes**: Drag axis labels to reorder the columns.
- **Select trial**: Click a line to select that trial and highlight it across all widgets.

## How to Read

- **Converging lines**: lines that converge to a narrow band on an axis indicate that good trials share a similar value for that variable.
- **Parallel bands**: multiple lines forming a consistent band across several axes suggest a cluster of similar trials.
- **Crossing lines**: lines that cross each other between two axes indicate a negative correlation between those two variables — when one goes up, the other tends to go down.
- **Parallel lines**: lines that do not cross between two axes indicate a positive correlation.
- **Outlier lines**: lines that deviate sharply from the main bundle are outlier trials — they may represent unusual solutions worth investigating.
- **Use brushing**: drag on the objective axis to select only top-performing trials, then observe which parameter values they share.
