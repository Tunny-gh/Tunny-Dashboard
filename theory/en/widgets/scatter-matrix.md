# Scatter Matrix

## Overview

The Scatter Matrix (also called a pairplot) shows pairwise scatter plots for every combination of selected parameters and objectives. It provides a compact overview of correlations and interactions across all variable pairs simultaneously.

## Operations

- **Zoom**: Scroll inside a cell to zoom into that pair's scatter plot.
- **Pan**: Click and drag inside a cell to pan.
- **Select**: Click a point in any cell to highlight that trial across all widgets.
- **Hover**: Hover over a point to see trial details in a tooltip.
- **Axis labels**: Row and column labels identify the variable for each cell. The diagonal cells show the variable name.
- **Color by**: Use the "Color by" dropdown in the control row to choose which objective function is used to color the scatter points. Points are colored according to the selected objective's value using the active colormap.

## How to Read

- **Linear pattern**: points forming a clear diagonal line indicate a strong linear correlation between the two variables.
- **Curved or nonlinear pattern**: a curved cloud of points indicates a nonlinear relationship — use PDP Chart 2D for a detailed view.
- **Circular cloud**: no systematic correlation between the two variables.
- **Clusters**: distinct groups of points in a cell suggest that trials fall into discrete categories for those two variables.
- **Off-diagonal cells**: each cell (i, j) shows variable i on the X axis and variable j on the Y axis. Cell (j, i) shows the mirror plot.
- **Looking for important parameters**: look for cells where a parameter axis shows a strong pattern with an objective axis — those parameters likely have high importance.
