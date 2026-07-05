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

Here "values" refers to each of the two series being compared (parameter $x_i$ and objective $y$). Ranking is done **independently for each series, based on that series' own values**: the ranks of $x$ come from the values of $x$, and the ranks of $y$ from the values of $y$. This does **not** mean reordering the parameters according to the objective values.

The trial correspondence (rows) stays fixed; the $x$ column and $y$ column are each converted to ranks independently. This measures whether the rank of $x$ moves together with the rank of $y$ (i.e. whether a monotonic relationship exists).

For example, suppose four trials yield the following values (rows — the trial ordering — are fixed):

| trial | $x$ | $y$  | $\operatorname{rank}(x)$ | $\operatorname{rank}(y)$ |
|-------|-----|------|--------------------------|--------------------------|
| A     | 3.1 | 10.0 | 3                        | 2                        |
| B     | 1.2 | 30.0 | 1                        | 4                        |
| C     | 4.5 | 5.0  | 4                        | 1                        |
| D     | 2.0 | 20.0 | 2                        | 3                        |

$\operatorname{rank}(x)$ is determined by the magnitudes in the $x$ column and $\operatorname{rank}(y)$ by those in the $y$ column, each independently.

### Step 2: Apply Pearson correlation to ranks

Let $R_x = \operatorname{rank}(x)$ and $R_y = \operatorname{rank}(y)$ denote the rank series from Step 1. That is, $R_{x_i}$ is the rank of the $x$ value of the $i$-th trial, and $R_{y_i}$ is the rank of the $y$ value of the same trial. $\bar{R}_x$ and $\bar{R}_y$ are the means of those rank series (equal to $\frac{n+1}{2}$ when there are no ties).

Pearson's correlation is then computed on these rank series $R_x$ and $R_y$:

$$
\rho = \operatorname{Corr}(R_x, R_y) = \frac{\sum (R_{x_i} - \bar{R}_x)(R_{y_i} - \bar{R}_y)}{\sqrt{\sum (R_{x_i} - \bar{R}_x)^2 \cdot \sum (R_{y_i} - \bar{R}_y)^2}}
$$

For ties-free data this simplifies to:

$$
\rho = 1 - \frac{6 \sum d_i^2}{n(n^2 - 1)}
$$

where $d_i = R_{x_i} - R_{y_i}$ (difference of ranks).

## Aside: What is Pearson's product-moment correlation?

For the definition and properties of the Pearson product-moment correlation used in Step 2, see [Pearson product-moment correlation](../statistics/pearson-correlation.md).

Spearman's correlation is simply this measure applied to the **rank series** instead of the raw values. Because ranks are evenly spaced, looking at the linear co-movement of ranks corresponds to looking at the monotonic co-movement of the original values.

## Multiple Objectives

Spearman's ρ is computed independently for each objective $y_k$:

$$
\rho_k(p_j) = \rho(p_j, y_k)
$$

There is no cross-objective averaging: the dashboard keeps one score per (parameter, objective) pair. `ImportanceChart` displays $|\rho_k(p_j)|$ for the currently selected objective $k$ only — switching the objective selector recomputes and redraws the scores for that objective.

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

## References

- Spearman, C. (1904). The proof and measurement of association between two things. _American Journal of Psychology_, 15(1), 72–101.
