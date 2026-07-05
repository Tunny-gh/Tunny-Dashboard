# Intermediate Values

## Overview

The Intermediate Values chart overlays the learning curve of every trial that reported intermediate values during optimization — the same values samplers and pruners see as a trial progresses (e.g. validation loss per epoch, per boosting round, per environment step). It lets you compare how trials evolve over their reporting steps and see, at a glance, which ones were cut short by pruning.

## Controls

- **Log Scale**: Toggle log-scale on the value axis. Points with a non-positive value are dropped from a curve while log scale is active (they cannot be represented on a log axis).

## Operations

- **Zoom**: Scroll the mouse wheel, or drag with the left button to zoom into the selected rectangle.
- **Pan**: Drag with the right mouse button to pan across the plot.
- **Reset view**: Double-click with the left button to restore the default view.
- **Hover**: Hover near a curve to highlight it and see a tooltip with the trial number, state, step, and value at that point. Other curves are dimmed while one is highlighted.

## How to Read

- **Each line is one trial**: the X axis is the reporting step (e.g. epoch or boosting round) and the Y axis is the intermediate value at that step.
- **Color by trial state**: complete trials, pruned trials, running trials, and failed trials are colored differently (see the legend). Pruned curves typically stop earlier than complete ones — that is the pruner cutting off an unpromising trial.
- **Tight bundle of curves**: most trials follow a similar trajectory, suggesting the search space is not very sensitive to the sampled hyperparameters, or the model/data dominates the outcome.
- **Wide spread, especially early**: hyperparameters strongly affect the learning trajectory; pruning based on early steps is likely to be effective.
- **Many short pruned curves**: the pruner is aggressively cutting unpromising trials early, which speeds up the search — as long as it is not also cutting trials that would have improved later.
- **Optuna context**: this view mirrors Optuna's own intermediate-value reporting (`trial.report()` + `should_prune()`). It is the primary way to sanity-check that your pruner (e.g. `MedianPruner`, `HyperbandPruner`) is behaving as intended before trusting its verdicts at scale.

Performance note: if the study has more than 2000 trials with intermediate values, the chart evenly subsamples down to 2000 curves and shows a note indicating how many trials are displayed.
