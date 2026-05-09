# Optimization History

## Overview

The Optimization History chart shows how each objective function value changes over the sequence of trials. It helps you assess whether the optimization has converged, is still improving, or has stalled.

## Operations

- **Toggle objectives**: Click on the legend entries to show or hide individual objective series.
- **Zoom**: Scroll the mouse wheel to zoom into a time range.
- **Pan**: Click and drag to pan along the trial axis.
- **Hover**: Hover over a point to see the trial number and objective value.
- **Best value line**: a dashed line (if shown) indicates the best value found up to each trial.

## How to Read

- **Downward trend** (minimization objective): the optimizer is finding better solutions. Optimization is progressing.
- **Flat plateau**: values stopped improving — the optimization may have converged, or the sampler has exhausted the search space.
- **High variance early, then stabilizing**: typical behavior for random or exploration-heavy samplers in early stages.
- **Sudden improvement after plateau**: the optimizer may have escaped a local optimum or explored a new region.
- **No clear trend**: if values jump randomly without improvement, try more trials or a different sampler.
- **Multiple objectives**: each objective is shown as a separate series. Observe whether they improve together or trade off against each other.
