# additive ε-indicator

## Overview

The unary additive ε-indicator $I_{\varepsilon+}$ measures the smallest translation that, if applied to the approximation set, makes it weakly dominate every point of the reference set. A smaller value (closer to, or below, zero) means the approximation set is closer to, or already dominates, the reference set. Unlike averaging indicators such as IGD+, its value is determined by a **worst case** — the single hardest-to-reach point in the reference set.

In Tunny Dashboard it is used by:

- Multi-objective indicators (MoIndicator::Epsilon)
- The Convergence chart

---

## Definition

For $m$ objectives to minimize, an approximation set $A \subset \mathbb{R}^m$, and a reference set $Z \subset \mathbb{R}^m$,

$$
I_{\varepsilon+}(A, Z) = \max_{z \in Z} \min_{a \in A} \max_j (a_j - z_j)
$$

The inner term $\max_j (a_j - z_j)$ is the smallest $\varepsilon$ such that translating $a$ by $-\varepsilon$ in every dimension makes it weakly dominate $z$ (i.e., the largest dimension along which $a$ is "worse" than $z$). This is minimized over every point $a$ in the approximation set (picking the closest one), and then maximized over every point $z$ in the reference set (determined by the hardest-to-reach reference point).

### Being a worst-case indicator

Because the outer operator is $\max$, $I_{\varepsilon+}$ is determined not by "how close on average" but by "how close to the reference set's single worst-served point." This is the opposite character from IGD+ ([igd-plus.md](igd-plus.md), a mean-based indicator), and makes it sensitive to a front that has a gap or a locally poor region even if it is otherwise well converged.

### Sign

$I_{\varepsilon+}$ can be negative. If the approximation set $A$ **strictly** dominates every point of the reference set $Z$, the translation can be applied in the "reverse" direction (making $a$ even better) while dominance is preserved, giving $\varepsilon < 0$. If $A = Z$, then $I_{\varepsilon+} = 0$.

In Tunny's convergence chart (below), however, the reference set is built as the non-dominated front of the union of observed points across all series, so a series' front can never strictly dominate it — in practice $I_{\varepsilon+} \ge 0$, and reaching $0$ means that series fully covers the reference front.

---

## Application in Tunny

### Self-referential convergence analysis

The true Pareto front is usually unknown, so Tunny fixes the reference set as the **non-dominated front of the union of observed points across all series** (the baseline Study plus comparison Studies) and measures convergence toward it at every trial step, for every series — the same design shared with IGD+ and R2. Sharing the reference set and scale across series lets convergence curves from multiple Studies be compared on the same chart.

### Normalized space

Each objective is first unified to a minimization direction (maximization objectives are sign-flipped), then scaled to $[0, 1]$ using the union's ideal and nadir (for scale invariance). A dimension with a degenerate range uses a scale of 1. The exact formula is shared with [IGD+ — Normalized space](igd-plus.md#normalized-space).

### Edge cases

- Empty reference set: $I_{\varepsilon+} = 0$
- Empty approximation set: $I_{\varepsilon+} = +\infty$
- Invalid points (NaN, infinity, or mismatched dimension count): carries forward the previous value

---

## Related indicators

- [Hypervolume](hypervolume.md) — a quality indicator based on dominated volume.
- [IGD+](igd-plus.md) — a mean-based convergence indicator; the additive ε-indicator is worst-case-based.
- [R2 indicator](r2-indicator.md) — a convergence indicator based on the expectation over weighted utility functions.

---

## References

- E. Zitzler, L. Thiele, M. Laumanns, C. M. Fonseca, V. Grunert da Fonseca, "Performance Assessment of Multiobjective Optimizers: An Analysis and Review", IEEE Transactions on Evolutionary Computation, 7(2), 2003. (defines the additive ε-indicator)
