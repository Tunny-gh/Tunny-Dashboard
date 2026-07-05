# Hierarchical Clustering (Dendrogram)

## Overview

Rather than committing to a single partition into $k$ clusters up front like [k-means](kmeans.md), agglomerative hierarchical clustering builds a full **merge tree** (dendrogram): starting from every trial as its own cluster, it repeatedly merges the two closest clusters until only one remains. The number of clusters $k$ is then chosen *after* seeing this structure, by cutting the tree at a chosen height — useful when it isn't clear in advance how many groups the data actually has, or when the natural grouping is itself nested (clusters within clusters).

The implementation uses **Ward's linkage criterion**, computed efficiently with the **nearest-neighbor chain algorithm**.

---

## Formula

**Ward's criterion** merges, at each step, whichever pair of clusters increases the total within-cluster variance the least — the same objective k-means minimizes, but built up incrementally instead of assigned globally.

Rather than recomputing variance from scratch after every merge, the **Lance-Williams recurrence** updates the distance from any existing cluster $k$ to a newly merged cluster $i \cup j$ directly from the three pairwise distances that preceded the merge:

$$
d^2(k,\, i \cup j) = \frac{(n_i+n_k)\, d^2_{ik} + (n_j+n_k)\, d^2_{jk} - n_k\, d^2_{ij}}{n_i+n_j+n_k}
$$

where $n_i, n_j, n_k$ are cluster sizes. Starting distances are squared Euclidean distances between (optionally standardized — see below) trial vectors; the value stored at each merge is $\sqrt{d^2}$, so the dendrogram's height axis is measured in this Ward variance-increase scale, not in the original units of the data.

**Nearest-neighbor chain algorithm.** Naively finding the globally closest pair at every step costs $O(n^3)$ overall. Instead, the implementation follows a chain of mutually-approaching clusters: from the current chain tip, find its nearest neighbor; if that neighbor is also the previous chain element (a **reciprocal nearest-neighbor pair**), merge them immediately; otherwise extend the chain to that neighbor and repeat. Every cluster enters and leaves the chain a bounded number of times over the whole run, which brings the total cost down to $O(n^2)$.

Because the chain can wander between different branches of the eventual tree before closing a merge, the $n-1$ merges are **not** produced in strictly ascending distance order — but Ward's criterion is *monotone* (a parent merge's distance is never smaller than either child's), so a final ascending sort by distance is guaranteed to be a valid topological order: every child merge still appears before its parent, and no crossings or inversions are introduced into the drawn dendrogram.

**Cutting the tree at $k$** clusters simply discards the $k-1$ largest (last, in ascending order) merges and reads off the resulting disjoint subtrees as the cluster labels — no distance threshold needs to be chosen by the user, only the cluster count.

---

## Characteristics

- **Ward's criterion favors compact, roughly spherical clusters** — the same bias k-means has, since both minimize the same within-cluster variance objective. Elongated or non-convex true clusters may be split or merged incorrectly.
- **Standardization matters whenever variables have unrelated units or scales**, exactly as for [PCA Biplot](pca-biplot.md) and [SOM](som.md) — otherwise the variable with the largest raw range dominates every distance computed. In this implementation, standardization is always applied and is not a user-facing toggle — the widget always calls Ward linkage with standardization enabled.
- **Memory is $O(n^2)$** (the full pairwise distance matrix), and dendrograms of more than a few hundred leaves are unreadable regardless. Above **800 rows**, the implementation subsamples deterministically at an even stride rather than randomly, so the same study always produces the same reduced dendrogram; sampled-out rows simply don't appear as leaves.
- **Dendrogram height is not a physical distance.** It is the accumulated Ward variance-increase scale from the Lance-Williams recurrence — comparable across merges within the same dendrogram, but not directly comparable to raw variable units or to distances from a different linkage method.

---

## Where It Is Used in the App

- **Hierarchical Clustering widget**: view the dendrogram, drag the $k$ slider to cut the tree at a chosen number of clusters, and see the resulting clusters highlighted by color on the leaves.

## References

- Ward, J. H. (1963). Hierarchical Grouping to Optimize an Objective Function. _Journal of the American Statistical Association_, 58(301), 236–244.
- Müllner, D. (2011). Modern hierarchical, agglomerative clustering algorithms. _arXiv:1109.2378_.
