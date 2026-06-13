# Expected Hypervolume Improvement (EHVI)

EHVI is the multi-objective analog of single-objective acquisition functions. Where Expected Improvement asks "how much would a new point improve the best scalar value?", EHVI asks "how much would a new point grow the **hypervolume** dominated by the Pareto front?". It is the standard acquisition function for multi-objective Bayesian optimization.

In Tunny Dashboard, EHVI is available in the **Surrogate Optimizer** widget in multi-objective mode after Gaussian Process surrogates (GP-FITC, GP-VFE, or GP-MOE) have been fitted for every objective. Click **Suggest next trials (EHVI)** to obtain recommended parameter settings.

---

## Requirement: a Gaussian Process per objective

Each objective $k$ has its **own** independent GP surrogate, exposing a posterior mean and variance. EHVI requires posterior variance for every objective, so all objectives must use a GP variant:

| Model | Supports EHVI |
|-------|---------------|
| GP-FITC | Yes |
| GP-VFE | Yes |
| GP-MOE | Yes |
| Ridge | No |
| LightGBM | No |

The objectives are treated as **independent** given $x$ (each has its own GP; no cross-objective covariance is modeled). This is the same assumption used by qEHVI and most practical EHVI implementations.

---

## Hypervolume

The hypervolume of a set of points $P$ relative to a reference point $r$ is the volume of objective space dominated by $P$ and bounded above by $r$ (minimization convention: a point contributes only where it is strictly below $r$ in **every** dimension). A larger dominated hypervolume means a better, more spread-out Pareto front, so hypervolume is the canonical scalar quality measure for a multi-objective front.

The hypervolume improvement of adding a candidate vector $v$ to the current front $P$ is

$$
\text{HVI}(v) = \max\!\big(0,\; \text{HV}(P \cup \{v\}) - \text{HV}(P)\big).
$$

---

## The z-scored minimization frame

All EHVI mathematics is done in a **z-scored, minimization** frame so that "smaller is always better" and all objectives share a comparable scale.

For objective $k$, define the normalized objective

$$
g_k(x) = \text{sign}_k \cdot \hat{\mu}^{\text{norm}}_k(x),
\qquad
\text{sign}_k =
\begin{cases}
+1 & \text{if objective } k \text{ is minimized} \\
-1 & \text{if objective } k \text{ is maximized}
\end{cases}
$$

where $\hat{\mu}^{\text{norm}}_k$ is the GP posterior mean in z-score units. The sign flip converts maximization into minimization. The posterior standard deviation is

$$
s_k(x) = \sqrt{\widehat{\text{Var}}^{\text{norm}}_k(x)}
$$

(the sign does not affect the standard deviation).

### Observed front $P$

Take the raw observed objective values $y$ per objective, convert each to the z-scored minimization frame $\text{sign}_k \cdot (y - \bar{y}_k)/\sigma_{y,k}$, and reduce to the non-dominated set under the minimization convention.

### Reference point $r$

Per dimension, the reference point is the **nadir** of the observed front plus a small margin:

$$
r_k = \max_{p \in P} g_k(p) + \text{REF\_MARGIN},
\qquad \text{REF\_MARGIN} = 0.1 \;\text{(z-score units)}.
$$

The margin guarantees that every observed front point lies strictly inside the reference box and therefore contributes positive hypervolume.

---

## Monte-Carlo estimator with common random numbers

EHVI has no closed form for more than two or three objectives, so Tunny estimates it by Monte-Carlo. For a candidate $x$, draw $S$ samples of the joint posterior objective vector and average the hypervolume improvement:

$$
\widehat{\text{EHVI}}(x) = \frac{1}{S} \sum_{s=1}^{S}
\max\!\big(0,\; \text{HV}(P \cup \{v_s\}) - \text{HV}(P)\big),
\qquad
v_s[k] = g_k(x) + s_k(x)\, Z[s][k],
$$

where $Z$ is an $S \times n_{\text{obj}}$ matrix of standard-normal values ($S = 128$). $\text{HV}(P)$ is fixed for a given iteration and is computed once.

### Why a fixed sample matrix (common random numbers)

The matrix $Z$ is drawn **once per `suggest_candidates_multi` call** from a fixed-seed RNG (seed 42) and **reused for every evaluation of $x$**. This is the *common random numbers* trick, and it matters for two reasons:

1. **Determinism** — two runs on the same data produce identical suggestions.
2. **Smoothness** — with the noise frozen, $\widehat{\text{EHVI}}(x)$ becomes a deterministic, smooth function of $x$. The optimizer maximizes it with multi-start L-BFGS using numerical (finite-difference) gradients; redrawing the samples at every evaluation would inject noise into the gradient and break the line search.

A sample $v_s$ that is not strictly below $r$ in all dimensions contributes zero (the hypervolume routine handles this), which is exactly the $\max(0, \cdot)$ behavior.

---

## Batch suggestions: Constant Liar

For a batch of $n > 1$ candidates, Tunny uses the **Constant Liar** strategy. After each candidate is selected:

1. Append the candidate's parameters and its per-objective **predicted means** (raw units) as a "lie" observation to a working copy of each objective's $(x, y)$ data.
2. Refit each objective's GP surrogate on the augmented data.
3. Recompute the observed front $P$ and reference point $r$.
4. Re-optimize EHVI for the next candidate.

This discourages the batch from collapsing onto a single point. A normalized-distance dedup guard retries from a random start once if a new candidate coincides with a previous one. If a refit fails midway, the candidates collected so far are returned.

---

## References

- M. Emmerich, A. Deutz, J. Klinkenberg, *Hypervolume-based expected improvement* (EHVI), 2011.
- K. Yang, M. Emmerich, A. Deutz, T. Bäck, *Multi-objective Bayesian global optimization using expected hypervolume improvement gradient*, 2019.
- S. Daulton, M. Balandat, E. Bakshy, *Differentiable Expected Hypervolume Improvement for Parallel Multi-Objective Bayesian Optimization* (qEHVI), NeurIPS 2020.
