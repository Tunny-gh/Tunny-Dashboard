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

The system is solved via **Cholesky (LLT) decomposition** (faer). X'X + αI is always symmetric positive-definite for α > 0, making Cholesky the optimal solver; if the decomposition fails, the implementation falls back to a zero coefficient vector.

## Application to PDP

1D PDP uses the following closed-form approximation:

$$
\text{PDP}(v) = \bar{y} + \beta_j \cdot \frac{v - \mu_j}{\sigma_j}
$$

The contribution of the other parameters cancels out on average. The 2D PDP adds the terms for the two parameters:

$$
\bar{f}(v_1, v_2) = \bar{y} + \beta_{j_1} \cdot \frac{v_1 - \mu_{j_1}}{\sigma_{j_1}} + \beta_{j_2} \cdot \frac{v_2 - \mu_{j_2}}{\sigma_{j_2}}
$$

Because the Ridge surrogate is a linear model, it does not return confidence bands (`y_upper` / `y_lower`).

## R² Interpretation

| R²    | Meaning                                        |
| ----- | ---------------------------------------------- |
| ≈ 1.0 | Linear model fits well. Surface is reliable.   |
| < 0.5 | Nonlinear relationship — use Random Forest, LightGBM, or GP-FITC. |

## Strengths and Limitations

**Strengths**
- Extremely fast: O(n·p²)
- Stable under multicollinearity due to L2 regularization
- Coefficient sign shows direction of influence (increasing parameter → objective goes up/down)

**Limitations**
- Assumes linearity — underestimates nonlinear effects
- No interaction terms (cannot capture parameter interaction)
- May miss important nonlinear structure visible in Random Forest / Gaussian Process

## When to Use

```
Objective looks roughly linear?              → Ridge (fastest)
Need a quick first approximation?            → Ridge, then upgrade if R² < 0.5
Nonlinear / noisy / tabular?                 → LightGBM or Random Forest
Smooth nonlinear?                            → GP-FITC (or GP-VFE / GP-MOE)
```

## References

- Hoerl, A. E., & Kennard, R. W. (1970). Ridge Regression: Biased Estimation for Nonorthogonal Problems. *Technometrics*, 12(1), 55–67.
