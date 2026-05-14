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

Impurity decrease at node $t$:

$$
\Delta I(t) = \operatorname{MSE}(y_t) - \left[ \frac{n_L}{n_t} \cdot \operatorname{MSE}(y_L) + \frac{n_R}{n_t} \cdot \operatorname{MSE}(y_R) \right]
$$

MDI for parameter $j$ in tree $b$:

$$
\operatorname{MDI}_b(j) = \sum_{\text{splits on } j} \frac{n_t}{n_{\text{root}}} \cdot \Delta I(t)
$$

Final normalized score:

$$
\operatorname{MDI}(j) = \frac{1}{T} \sum_{b} \operatorname{MDI}_b(j)
$$

$$
\operatorname{MDI}_{\text{norm}}(j) = \frac{\operatorname{MDI}(j)}{\sum_{j'} \operatorname{MDI}(j')}
$$

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
