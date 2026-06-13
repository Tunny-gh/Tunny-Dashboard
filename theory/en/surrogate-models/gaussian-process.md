# Gaussian Process Regression (GP-FITC / GP-VFE)

Gaussian Process regression (also known as Kriging) uses a GP with an ARD Matérn 5/2 kernel to produce smooth, high-quality response surfaces. The implementation is backed by the **egobox-gp** crate (v0.36, Apache-2.0), a Rust port of the SMT surrogate-modelling toolkit.

Two sparse-GP variants are available: **GP-FITC** and **GP-VFE**. Both share all architecture, hyperparameter search, and complexity; they differ only in the marginal-likelihood bound used during training (see [FITC vs VFE](#fitc-vs-vfe)).

## Overview

Both GP-FITC and GP-VFE use egobox's sparse GP (FITC or VFE approximation) internally.

| Option   | Approximation | M (inducing points)          |
| -------- | ------------- | ---------------------------- |
| GP-FITC  | FITC          | min(N, 100) — see below      |
| GP-VFE   | VFE           | min(N, 100) — see below      |

**Inducing points:** when N ≤ 100, Z = X (training points themselves); FITC/VFE then becomes mathematically equivalent to an exact GP with homoscedastic noise estimation. When N > 100, M = 100 inducing points are selected by k-means centroids (deterministic seed). The model trains on **all N points** — no subsampling. In the multi-objective surrogate optimizer the inducing points can be biased toward the Pareto front, concentrating them on the non-dominated trials so the surrogate is most accurate where it improves the front.

Measured training times at N = 10,000: GP-FITC ≈ 2.4 s, GP-VFE ≈ 2.0 s (release build).

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

## Sparse GP: Inducing Points

Instead of using all N training points directly in the kernel solve, M representative points Z = {z₁, …, z_M} are introduced as mediators:

$$
u = f(Z) \sim \mathcal{GP}(0, K_{ZZ})
$$

FITC assumes conditional independence among training points given u:

$$
p(f(X) \mid u) \approx \prod_i p(f(x_i) \mid u)
$$

The key matrices are:

| Matrix  | Size  | Content                                            |
| ------- | ----- | -------------------------------------------------- |
| K_ZZ    | M × M | Kernel matrix between inducing points              |
| K_XZ    | N × M | Kernel matrix between training and inducing points |

**Q matrix (low-rank approximation):**

$$
Q_{XX} \approx K_{XZ} \cdot K_{ZZ}^{-1} \cdot K_{XZ}^\top
$$

**FITC diagonal Λ:**

$$
\Lambda = \operatorname{diag}(\sigma_f^2 - Q_{\mathrm{diag}}) + \sigma_n^2 I
$$

Using the Woodbury identity the expensive N×N inverse reduces to M×M operations:

$$
(Q + \Lambda)^{-1} = \Lambda^{-1} - \Lambda^{-1} K_{XZ} \Sigma^{-1} K_{XZ}^\top \Lambda^{-1}
$$

$$
\Sigma = K_{ZZ} + K_{XZ}^\top \Lambda^{-1} K_{XZ}
$$

Main cost: **O(N · M²)**.

## FITC vs VFE

Both FITC and VFE lead to the same M×M computational form; the difference is in the objective maximised during hyperparameter training:

| Criterion | Objective                              | Noise behaviour                               |
| --------- | -------------------------------------- | --------------------------------------------- |
| **FITC**  | FITC marginal likelihood approximation | Learns noise aggressively; fits tight to data |
| **VFE**   | Variational Free Energy (ELBO)         | True lower bound on exact GP marginal likelihood; slightly more conservative noise estimate → smoother fits |

The VFE objective is:

$$
\mathcal{L}_\text{VFE} = \log \mathcal{N}(y; 0, Q_{XX} + \sigma_n^2 I) - \frac{1}{2\sigma_n^2} \operatorname{tr}(K_{XX} - Q_{XX})
$$

The trace term penalises the information lost by the inducing-point approximation and is absent in FITC.

**Practical benchmark** (noise-free function, N = 100, M = 100):  
GP-FITC R² ≈ 0.88, GP-VFE R² ≈ 0.76 — VFE trades a small amount of fit tightness for a more principled bound.  
On noisy data the two converge almost exactly.

**When to prefer which:**

- **GP-FITC** — default; best fit on clean or lightly-noisy data; recommended starting point.
- **GP-VFE** — if the GP-FITC surface looks overfit or unrealistically spiky; VFE's conservative noise estimate produces a smoother surface.

## Noise Variance

The homoscedastic noise variance σ_n² is estimated jointly with the kernel hyperparameters. Floor: 1e-6 in normalized y units; on numerical failure the bound is retried at 1e-3. This ensures positive-definiteness of the covariance matrix.

## Prediction

Given training data (X, y), posterior mean at x*:

$$
\mu(x^*) = k(x^*, Z) \cdot w, \qquad w = K_{ZZ}^{-1} K_{XZ}^\top (Q + \Lambda)^{-1} y
$$

Predictions (mean and variance) are computed in batch. The 95% CI band is mean ± 1.96·σ.

## Hyperparameter Optimization

egobox maximizes the chosen bound (FITC likelihood or VFE ELBO) using the gradient-free **COBYLA** optimizer with **10 multistart points** (deterministic: fixed grid and fixed seed):

θ = [log l₁, …, log l_D, log σ_f, log σ_n]. Optimized in log space so all parameters remain positive without explicit constraints.

## Data Normalization

- **X**: each dimension scaled to [0, 1] using min/max
- **y**: Z-score normalized (mean 0, std 1)
- Grid predictions are inverse-transformed to the original scale

## Complexity

| Step                    | Cost     | N = 10000, M = 100 estimate |
| ----------------------- | -------- | --------------------------- |
| FITC/VFE kernel matrices| O(N·M²)  | 1×10⁸                       |
| Cholesky (M×M)          | O(M³)    | 1×10⁶                       |
| Grid prediction (50×50) | O(2500·M)| 2.5×10⁵                     |

Target: under 10,000 ms (release build). Measured at N = 10,000: GP-FITC ≈ 2.4 s, GP-VFE ≈ 2.0 s.

## R² Interpretation

| R²    | Action                                                                         |
| ----- | ------------------------------------------------------------------------------ |
| ≥ 0.8 | Good fit. Surface is reliable.                                                 |
| < 0.5 | Poor fit — try Random Forest or LightGBM, or increase data if N is small.     |

## Strengths and Limitations

**Strengths**
- Highest-quality smooth surface (C² continuity)
- Works well with small N (as few as 20 points)
- ARD automatically identifies important dimensions
- Trains on all N points (no subsampling); backed by egobox-gp
- Principled uncertainty estimates (95% CI band)

**Limitations**
- For N > 100 the M = 100 inducing-point cap bounds cost but introduces slight approximation error
- GP-FITC may overfit noisy data (use GP-VFE in that case)
- COBYLA multistart can converge to local optima

## When to Use

```
Smooth nonlinear, default?                  → GP-FITC (best quality, default)
Smooth nonlinear, surface looks overfit?    → GP-VFE (smoother, more conservative)
Discontinuous / multi-regime response?      → GP-MOE (see gaussian-process-moe.md)
Nonlinear / noisy / large N?               → LightGBM or Random Forest
Linear response?                            → Ridge
```
