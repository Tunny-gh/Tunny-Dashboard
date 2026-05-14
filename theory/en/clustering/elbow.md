# Elbow Method — Automatic k Selection

## Overview

The Elbow Method automatically estimates the optimal number of clusters k for k-means by finding the point where WCSS stops decreasing significantly — the "elbow" in the curve.

## Algorithm

### Step 1: Collect WCSS

Run k-means for k = 2, 3, …, k_max and record:

$$
W_k = \text{WCSS}(k) \quad \text{for } k = 2, \ldots, k_{\max}
$$

k_max = min(user limit, N).

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
- If max_k is too small, the true elbow may be outside the search range

## When the Estimate Seems Off

Switch to **Manual** mode and pick k directly. A useful heuristic: choose the k just after the steepest WCSS drop, preferring smaller k when interpretability matters.
