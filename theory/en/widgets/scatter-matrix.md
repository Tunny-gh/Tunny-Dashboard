# Scatter Matrix

## Overview

The Scatter Matrix (also called a pairplot) shows pairwise scatter plots for every combination of selected parameters and objectives. It provides a compact overview of correlations and interactions across all variable pairs simultaneously.

## Controls

- **Color by**: Use the "Color by" dropdown to choose which objective function is used to color the scatter points. Points are colored according to the selected objective's value using the active colormap.
- **Show Infeasible** (constrained studies only): Toggle display of infeasible trials.

## How to Read

- **Axis labels**: Row and column labels identify the variable for each cell. The diagonal cells show a histogram of that variable's distribution.
- **Selection**: the scatter matrix does not participate in cross-widget selection — it neither highlights selections made in other widgets nor generates its own when clicked.

## Correlation Coefficient Cells (Upper Triangle)

The correlation coefficient shown in the upper-triangle cells is the [Pearson product-moment correlation](../statistics/pearson-correlation.md) of the two variables' **raw values**. It ranges from $-1$ (negative linear correlation) through $0$ (no correlation) to $+1$ (positive linear correlation); a larger absolute value means a stronger linear co-movement. The cell is shaded according to the magnitude.

Because Pearson correlation measures linear co-movement, note that a curved (non-linear) relationship can yield a small coefficient even when the relationship is strong.

## Interpreting Cells

- **Linear pattern**: points forming a clear diagonal line indicate a strong linear correlation between the two variables.
- **Curved or nonlinear pattern**: a curved cloud of points indicates a nonlinear relationship — use PDP Chart 2D for a detailed view.
- **Circular cloud**: no systematic correlation between the two variables.
- **Clusters**: distinct groups of points in a cell suggest that trials fall into discrete categories for those two variables.
- **Triangular layout**: the diagonal cells show a histogram of each variable's distribution; the lower-triangle cells (row > column) show a scatter plot of the column variable (X) against the row variable (Y); the upper-triangle cells (row < column) show the pairwise correlation coefficient instead of a mirrored scatter plot — each variable pair is plotted only once.
- **Looking for important parameters**: look for cells where a parameter axis shows a strong pattern with an objective axis — those parameters likely have high importance.
