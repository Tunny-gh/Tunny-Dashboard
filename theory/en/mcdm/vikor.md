# VIKOR

## Overview

VIKOR (VIseKriterijumska Optimizacija I Kompromisno Resenje) finds the compromise solution closest to the ideal by combining L1 (Manhattan) and L∞ (Chebyshev) distance measures. A lower Q score means a better compromise.

| Return value       | Description                                                        |
| ------------------ | ------------------------------------------------------------------ |
| `s_values[i]`      | Utility measure (L1 gap from ideal, smaller = better)              |
| `r_values[i]`      | Regret measure (L∞ gap, worst-case objective)                      |
| `q_values[i]`      | Compromise score (linear combination of S and R); NaN trials → 1.0 |
| `display_scores[i]`| Display score = 1 − Q (higher is better; used for UI rendering)    |
| `ranked_indices`   | Trial indices sorted by Q ascending (lower Q = better)             |
| `best_values[j]`   | Best value for objective j across valid trials                     |
| `worst_values[j]`  | Worst value for objective j across valid trials                    |

## Algorithm

### Inputs

| Variable | Description                                 |
| -------- | ------------------------------------------- |
| f_ij     | Value of trial i for objective j            |
| w_j      | Weight for objective j (normalized, Σ = 1)  |
| v        | Strategy weight (default 0.5)               |

### Step 1: Best and Worst Values

For each objective j:

$$f_j^* = \text{best value across all trials} \quad (\min \text{ if minimize, } \max \text{ if maximize})$$

$$f_j^- = \text{worst value across all trials} \quad (\max \text{ if minimize, } \min \text{ if maximize})$$

### Step 2: S and R Values

$$S_i = \sum_j w_j \cdot \frac{f_j^* - f_{ij}}{f_j^* - f_j^-}$$

$$R_i = \max_j \left[ w_j \cdot \frac{f_j^* - f_{ij}}{f_j^* - f_j^-} \right]$$

- **S_i** (utility): sum of weighted gaps — lower is better overall
- **R_i** (regret): maximum weighted gap — lower means even the worst criterion is acceptable

When f*_j = f-_j (all trials identical for that objective), the contribution is 0.

### Step 3: S*, S-, R*, R-

$$S^* = \min_i S_i, \quad S^- = \max_i S_i$$

$$R^* = \min_i R_i, \quad R^- = \max_i R_i$$

### Step 4: Q Score

$$Q_i = v \cdot \frac{S_i - S^*}{S^- - S^*} + (1 - v) \cdot \frac{R_i - R^*}{R^- - R^*}$$

| v value | Emphasis    | Meaning                          |
| ------- | ----------- | -------------------------------- |
| v > 0.5 | S (utility) | Maximize overall consensus       |
| v = 0.5 | Balanced    | Balance utility and regret       |
| v < 0.5 | R (regret)  | Minimize worst-case objective    |

Zero-division guard: if S- = S*, the first term is 0; if R- = R*, the second term is 0.

### Step 5: Ranking

Sort trials by Q ascending — lower Q is the better compromise solution.

## Comparison with TOPSIS

| Aspect           | TOPSIS              | VIKOR                                  |
| ---------------- | ------------------- | -------------------------------------- |
| Distance measure | Euclidean (L2)      | Manhattan (L1) + Chebyshev (L∞)        |
| Ranking order    | Score descending    | Q ascending                            |
| Strategy param   | None                | v (utility vs. regret)                 |
| Zero-div guard   | score = 0.5         | contribution = 0                       |
| Best for         | Overall similarity  | Compromise / balance-focused decisions |

## Complexity

O(m × n + m log m) — under 100 ms for 50,000 trials × 4 objectives.

## When to Use

```
Minimize worst-case objective (max-regret)?   → VIKOR with v < 0.5
Maximize overall consensus across objectives? → VIKOR with v > 0.5
Want a parameter-free intuitive [0,1] score?  → TOPSIS
```
