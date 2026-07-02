# Hypervolume

## Overview

Hypervolume (HV) is the standard scalar quality indicator for Pareto fronts in multi-objective optimization. It measures the volume of objective space dominated by a point set $P$ and bounded above by a reference point $r$ (minimization convention: a point only contributes where it is strictly smaller than $r$ in every dimension). A larger dominated volume means a better, more spread-out front.

In Tunny Dashboard it is used by:

- The HV display in Pareto ranking (`compute_pareto_ranks`)
- The Hypervolume History widget
- Multi-objective indicators (MoIndicator::Hypervolume)
- The internals of the EHVI acquisition function (surrogate optimizer)

---

## Definition

For $m$ objectives to minimize, a point set $P \subset \mathbb{R}^m$, and a reference point $r \in \mathbb{R}^m$,

$$
\mathrm{HV}(P; r) = \mathrm{Leb}\left( \bigcup_{p \in P} [p, r] \right)
$$

where $[p, r] = \{ y : p_k \le y_k \le r_k \ \forall k \}$ and $\mathrm{Leb}$ is the Lebesgue measure (volume). A point that is not strictly smaller than $r$ in every dimension contributes zero.

---

## Automatic Reference Point

The reference point is computed from the front's nadir (worst value per objective) plus a margin proportional to the observed range:

$$
r_j = \mathrm{nadir}_j + 0.1 \cdot (\mathrm{nadir}_j - \mathrm{ideal}_j)
$$

Making the margin proportional to the observed range keeps HV **invariant to the scale of the objective values** (multiplying objectives by a constant does not change relative HV comparisons). For a degenerate dimension (all points equal), the margin falls back to $|\mathrm{nadir}_j| \cdot 0.1$, or $1.0$ if that is also zero.

The HV history widget lets the user specify a reference point explicitly, which skips the automatic computation.

> **Note:** EHVI (`theory/en/optimization/ehvi.md`) uses a separate path with a fixed margin of 0.1 in z-score-normalized space; this formula does not apply there.

---

## Algorithms

### m = 2: sweep

Sort points ascending by the first objective and accumulate rectangles between adjacent points. $O(n \log n)$.

### m ≥ 3: the WFG algorithm

Uses the WFG algorithm of While, Bradstreet, Barone (2012). HV is computed as the sum of per-point **exclusive contributions** (exclusive hypervolume):

$$
\mathrm{HV}(\{p_1, \dots, p_n\}; r) = \sum_{i=1}^{n} \mathrm{exclhv}(p_i \mid \{p_{i+1}, \dots, p_n\})
$$

Each term is the inclusive HV (volume of the point's own box) minus the HV of its "shadows":

$$
\mathrm{exclhv}(p_i \mid Q) = \underbrace{\prod_k (r_k - p_{ik})}_{\mathrm{inclhv}(p_i)} - \mathrm{HV}(\mathrm{nds}(\mathrm{limitset}(p_i, Q)); r)
$$

- **limitset**: the set of shadows $\max(p_i, q)$ (component-wise max) of the later points $q \in Q$ projected into $p_i$'s box
- **nds**: reduction to the nondominated subset. Many shadows become dominated and are discarded here — this pruning is the core of WFG and yields its practical complexity (empirically around $O(n^{m/2})$)

The recursion bottoms out at 0 points (HV = 0), 1 point (inclhv), and 2 points (closed-form inclusion–exclusion).

The previous implementation (recursive slicing over the last dimension, roughly $O(n^m)$) is kept inside the tests as a verification reference, and a property test checks that both implementations agree on random fronts.

### Implementation notes

- The input may contain dominated or duplicate points (it is reduced to the nondominated set before computing)
- Sorting ascending by the last objective is a heuristic to strengthen limitset pruning; correctness does not depend on the order
- Maximization objectives are converted to minimization by sign flip ($-y$) before computation

---

## References

- L. While, L. Bradstreet, L. Barone, "A Fast Way of Calculating Exact Hypervolumes", IEEE Transactions on Evolutionary Computation, 16(1), 2012.
- E. Zitzler, L. Thiele, "Multiobjective evolutionary algorithms: a comparative case study and the strength Pareto approach", IEEE TEVC, 3(4), 1999. (origin of the HV indicator)
