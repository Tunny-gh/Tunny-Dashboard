# Timeline

## Overview

The Timeline chart draws one horizontal bar per trial spanning its start and completion time, similar to a Gantt chart. It reveals how trials were scheduled over wall-clock time — how many ran in parallel, whether workers were idle, and where slow trials created bottlenecks.

## Controls

This chart has no additional controls; the X-axis unit (seconds, minutes, or hours) is chosen automatically based on the total time span of the study.

## Operations

- **Zoom**: Scroll the mouse wheel, or drag with the left button to zoom into the selected rectangle.
- **Pan**: Drag with the right mouse button to pan across the plot.
- **Reset view**: Double-click with the left button to restore the default view.
- **Hover**: Hover over a bar to highlight it and see a tooltip with the trial number, state, start, end, and duration. Other bars are dimmed while one is highlighted.

## How to Read

- **X axis**: elapsed time since the study's earliest recorded trial start (re-based to zero). The unit switches automatically — seconds for short studies, minutes beyond 10 minutes, hours beyond 2 hours.
- **Y axis**: trial number.
- **Bar length**: the trial's execution duration. Running trials (no recorded completion time yet) are drawn up to the latest known timestamp in the study.
- **Color by trial state**: complete, pruned, running, and failed trials are colored differently (see the legend).
- **Many overlapping bars at the same time**: trials ran in parallel — the number of overlapping bars at any vertical slice approximates the number of concurrent workers.
- **Gaps between bars on the time axis**: workers were idle, e.g. waiting for a new trial to be scheduled or for a shared resource.
- **A few very long bars**: some trials took much longer than others — worth checking whether those hyperparameters are inherently expensive (e.g. large models) or the trial got stuck.
- **Optuna context**: this view is most informative for distributed / parallel optimization (multiple workers polling the same storage). It complements Optimization History (which is ordered by trial number, not by time) by showing the actual scheduling behavior.
