# EDF

## Overview

The EDF (Empirical Distribution Function) chart shows what fraction of completed trials achieved an objective value at or below each point on the X axis. It is Optuna's `plot_edf` equivalent: instead of looking at objective value versus trial order (as Optimization History does), it looks at the overall shape of the distribution of results, independent of when each trial ran.

## Controls

- **Objective** (multi-objective studies): Use the dropdown to select which objective's distribution to display.
- **Log Scale**: Toggle log-scale on the value (X) axis. Trials with a non-positive objective value are dropped from the curve while log scale is active (they cannot be represented on a log axis).

## Operations

- **Zoom**: Scroll the mouse wheel, or drag with the left button to zoom into the selected rectangle.
- **Pan**: Drag with the right mouse button to pan across the plot.
- **Reset view**: Double-click with the left button to restore the default view.
- **Hover**: Hover near the curve to see a tooltip with the objective value and the cumulative fraction of trials at that point.
- **Toggle series**: When comparison studies are loaded, click a legend entry to show or hide that study's curve.

## How to Read

- **X axis**: the objective value. **Y axis**: the fraction (0 to 1) of completed trials whose objective value is at or below that X value — this is the empirical CDF.
- **Steep, narrow curve**: most trials converge to a similar objective value — the search is well-behaved and consistent, or the search space is not very sensitive to the sampled hyperparameters.
- **Shallow, wide curve**: objective values are spread out over a broad range — trials vary widely in quality, suggesting high sensitivity to hyperparameters or an unstable optimization process.
- **Left-shifted curve** (for a minimization objective): the study as a whole found better values more often — comparing two studies' EDFs this way tells you which sampler/configuration produced better results overall, not just in the single best trial.
- **Right-shifted curve**: worse values are more common in that study.
- **Comparing studies**: overlaying the base study and one or more comparison studies makes it easy to see which one has a systematically better (further left, for minimization) distribution of outcomes — a more robust comparison than looking only at the single best value, since it reflects the whole run rather than a lucky outlier.
- **Flat sections**: no trials landed in that value range — often visible as gaps or plateaus in the staircase.
- **Vertical jumps**: many trials share (or are very close to) the same objective value at that point — common with discrete/categorical search spaces or with a sampler that repeatedly proposes similar configurations.
