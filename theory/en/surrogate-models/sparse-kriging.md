# Sparse Kriging (FITC Approximation)

## Overview

Sparse Kriging reduces standard GP's O(N³) cost to O(N × M²) using the **FITC (Fully Independent Training Conditional)** approximation with M = 50 inducing points. Unlike standard Kriging which subsamples to 500 points, Sparse Kriging uses all N points — giving better accuracy at larger scales.

## Inducing Points

Instead of using all N training points, M ≪ N representative points Z = {z₁, …, z_M} are selected as mediators:

$$
u = f(Z) \sim \mathcal{GP}(0, K_{ZZ})
$$

FITC assumes conditional independence among training points given u:

$$
p(f(X) \mid u) \approx \prod_i p(f(x_i) \mid u)
$$

Inducing points are selected using k-means centroids (seed 42 for reproducibility).

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

Λ captures the residual variance not explained by the inducing points, plus observation noise.

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

θ = [log l₁, log l₂, log σ_f, log σ_n] optimized via L-BFGS with numerical gradients (central finite differences, ε = 1e-5), clamped to [-6, 6].

Iteration count adapts to N:

| N       | max_iter |
| ------- | -------- |
| ≥ 2000  | 3        |
| ≥ 500   | 10       |
| < 500   | 20       |

## Prediction

Post-training weight vector w (M-dimensional):

$$
w = K_{ZZ}^{-1} \cdot K_{XZ}^\top \cdot (Q + \Lambda)^{-1} \cdot y
$$

$$
\mu(x^*) = K_{x^*,Z} \cdot w = \sum_j k(x^*, z_j) \cdot w_j
$$

O(M) per grid point — 50×50 = 2,500 grid points → 125,000 kernel evaluations.

## Fallback

| Situation                         | Behavior                     |
| --------------------------------- | ---------------------------- |
| N < M (too few points)            | Falls back to standard Kriging |
| Cholesky fails (numerical issue)  | Falls back to standard Kriging |
| Weight vector contains NaN/Inf    | Falls back to standard Kriging |

## Complexity Comparison

| Method                          | Training cost | N=5000, M=50  |
| ------------------------------- | ------------- | ------------- |
| Standard GP                     | O(N³)         | 1.25×10¹¹ (infeasible) |
| Kriging (subsample 500 pts)     | O(500³)       | 1.25×10⁸      |
| **Sparse Kriging**              | **O(N×M²)**   | **1.25×10⁷**  |

## When to Use

```
Smooth nonlinear, N ≤ 500?   → Kriging
Smooth nonlinear, N ≤ 5,000? → Sparse Kriging (faster and uses all data)
N > 5,000 or noisy?          → Random Forest
```
