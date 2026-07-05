# Optimization History

## Overview

The Optimization History chart shows how each objective function value changes over the sequence of trials. It helps you assess whether the optimization has converged, is still improving, or has stalled.

## Controls

- **Objective** (multi-objective studies): Use the dropdown to select which objective to display.
- **Moving Average**: Toggle the moving average series to smooth out trial-to-trial variance.
- **Log Scale**: Toggle log-scale on the value axis.

## Operations

- **Toggle series**: Click on the legend entries to show or hide individual series (All Trials, Best Value, Moving Average).
- **Zoom**: Scroll the mouse wheel, or drag with the left button to zoom into the selected rectangle.
- **Pan**: Drag with the right mouse button to pan across the plot.
- **Reset view**: Double-click with the left button to restore the default view.
- **Hover**: Hover over a point to see the trial number and objective value.
- **Trial detail**: Click a point to open the trial detail modal for that trial.
- **Best value line**: a solid line indicates the running best value found up to each trial.

## How to Read

- **All Trials scatter**: each dot represents one trial's objective value at that trial number.
- **Best Value line**: the cumulative best objective value — this line only moves when a new best is found.
- **Moving Average line**: a smoothed average over a sliding window of recent trials, useful for seeing the overall trend through noisy data.
- **Downward trend** (minimization objective): the optimizer is finding better solutions. Optimization is progressing.
- **Flat plateau**: values stopped improving — the optimization may have converged, or the sampler has exhausted the search space.
- **High variance early, then stabilizing**: typical behavior for random or exploration-heavy samplers in early stages.
- **Sudden improvement after plateau**: the optimizer may have escaped a local optimum or explored a new region.
- **No clear trend**: if values jump randomly without improvement, try more trials or a different sampler.
- **Multiple objectives**: each objective is shown as a separate series. Observe whether they improve together or trade off against each other.
