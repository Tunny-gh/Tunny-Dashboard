# Ridge Regression Surrogate

## Overview

Ridge regression (L2-regularized linear regression) fits a linear model to the trial data and uses the regression coefficients as a surrogate for the response surface. |β| measures each parameter's contribution.

## Formula

```
ŷ = β₀ + β₁x₁ + β₂x₂ + ... + βₚxₚ
```

Objective (L2-regularized least squares):

```
min_β ||y − Xβ||² + α||β||²     (α = 1.0)
```

Closed-form solution:

```
β = (XᵀX + αI)⁻¹ Xᵀy
```

## Preprocessing

**Z-score standardization** applied to each column of X before fitting:

```
x̃_j = (x_j − μ_j) / σ_j
```

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
