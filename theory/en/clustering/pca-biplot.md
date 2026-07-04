# PCA Biplot

## Overview

Principal Component Analysis (PCA) finds the orthogonal directions along which a data set varies the most, and re-expresses each trial as coordinates along those directions (its **scores**). A **biplot** overlays two things in the same 2D plane: the trials as points (using the first two PC scores) and the original variables as arrows (their **loadings** on those same two components) — so the question "which trials are similar" and the question "which variables drive that similarity" can be read off a single picture.

Because a study mixes parameters and objectives with unrelated units (a length in mm next to a stress in MPa), PCA here always runs on **standardized** data — correlation PCA — so that no variable dominates the projection purely because of its numeric scale.

---

## Formula

Let $Z$ be the standardized data matrix, one row per trial and one column per selected variable:

$$
Z_{ij} = \frac{x_{ij} - \mu_j}{\sigma_j}
$$

with $\mu_j, \sigma_j$ the mean and standard deviation of column $j$. This is a correlation-matrix PCA rather than a covariance-matrix PCA — required here because parameters and objectives carry different units, and without standardization a variable with a numerically large range would dominate the first component regardless of its actual explanatory value.

The correlation matrix and its eigendecomposition are

$$
R = \frac{1}{n-1} Z^{\top} Z = W \Lambda W^{\top}, \qquad \Lambda = \mathrm{diag}(\lambda_1 \ge \lambda_2 \ge \dots \ge \lambda_p)
$$

with $W = [w_1, \dots, w_p]$ orthonormal. The **scores** (trial positions in the biplot) are the projection of the standardized data onto the eigenvectors,

$$
T = Z W, \qquad T_{i,k} = \sum_j Z_{ij} w_{jk}
$$

and the **loadings** for component $k$ are the components of the eigenvector $w_k$ itself — one coefficient per variable. The **explained variance ratio** of component $k$ is

$$
\text{ratio}_k = \frac{\lambda_k}{\sum_{j=1}^{p} \lambda_j}
$$

with the denominator summed over *all* $p$ components, not just the two shown — so the two axis-label percentages need not add up to a large share of the total variance.

**Implementation notes.** The correlation matrix is eigendecomposed directly (`faer`'s symmetric eigensolver), not via SVD of $Z$ — equivalent in exact arithmetic, cheaper here since $p$ (number of variables) is typically much smaller than $n$ (number of trials). A column with zero variance is mapped to the constant $0$ during standardization rather than dividing by zero; it carries no variance and contributes nothing to any component, so it stays visually inert (a zero-length loading arrow) without corrupting the others. For display, the loading arrows are rescaled by a single global factor so their lengths sit within the same plot range as the trial scores — this is a presentation choice, not a rescaling of the underlying loadings, and it preserves the *relative* lengths and angles between arrows.

---

## Characteristics

- **The axes are linear combinations, not original variables.** A principal component is a weighted sum of every selected variable; "high PC1" rarely maps to a single intuitive quantity. Read the loading arrows to see which variables compose each axis before interpreting trial positions along it.
- **The sign of a component is arbitrary.** Eigenvectors are only defined up to a sign flip; a mirrored biplot (left-right or top-bottom) represents the identical structure.
- **Watch the explained-variance percentages in the axis labels.** Two components can capture a small fraction of the total variance when the data has many weakly-correlated variables — in that case, distances between trials in the biplot understate their true dissimilarity in the full variable space.
- **Loading arrows are an approximation, not an exact correlation.** Because the implementation displays the raw eigenvector coefficients (uniformly rescaled for plotting, not individually rescaled by $\sqrt{\lambda_k}$), an arrow's direction is a reliable indicator of how a variable correlates with the two shown components, and the angle between two arrows approximates the correlation between those two variables — but arrow *length* comparisons across variables are only qualitative, and the approximation degrades the less the first two components dominate the total variance.
- **Relation to the Cluster Scatter's internal PCA.** The Cluster Scatter widget also has an internal PCA-based 2D projection for display purposes, but that one only centers the data (no per-variable standardization) — appropriate when the widget projects a single homogeneous space. This widget standardizes deliberately because it routinely mixes parameters and objectives on unrelated scales; the two projections of the same trials will generally differ.

---

## Where It Is Used in the App

- **PCA Biplot widget**: choose the variable space (Parameters / Objectives / All), optionally toggle loading arrows and objective-based point coloring, and read trial clustering and variable structure together on the PC1–PC2 plane.
