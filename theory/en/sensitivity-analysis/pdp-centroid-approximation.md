# Fast Confidence Band Computation for PDP via Centroid Approximation

## Overview

In 1D PDP using GP-FITC or GP-VFE (Gaussian Process), averaging the **predicted variance at each grid point** over all training data points is theoretically exact but has a computational cost of $O(G \times N \times N^2) = O(G \cdot N^3)$ ($G$ = number of grid points, $N$ = number of training points).

**Centroid approximation** is an approximation method that reduces computational complexity to $O(G \times N^2)$ by selecting only the **centroid** as the representative point for the non-target dimensions and evaluating variance at that single point.

| Method | Variance computation cost | Number of calls at N=100, G=30 |
| --- | --- | --- |
| Full average (theoretical) | $O(G \cdot N \cdot N^2) = O(G N^3)$ | 30,000 calls |
| **Centroid approximation** | $O(G \cdot N^2)$ | 30 calls |

---

## Theoretical Background

### Definition of Full PDP Variance

The confidence band of a 1D PDP requires the predicted standard deviation at grid point $v$:

$$
\hat{\sigma}_{\mathrm{PDP}}(v) = \sqrt{ \frac{1}{N} \sum_{i=1}^{N} \sigma^2(v, x_{C,i}) }
$$

where:
- $x_{C,i}$ = the **non-target dimension components** of the $i$-th training point
- $\sigma^2(v, x_{C,i})$ = GP predicted variance (described below)

Since the GP predicted variance computation itself is $O(N^2)$, evaluating this over all $N$ points gives $O(N^3)$, and over $G$ grid points gives $O(G N^3)$.

---

## Centroid Approximation

### Definition of the Approximation

Use the **arithmetic mean (centroid)** of each non-target dimension $\bar{x}_C$ as the representative point:

$$
\bar{x}_{C,d} = \frac{1}{N} \sum_{i=1}^{N} x_{C,i,d}, \qquad \forall d \ne j
$$

($j$ = dimension index of the target parameter)

The approximation using this centroid:

$$
\hat{\sigma}_{\mathrm{centroid}}(v) = \sqrt{ \sigma^2(v, \bar{x}_C) }
$$

With a single variance evaluation ($O(N^2)$), the confidence band for $G$ grid points can be computed.

### Representativeness of the Mean

For a nonlinear function $g(x_C)$, in general $g(\bar{x}_C) \ne \mathbb{E}[g(x_C)]$ (Jensen's inequality). However, the approximation is accurate when the following conditions hold:

1. **Variance changes slowly over space** (typical GPs have large length scales $l_d$)
2. **Training points are approximately symmetrically distributed** (e.g., uniform sampling)
3. **Many non-target dimensions** (high-dimensional averages tend to concentrate — concentration inequality)

Since Bayesian optimization search points are placed to approximately cover the space, conditions 1 and 2 hold in most cases.

---

## GP Predicted Variance (Gaussian Process)

Formula for computing GP predicted variance:

$$
\sigma^2(x^*) = k(x^*, x^*) - \mathbf{k}_*^T K^{-1} \mathbf{k}_*
$$

$$
= k(x^*, x^*) - \mathbf{k}_*^T (L^{-T} L^{-1}) \mathbf{k}_*
$$

$$
= k(x^*, x^*) - \mathbf{v}^T \mathbf{v}, \qquad \mathbf{v} = L^{-1} \mathbf{k}_*
$$

where:
- $K = L L^T$: Cholesky decomposition (computed only once during $O(N^3)$ training)
- $\mathbf{k}_* = [k(x^*, x_1), \ldots, k(x^*, x_N)]^T$: kernel vector between the prediction point and training points
- $\mathbf{v} = L^{-1}\mathbf{k}_*$: forward substitution ($O(N^2)$)

A single variance prediction is $O(N^2)$ (kernel vector computation $O(N)$ + forward substitution $O(N^2)$).

---

## Implementation

### Centroid Computation in Normalized Space

```rust
// Centroid of non-target dimensions (computed on normalized x_norm)
let centroid_norm: Vec<f64> = (0..n_dims).map(|d| {
    if d == target_param_idx {
        0.0  // dummy value for target dimension, overwritten by grid value
    } else {
        x_norm.iter().map(|row| row[d]).sum::<f64>() / n as f64
    }
}).collect();
```

### Grid Loop

```rust
for &v in &grid {
    let v_norm = (v - min_j) / range_j;

    // Mean: marginalize over all training points (O(N²) total)
    let mean_avg: f64 = x_norm.iter().map(|row| {
        let mut pt = row.clone();
        pt[target_param_idx] = v_norm;
        gaussian_process::predict_mean(&model, &pt)
    }).sum::<f64>() / n as f64;

    // Variance: evaluated once at centroid (O(N²) × 1)
    let mut centroid_pt = centroid_norm.clone();
    centroid_pt[target_param_idx] = v_norm;
    let var = gaussian_process::predict_variance(&model, &centroid_pt).max(0.0);

    // 95% confidence band (back-transform to original scale)
    let pdp = mean_avg * y_std + y_mean;
    let std = var.sqrt() * y_std;
    y_upper.push(pdp + 1.96 * std);
    y_lower.push(pdp - 1.96 * std);
}
```

### Cost Is Bounded by Inducing Points, Not by Subsampling Training Rows

The mean computation above (full MC over all $N$ training rows) is $O(G \cdot N \cdot N^2) = O(G N^3)$ if evaluated against the full $N \times N$ training covariance. In practice, GP-FITC / GP-VFE already bound this cost using $M \le 100$ **inducing points** (see [Gaussian Process](../surrogate-models/gaussian-process.md)) — this is a property of the FITC/VFE approximation itself, independent of $N$. It is **not** a subsample of the $N$ training rows: the mean computation still marginalizes over **all** $N$ training rows.

| Operation | Cost (N=100, G=30) |
| --- | --- |
| GP training (Cholesky, bounded by $M \le 100$ inducing points) | $O(N^3) = 10^6$ operations (once) |
| Mean computation (all grid points, all $N$ training rows) | $O(G \cdot N \cdot N^2) = 3 \times 10^7$ (forward substitution × 3,000 calls) |
| **Variance computation (centroid approximation)** | $O(G \cdot N^2) = 3 \times 10^5$ (forward substitution × 30 calls) |

Compared to evaluating the predictive variance at every training row for every grid point (instead of once at the centroid per grid point), the **variance step alone is roughly $N\times$ faster** in this example.

---

## Accuracy Considerations

### Tendency to Underestimate

Since the centroid is in the "interior" of the training points, variance tends to be underestimated in sparse regions (the periphery of the search space).

$$
\sigma^2(v, \bar{x}_C) \le \frac{1}{N}\sum_i \sigma^2(v, x_{C,i}) \quad \text{(does not hold in general)}
$$

In practice this depends on convexity/concavity, but in GPs, distant points tend to have larger variance and the centroid is closer than each individual point, so slight underestimation is likely.

### Practical Accuracy

Since confidence bands are used purely as **visual reference indicators**, this is not a problem in practice. When an accurate confidence band is required, switch to Monte Carlo approximation (subsample $N$ points and average the variances).

---

## References

- Friedman, J. H. (2001). Greedy function approximation: A gradient boosting machine. *Annals of Statistics*, 29(5), 1189–1232. (Original PDP paper)
- Goldstein, A., et al. (2015). Peeking inside the black box: Visualizing statistical learning with plots of individual conditional expectation. *Journal of Computational and Graphical Statistics*, 24(1), 44–65.
