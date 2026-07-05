# Convergence Indicators

## Overview

The Convergence Indicators widget tracks how well the current Pareto front covers the objective space as trials accumulate. A dropdown lets you switch among four multi-objective convergence indicators:

| Indicator | Direction | Description |
|-----------|-----------|-------------|
| **[Hypervolume (HV)](../optimization/hypervolume.md)** | Higher is better | Volume of objective space dominated by the Pareto front and bounded by a reference point. |
| **[IGD+](../optimization/igd-plus.md)** | Lower is better | Modified Inverted Generational Distance — average distance from a reference set to the nearest Pareto-front point. |
| **[ε-indicator](../optimization/epsilon-indicator.md)** | Lower is better | Smallest ε such that every reference-set point is ε-dominated by some Pareto-front point. |
| **[R2](../optimization/r2-indicator.md)** | Lower is better | Utility-based indicator measuring expected gap from an ideal reference set. |

For each indicator's formula, algorithm, and how the reference set is constructed, follow the links in the table above.

> **Note:** These indicators are defined only for multi-objective studies with ≥ 2 objectives.

## Comparison Studies

When comparison studies are added, how series are made comparable depends on the indicator:

- **IGD+ / ε-indicator / R2**: all series are evaluated against a **shared reference set** (computed from the union of all studies) and normalized to [0, 1] so they are directly comparable on the same chart.
- **Hypervolume**: all series instead share a single **reference point** (the union's nadir plus a 10 % margin, or the manual override below), and the resulting values are **not** normalized to [0, 1] — they remain in raw hypervolume units.

## Operations

- **Indicator selector**: Use the dropdown at the top to switch indicators. The chart recomputes automatically.
- **Zoom**: Scroll the mouse wheel, or drag with the left button to zoom into the selected rectangle.
- **Pan**: Drag with the right mouse button to pan across the plot.
- **Reset view**: Double-click with the left button to restore the default view.
- **Reference point** (Hypervolume only): Override the auto-computed nadir + 10 % margin reference point per objective.

## How to Read

- **Monotonically improving curve**: each new trial either extends the Pareto front (improving the indicator) or is dominated (indicator unchanged).
- **Steep early rise / fall, then flattening**: the optimizer found good solutions quickly but is now struggling to improve — typical near convergence.
- **Sudden jump / drop**: a breakthrough solution significantly extended the Pareto front.
- **Plateau**: no improvement in Pareto front quality. The optimizer may have converged or need more trials.
