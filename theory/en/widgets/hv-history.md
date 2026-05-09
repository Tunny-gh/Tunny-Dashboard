# Hypervolume History

## Overview

The Hypervolume History chart tracks the **hypervolume indicator (HV)** — a scalar measure of how well the current Pareto front covers the objective space — as trials accumulate. A higher hypervolume means a better-spread and higher-quality Pareto front.

Hypervolume is the volume of the objective space dominated by the current Pareto front and bounded by a reference point.

## Operations

- **Zoom**: Scroll the mouse wheel to zoom into a range of trials.
- **Pan**: Click and drag to pan along the trial axis.
- **Hover**: Hover over a data point to see the trial number and current hypervolume value.
- **Reference point**: the reference point used for HV calculation is set from the study configuration.

## How to Read

- **Monotonically increasing curve**: each new trial either adds a non-dominated solution (increasing HV) or is dominated (HV unchanged). A correctly implemented multi-objective optimizer always shows non-decreasing HV.
- **Steep early rise, then flattening**: the optimizer found good solutions quickly but is now struggling to improve — typical near convergence.
- **Sudden jump**: a new trial significantly extended the Pareto front — a breakthrough solution was found.
- **Plateau (flat line)**: no improvement in Pareto front quality. The optimizer may have converged or needs more trials / a different sampler.
- **Compare across studies**: use HV history to compare the convergence speed of different samplers or parameter settings on the same problem.
