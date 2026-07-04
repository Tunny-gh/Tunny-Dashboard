# Correlation Matrix

## Overview

A correlation matrix computes the pairwise correlation between **every combination of variables** (parameters and objectives) and renders it as a single heatmap. It gives an at-a-glance overview that would otherwise require inspecting many individual scatter plots — useful for spotting redundant (highly correlated) parameters and for finding which parameters are most related to the objectives.

---

## Formula

Each cell can be computed with either of two coefficients, and the app lets you choose between them:

- [Pearson product-moment correlation](pearson-correlation.md): captures **linear** co-movement of the raw values.
- [Spearman rank correlation](../sensitivity-analysis/spearman.md): captures **monotonic** co-movement by applying Pearson correlation to the ranks.

Missing values are handled by **pairwise complete-case** analysis: for each pair of variables $(i, j)$, the correlation is computed using only the rows where *both* $i$ and $j$ are present, independently of every other pair. This differs from **listwise deletion**, which would drop a row from the entire matrix if any single variable in that row is missing. Pairwise deletion retains more data per cell, but different cells can end up computed from different subsets of rows (different effective sample sizes), which can occasionally make the resulting matrix internally inconsistent (e.g. not positive semi-definite).

---

## Characteristics

- **Correlation is not causation.** A high correlation only shows that two variables move together; it says nothing about which one (if either) drives the other.
- **Multiple-comparisons effect.** With $p$ variables there are $p(p-1)/2$ pairs being examined at once; the more variables you include, the more likely some pair shows a high correlation purely by chance. Treat any single striking cell with caution.
- **Non-monotonic relationships are invisible to both coefficients.** Even Spearman only detects monotonic relationships — a U-shaped or periodic relationship can still produce a coefficient near zero.

---

## Where It Is Used in the App

- **Correlation Matrix widget**: a single heatmap giving a bird's-eye view of every variable pair's correlation at once.
- **vs. [Scatter Matrix](../widgets/scatter-matrix.md)**: use the Correlation Matrix to survey all pairs quickly, then use the Scatter Matrix (pairplot) to inspect the actual shape of any pair that looks interesting — including non-linear patterns, clusters, and outliers that a single coefficient cannot show.
