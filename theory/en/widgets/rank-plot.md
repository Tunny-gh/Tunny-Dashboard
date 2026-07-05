# Rank Plot

## Overview

The Rank Plot is Optuna's `plot_rank` equivalent. It is a scatter plot of two selected parameters where each completed trial is colored not by its raw objective value, but by its **rank** — the percentile position of its objective value among all completed trials (0 = best, 1 = worst). Coloring by rank instead of raw value makes the plot robust to outliers and highlights how the combination of two parameters relates to overall trial quality.

## Controls

- **X / Y parameter selectors**: choose which two parameters to plot against each other. Defaults to the first two parameters in the study.
- **Objective selector** (multi-objective studies): choose which objective's rank to color by.
- If the study has fewer than 2 parameters, the chart shows a message instead of a plot (a rank plot needs two axes).

## Operations

- **Zoom**: Scroll the mouse wheel, or drag with the left button to zoom into the selected rectangle.
- **Pan**: Drag with the right mouse button to pan across the plot.
- **Reset view**: Double-click with the left button to restore the default view.
- **Hover**: Hover over a point to see the trial number, the X/Y parameter values, the objective value, and the rank percentile.
- **Trial detail**: Click a point to open the trial detail modal for that trial, including any attached artifacts.

## How to Read

- **Color scale**: points colored toward the "Best" end of the legend have objective values near the top of the study (best observed outcomes); points toward "Worst" have objective values near the bottom. The direction (minimize/maximize) of the selected objective is already accounted for — "best" always means best, regardless of direction.
- **Why rank instead of raw value**: a handful of extreme outliers can wash out a raw-value color scale, making most of the plot look like a single flat color. Ranking removes that problem — colors are always spread evenly across the full range of trials, so subtle structure is visible even when a few trials have unusually extreme results.
- **Clusters of "Best"-colored points in a region**: that combination of the two parameters tends to produce good outcomes — a promising region of the search space.
- **Clusters of "Worst"-colored points in a region**: that combination tends to produce poor outcomes — worth avoiding or investigating why.
- **No visible pattern / colors evenly mixed everywhere**: the two selected parameters (as a pair) don't strongly determine the outcome on their own — other parameters, or interactions with a third parameter, may dominate. Try other parameter pairs, or check the Importance Chart to see which parameters matter most.
- **Diagonal or curved gradients**: suggests an interaction effect between the two parameters — the optimal value of one depends on the value of the other.
- **Categorical parameters**: shown using their encoded category index on the axis, same as other scatter-based widgets (Slice Chart, Scatter Matrix).
