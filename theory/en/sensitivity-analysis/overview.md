# Sensitivity Analysis Methods — Quick Guide

This guide summarizes the available sensitivity analysis methods and helps you choose the right one for your use case.

## Method Comparison

| Method | What it measures | Strengths | Limitations | Cost |
| --- | --- | --- | --- | --- |
| Spearman | Monotonic relationship strength (rank correlation) | Fast, robust, works with small datasets | Cannot detect non-monotonic (U-shaped) or interaction effects | Low |
| Ridge | Linear contribution strength (regression coefficients) | Fast, interpretable, handles multicollinearity | Cannot capture nonlinearity or strong interactions | Low |
| MDI | Feature importance from tree splits during training | Fast, intuitive | Tends to overestimate high-cardinality features | Low–Medium |
| RF-ANOVA | Accuracy drop when a feature is permuted (holdout) | Closer to real predictive contribution | Affected by correlated features | Medium |
| Permutation | Average of 5 independent RF-ANOVA runs | Lower variance than RF-ANOVA | ~5x slower than RF-ANOVA | Medium–High |
| SHAP | Shapley value decomposition of predictions | Theoretically consistent, local and global interpretability | Heavier computation | Medium–High |
| Sobol | Variance decomposition (first-order and total-effect indices) | Captures interactions, global sensitivity | Depends on surrogate quality and sample size | Medium–High |
| PDP (1D/2D) | Response shape of the objective function | Visualizes how the objective changes with parameters | May extrapolate when features are correlated | Low–High (model-dependent) |
| ARD importance | GP length-scale relevance per parameter | Free after fitting a GP; global smoothness-based sensitivity | GP models only; not a variance decomposition | Low (free with GP) |

## How to Choose

### Quick overall screening
- **First choice: Spearman** — Fast, few assumptions, good for screening.

### Linear contribution
- **First choice: Ridge** + Spearman for confirmation.

### Stable feature importance
- **First choice: Permutation** (lowest variance).
- **Fallback: RF-ANOVA** (faster), MDI (zero extra training cost).

### Explainability / accountability
- **First choice: SHAP** — Shapley-value based, theoretically rigorous.

### Global sensitivity including interactions
- **First choice: Sobol** — Use ST − S to quantify interaction strength.

### Response shape visualization
- **First choice: PDP** — 1D/2D response curves.

### Free sensitivity from a fitted GP surrogate
- **ARD importance** — see [ard-importance.md](./ard-importance.md). When you already fit a GP in the Surrogate Optimizer, read its per-parameter relevance at no extra cost.

## Recommended Workflow

1. Spearman + Ridge for initial screening.
2. RF-ANOVA for quick importance check; Permutation for stability.
3. Visualize top parameters with PDP.
4. If interactions matter, add Sobol.
5. For reports, use SHAP.

## Quick-Decision Checklist

- Small dataset (n < 50)? → Spearman first.
- Approximately linear? → Ridge.
- Strong nonlinearity suspected? → RF-ANOVA / SHAP / Sobol.
- Interactions suspected? → Check Sobol ST − S.
- Tight time budget? → Spearman + MDI.
- Methods give conflicting results? → Check correlated features, R², surrogate quality.

> Importance scores measure **association**, not causation. Always compare two or more methods before drawing conclusions.
