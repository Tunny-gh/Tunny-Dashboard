# Ridge Regression Surrogate

## Overview

Ridge regression (L2-regularized linear regression) fits a linear model to the trial data and uses the regression coefficients as a surrogate for the response surface. |β| measures each parameter's contribution.

## Formula

$$
\hat{y} = \beta_0 + \beta_1 x_1 + \beta_2 x_2 + \cdots + \beta_p x_p
$$

Objective (L2-regularized least squares):

$$
\min_\beta \|y - X\beta\|^2 + \alpha\|\beta\|^2 \quad (\alpha = 1.0)
$$

Closed-form solution:

$$
\beta = (X^\top X + \alpha I)^{-1} X^\top y
$$

## Preprocessing

**Z-score standardization** applied to each column of X before fitting:

$$
\tilde{x}_j = \frac{x_j - \mu_j}{\sigma_j}
$$

If σ_j ≈ 0, set σ_j = 1.0 (constant column guard). y is mean-centered only (y_c = y − ȳ).

The system is solved via Gaussian elimination with partial pivoting.

## R² Interpretation

| R²    | Meaning                                        |
| ----- | ---------------------------------------------- |
| ≈ 1.0 | Linear model fits well. Surface is reliable.   |
| < 0.5 | Nonlinear relationship — use Random Forest or Kriging. |

## Strengths and Limitations

**Strengths**
- Extremely fast: O(n·p²)
- Stable under multicollinearity due to L2 regularization
- Coefficient sign shows direction of influence (increasing parameter → objective goes up/down)

**Limitations**
- Assumes linearity — underestimates nonlinear effects
- No interaction terms (cannot capture parameter interaction)
- May miss important nonlinear structure visible in Random Forest / Kriging

## When to Use

```
Objective looks roughly linear?              → Ridge (fastest)
Need a quick first approximation?            → Ridge, then upgrade if R² < 0.5
Need nonlinear / interaction effects?        → Random Forest or Kriging
```
