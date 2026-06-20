# k-means Clustering (Lloyd's Algorithm)

## Overview

k-means partitions trials into k clusters by minimizing the Within-Cluster Sum of Squares (WCSS). Each trial belongs to the nearest centroid.

## Objective

$$
\text{WCSS} = \sum_{k} \sum_{x_i \in C_k} \|x_i - \mu_k\|^2
$$

μ_k is the centroid of cluster C_k.

## Algorithm

1. **Initialize** — select k starting centroids using the chosen strategy
2. **Assign** — assign each point to the nearest centroid
3. **Update** — recompute each centroid as the mean of its points
4. **Converge** — stop when WCSS change between iterations is below tolerance 1e-5 (max 300 iterations)

## Initialization Strategies

### k-means++ (Default)

Selects centroids far from existing ones using D²-weighted probability:

$$
p(x_i) = \frac{D(x_i)^2}{\sum_j D(x_j)^2}
$$

D(x_i) = distance from x_i to the nearest existing centroid.

Initialization is delegated to `linfa_clustering::KMeans` using a **Xoshiro256Plus** PRNG (`rand_xoshiro` crate) with a seed derived from n and k:

$$\text{seed} = (n \times \texttt{0x9e3779b97f4a7c15}) \oplus (k \times \texttt{0x6c62272e07bb0142})$$

Same data and k always produce the same result.

**Theoretical guarantee**: expected WCSS ≤ 8(ln k + 2) × WCSS_opt.

### Deterministic

Uses `linfa_clustering::KMeans` with a **fixed seed (42)** via Xoshiro256Plus PRNG. The centroid selection algorithm is the same as k-means++ (delegated to linfa), but the constant seed guarantees fully reproducible results on every run.

Used internally by the Elbow method for auto-k estimation.

| Aspect          | k-means++                 | Deterministic              |
| --------------- | ------------------------- | -------------------------- |
| Selection       | D²-proportional sampling (linfa) | D²-proportional sampling (linfa, fixed seed) |
| Randomness      | Xoshiro256Plus (seed from n,k) | Xoshiro256Plus (seed=42) |
| Reproducibility | Same data+k → same result | Always identical (seed=42) |
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
