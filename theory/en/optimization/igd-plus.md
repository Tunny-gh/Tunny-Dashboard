# IGD+ (Inverted Generational Distance Plus)

## Overview

IGD+ measures how well an approximation set (a Pareto front) converges to a reference set. It averages the "modified distance" from each point of the reference set $Z$ to the nearest point in the approximation set $A$. A smaller value means the approximation set is closer to (has better converged to) the reference set.

It is a modified version of the classical IGD (Inverted Generational Distance) that replaces the Euclidean distance with a distance $d^+$ restricted to the dominated direction only.

In Tunny Dashboard it is used by:

- Multi-objective indicators (MoIndicator::IgdPlus)
- The Convergence chart

---

## Definition

For $m$ objectives to minimize, an approximation set $A \subset \mathbb{R}^m$, and a reference set $Z \subset \mathbb{R}^m$,

$$
\mathrm{IGD}^+(A) = \frac{1}{|Z|} \sum_{z \in Z} \min_{a \in A} d^+(a, z)
$$

$$
d^+(a, z) = \sqrt{\sum_j \max(a_j - z_j, 0)^2}
$$

$d^+$ only includes in the sum of squares the dimensions where $a$ is worse (larger) than $z$. If $a$ is better than (or equal to) $z$ in every dimension, $d^+(a, z) = 0$. In other words, an approximation point on the "good" side of a reference point contributes zero to that reference point's distance.

### Difference from IGD

Classical IGD uses the ordinary Euclidean distance $d(a, z) = \|a - z\|$. Because of this, even when $A$ dominates $Z$, the distance stays positive as long as $A \ne Z$, so the indicator can still worsen. This inconsistency means IGD is **not Pareto compliant**.

IGD+ uses $d^+$ instead, which makes it a **weakly Pareto compliant** indicator: it cannot worsen as long as $A$ (weakly) dominates $Z$ (Ishibuchi et al. 2015). When used as a convergence indicator, IGD+ is therefore the theoretically sounder choice over IGD.

---

## Application in Tunny

### Self-referential convergence analysis

The true Pareto front is usually unknown, so measuring convergence even for a single Study requires an approximation. This implementation fixes the reference set as the **non-dominated front of the union of observed points across all series** being compared (the baseline Study plus every added comparison Study), and measures IGD+ convergence toward it at every trial step, for every series.

Because the reference set and the scale (below) are shared across all series, convergence curves from multiple Studies can be compared directly on the same chart.

### Normalized space

Each objective is first unified to a minimization direction (maximization objectives are sign-flipped, $-y$), then scaled to $[0, 1]$ using the union's ideal (best value per objective) and nadir (worst value):

$$
\hat{y}_j = \frac{y_j - \mathrm{ideal}_j}{\mathrm{nadir}_j - \mathrm{ideal}_j}
$$

This makes the indicator invariant to the scale of the objective values. If an objective's range is degenerate (all points equal, so $\mathrm{nadir}_j = \mathrm{ideal}_j$), that dimension's scale is set to 1.

### Edge cases

- Empty reference set: IGD+ = 0
- Empty approximation set (the front accumulated so far): IGD+ = $+\infty$
- Invalid points (containing NaN/infinity, or with a mismatched dimension count): that trial step carries forward the previous value, so the convergence curve is never interrupted.

---

## Related indicators

- [Hypervolume](hypervolume.md) — a quality indicator based on dominated volume. Needs only a reference point, not a reference set.
- [additive ε-indicator](epsilon-indicator.md) — a worst-case (max-based) convergence indicator; IGD+ is mean-based.
- [R2 indicator](r2-indicator.md) — a convergence indicator based on the expectation over weighted utility functions.

---

## References

- H. Ishibuchi, H. Masuda, Y. Tanigaki, Y. Nojima, "Modified Distance Calculation in Generational Distance and Inverted Generational Distance", EMO 2015. (proposes IGD+ and proves weak Pareto compliance)
- C. A. Coello Coello, M. R. Sierra, "A Study of the Parallelization of a Coevolutionary Multi-Objective Evolutionary Algorithm", MICAI 2004. (one of the origins of IGD)
