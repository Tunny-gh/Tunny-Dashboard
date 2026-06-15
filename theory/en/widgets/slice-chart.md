# Slice Chart

The Slice Chart is a scatter plot of one **parameter** (X axis) against one **objective**
(Y axis), using the **actual observed trials**. It is a quick way to see how an objective relates
to a single parameter across everything you have run. It uses no model — every dot is a real
trial.

## What it tells you

- The **shape of the relationship** between a parameter and an objective: a downward trend, a
  band that narrows toward good values, clusters, or no clear relationship at all.
- **Where good values concentrate** along a parameter's range, which hints at promising regions.
- The **spread** at a given parameter value: wide vertical scatter means other parameters also
  strongly influence the objective there.

## How to read it

- **Dots**: each observed trial. The X position is the chosen parameter, the Y position is the
  chosen objective.
- **Highlighted dots**:
  - Single-objective study: **Best** trials are highlighted against the rest (**Trials**).
  - Multi-objective study: **Pareto** trials are highlighted against **Non-Pareto** ones.
- A clear downward (or upward) trend suggests the parameter matters for the objective; a flat,
  evenly spread cloud suggests it matters little **on its own**.

## Controls

- **Parameter**: choose the parameter on the X axis.
- **Objective**: choose the objective on the Y axis.
- **Log Scale**: plot the Y axis (objective) on a logarithmic scale (positive values only).
- **Click a point**: inspect that trial's details (parameters, objective values, Pareto rank).

## How to read it carefully

- This is a **marginal view**: the other parameters are *not* held fixed — they vary freely from
  dot to dot. So a flat band does not prove the parameter is unimportant; its effect may be masked
  by interactions with other parameters. Cross-check with the Importance Chart.
- Because it plots only observed trials, sparsely sampled parameter ranges will simply have fewer
  dots — there is no interpolation or extrapolation.

## Difference from PDP

- **PDP Chart**: fits a surrogate model and shows one parameter's *averaged* effect with the other
  variables marginalized out by the model — including extrapolation into unsampled regions.
- **Slice Chart**: plots the raw observed trials with no model. The other parameters are not
  controlled, so the scatter reflects what actually happened, not an isolated model effect.
