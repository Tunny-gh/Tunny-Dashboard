# Gaussian Process Regression

Gaussian Process regression (also known as Kriging) uses a GP with an ARD Matérn 5/2 kernel to produce smooth, high-quality response surfaces. The implementation is backed by the **egobox-gp** crate (v0.36, Apache-2.0), a Rust port of the SMT surrogate-modelling toolkit.

## Overview

Both the **Gaussian Process** and **Sparse Gaussian Process** model options use egobox's Sparse GP (FITC approximation) internally. The distinction is in how many inducing points M are used:

- **Gaussian Process**: M = min(N, 100). When N ≤ 100 the inducing points equal the training points (Z = X), making FITC mathematically equivalent to an exact GP with noise estimation. When N > 100, M = 100 inducing points are selected by k-means (deterministic seed). The model trains on **all N points** — no subsampling.
- **Sparse Gaussian Process**: M = 20 (1D PDP / surrogate optimizer) or M = 50 (2D PDP). See [sparse-gaussian-process.md](sparse-gaussian-process.md).

## Kernel: ARD Matérn 5/2

$$
k(x_1, x_2) = \sigma_f^2 \left(1 + \sqrt{5}\,r + \frac{5r^2}{3}\right) \exp(-\sqrt{5}\,r)
$$

$$
r^2 = \sum_{d} \left(\frac{x_{1,d} - x_{2,d}}{l_d}\right)^2
$$

| Parameter | Meaning                                        |
| --------- | ---------------------------------------------- |
| σ_f       | Signal standard deviation (amplitude scale)    |
| l_d       | Length scale for dimension d                   |
| σ_n       | Observation noise standard deviation           |

**ARD (Automatic Relevance Determination)**: per-dimension length scales let the model automatically down-weight irrelevant parameters (large l_d) and up-weight important ones (small l_d).

**Why Matérn 5/2 over RBF?** Engineering objectives are typically C² smooth, not C∞. RBF (Gaussian) overestimates smoothness and underestimates uncertainty far from data.

## Prediction

Given training data (X, y), posterior mean at x*:

$$
\mu(x^*) = k(x^*, X) \cdot K^{-1} \cdot y = k(x^*, X) \cdot \alpha
$$

Predictions (mean and variance) are computed in batch. The 95% CI band is mean ± 1.96·σ. The noise variance σ_n² is estimated (homoscedastic) and bounded below by 1e-6 in normalized y units to keep covariance matrices positive definite; on numerical failure the bound is retried at 1e-3.

## Hyperparameter Optimization

egobox maximizes the log marginal likelihood (LML) using the gradient-free **COBYLA** optimizer with **10 multistart points** (deterministic: fixed grid and fixed seed):

$$
\text{LML}(\theta) = -\frac{1}{2} y^\top \alpha - \sum_i \log L_{ii} - \frac{N}{2} \log(2\pi)
$$

θ = [log l₁, …, log l_D, log σ_f, log σ_n]. Optimized in log space so all parameters remain positive without explicit constraints.

## Data Normalization

- **X**: each dimension scaled to [0, 1] using min/max
- **y**: Z-score normalized (mean 0, std 1)
- Grid predictions are inverse-transformed to the original scale

## Complexity

| Step                    | Cost     | N = 100, M = 100 estimate |
| ----------------------- | -------- | ------------------------- |
| FITC kernel matrices    | O(N·M²)  | 1×10⁶                     |
| Cholesky (M×M)          | O(M³)    | 1×10⁶                     |
| Grid prediction (50×50) | O(2500·M)| 2.5×10⁵                   |

Target: under 10,000 ms (release build).

## R² Interpretation

| R²    | Action                                                        |
| ----- | ------------------------------------------------------------- |
| ≥ 0.8 | Good fit. Surface is reliable.                                |
| < 0.5 | Poor fit — try Random Forest or increase data if N is small.  |

## Strengths and Limitations

**Strengths**
- Highest-quality smooth surface (C² continuity)
- Works well with small N (as few as 20 points)
- ARD automatically identifies important dimensions
- Trains on all N points (no subsampling); backed by egobox-gp

**Limitations**
- For N > 100 the M = 100 inducing-point cap bounds cost but introduces slight approximation error
- May overfit noisy data
- COBYLA multistart can converge to local optima

## When to Use

```
Smooth nonlinear, any N?             → Gaussian Process (best quality)
Smooth nonlinear, large N / faster?  → Sparse Gaussian Process
Nonlinear / noisy / large N?         → Random Forest
Linear response?                     → Ridge
```
