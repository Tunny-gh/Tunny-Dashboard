# k-means Clustering (Lloyd's Algorithm)

## Overview

k-means partitions trials into k clusters by minimizing the Within-Cluster Sum of Squares (WCSS). Each trial belongs to the nearest centroid.

## Objective

```
WCSS = Σ_k Σ_{x_i ∈ C_k} ||x_i − μ_k||²
```

μ_k is the centroid of cluster C_k.

## Algorithm

1. **Initialize** — select k starting centroids using the chosen strategy
2. **Assign** — assign each point to the nearest centroid
3. **Update** — recompute each centroid as the mean of its points
4. **Converge** — stop when no point changes cluster (max 300 iterations)

## Initialization Strategies

### k-means++ (Default)

Selects centroids far from existing ones using D²-weighted probability:

```
p(x_i) = D(x_i)² / Σ_j D(x_j)²
```

D(x_i) = distance from x_i to the nearest existing centroid.

Starting point: the ⌊N/2⌋-th point (fixed). Subsequent centroids sampled via xorshift64 PRNG with a seed derived from n and k (reproducible).

**Theoretical guarantee**: expected WCSS ≤ 8(ln k + 2) × WCSS_opt.

### Deterministic

No randomness — selects centroids by equal spacing via cumulative-distance threshold:

```
θ = Σ_i d_i / (remaining_selections + 1)
```

Scans points in order; picks the first point where cumulative distance ≥ θ. Always produces identical results regardless of seed.

| Aspect          | k-means++                 | Deterministic              |
| --------------- | ------------------------- | -------------------------- |
| Selection       | D²-proportional sampling  | Cumulative-distance spread |
| Randomness      | xorshift64 (fixed seed)   | None                       |
| Reproducibility | Same seed → same result   | Always identical           |
| Theory          | O(log k) approximation    | None                       |
| Local optima    | Low risk                  | Moderate risk              |

## Implementation Parameters

| Parameter          | Value                     |
| ------------------ | ------------------------- |
| max_iter           | 300                       |
| Distance metric    | Squared Euclidean         |
| Empty cluster      | Keep previous centroid    |

## Strengths and Limitations

**Strengths**
- Fast and interpretable
- WCSS quantifies solution quality

**Limitations**
- Requires k upfront (use Elbow method for auto-selection)
- Assumes convex / spherical cluster shapes
- Sensitive to outliers (centroid pulled toward them)
- May converge to local optima

## Input Space

| Setting         | Features used           | Best for                              |
| --------------- | ----------------------- | ------------------------------------- |
| Objective Space | Objective values only   | Cluster by performance similarity     |
| Variable Space  | Parameter values only   | Cluster by design space patterns      |
| Combined        | Both                    | Joint structure analysis              |
