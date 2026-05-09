# Kriging (Gaussian Process Regression)

## Overview

Kriging uses a Gaussian Process (GP) with an ARD Matérn 5/2 kernel to produce smooth, high-quality response surfaces. Hyperparameters are optimized by maximizing the log marginal likelihood via L-BFGS.

For N > 500, the model automatically subsamples to 500 points. For larger datasets, prefer Sparse Kriging.

## Kernel: ARD Matérn 5/2

```
k(x₁, x₂) = σ_f² · (1 + √5·r + 5r²/3) · exp(−√5·r)

r² = Σ_d ((x₁_d − x₂_d) / l_d)²
```

| Parameter | Meaning                                        |
| --------- | ---------------------------------------------- |
| σ_f       | Signal standard deviation (amplitude scale)    |
| l_d       | Length scale for dimension d                   |
| σ_n       | Observation noise standard deviation           |

**ARD (Automatic Relevance Determination)**: per-dimension length scales let the model automatically down-weight irrelevant parameters (large l_d) and up-weight important ones (small l_d).

**Why Matérn 5/2 over RBF?** Engineering objectives are typically C² smooth, not C∞. RBF (Gaussian) overestimates smoothness and underestimates uncertainty far from data.

## Prediction

Given training data (X, y), posterior mean at x*:

```
μ(x*) = k(x*, X) · K⁻¹ · y = k(x*, X) · α
```

K is the N×N kernel matrix (plus σ_n²·I on diagonal). Solved via Cholesky factorization with jitter = 1e-6 for numerical stability.

## Hyperparameter Optimization

Maximizes the log marginal likelihood (LML):

```
LML(θ) = −½ yᵀα − Σ_i log L_ii − N/2·log(2π)
```

θ = [log l₁, …, log l_D, log σ_f, log σ_n]. Analytic gradients are used with L-BFGS (50 iterations max, convergence at ||∇LML||₂ < 1e-5).

## Data Normalization

- **X**: each dimension scaled to [0, 1] using min/max
- **y**: Z-score normalized (mean 0, std 1)
- Grid predictions are inverse-transformed to the original scale

## Complexity

| Step                    | Cost    | N = 500 estimate |
| ----------------------- | ------- | ---------------- |
| Kernel matrix           | O(N²)   | 2.5×10⁵          |
| Cholesky factorization  | O(N³)   | 4.2×10⁷          |
| Grid prediction (50×50) | O(2500N)| 1.25×10⁶         |

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

**Limitations**
- O(N³) — requires subsampling for N > 500
- May overfit noisy data
- L-BFGS can converge to local optima

## When to Use

```
Smooth nonlinear, N ≤ 500?   → Kriging (best quality)
Smooth nonlinear, N ≤ 5000?  → Sparse Kriging (fast + quality)
Nonlinear / noisy / large N? → Random Forest
Linear response?              → Ridge
```
