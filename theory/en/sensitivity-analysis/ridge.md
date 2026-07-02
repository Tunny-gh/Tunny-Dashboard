# Ridge Regression Sensitivity

## Overview

Ridge regression fits a linear model with L2 regularization and uses the absolute coefficient values |β_j| as parameter importance scores.

- **Sign of β_j**: whether the parameter increases or decreases the objective.
- **|β_j|**: magnitude of linear influence (sensitivity).

Like Spearman, it is lightweight and useful for a quick global overview before running heavier methods.

## Formula

Given standardized input matrix $X$ and centered objective $y_c = y - \bar{y}$:

$$
\beta = (X^T X + \alpha I)^{-1} X^T y_c \quad (\alpha = 1.0)
$$

Coefficient of determination:

$$
R^2 = 1 - \frac{\sum (y_{c,i} - \hat{y}_{c,i})^2}{\sum y_{c,i}^2}
$$

Clipped at 0 for numerical stability.

Importance score for parameter j:

$$
\operatorname{score}_j = |\beta_j|
$$

## Preprocessing and holdout evaluation

Ridge is unified with the same convention as RF-ANOVA / MDI / SHAP / PFI (previously Ridge
reported an in-sample R² computed on the training data itself, which overstated fit quality
when its R² badge was shown next to the other methods' holdout R²).

```
Trial data (X ∈ R^{N×P}, y ∈ R^N)
    ↓
Step 1: Drop rows with any non-finite value in x or y (before fitting or scoring)
  └── Fewer than 2 valid rows → return zero importances, R² = 0.0
Step 2: 80/20 holdout split (Fisher-Yates shuffle, seeds 42/43)
  ├── ≥ 4 valid rows: first 80% train, remaining 20% eval
  └── < 4 valid rows: fall back to using all data for both train and eval
Step 3: Standardize/center using TRAIN statistics only
  └── EVAL data is transformed with the same TRAIN-derived mean/std before scoring
Step 4: Solve (X_train^T X_train + αI)β = X_train^T y_{c,train}
Step 5: R² is computed on EVAL only (data the fit never saw)
```

Because the closed-form solve is cheap (O(n·p²)), Ridge does not apply the row cap
(e.g. 2,000 rows) used by the tree-ensemble methods.

## Interpreting R²

| R² | Interpretation |
| --- | --- |
| ≥ 0.8 | Good linear fit. Scores are reliable. |
| 0.5–0.8 | Moderate. Use as reference only. |
| < 0.5 | Poor fit. Switch to RF-ANOVA / SHAP / Sobol. |

## Strengths and Limitations

**Strengths:**
- Very fast.
- Handles mild multicollinearity (L2 regularization).
- Easy to interpret.

**Limitations:**
- Cannot capture nonlinearity, thresholds, or strong interactions.
- Coefficient allocation can be unstable when features are highly correlated.

## When to Use

- Quick initial screening.
- Approximately linear objective function.
- When direction and magnitude of effect are needed alongside Spearman.

Typical workflow: Ridge + Spearman for screening → RF-ANOVA / SHAP / Sobol if nonlinearity is suspected.
