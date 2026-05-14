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

Importance score for parameter j:

$$
\operatorname{score}_j = |\beta_j|
$$

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
