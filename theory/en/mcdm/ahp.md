# AHP — Analytic Hierarchy Process

## Overview

AHP derives objective weights from a **pairwise comparison matrix** entered by the user, then ranks trials using a weighted sum score. The method also reports a Consistency Ratio (CR) to flag contradictory comparisons.

| Return value      | Description                                            |
| ----------------- | ------------------------------------------------------ |
| `priority_vector` | Derived weight vector (Σ = 1.0)                        |
| `scores[i]`       | AHP score for trial i (0–1, higher is better)          |
| `ranked_indices`  | Trial indices sorted by score descending               |
| `lambda_max`      | Maximum eigenvalue                                     |
| `ci`              | Consistency Index                                      |
| `ri`              | Random Index                                           |
| `cr`              | Consistency Ratio                                      |
| `is_consistent`   | true if CR ≤ 0.10                                      |

## Algorithm

### Step 1: Pairwise Comparison Matrix

Enter relative importance between objectives using Saaty's 1–9 scale:

```
A[i][j] = a_ij    (i is a_ij times more important than j)
A[j][i] = 1/a_ij  (reciprocal)
A[i][i] = 1.0     (diagonal)
```

**Saaty scale:**

| Value   | Meaning             |
| ------- | ------------------- |
| 1       | Equally important   |
| 3       | Slightly more       |
| 5       | Considerably more   |
| 7       | Much more           |
| 9       | Extremely more      |
| 2,4,6,8 | Intermediate values |

### Step 2: Priority Vector (Approximate Eigenvector)

**Column normalize**: divide each element by its column sum:

```
B_ij = A_ij / Σ_k A_kj
```

**Row average**: average each row to get the weight vector:

```
w_i = (1/n) · Σ_j B_ij         (Σ w_i = 1)
```

### Step 3: Consistency Check

**Maximum eigenvalue approximation:**

```
λ_max = Σ_j (Σ_i A_ij) · w_j
```

**Consistency Index:**

```
CI = (λ_max − n) / (n − 1)
```

**Random Index (Saaty's table):**

| n  | RI   |
| -- | ---- |
| 1  | 0.00 |
| 2  | 0.00 |
| 3  | 0.58 |
| 4  | 0.90 |
| 5  | 1.12 |
| 6+ | 1.24 |

**Consistency Ratio:**

```
CR = CI / RI
```

- CR ≤ 0.10: consistent — comparisons are logically coherent
- CR > 0.10: inconsistent — review and revise the pairwise comparisons
- n ≤ 2: RI = 0, so CR is defined as 0 (always consistent)

### Step 4: Trial Score Calculation

**Min-Max normalize** each objective column according to direction:

| Direction | Formula                                         |
| --------- | ----------------------------------------------- |
| minimize  | (V_max_j − v_ij) / (V_max_j − V_min_j)         |
| maximize  | (v_ij − V_min_j) / (V_max_j − V_min_j)         |

If V_max_j = V_min_j, normalized value = 0.

**Weighted sum score:**

```
score_i = Σ_j w_j · v̂_ij
```

Rank by score descending.

## Edge Cases

**NaN trials**: excluded from normalization; score = 0.0, placed last in ranking.

**n ≤ 2**: CR is always 0; consistency check is trivially satisfied.

## Complexity

O(n² + m × n + m log m). Because n (objectives) is typically 2–6, the bottleneck is O(m × n) for scoring.

## Strengths and Limitations

**Strengths**
- Pairwise comparisons are intuitive — easier than setting raw weights directly
- Consistency Ratio catches contradictory inputs before they affect results
- Handles mixed minimize / maximize directions

**Limitations**
- For n objectives, n(n−1)/2 comparisons are needed — burdensome for n > 6
- Saaty's discrete 1–9 scale can't express subtle differences
- Min-Max normalization is sensitive to outliers

## When to Use

```
3–5 objectives and want guided weight elicitation?   → AHP
Weights already known numerically?                   → TOPSIS / VIKOR with sliders
Weights from data variance?                          → Entropy Weight Method
```
