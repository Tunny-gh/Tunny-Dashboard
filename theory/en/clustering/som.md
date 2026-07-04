# Self-Organizing Map (SOM)

## Overview

A Self-Organizing Map learns a 2D grid of nodes such that nearby nodes on the grid represent similar points in the (typically much higher-dimensional) design space — a topology-preserving mapping. Where [Parallel coordinates](../widgets/parallel-coords.md) shows every dimension as a separate axis, the SOM folds all of them onto a single 2D sheet that can be surveyed at a glance, at the cost of some distortion. It complements clustering: instead of a fixed partition into $k$ groups, it gives a continuous map on which cluster-like regions, gradients, and outliers can all be read visually.

The implementation is a **batch SOM** on a rectangular grid with Gaussian neighborhoods, initialized deterministically from a PCA plane — no random seed anywhere in training, so the same data always produces the same map.

---

## Formula

Each column of the input is standardized to zero mean and unit variance before training, for the same reason as [PCA Biplot](pca-biplot.md): parameters and objectives on unrelated scales would otherwise be weighted arbitrarily by their raw numeric range. A zero-variance column is mapped to $0$ and stays inert.

For a data point $x$, the **Best Matching Unit (BMU)** is the grid node whose weight vector is closest in standardized space:

$$
b(x) = \arg\min_i \lVert x - w_i \rVert^2
$$

Training proceeds in **batch epochs** rather than one point at a time: at each epoch, every point's BMU is found against the *current* weights, and then every node's weight is replaced in one shot by a neighborhood-weighted average of all points, weighted by a Gaussian kernel of grid distance to each point's BMU:

$$
w_i \leftarrow \frac{\sum_x h\bigl(b(x), i\bigr)\, x}{\sum_x h\bigl(b(x), i\bigr)}, \qquad
h(b, i) = \exp\!\left(-\frac{d_{\text{grid}}(b, i)^2}{2\,\sigma(t)^2}\right)
$$

where $d_{\text{grid}}$ is Euclidean distance on the grid (in node units) and $\sigma(t)$ is the neighborhood radius, which decays exponentially over the epoch index $t \in [0, 1]$ (normalized across the configured epoch count) from $\sigma_0 = \max(W, H)/2$ down to $\sigma_{\text{end}} = 0.5$:

$$
\sigma(t) = \sigma_0 \left(\frac{\sigma_{\text{end}}}{\sigma_0}\right)^{t}
$$

A wide neighborhood early on lets distant nodes move together (unfolding the map globally); as $\sigma$ shrinks, updates become increasingly local, refining fine structure.

**Initialization** is linear along the data's first two principal components (see [PCA Biplot](pca-biplot.md) for the eigendecomposition) rather than random: node $(g_x, g_y)$ starts at

$$
w_{(g_x, g_y)} = a \sqrt{\lambda_1}\, w_1 + b \sqrt{\lambda_2}\, w_2, \qquad a, b \in [-2, 2] \text{ linear in } g_x, g_y
$$

spanning $\pm 2$ standard deviations along each of the top two PCs. This makes the whole training procedure — initialization and every batch update — completely deterministic: identical input always yields an identical map, with no PRNG involved anywhere.

**Reading the trained map:**
- **U-matrix**: for each node, the mean standardized-space distance to its up/down/left/right grid neighbors. High values ("ridges") sit between regions that are dissimilar in the original variable space — visual cluster boundaries.
- **Component planes**: one per feature, showing that feature's node weights de-standardized back to original units (`weight * std + mean`) — a smooth heatmap of how that one variable varies across the map.
- **Hit counts**: how many trials map to each node as their BMU — a coarse density estimate.

---

## Characteristics

- **Map quality depends on grid size relative to sample count.** Too few nodes for the data crowds many dissimilar trials onto the same node (high hit counts, low resolution); too many nodes for the data leaves most nodes empty and the U-matrix noisy.
- **Topology preservation is approximate, not guaranteed.** A high-dimensional manifold folded onto a 2D grid can still fold onto itself ("kinks") when the intrinsic structure doesn't fit flatly in 2D — neighboring nodes are then not perfectly similar.
- **Deterministic by construction, unlike classic online SOM.** Textbook SOM presents points one at a time in (typically randomized) order with a shrinking learning rate, which introduces run-to-run variation. The batch formulation used here has no such ordering dependency or learning-rate schedule to seed — same data and settings always converge to the same map, which also makes epoch count a purely computational-budget choice rather than a source of randomness to manage.
- **All distances are in standardized space.** A node's raw weight vector is not directly comparable to unstandardized data; only the de-standardized component planes are in original units.
- **U-matrix ridges are a qualitative visual cue, not a statistical test.** They mark where the map's local geometry stretches, which correlates with cluster boundaries in practice but carries no significance threshold — compare against the component planes and hit counts before drawing conclusions.

---

## Where It Is Used in the App

- **SOM widget**: configure grid size and epoch count, then switch between the U-matrix, per-feature component plane, and hit-count views of the trained map.
