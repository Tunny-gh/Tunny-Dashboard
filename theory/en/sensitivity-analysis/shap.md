# SHAP — SHapley Additive exPlanations

## Overview

SHAP computes parameter importance using **Shapley values** from cooperative game theory (Lundberg & Lee, 2017). Each parameter receives a contribution value that represents its average marginal contribution across all possible feature coalitions.

The ImportanceChart uses **TreeSHAP** for efficient exact Shapley value computation on Random Forest trees. Global importance is the mean $|\varphi_j(x)|$ across all samples and trees, normalized to sum to 1.

## Formula

Shapley value for parameter $j$ at sample $x$:

$$
\varphi_j(x) = \sum_{S \subseteq F \setminus \{j\}} \frac{|S|!\,(|F| - |S| - 1)!}{|F|!} \left[ f(S \cup \{j\}) - f(S) \right]
$$

Global SHAP importance:

$$
\operatorname{score}_j = \operatorname{mean} |\varphi_j(x)| \text{ over all samples and trees}
$$

Shapley values satisfy four axioms: efficiency, symmetry, linearity, dummy (a feature with no effect gets zero).

## Comparison with Other Methods

| Aspect | SHAP | MDI | RF-ANOVA |
| --- | --- | --- | --- |
| Theoretical basis | Shapley axioms | Impurity reduction | Permutation accuracy drop |
| High-cardinality bias | None | High | Low |
| Local interpretability | Yes | No | No |
| Cost | Medium–High | Low | Medium |

## Hyperparameters

| Parameter | Value |
| --- | --- |
| Trees | 64 |
| Max depth | 10 |
| Max rows | 1,000 (downsampled) |
| Seed | 42 |

## R² Interpretation

| R² | Meaning |
| --- | --- |
| ≥ 0.8 | Good fit. Scores are reliable. |
| 0.5–0.8 | Moderate. Use with caution. |
| < 0.5 | Poor fit. |

## Notes

- SHAP shows **global** importance (mean $|\varphi|$). Per-sample local $\varphi$ values are not displayed.
- When features are strongly correlated, path-dependent TreeSHAP can be unstable.
- Runs on a background thread; the UI remains responsive.

## When to Use

- When explainability and theoretical consistency are priorities.
- For reports requiring rigorous attribution.
- After RF-ANOVA / Permutation screening has identified top parameters.
