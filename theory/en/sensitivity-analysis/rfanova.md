# RF-ANOVA — Random Forest Permutation Importance

## Overview

RF-ANOVA measures how much prediction accuracy drops on a **holdout set** when a parameter's values are randomly shuffled. Shuffling breaks the parameter–objective relationship, so the resulting accuracy drop reflects how much the model relies on that parameter.

This approach avoids MDI's high-cardinality bias because it evaluates real prediction performance, not tree structure.

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
