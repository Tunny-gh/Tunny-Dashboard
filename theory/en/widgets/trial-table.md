# Trial Table

## Overview

The Trial Table shows all optimization trials in a sortable, filterable table. Each row represents one trial with columns for trial number, parameters, objective values, and trial state (complete, pruned, failed).

Use this table to inspect individual trial results, sort by objective values to find top performers, and export data for further analysis.

## Operations

- **Sort**: Click any column header to sort trials by that column (ascending). Click again to sort descending.
- **Select trial**: Click a row to select that trial and highlight it across all other widgets (scatter plots, parallel coordinates, etc.).
- **Multi-select**: Hold Ctrl (or Cmd on macOS) and click additional rows to select multiple trials.
- **Filter by state**: Use the state filter dropdown (if available) to show only complete, pruned, or failed trials.
- **Export**: Click the Export button to download the current table data as a CSV file.
- **Column resize**: Drag the column dividers in the header to resize columns.

## How to Read

- **Trial number**: sequential index of each trial in the study. Lower numbers = earlier trials.
- **Parameter columns**: the parameter values used for that trial.
- **Objective columns**: the objective function values returned by that trial. Lower is better for minimization objectives; higher is better for maximization.
- **State column**:
  - `Complete` — trial finished successfully with valid objective values
  - `Pruned` — trial was stopped early (e.g., by Optuna's pruner)
  - `Failed` — trial raised an exception or returned invalid values
- **Sorting by objective**: sort the objective column descending (or ascending for minimization) to quickly find the best trials.
- **Cross-widget interaction**: selecting a row highlights that trial in all other open widgets, making it easy to locate in scatter plots or parallel coordinates.
