# Permutation Feature Importance

## Overview

Permutation Feature Importance (PFI) measures the **accuracy drop when a parameter is shuffled on a holdout set**, averaging **5 independent shuffles** per feature to reduce estimation variance. Unlike RF-ANOVA (fANOVA), which decomposes the trained forest's variance over its leaf boxes without shuffling anything, PFI's importance signal comes entirely from the shuffle-induced accuracy drop.

> **Edge case:** when the dataset is very small (fewer than 4 samples after filtering), the holdout split is disabled and evaluation falls back to the training data. Importance scores are less reliable in this regime.

## Formula

For each parameter $j$, repeat $r = 1 \ldots 5$:

$$
\Delta_j^{(r)} = \max\!\left(\operatorname{MSE}_{\text{perm},j}^{(r)} - \operatorname{MSE}_{\text{baseline}},\; 0\right)
$$

Average across repeats:

$$
I_j = \frac{1}{5} \sum_{r=1}^{5} \Delta_j^{(r)}
$$

Normalized:

$$
I_{j,\text{norm}} = \frac{I_j}{\sum_{j'} I_{j'}}
$$

## Comparison with RF-ANOVA

| Aspect | Permutation (this method) | RF-ANOVA (fANOVA) |
| --- | --- | --- |
| Shuffling | Shuffles each feature 5 times on holdout data | Does not shuffle anything — decomposes the trained forest's variance over its leaf boxes instead |
| Trees | 100 (trained once) | 64 |
| Speed | Training once + 5× permutation evals per feature | Baseline (training + per-tree variance decomposition, no extra evals) |
| Holdout split seed | Same (43) | Same (43) |

Both methods use the same holdout split seed, but RF-ANOVA's importance itself is computed from the trained forest (not from the holdout split — the holdout split there is only used for its R²), so the two scores are not a shuffle-vs-shuffle comparison.

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

## References

- Breiman, L. (2001). Random Forests. _Machine Learning_, 45(1), 5–32.
- Fisher, A., Rudin, C., & Dominici, F. (2019). All Models are Wrong, but Many are Useful. _JMLR_, 20(177), 1–81.
