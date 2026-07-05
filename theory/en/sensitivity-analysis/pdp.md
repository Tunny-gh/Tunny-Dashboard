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

| Model     | Speed       | Quality                          | Best for                       |
| --------- | ----------- | -------------------------------- | ------------------------------ |
| Ridge     | < 100 ms    | Linear only                      | Any size                       |
| Random Forest | < 2,000 ms | Nonlinear                    | Any size                       |
| GP-FITC   | < 10,000 ms | Smooth, highest quality          | Any N — default GP             |
| GP-VFE    | < 10,000 ms | Smooth, conservative fit         | Any N — overfit GP-FITC        |
| GP-MOE    | < 30,000 ms | Smooth, multi-regime             | Discontinuous / regime-switch  |

All GP variants use M = min(N, 100) inducing points backed by egobox-gp / egobox-moe (Apache-2.0). When N ≤ 100 this is equivalent to an exact GP with noise estimation.

## Interpreting the Plot

- **1D PDP**: shows how the objective changes as one parameter varies (all others held at their mean).
- **2D PDP**: shows the joint response surface for two parameters as a 3D surface plot. With
  **Show data** on, hovering an overlaid observed point shows a tooltip with its parameter and
  objective values, and clicking it opens the trial-detail modal (same interaction as the other
  3D charts).
- **Flat line / surface**: the parameter has little effect.
- **Steep slope**: the parameter strongly influences the objective.
- **Curved/non-monotonic shape**: nonlinear relationship — consider GP-FITC or Random Forest for accuracy.

## R² and Model Selection

Each surrogate reports R² (fit to training data):

| R² | Action |
| --- | --- |
| ≈ 1.0 | Surrogate is accurate. PDP is reliable. |
| < 0.5 | Switch to a more expressive model (GP-FITC or GP-MOE for smooth; Random Forest / LightGBM for noisy). |

## Limitations

- When features are correlated, the PDP may show extrapolated (unrealistic) regions.
- Only numerical parameters are supported.
- Ridge PDP is linear; use Random Forest or GP-FITC for nonlinear responses.
- If GP-MOE training fails, PDP falls back to GP-FITC automatically.

## When to Use

- After identifying important parameters with Importance Chart / Sensitivity Heatmap.
- To understand **how** a parameter affects the objective (not just how much).
- To find the optimal region or interaction pattern between two parameters.

## References

- Friedman, J. H. (2001). Greedy Function Approximation: A Gradient Boosting Machine. _Annals of Statistics_, 29(5), 1189–1232.
