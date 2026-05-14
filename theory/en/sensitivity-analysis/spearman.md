# Spearman Rank Correlation

## Overview

Spearman's rank correlation coefficient ρ measures the **strength of a monotonic relationship** between a parameter x and an objective y. It works on ranks rather than raw values, so it handles non-linear but monotonic relationships and is robust to outliers.

The ImportanceChart displays |ρ| (absolute value): it captures how strongly a parameter is related to the objective, regardless of direction.

## Formula

### Step 1: Convert values to ranks

$$
\mathbf{x} = [3.1, 1.2, 4.5, 2.0]
$$

$$
\operatorname{rank}(\mathbf{x}) = [3, 1, 4, 2]
$$

Ties receive the average rank:

$$
\mathbf{x} = [1.0, 2.0, 2.0, 3.0] \implies \operatorname{rank}(\mathbf{x}) = [1, 2.5, 2.5, 4]
$$

### Step 2: Apply Pearson correlation to ranks

After converting to ranks $R_x$ and $R_y$:

$$
\rho = \operatorname{Corr}(R_x, R_y) = \frac{\sum (R_{x_i} - \bar{R}_x)(R_{y_i} - \bar{R}_y)}{\sqrt{\sum (R_{x_i} - \bar{R}_x)^2 \cdot \sum (R_{y_i} - \bar{R}_y)^2}}
$$

For ties-free data this simplifies to:

$$
\rho = 1 - \frac{6 \sum d_i^2}{n(n^2 - 1)}
$$

where $d_i = R_{x_i} - R_{y_i}$ (difference of ranks).

## Multiple Objectives

The importance score for parameter j is the mean |ρ| across all objectives:

$$
\operatorname{score}(p_j) = \frac{1}{m} \sum_{k} |\rho(p_j, y_k)|
$$

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
