# Trial Table

## Overview

The Trial Table shows optimization trials in a sortable table. Each row represents one trial with columns for trial number, parameters, objective values, and Pareto rank.

Use this table to inspect individual trial results, sort by objective values to find top performers, and pin important trials for easy reference.

## Controls

- **View**: Use the "View" dropdown to switch between three display modes:
  - **All Trials**: shows every trial with all parameter, objective, and Pareto Rank columns.
  - **By Cluster**: groups trials by cluster assignment (available when clustering has been run).
  - **By MCDM Rank**: orders trials by their MCDM ranking score (available when MCDM analysis has been run).

## Operations

- **Sort**: Click any column header to sort trials by that column (ascending). Click again to sort descending.
- **Select trial**: Click a row to select that trial and highlight it across all other widgets (scatter plots, parallel coordinates, etc.).
- **Pin / Unpin**: Click the pin button on a row to pin that trial to the top of the table so it remains visible regardless of sort order.
- **Column resize**: Drag the column dividers in the header to resize columns.

## How to Read

- **Trial number**: sequential index of each trial in the study. Lower numbers = earlier trials.
- **Parameter columns**: the parameter values used for that trial.
- **Objective columns**: the objective function values returned by that trial. Lower is better for minimization objectives; higher is better for maximization.
- **Pareto Rank column**: the Pareto dominance rank of the trial (0 = Pareto front, higher = more dominated).
- **Sorting by objective**: sort the objective column ascending (for minimization) to quickly find the best trials.
- **Cross-widget interaction**: selecting a row highlights that trial in all other open widgets, making it easy to locate in scatter plots or parallel coordinates.
