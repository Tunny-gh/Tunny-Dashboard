# NSGA-II (Non-dominated Sorting Genetic Algorithm II)

## Overview

NSGA-II (Deb, Pratap, Agarwal, Meyarivan, 2002) is a derivative-free, population-based evolutionary optimization algorithm. A population of candidate solutions (individuals) is evolved generation by generation through crossover and mutation, and the next generation is selected using Pareto-dominance ranking combined with a diversity-preserving crowding measure. The same procedure works for both single-objective and multi-objective problems, which is its defining strength.

In Tunny Dashboard it is used in the **surrogate optimizer** stage. In single-objective mode it is one of the selectable optimization methods; in multi-objective mode it is used automatically to compute the predicted Pareto front, independent of the user's optimizer selection. The search space is always the normalized $[0,1]^d$ box.

| Method | Gradient info | Population/individual | Multi-objective | Robustness to multimodality |
| --- | --- | --- | --- | --- |
| L-BFGS | Uses numerical gradient | Single individual (multi-start) | Not supported (single-objective only) | Prone to local optima |
| CMA-ES | Not required (distribution adaptation) | Population (Gaussian sampling) | Not supported (single-objective only) | Moderate (leans toward unimodal design) |
| **NSGA-II** | **Not required** | **Population ($n$ individuals)** | **Supported (front computation)** | **High (population-based search)** |

---

## Fast Non-dominated Sort

Assuming all objectives are minimized, individual $a$ **Pareto-dominates** individual $b$ when:

$$
a \prec b \iff \forall k,\ f_k(a) \le f_k(b) \ \land\ \exists k,\ f_k(a) < f_k(b)
$$

The population is decomposed into fronts $F_1, F_2, \dots$ based on this dominance relation. $F_1$ is the set of individuals dominated by no one (the non-dominated solutions); $F_2$ is the set of individuals that are non-dominated once $F_1$ is removed; and so on.

The algorithm builds, for every individual $i$ via an all-pairs comparison in $O(N^2)$, the set of individuals $i$ dominates and the number of individuals that dominate $i$ (the domination count). $F_1$ consists of the individuals whose domination count is 0. For each individual in $F_1$, the domination count of the individuals it dominates is decremented by 1; individuals whose count drops to 0 are registered into $F_2$, and this propagation repeats. With $M$ objectives, the total cost is $O(M N^2)$.

---

## Crowding Distance

A measure of diversity within a single front. For an individual $i$ in a front, values are sorted per objective $m$, and the normalized gap between neighboring individuals is accumulated:

$$
d_i = \sum_{m=1}^{M} \frac{f_m(i+1) - f_m(i-1)}{f_m^{\max} - f_m^{\min}}
$$

The two **boundary individuals** at each objective's sort extremes get $d_i = +\infty$ and are preserved with priority. A larger crowding distance means the individual's neighborhood on that front is sparser (i.e., it is more isolated from other solutions), so it is preferred when preserving diversity.

---

## Binary Tournament Selection via Crowded Comparison

Parent selection uses binary tournament selection. Two individuals are picked at random, and the winner is decided by the following priority (the crowded comparison operator):

1. Prefer the individual with the smaller rank (front number).
2. If ranks are tied, prefer the individual with the larger crowding distance.

This favors individuals that are better in terms of dominance while preserving population diversity among equally ranked individuals.

---

## SBX Crossover (Simulated Binary Crossover)

SBX, introduced by Deb & Agrawal (1995), is a crossover operator for real-valued genes that mimics the behavior of binary crossover (children tend to appear near their parents, but occasionally far away). For pairs that satisfy the pair-level crossover probability $p_c$ (Tunny uses $0.9$), $\beta$ is sampled independently for **every variable** and used to blend the parents:

$$
\beta =
\begin{cases}
(2u)^{\frac{1}{\eta_c+1}} & u \le 0.5 \\[4pt]
\left(\dfrac{1}{2(1-u)}\right)^{\frac{1}{\eta_c+1}} & u > 0.5
\end{cases}
\qquad u \sim \mathrm{Uniform}(0,1)
$$

$$
c_1 = \tfrac{1}{2}\big[(1+\beta)x_1 + (1-\beta)x_2\big], \qquad
c_2 = \tfrac{1}{2}\big[(1-\beta)x_1 + (1+\beta)x_2\big]
$$

The distribution index $\eta_c$ controls the shape of the $\beta$ distribution. A larger $\eta_c$ concentrates $\beta$ near 1, so children tend to appear near their parents (**local exploitation**). A smaller $\eta_c$ spreads the $\beta$ distribution out, making children farther from their parents more likely (**global exploration**).

**Tunny-specific design**: single-objective mode uses $\eta_c = 20$ (`SBX_ETA_SINGLE_OBJECTIVE`) to prioritize local refinement near the best-known solution. Multi-objective mode uses $\eta_c = 2$ (`SBX_ETA_MULTI_OBJECTIVE`) to make children stray farther from their parents, covering the full extent of the Pareto front (`Nsga2Config::for_objectives` selects automatically based on the objective count). The rationale: single-objective search wants to dig deeper around one best value, while multi-objective search needs to reproduce the shape of the entire front.

---

## Polynomial Mutation

Each gene (dimension) mutates independently with probability $1/d$ ($d$ = number of dimensions). The mutation magnitude $\delta$ is:

$$
\delta =
\begin{cases}
(2u)^{\frac{1}{\eta_m+1}} - 1 & u < 0.5 \\[4pt]
1 - \big(2(1-u)\big)^{\frac{1}{\eta_m+1}} & u \ge 0.5
\end{cases}
\qquad u \sim \mathrm{Uniform}(0,1)
$$

The new gene value is $x' = \mathrm{clamp}(x + \delta,\ 0,\ 1)$. Tunny uses $\eta_m = 20$. As with $\eta_c$, a larger $\eta_m$ produces smaller, more local perturbations.

---

## Elitist Environmental Selection

The core generational-replacement rule of NSGA-II. The parent population $P_t$ ($n$ individuals) and offspring population $Q_t$ ($n$ individuals) are simply merged into $R_t = P_t \cup Q_t$ ($2n$ individuals), which is decomposed into fronts $F_1, F_2, \dots$ via non-dominated sorting. Fronts are added to the next generation $P_{t+1}$ in order (starting from $F_1$), taking each front in full up until adding the next one would exceed $n$ individuals. The last front that would overflow $n$ is truncated by taking as many individuals as needed **in descending order of crowding distance**.

This guarantees elitism (good individuals from the parent generation are never lost to the offspring generation) while preserving diversity within each front.

---

## Application in the Surrogate Optimizer

NSGA-II is used in the **surrogate optimizer** stage to search the fitted response surface (GP-FITC, GP-VFE, GP-MOE, Ridge, or LightGBM) for optimal parameter values. Because it requires no numerical gradient, it is robust to multimodal or discontinuous surfaces (including piecewise-constant models like LightGBM) that gradient-based methods (L-BFGS) tend to struggle with.

- In single-objective mode, selecting **NSGA-II** searches from a population seeded with the observed best point, using the locally-exploitative setting ($\eta_c=20$). It serves as an effective alternative when L-BFGS gets stuck in a local optimum on a multimodal surface.
- In multi-objective mode, NSGA-II is always used regardless of the user's selection, and computes the predicted Pareto front ($\eta_c=2$, covering the full front).
- The distinction from CMA-ES is population-based search: CMA-ES adapts a single search distribution, whereas NSGA-II lets multiple individuals in a population independently form a front, making it well-suited for multi-objective optimization and strongly multimodal surfaces.

### Implementation Parameters

| Item | Value |
| --- | --- |
| Population size `pop_size` | 64 (rounded up to even, minimum 4) |
| Generations `generations` | 120 |
| Crossover probability `crossover_prob` | 0.9 (per pair) |
| SBX distribution index $\eta_c$ | 20 for single-objective / 2 for multi-objective |
| Polynomial Mutation distribution index $\eta_m$ | 20 |
| Mutation probability (per gene) | $1/d$ |
| Random seed `seed` | 42 (deterministic) |
| Initial population | Seeded with the observed best individual, rest randomly generated |
| Boundary handling | Children are clamped to $[0,1]$ |
| Return value | The final generation's first front (a list of `(genome, fitness)` pairs) |

For single-objective calls, fitness is treated as a length-1 vector via the generic implementation (`nsga2_minimize`), and the individual with the smallest fitness in the first front is chosen as the final solution. For multi-objective calls, the entire first front is returned as the predicted Pareto front.

---

## References

- Deb, K., Pratap, A., Agarwal, S., & Meyarivan, T. (2002). A fast and elitist multiobjective genetic algorithm: NSGA-II. *IEEE Transactions on Evolutionary Computation*, 6(2), 182–197.
- Deb, K., & Agrawal, R. B. (1995). Simulated binary crossover for continuous search space. *Complex Systems*, 9(2), 115–148.

## Related Documents

- [L-BFGS](lbfgs.md) — the gradient-based method in the surrogate optimizer.
- [CMA-ES](cma-es.md) — the distribution-adaptation method in the surrogate optimizer.
- [Surrogate Optimizer (widget)](../widgets/surrogate-optimizer.md) — the usage context for this method.
