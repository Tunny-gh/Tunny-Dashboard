# Elbow Method — Automatic k Selection

## Overview

The Elbow Method automatically estimates the optimal number of clusters k for k-means by finding the point where WCSS stops decreasing significantly — the "elbow" in the curve.

## Algorithm

### Step 1: Collect WCSS

Run k-means for k = 2, 3, …, k_max and record:

$$
W_k = \text{WCSS}(k) \quad \text{for } k = 2, \ldots, k_{\max}
$$

k_max = min(N, max-k setting) — the upper limit defaults to 10 and can be changed in the cluster widget UI ("Max k", range 2–50) when k selection is set to Elbow (Auto); N is the number of trials.

**Note.** Each $W_k$ collected here is actually `model.inertia()` from linfa — the mean of squared distances to the nearest centroid (WCSS/N), not the summed WCSS — see the note in [kmeans.md](./kmeans.md#objective). Since N is constant across all k tried, this only rescales every $W_k$ by the same factor $1/N$ and does not change the location of the second-difference maximum below.

### Step 2: Second-Order Finite Difference

Compute the second-order difference of the WCSS sequence:

$$
\Delta^2 W_i = W_i - 2 W_{i+1} + W_{i+2}
$$

Δ²W_i captures the curvature at position k = i + 3. A straight line gives Δ²W = 0; the elbow point gives the maximum.

### Step 3: Recommended k

$$
\hat{k} = \arg\max_i \Delta^2 W_i + 3
$$

The +3 offset accounts for the k = 2 start of the WCSS sequence:

| Variable    | Corresponds to k                        |
| ----------- | --------------------------------------- |
| W_0         | k = 2                                   |
| Δ²W_0       | uses k=2,3,4 → elbow at k = 3 (= 0+3)  |
| Δ²W_i       | elbow at k = i + 3                      |

Result is clamped to [2, k_max].

## Numerical Example

| k | WCSS |
| - | ---- |
| 2 | 1000 |
| 3 | 400  |
| 4 | 350  |
| 5 | 340  |

Δ²W_0 = 1000 − 2×400 + 350 = **150** → elbow at k = 3  
Δ²W_1 = 400 − 2×350 + 340 = 40

Recommended k = **3**.

## Edge Cases

| Case                        | Behavior                                          |
| --------------------------- | ------------------------------------------------- |
| k_max < 2                   | Skip computation, return k̂ = 2                   |
| Fewer than 3 WCSS values    | Skip second difference, return k̂ = n_tried + 1   |
| All points coincide         | Fallback avoids zero-distance centroids            |

## Strengths and Limitations

**Strengths**
- No manual k input required
- Second-difference cancels linear trend — robust to scale
- Low overhead: only k_max k-means runs

**Limitations**
- Unreliable when WCSS decreases smoothly with no clear elbow
- May over-estimate k on uniformly distributed data
- k_max defaults to 10; if the data genuinely needs more clusters, raise the "Max k" setting (up to 50) so the true elbow falls inside the search range — at the cost of more k-means runs

## When the Estimate Seems Off

Switch to **Manual** mode and pick k directly. A useful heuristic: choose the k just after the steepest WCSS drop, preferring smaller k when interpretability matters.

## References

- Thorndike, R. L. (1953). Who Belongs in the Family? _Psychometrika_, 18(4), 267–276.
