# RF-ANOVA — Random Forest Permutation Importance

## Overview

RF-ANOVA measures how much prediction accuracy drops on a **holdout set** when a parameter's values are randomly shuffled. Shuffling breaks the parameter–objective relationship, so the resulting accuracy drop reflects how much the model relies on that parameter.

This approach avoids MDI's high-cardinality bias because it evaluates real prediction performance, not tree structure.

> **Naming note:** despite the name, "RF-ANOVA" is **not** the functional ANOVA (fANOVA) method of Hutter et al. (2014), which decomposes objective variance by marginalizing over tree leaf intervals. This implementation is instead a **permutation importance** (single shuffle, one pass) evaluated on a holdout split of a trained model. Because the underlying computation differs, **its values do not match Optuna's `fANOVA importance`**. Both aim to rank parameter importance, but they are distinct statistical methods.

## Formula

Importance of parameter $j$:

$$
I_j = \operatorname{MSE}_{\text{permuted},j} - \operatorname{MSE}_{\text{baseline}} \quad (\text{clipped to } 0 \text{ if negative})
$$

Normalized:

$$
I_{j,\text{norm}} = \frac{I_j}{\sum_{j'} I_{j'}}
$$

## Holdout Evaluation

Permutation is performed on holdout data (20% split), **not** training data. Using training data would give near-zero importance for all features because the model has memorized them.

> **Edge case:** when the dataset is very small (fewer than 4 samples after filtering), the holdout split is disabled and evaluation falls back to the training data. Importance scores are less reliable in this regime.

## Hyperparameters

| Parameter | Value |
| --- | --- |
| Trees | 100 |
| Max depth | 10 |
| Min leaf samples | 2 |
| Random seed | 42 |
| Max rows | 2,000 |

## R² Interpretation

| R² | Meaning |
| --- | --- |
| ≥ 0.8 (green) | Good fit. Scores are reliable. |
| 0.5–0.8 (yellow) | Moderate. Use as reference. |
| < 0.5 (red) | Poor fit. |

## Comparison with MDI

| Aspect | RF-ANOVA | MDI |
| --- | --- | --- |
| Measured | Holdout prediction drop | Training split quality |
| High-cardinality bias | Low | High |
| Speed | Moderate (1× permutation) | Similar |

## Limitation

When multiple features are correlated, shuffling one feature still allows the model to use others as substitutes, which can **underestimate** its importance.

## When to Use

- When a reliable, bias-reduced importance estimate is needed.
- When MDI shows suspicious results for high-cardinality features.
- When speed matters (faster than Permutation, comparable to MDI).
