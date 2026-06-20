# Gaussian Process Mixture of Experts (GP-MOE)

GP-MOE is a mixture-of-experts surrogate that partitions the input space into regions and fits one FITC sparse GP expert per region. Predictions from all experts are recombined smoothly, giving a continuous surface even when the underlying function has discontinuities or regime shifts. The implementation is backed by the **egobox-moe** crate (v0.35, Apache-2.0).

## Motivation

A single GP with a stationary kernel (e.g. Matérn 5/2) assumes the same smoothness everywhere. When the objective has:

- abrupt discontinuities (e.g. mode switches, constraint activations),
- clearly different curvatures in different regions (regime-switching behaviour),
- or multi-modal responses,

a single GP either underestimates one region's variation or overfits another. GP-MOE addresses this by allowing each region to have its own GP with independently fitted hyperparameters.

**Benchmark** (piecewise discontinuous function): GP-MOE R² ≈ 0.97 vs 0.89 for a single smooth-blended GP. On smooth or noisy data GP-MOE typically selects k = 1 cluster and matches GP-FITC.

## Architecture

### GMM Clustering

The input space is partitioned using a **Gaussian Mixture Model (GMM)**:

$$
p(x) = \sum_{k=1}^{K} \pi_k \, \mathcal{N}(x \mid \mu_k, \Sigma_k)
$$

Each training point is assigned a soft responsibility weight toward each cluster. GMM is preferred over hard k-means because it produces smooth, overlapping cluster boundaries that enable smooth expert recombination.

### Per-cluster FITC GP Experts

One sparse GP expert is trained per cluster. Each expert uses:

- ARD Matérn 5/2 kernel (same as GP-FITC / GP-VFE)
- Constant trend function
- FITC approximation with M = min(N, 100) inducing points (see below)
- All N training points — no subsampling

The expert sees all N points weighted by their cluster responsibility, giving it a global view while emphasizing its own region.

### Smooth Recombination

Expert predictions are blended using a **smooth heaviside recombination** with an optimized transition factor β:

$$
\hat{y}(x) = \sum_{k=1}^{K} w_k(x) \, \mu_k(x)
$$

where w_k(x) are smooth, non-negative weights that sum to one and are derived from the heaviside function applied to GMM responsibilities. The transition factor β is optimized to maximize cross-validation performance. This ensures the combined surface is continuous everywhere — the seams between experts are smooth.

## Cluster-count Selection

The number of clusters K is selected automatically by cross-validation:

1. A subsample of up to **500 points** is drawn from the training data (deterministic: evenly-spaced stride, no random draw).
2. For each candidate K ∈ {1, 2, 3} a GpMixture is fitted and cross-validated on the subsample.
3. The K with the best cross-validation score is selected — **capped at K = 3** (the CV search is expensive; larger K rarely improves real engineering data).
4. The chosen K is used to fit the full model on all N points.

Capping at 3 keeps the search tractable. In practice, on smooth or noisy data K = 1 is selected and GP-MOE degenerates to GP-FITC.

## Inducing Points and Determinism

Inducing points are chosen to keep training fully deterministic:

- **N ≤ 100**: Z = X (training points themselves); mathematically equivalent to exact GP with noise estimation.
- **N > 100**: M = 100 inducing points per expert, selected by k-means centroids (deterministic seed).

The same explicit inducing point locations are shared across experts. No random seeds are introduced beyond those fixed deterministic ones.

## Complexity

| Step                          | Cost                 |
| ----------------------------- | -------------------- |
| GMM fit (subsample ≤ 500)     | O(500 · K · D²)      |
| Per-expert FITC training      | O(N · M²) per expert |
| Grid prediction (50×50)       | O(2500 · M · K)      |

With K = 3 and M = 100 the training cost is roughly 3× that of GP-FITC. Target: under 30,000 ms (release build).

## Fallback Behaviour

If GP-MOE training fails (e.g. a cluster has too few points to fit a GP):

- **PDP**: falls back to GP-FITC silently.
- **Surrogate optimizer**: surfaces an error to the user instead of silently substituting a different model.

## R² Interpretation

| R²    | Action                                                                     |
| ----- | -------------------------------------------------------------------------- |
| ≥ 0.8 | Good fit. Surface is reliable.                                             |
| < 0.5 | Poor fit — try LightGBM or Random Forest, or increase N.                  |

## Strengths and Limitations

**Strengths**
- Handles discontinuous, regime-switching, and multi-modal response surfaces
- Automatically selects the number of clusters — no manual tuning
- Smooth output surface (no seam artefacts)
- Retains GP uncertainty estimates (95% CI)

**Limitations**
- Higher training cost than GP-FITC / GP-VFE (roughly K× more expensive)
- CV subsample capped at 500 — cluster selection may be suboptimal for very small N
- On smooth / noisy data selects K = 1, giving no advantage over GP-FITC

## When to Use

```
Discontinuous or regime-switching response?     → GP-MOE (best choice)
Smooth nonlinear, default?                      → GP-FITC
Smooth nonlinear, surface looks overfit?        → GP-VFE
Nonlinear / noisy / large N?                    → LightGBM or Random Forest
Linear response?                                → Ridge
```

See also: [GP-FITC / GP-VFE](gaussian-process.md)
