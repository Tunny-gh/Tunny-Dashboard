# k-means Clustering (Lloyd's Algorithm)

## Overview

k-means partitions trials into k clusters by minimizing the Within-Cluster Sum of Squares (WCSS). Each trial belongs to the nearest centroid.

## Objective

$$
\text{WCSS} = \sum_{k} \sum_{x_i \in C_k} \|x_i - \mu_k\|^2
$$

μ_k is the centroid of cluster C_k.

**Note on the app's `wcss` field.** The value the app reports as `wcss` is `model.inertia()` from linfa, which is the **mean** of squared distances to the nearest centroid ($\text{WCSS}/N$), not the summed WCSS defined above. This does not affect the Elbow method: since N is the same across all k tried, using the mean instead of the sum only rescales every $W_k$ by a common constant $1/N$ and does not shift the position of the second-difference maximum (see [elbow.md](./elbow.md)).

## Algorithm

1. **Initialize** — select k starting centroids using the chosen strategy
2. **Assign** — assign each point to the nearest centroid
3. **Update** — recompute each centroid using linfa's m\_k-means update (see below), which folds in the previous centroid rather than taking a plain mean
4. **Converge** — stop when the Euclidean distance between the old and new centroid arrays is below tolerance 1e-5 (max 300 iterations)

## Update Step

`linfa_clustering::KMeans` does not average each cluster's points directly. It uses an m\_k-means-style update that folds the previous centroid in as an extra point:

$$
\mu_k^{\text{new}} = \frac{\mu_k^{\text{old}} + \sum_{x_i \in C_k} x_i}{|C_k| + 1}
$$

**Empty cluster**: when $|C_k| = 0$, the formula reduces to $\mu_k^{\text{new}} = \mu_k^{\text{old}}$ — the previous centroid is kept automatically, with no special-cased branch needed.

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
| Theory          | O(log k) approximation    | Same guarantee (identical algorithm) |
| Local optima    | Reduced by best-of-10 (see below) | Reduced by best-of-10; always the same run since the seed is fixed |

Both strategies run the *same* D²-proportional selection and the same best-of-10 re-run (below); the only difference is the seed. The "Theory" and "Local optima" rows are therefore not a meaningful basis for choosing one over the other — pick k-means++ for a fresh seed derived from the data, or Deterministic when the caller (e.g. the Elbow method) needs a fixed, repeatable seed.

## Multiple Runs (best-of-10)

For a given seed, `linfa_clustering::KMeans` is configured with `.n_runs(10)`: it runs the full initialize → assign → update → converge procedure **10 independent times** and keeps the result with the lowest inertia (WCSS). This reduces — but does not eliminate — the risk of returning a poor local optimum, for both k-means++ and Deterministic alike.

## Implementation Parameters

| Parameter          | Value                     |
| ------------------ | ------------------------- |
| max_iter           | 300                       |
| n_runs             | 10 (best-of-10 by inertia)|
| Distance metric    | Squared Euclidean         |
| Empty cluster      | Keep previous centroid (see [Update Step](#update-step)) |

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

## References

- Lloyd, S. P. (1982). Least squares quantization in PCM. _IEEE Transactions on Information Theory_, 28(2), 129–137.
- Arthur, D., & Vassilvitskii, S. (2007). k-means++: The Advantages of Careful Seeding. _SODA 2007_, 1027–1035.
