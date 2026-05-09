# PROMETHEE I / II

## Overview

PROMETHEE (Preference Ranking Organisation METHod for Enrichment Evaluations) evaluates every pair of trials (a, b) — "how much is a preferred over b?" — and aggregates those preferences into flow scores.

- **PROMETHEE I**: partial ranking using Φ+ and Φ-
- **PROMETHEE II**: complete ranking using Φnet

| Return value        | Description                                              |
| ------------------- | -------------------------------------------------------- |
| `phi_plus[i]`       | Positive flow Φ+ — how much trial i outperforms others   |
| `phi_minus[i]`      | Negative flow Φ- — how much trial i is outperformed      |
| `phi_net[i]`        | Net flow Φnet = Φ+ − Φ-  ∈ [−1, 1]                      |
| `ranked_indices_i`  | PROMETHEE I ranking (Φ+ desc, tiebreak Φ- asc)          |
| `ranked_indices_ii` | PROMETHEE II ranking (Φnet desc)                         |

## Algorithm

### Step 1: Threshold Auto-Computation

For each objective j, compute the preference threshold from the data range:

```
range_j = max_i f_ij − min_i f_ij
p_j     = 0.2 × range_j          (strict preference threshold)
q       = 0                       (indifference threshold)
```

### Step 2: Linear Preference Function

For a trial pair (a, b), compute the signed difference for objective j:

```
d_j = f_bj − f_aj   (minimize)
d_j = f_aj − f_bj   (maximize)
```

Linear preference function:

```
P_j(d) = 0          if d ≤ 0
P_j(d) = d / p_j    if 0 < d < p_j
P_j(d) = 1          if d ≥ p_j
```

### Step 3: Aggregated Preference Index

```
π(a, b) = Σ_j w_j · P_j(d_j(a, b))   ∈ [0, 1]
```

π(a, b) = 1 means a is strictly preferred over b across all objectives.

### Step 4: Positive and Negative Flows

```
Φ+(i) = 1/(m−1) · Σ_{b≠i} π(i, b)
Φ-(i) = 1/(m−1) · Σ_{b≠i} π(b, i)
Φnet(i) = Φ+(i) − Φ-(i)
```

### Step 5: Ranking

**PROMETHEE I** (Φ+ desc, tiebreak Φ- asc): may leave some pairs incomparable when Φ+(a) > Φ+(b) but Φ-(a) > Φ-(b).

**PROMETHEE II** (Φnet desc): always produces a complete ranking.

## Comparison with TOPSIS / VIKOR

| Aspect        | TOPSIS           | VIKOR              | PROMETHEE              |
| ------------- | ---------------- | ------------------ | ---------------------- |
| Approach      | Distance ideal   | Gap linear combo   | Pairwise preference    |
| Ranking       | Complete (score) | Complete (Q)       | I: partial / II: full  |
| Score range   | [0, 1]           | [0, 1]             | Φnet ∈ [−1, 1]         |
| Complexity    | O(m × n)         | O(m × n)           | **O(m² × n)**          |

## Edge Cases

**NaN trials**: excluded from pairwise computation; flows set to 0.0, placed last in ranking.

**Single trial**: m = 1, denominator max(m−1, 1) = 1, all flows = 0.0.

**All trials identical**: p_j = 0, all differences = 0, all flows = 0.0.

## Complexity

O(m² × n + m log m). The pairwise comparison is the bottleneck — under 20 ms for 10,000 trials × 4 objectives in a release build.

## When to Use

```
Want pairwise "how much is a better than b?"    → PROMETHEE I / II
Need partial ranking (incomparable pairs)?       → PROMETHEE I
Need a complete ranking with net flow score?     → PROMETHEE II
Dataset > 10,000 trials and speed matters?       → prefer TOPSIS / VIKOR
```
