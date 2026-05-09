# Entropy Weight Method

## Overview

The Entropy Weight Method uses Shannon entropy to compute objective weights **automatically from data variance**. Objectives that vary more across trials carry more information and receive higher weights. No manual input is required.

| Return value        | Description                                                   |
| ------------------- | ------------------------------------------------------------- |
| `weights[j]`        | Weight for objective j (sum = 1)                              |
| `entropies[j]`      | Information entropy e_j ∈ [0, 1]                              |
| `diversities[j]`    | Diversity d_j = 1 − e_j ∈ [0, 1]                             |
| `normalized_matrix` | Proportionally normalized matrix (rows = trials, cols = objs) |

## Algorithm

### Step 1: Handle Negative Values

Entropy requires non-negative values. For any objective column j that contains negative values, apply Min-Max normalization:

```
x'_ij = (x_ij − min_i x_ij) / (max_i x_ij − min_i x_ij)
```

Columns with no negative values keep their original values. Zero-division guard: if max = min, set x'_ij = 0.

### Step 2: Proportional Normalization

Normalize each column so its sum = 1, giving a probability-like matrix P:

```
p_ij = x'_ij / Σ_i x'_ij
```

Zero-division guard: if the column sum is 0, set p_ij = 0.

### Step 3: Shannon Entropy

```
e_j = −(1 / ln m) · Σ_i p_ij · ln(p_ij)
```

The factor 1/ln(m) normalizes entropy to [0, 1]. Terms where p_ij = 0 contribute 0 (0·ln(0) = 0). When m = 1, set e_j = 0.

### Step 4: Diversity

```
d_j = 1 − e_j
```

High entropy → low diversity → low weight (objective is uninformative).

### Step 5: Weight Normalization

```
w_j = d_j / Σ_k d_k
```

Uniform fallback: if all d_k = 0 (all objectives are constant), assign equal weights w_j = 1/n.

## Numerical Example

3 trials × 2 objectives (both non-negative):

| Trial | Obj 1 | Obj 2 |
| ----- | ----- | ----- |
| 0     | 5     | 1     |
| 1     | 5     | 2     |
| 2     | 5     | 3     |

Obj 1 is constant → e_1 = 1.0, d_1 = 0, **w_1 = 0**.  
Obj 2 varies → e_2 ≈ 0.982, d_2 ≈ 0.018, **w_2 = 1.0**.

The constant objective contributes nothing; the varying one carries all the weight.

## Complexity

O(m × n) — under 100 ms for 50,000 trials × 4 objectives.

## Strengths and Limitations

**Strengths**
- Fully data-driven — no manual weight input needed
- Eliminates analyst bias
- Lightweight: scales linearly with data size

**Limitations**
- Assumes "more variance = more important," which may not match domain knowledge
- Near-constant objectives may have their weight dominated by small noise
- Cannot encode domain expertise about objective priority

## When to Use

```
No domain knowledge about relative importance?    → Entropy Weight (auto)
Want data-driven weights for TOPSIS / VIKOR?      → Entropy Weight + any method
Have strong preferences about objective priority? → Manual weight sliders
Comparing both approaches?                        → toggle Entropy ↔ Manual in the UI
```
