# Permutation Feature Importance

## Overview

Permutation Feature Importance (PFI) follows the same principle as RF-ANOVA — measuring the accuracy drop when a parameter is shuffled on a holdout set — but **averages 5 independent shuffles** per feature to reduce estimation variance.

## Formula

For each parameter j, repeat r = 1…5:

```
Δ_j^(r) = max(MSE_perm_j^(r) − MSE_baseline, 0)
```

Average across repeats:

```
I_j = (1/5) · Σ_r Δ_j^(r)
```

Normalized:

```
I_j_norm = I_j / Σ_j' I_j'
```

## Comparison with RF-ANOVA

| Aspect | Permutation (this method) | RF-ANOVA |
| --- | --- | --- |
| Repeats per feature | 5 | 1 |
| Variance | ~1/5 of RF-ANOVA | Higher |
| Speed | ~5× slower | Baseline |
| Holdout split seed | Same (43) | Same (43) |

Because both methods use the same holdout split, running them on the same dataset gives directly comparable results.

## Hyperparameters

| Parameter | Value |
| --- | --- |
| Trees | 100 |
| Max depth | 10 |
| Min leaf samples | 2 |
| Train seed | 42 |
| Permutation repeats | 5 |
| Max rows | 2,000 |

## R² Interpretation

| R² | Meaning |
| --- | --- |
| ≥ 0.8 | Good model fit. Scores are reliable. |
| 0.5–0.8 | Moderate. Reference only. |
| < 0.5 | Poor fit (negative R² possible). |

Note: Unlike RF-ANOVA, negative R² is **not** clipped to 0 here. Negative R² means the model is worse than a constant predictor.

## Limitation

Like RF-ANOVA, correlated features can be underestimated because the model substitutes one correlated feature for another after shuffling.

## When to Use

- When you want the most stable importance estimates.
- When RF-ANOVA results show high variance across runs.
- When datasets are small or noisy.
