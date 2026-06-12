# Sparse Gaussian Process (FITC Approximation)

Sparse Gaussian Process regression (also known as Sparse Kriging) reduces the exact GP's O(N³) cost to O(N × M²) using the **FITC (Fully Independent Training Conditional)** approximation. The implementation is backed by the **egobox-gp** crate (v0.36, Apache-2.0), a Rust port of the SMT surrogate-modelling toolkit.

## Overview

Both the **Gaussian Process** and **Sparse Gaussian Process** options use the same egobox FITC machinery. Sparse Gaussian Process uses fewer inducing points for lower cost:

- M = **20** for 1D PDP and the surrogate optimizer
- M = **50** for 2D PDP

Inducing points are always chosen by k-means (deterministic seed). The model trains on **all N points** — no subsampling. Hyperparameters are optimized directly by maximizing the FITC marginal likelihood (no hyperparameter borrowing from a separate standard-GP stage).

## Inducing Points

Instead of using all N training points, M ≪ N representative points Z = {z₁, …, z_M} are selected as mediators:

$$
u = f(Z) \sim \mathcal{GP}(0, K_{ZZ})
$$

FITC assumes conditional independence among training points given u:

$$
p(f(X) \mid u) \approx \prod_i p(f(x_i) \mid u)
$$

Inducing points are selected using k-means centroids (deterministic seed for reproducibility).

## Key Matrices

| Matrix  | Size  | Content                                   |
| ------- | ----- | ----------------------------------------- |
| K_ZZ    | M × M | Kernel matrix between inducing points     |
| K_XZ    | N × M | Kernel matrix between training and inducing points |

**Q matrix (low-rank approximation):**

$$
Q_{XX} \approx K_{XZ} \cdot K_{ZZ}^{-1} \cdot K_{XZ}^\top
$$

**FITC diagonal Λ:**

$$
\Lambda = \text{diag}(\sigma_f^2 - Q_{\text{diag}}) + \sigma_n^2 I
$$

Λ captures the residual variance not explained by the inducing points, plus the estimated homoscedastic observation noise σ_n². Noise variance is bounded below by 1e-6 (in normalized y units); on numerical failure the bound is retried at 1e-3.

## Woodbury Identity for Fast Computation

Using the Woodbury identity, the expensive N×N inverse reduces to M×M operations:

$$
(Q + \Lambda)^{-1} = \Lambda^{-1} - \Lambda^{-1} K_{XZ} \Sigma^{-1} K_{XZ}^\top \Lambda^{-1}
$$

$$
\Sigma = K_{ZZ} + K_{XZ}^\top \Lambda^{-1} K_{XZ}
$$

Main cost: O(N × M²).

## Hyperparameter Optimization

θ = [log l₁, …, log l_D, log σ_f, log σ_n] are optimized by egobox using the gradient-free **COBYLA** optimizer with **10 multistart points** (deterministic: fixed grid and fixed seed). Hyperparameters are optimized directly against the FITC marginal likelihood — there is no separate "borrow from standard GP" stage.

## Prediction

Predictions (mean and variance) are computed in batch. The 95% CI band is mean ± 1.96·σ.

Post-training weight vector w (M-dimensional):

$$
w = K_{ZZ}^{-1} \cdot K_{XZ}^\top \cdot (Q + \Lambda)^{-1} \cdot y
$$

$$
\mu(x^*) = K_{x^*,Z} \cdot w = \sum_j k(x^*, z_j) \cdot w_j
$$

O(M) per grid point — 50×50 = 2,500 grid points → 50,000 (M=20) or 125,000 (M=50) kernel evaluations.

## Complexity Comparison

| Method                               | Training cost | N=5000, M=50  |
| ------------------------------------ | ------------- | ------------- |
| Standard GP                          | O(N³)         | 1.25×10¹¹ (infeasible) |
| Gaussian Process (M=100 k-means)     | O(N·M²)       | 2.5×10⁹       |
| **Sparse Gaussian Process (M=50)**   | **O(N×M²)**   | **1.25×10⁹**  |

## When to Use

```
Best quality, any N?             → Gaussian Process (M = min(N, 100))
Faster / large N?                → Sparse Gaussian Process (M = 20 or 50)
Nonlinear / noisy / very large?  → Random Forest
Linear response?                 → Ridge
```
