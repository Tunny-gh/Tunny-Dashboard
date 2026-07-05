# TOPSIS

## Overview

TOPSIS (Technique for Order Preference by Similarity to Ideal Solution) ranks optimization trials by scoring each trial based on its distance from the ideal solution (best possible) and the anti-ideal solution (worst possible). Score 1 = ideal, Score 0 = worst.

| Return value       | Description                                            |
| ------------------ | ------------------------------------------------------ |
| `scores[i]`        | TOPSIS score for trial i (0–1, higher is better)       |
| `rankedIndices`    | Trial indices sorted by score descending               |
| `positiveIdeal[j]` | Ideal value for objective j (A+)                       |
| `negativeIdeal[j]` | Anti-ideal value for objective j (A-)                  |

## Algorithm

Given a decision matrix V (m trials × n objectives), TOPSIS computes scores in 6 steps.

### Step 1: Vector Normalization

Normalize each objective column j by its Euclidean norm:

$$r_{ij} = \frac{v_{ij}}{\sqrt{\sum_i v_{ij}^2}}$$

This makes objectives with different scales comparable.

### Step 2: Weighted Normalized Matrix

Multiply normalized values by user-assigned weights w_j:

$$w_{ij} = w_j \cdot r_{ij}$$

This library function does not normalize the weights itself — it uses `weights` as given. The UI layer normalizes weights to sum to 1 before calling into TOPSIS (see `mcdm_chart.rs`). This is why weight scale does not matter: `w_ij = w_j · r_ij` scales every term by the same constant, so `D_i^+`, `D_i^-`, and therefore the score are unaffected by a uniform rescaling of the weight vector.

### Step 3: Ideal and Anti-Ideal Solutions

For each objective, select the best and worst values according to direction:

| Direction | Positive ideal A+_j | Negative ideal A-_j |
| --------- | ------------------- | ------------------- |
| minimize  | min_i w_ij          | max_i w_ij          |
| maximize  | max_i w_ij          | min_i w_ij          |

### Step 4: Euclidean Distances

$$D_i^+ = \sqrt{\sum_j (w_{ij} - A_j^+)^2}$$

$$D_i^- = \sqrt{\sum_j (w_{ij} - A_j^-)^2}$$

### Step 5: TOPSIS Score (Relative Closeness)

$$\text{score}_i = \frac{D_i^-}{D_i^+ + D_i^-}$$

- score → 1: close to positive ideal (good trial)
- score → 0: close to negative ideal (poor trial)
- D+ + D- = 0: degenerate case → score = 0.5

### Step 6: Ranking

Sort by score descending.

## Edge Cases

**NaN/Inf trials**: any trial with a non-finite objective value (NaN or ±Inf) is excluded from computation; its score is set to 0.0 and placed at the end of the ranking.

**All trials same value**: column norm = 0, so r_ij = 0, and D+ = D- = 0 → score = 0.5.

**Weight scale invariance**: weights [0.7, 0.3] and [7.0, 3.0] give identical results, because `compute_topsis` uses the weights as given (no internal normalization) and the score is invariant to a uniform rescaling of `w` — only the ratio between weights matters.

## Complexity

| Step        | Cost       |
| ----------- | ---------- |
| Normalize   | O(m × n)   |
| Ideal sols  | O(m × n)   |
| Distances   | O(m × n)   |
| Sort        | O(m log m) |
| **Total**   | **O(m × n + m log m)** |

Under 100 ms for 50,000 trials × 4 objectives.

## Strengths and Limitations

**Strengths**
- Aggregates multiple objectives into a single intuitive [0, 1] score
- Handles mixed minimize / maximize directions
- Weights let you tune relative importance in real time

**Limitations**
- Weight choice is subjective — start with equal weights and adjust sliders
- Rank reversal can occur when alternatives are added or removed
- Objectives with very different scales may not be fully compensated by vector normalization

## When to Use

```
Want a fast, intuitive overall ranking?    → TOPSIS
Need to balance utility vs. worst-case?   → VIKOR
Want pairwise preference detail?           → PROMETHEE
```
