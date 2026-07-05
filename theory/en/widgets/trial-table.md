# Trial Table

## Overview

The Trial Table shows optimization trials in a table. Each row represents one trial with columns for trial number, parameters, objective values, and Pareto rank.

Use this table to inspect individual trial results, switch views to see trials grouped by cluster or ordered by MCDM rank, and pin important trials for easy reference.

## Controls

- **View**: Use the "View" dropdown to switch between three display modes:
  - **All Trials**: shows the union of selected and pinned trials (all trials if none are selected), with all parameter, objective, and Pareto Rank columns, ordered by trial number.
  - **By Cluster**: groups trials by cluster assignment, ordered by cluster (unassigned trials last) and then by trial number within each cluster (available when clustering has been run).
  - **By MCDM Rank**: orders trials by their MCDM ranking, best rank first (available when MCDM analysis has been run).

## Operations

- **Select trial**: Click a row to select that trial and highlight it across all other widgets (scatter plots, parallel coordinates, etc.).
- **Pin / Unpin**: Click the pin button on a row to pin that trial so it remains visible in the table.
- **Column resize**: Drag the column dividers in the header to resize columns.

## How to Read

- **Trial number**: sequential index of each trial in the study. Lower numbers = earlier trials.
- **Parameter columns**: the parameter values used for that trial.
- **Objective columns**: the objective function values returned by that trial. Lower is better for minimization objectives; higher is better for maximization.
- **Pareto Rank column**: the Pareto dominance rank of the trial (0 = Pareto front, higher = more dominated).
- **Cross-widget interaction**: selecting a row highlights that trial in all other open widgets, making it easy to locate in scatter plots or parallel coordinates.
