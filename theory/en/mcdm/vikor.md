# VIKOR

## Overview

VIKOR (VIseKriterijumska Optimizacija I Kompromisno Resenje) finds the compromise solution closest to the ideal by combining L1 (Manhattan) and L∞ (Chebyshev) distance measures. A lower Q score means a better compromise.

| Return value    | Description                                           |
| --------------- | ----------------------------------------------------- |
| `s[i]`          | Utility measure (L1 gap from ideal, smaller = better) |
| `r[i]`          | Regret measure (L∞ gap, worst-case objective)         |
| `q[i]`          | Compromise score (linear combination of S and R)      |
| `rankedIndices` | Trial indices sorted by Q ascending                   |

## Algorithm

### Inputs

| Variable | Description                                 |
| -------- | ------------------------------------------- |
| f_ij     | Value of trial i for objective j            |
| w_j      | Weight for objective j (normalized, Σ = 1)  |
| v        | Strategy weight (default 0.5)               |

### Step 1: Best and Worst Values

For each objective j:

```
f*_j = best value across all trials   (min if minimize, max if maximize)
f-_j = worst value across all trials  (max if minimize, min if maximize)
```

### Step 2: S and R Values

```
S_i = Σ_j  w_j · (f*_j − f_ij) / (f*_j − f-_j)
R_i = max_j [ w_j · (f*_j − f_ij) / (f*_j − f-_j) ]
```

- **S_i** (utility): sum of weighted gaps — lower is better overall
- **R_i** (regret): maximum weighted gap — lower means even the worst criterion is acceptable

When f*_j = f-_j (all trials identical for that objective), the contribution is 0.

### Step 3: S*, S-, R*, R-

```
S* = min_i S_i,   S- = max_i S_i
R* = min_i R_i,   R- = max_i R_i
```

### Step 4: Q Score

```
Q_i = v · (S_i − S*) / (S- − S*) + (1 − v) · (R_i − R*) / (R- − R*)
```

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
