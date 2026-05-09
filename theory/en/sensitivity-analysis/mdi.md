# MDI — Mean Decrease Impurity

## Overview

MDI (Mean Decrease Impurity) computes parameter importance from the **impurity reduction at each split during Random Forest training**. Every time a parameter is used for a split, the weighted reduction in MSE is accumulated across all trees.

The ImportanceChart normalizes scores to sum to 1.

## Comparison with RF-ANOVA

| Aspect | MDI | RF-ANOVA |
| --- | --- | --- |
| Measured | During training | After training (holdout) |
| Bias | Overestimates high-cardinality features | Affected by correlated features |
| Interpretation | "How useful was it during learning?" | "How much accuracy is lost if removed?" |

## Formula

Impurity decrease at node t:

```
ΔI(t) = MSE(y_t) − [n_L/n_t · MSE(y_L) + n_R/n_t · MSE(y_R)]
```

MDI for parameter j in tree b:

```
MDI_b(j) = Σ_{splits on j} (n_t / n_root) · ΔI(t)
```

Final normalized score:

```
MDI(j) = (1/T) Σ_b MDI_b(j)
MDI_norm(j) = MDI(j) / Σ_j' MDI(j')
```

## Hyperparameters

| Parameter | Value |
| --- | --- |
| Trees | 100 |
| Max depth | 10 |
| Min leaf samples | 2 |
| Random seed | 42 |
| Max rows | 2,000 |

## R² Interpretation

R² is computed on holdout data:

| R² | Meaning |
| --- | --- |
| ≥ 0.8 (green) | Good fit. Scores are reliable. |
| 0.5–0.8 (yellow) | Moderate. Use as reference. |
| < 0.5 (red) | Poor fit. Scores are less reliable. |

## Known Bias

MDI tends to **overestimate high-cardinality features** (features with many distinct values have more split candidates). When this is a concern, use RF-ANOVA instead.

## When to Use

- When you want fast importance with zero additional cost after training.
- For a quick sanity check alongside RF-ANOVA or Permutation.
