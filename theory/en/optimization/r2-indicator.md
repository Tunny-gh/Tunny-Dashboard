# R2 indicator

## Overview

The R2 indicator is a convergence indicator that averages, over a large collection of weighted utility functions (Tchebycheff scalarizations), how close the approximation set can get to the ideal point. A smaller value means the approximation set is close to the ideal solution under a wide range of preference directions. It is cheaper to compute than hypervolume, and is sensitive to both convergence and spread of the front.

In Tunny Dashboard it is used by:

- Multi-objective indicators (MoIndicator::R2)
- The Convergence chart

---

## Definition

For $m$ objectives to minimize, an approximation set $A \subset \mathbb{R}^m$ (normalized to $[0,1]$, with the ideal point at this space's origin) and a set of weight vectors $W$,

$$
R2(A; W) = \frac{1}{|W|} \sum_{w \in W} \min_{a \in A} \max_j w_j\, a_j
$$

For each weight $w$, the weighted Tchebycheff scalarization $\max_j w_j a_j$ (the weighted maximum distance from the ideal at the origin) is minimized over the approximation set, and the result is averaged over every weight. The closer $A$ gets to the origin (the ideal), the smaller each term is, and the smaller $R2$ becomes.

### Weight vectors: the Das–Dennis simplex lattice

$W$ is generated from the Das–Dennis simplex lattice. Each component has the form $k / h$ (with $\sum_j k_j = h$, so $\sum_j w_j = 1$), and the candidates are every lattice point at subdivision depth $h$. The depth $h$ is chosen as the largest value for which the number of generated weights, $\binom{h+m-1}{m-1}$, stays at or below 100 ($h=99$ for $m=2$, $h \approx 13$ for $m=3$). Because a weight of exactly 0 would let the corresponding objective be ignored entirely, weights are clipped to a floor of $\varepsilon = 10^{-6}$ and then renormalized (so $\sum_j w_j = 1$ still holds).

### Meaning

R2 approximates, as an expected gap over many weight vectors, the situation where the decision maker's relative preference among objectives is not known in advance. Biases that a single weight would miss are captured by averaging over a lattice spanning the whole weight simplex.

---

## Application in Tunny

### Self-referential convergence analysis

The true Pareto front is usually unknown, so Tunny fixes the reference set as the **non-dominated front of the union of observed points across all series** (the baseline Study plus comparison Studies) — the same design shared with IGD+ and the ε-indicator. The R2 computation itself does not use the reference set directly; it only uses the weight vectors $W$ and the ideal (the origin of the normalized space). The reference set is used to derive the union's ideal/nadir (next section), and sharing that across series lets convergence curves from multiple Studies be compared on the same chart.

### Normalized space

Each objective is first unified to a minimization direction (maximization objectives are sign-flipped), then scaled to $[0, 1]$ using the union's ideal and nadir (for scale invariance). A dimension with a degenerate range uses a scale of 1. The exact formula is shared with [IGD+ — Normalized space](igd-plus.md#normalized-space). R2's ideal point is treated as the origin ($[0, \dots, 0]$) of this normalized space.

### Edge cases

- Empty weight-vector set (when the objective count is 0): $R2 = 0$
- Empty approximation set: $R2 = +\infty$
- Invalid points (NaN, infinity, or mismatched dimension count): carries forward the previous value

---

## Related indicators

- [Hypervolume](hypervolume.md) — a quality indicator based on dominated volume.
- [IGD+](igd-plus.md) — a convergence indicator based on average distance to the reference set.
- [additive ε-indicator](epsilon-indicator.md) — a worst-case convergence indicator.

---

## References

- M. P. Hansen, A. Jaszkiewicz, "Evaluating the Quality of Approximations to the Non-dominated Set", Technical Report IMM-REP-1998-7, 1998. (origin of the R2 indicator)
- D. Brockhoff, T. Wagner, H. Trautmann, "On the Properties of the R2 Indicator", GECCO 2012. (analysis of its properties)
- I. Das, J. E. Dennis, "Normal-Boundary Intersection: A New Method for Generating the Pareto Surface in Nonlinear Multicriteria Optimization Problems", SIAM Journal on Optimization, 8(3), 1998. (generation method for the Das–Dennis simplex lattice weight vectors)
