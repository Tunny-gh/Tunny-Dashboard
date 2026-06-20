# Sensitivity Analysis Methods — Quick Reference Guide

A quick overview and selection guide for the methods in this folder.
For detailed theory and implementation, refer to the individual method documents.

## Method Summary

| Method      | What it measures                                     | Strengths                                                  | Limitations                                                          | Cost                     | Details                                  |
| ----------- | ---------------------------------------------------- | ---------------------------------------------------------- | -------------------------------------------------------------------- | ------------------------ | ---------------------------------------- |
| Spearman    | Monotonic relationship strength (rank correlation)   | Fast, robust, works well with small datasets               | Cannot detect non-monotonic (U-shaped) or interaction effects        | Low                      | [spearman.md](./spearman.md)             |
| Ridge       | Linear contribution strength (regression coefficients) | Fast, interpretable, handles multicollinearity relatively well | Cannot represent strong nonlinearity or interactions             | Low                      | [ridge.md](./ridge.md)                   |
| MDI         | Feature importance from splits during training       | Fast and intuitive                                         | Tends to overestimate high-cardinality features                      | Low–Medium               | [mdi.md](./mdi.md)                       |
| RF-ANOVA    | Accuracy drop when a feature is permuted (holdout evaluation) | Real-world-close contribution (holdout evaluation)  | Contribution may become unstable with correlated features            | Medium                   | [rfanova.md](./rfanova.md)               |
| Permutation | Average of 5 independent RF-ANOVA runs              | Lower variance than RF-ANOVA, more stable                  | ~5x slower than RF-ANOVA                                            | Medium–High              | [permutation.md](./permutation.md)       |
| SHAP        | Shapley value decomposition of predictions           | Theoretically consistent; strong local and global interpretability | Heavier computation than other methods                      | Medium–High              | [shap.md](./shap.md)                     |
| Sobol       | Variance decomposition (first-order and total-effect indices) | Can evaluate global sensitivity including interactions | Affected by surrogate quality and sample size                  | Medium–High              | [sobol.md](./sobol.md)                   |
| PDP (1D/2D) | Response shape of the objective function (surface/cross-section) | Easy to visualize "how it changes"               | May appear extrapolated when correlations are strong                 | Low–High (model-dependent) | [pdp.md](./pdp.md)                     |
| ARD importance | GP length-scale relevance per parameter          | Global, smoothness-based sensitivity from GP length scales  | Involves GP-FITC training; not a variance decomposition              | Medium–High (involves GP training) | [ard-importance.md](./ard-importance.md) |

## How to Choose

### Start with a quick overall picture

- First choice: Spearman
- Reason: Fast, few assumptions, suitable for screening

### Quickly inspect linear contributions

- First choice: Ridge
- Auxiliary: Spearman
- Reason: Ridge makes it easy to see direction and magnitude from coefficients; Spearman is good for robust confirmation of monotonic relationships

### Stable feature importance from the model

- First choice: Permutation (lower variance in importance estimates)
- Auxiliary: RF-ANOVA (quick check), MDI (zero training-cost quick check)
- Reason: Permutation is the average of 5 RF-ANOVA runs, producing stable results with small samples or high-variance data. Choose RF-ANOVA when speed is the priority

### Explainability / accountability (theoretically justify contributions)

- First choice: SHAP
- Reason: Shapley-value based, high explainability

### Global sensitivity including interactions

- First choice: Sobol (especially look at ST − S)
- Reason: Can quantify interaction contributions

### Visualize how parameters affect the response shape

- First choice: PDP
- Reason: Can visualize 1D/2D response shapes; easy to develop optimization intuition

### Get sensitivity for free from a fitted GP surrogate

- ARD importance: see [ard-importance.md](./ard-importance.md)
- Reason: If a GP was already fitted for surrogate optimization, per-parameter relevance can be read out with no additional computation

## Recommended Workflow

1. Spearman + Ridge for initial screening (narrow down candidates)
2. RF-ANOVA for quick importance check; prioritize Permutation when stability matters (optionally include MDI)
3. Visualize top parameters with PDP
4. Add Sobol if interactions are important
5. Use SHAP for reports and documentation

## Quick-Decision Checklist

- Small dataset: prioritize Spearman; treat heavy methods as supplementary
- Linear model is sufficient: prioritize Ridge; if R² is low, switch to nonlinear methods
- Strong nonlinearity suspected: prioritize RF-ANOVA / SHAP / Sobol
- Interactions suspected: check Sobol ST − S
- Tight time budget: run Spearman + MDI first
- Methods give conflicting results: re-examine correlated features, R², and surrogate quality

## Notes

- Importance scores are fundamentally indicators of association/predictive contribution on observed data, not causal effects.
- Rather than drawing conclusions from a single method, it is recommended to confirm consistency across two or more methods.
