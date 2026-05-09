# Spearman Rank Correlation

## Overview

Spearman's rank correlation coefficient ρ measures the **strength of a monotonic relationship** between a parameter x and an objective y. It works on ranks rather than raw values, so it handles non-linear but monotonic relationships and is robust to outliers.

The ImportanceChart displays |ρ| (absolute value): it captures how strongly a parameter is related to the objective, regardless of direction.

## Formula

### Step 1: Convert values to ranks

```
x = [3.1, 1.2, 4.5, 2.0]
rank(x) = [3, 1, 4, 2]
```

Ties receive the average rank:

```
x = [1.0, 2.0, 2.0, 3.0]  →  rank(x) = [1, 2.5, 2.5, 4]
```

### Step 2: Apply Pearson correlation to ranks

After converting to ranks R_x and R_y:

```
ρ = Corr(R_x, R_y)
  = Σ(R_xi − R̄_x)(R_yi − R̄_y) / sqrt(Σ(R_xi − R̄_x)² · Σ(R_yi − R̄_y)²)
```

For ties-free data this simplifies to:

```
ρ = 1 − 6·Σd² / (n·(n²−1))
```

where d_i = R_xi − R_yi (difference of ranks).

## Multiple Objectives

The importance score for parameter j is the mean |ρ| across all objectives:

```
score(p_j) = (1/m) · Σ_k |ρ(p_j, y_k)|
```

## Characteristics

**Strengths:**
- Robust to outliers (uses ranks, not raw values).
- Detects any monotonic relationship, not just linear.
- Non-parametric — no distributional assumptions.
- Very fast: O(n log n).

**Limitations:**
- Cannot detect non-monotonic (U-shaped, multi-modal) relationships.
- Does not capture parameter **interactions**.
- No magnitude scale (tells you strength, not direction of effect size).

## When to Use

- Small datasets (n < 50) — still reliable.
- First pass to screen all parameters.
- When the shape of the objective function is unknown.
