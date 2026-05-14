# Partial Dependence Plot (PDP)

## Overview

A Partial Dependence Plot visualizes the **marginal effect** of one or two parameters on the objective function, averaging out the influence of all other parameters.

Tunny Dashboard shows a 1D line chart (PDP Chart) and a 2D surface (PDP Chart 2D) using surrogate models fitted to the trial data.

## Theory

For a set of target parameters $S$ and complement $C = X \setminus S$:

$$
\bar{f}_S(x_S) = E_{x_C}[f(x_S, x_C)] \approx \frac{1}{N} \sum_{i=1}^{N} f(x_S, x_{C,i})
$$

By marginalizing (averaging) $x_C$, we isolate the pure effect of $x_S$.

## Surrogate Models for 2D PDP

| Model | Speed | Quality | Best for |
| --- | --- | --- | --- |
| Ridge | < 100ms | Linear only | Any size |
| Random Forest | < 2,000ms | Nonlinear | Any size |
| Kriging | < 10,000ms | Smooth, highest quality | N ≤ 500 |
| Sparse Kriging | < 5,000ms | Near-Kriging via FITC | N ≤ 5,000 |

## Interpreting the Plot

- **1D PDP**: shows how the objective changes as one parameter varies (all others held at their mean).
- **2D PDP**: shows the joint response surface for two parameters as a 3D surface plot.
- **Flat line / surface**: the parameter has little effect.
- **Steep slope**: the parameter strongly influences the objective.
- **Curved/non-monotonic shape**: nonlinear relationship — consider Kriging or Random Forest for accuracy.

## R² and Model Selection

Each surrogate reports R² (fit to training data):

| R² | Action |
| --- | --- |
| ≈ 1.0 | Surrogate is accurate. PDP is reliable. |
| < 0.5 | Switch to a more expressive model (Kriging / Sparse Kriging). |

## Limitations

- When features are correlated, the PDP may show extrapolated (unrealistic) regions.
- Only numerical parameters are supported.
- Ridge PDP is linear; use Random Forest or Kriging for nonlinear responses.

## When to Use

- After identifying important parameters with Importance Chart / Sensitivity Heatmap.
- To understand **how** a parameter affects the objective (not just how much).
- To find the optimal region or interaction pattern between two parameters.
