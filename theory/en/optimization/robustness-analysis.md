# Robustness Analysis

## Overview

An optimum found by an optimizer is a *deterministic* optimum: it assumes the design variables can be realized exactly. In practice, inputs carry uncertainty — manufacturing tolerances, material scatter, fluctuating operating conditions — and a design that sits on a steep ridge of the objective surface may perform far worse than predicted once that scatter is applied. Robustness analysis asks: **how does the predicted performance distribute when the inputs are perturbed around a candidate design?**

Evaluating this directly would require many additional real evaluations around the candidate. Instead, the analysis reuses the **GP surrogate** already fitted to the observed trials, and propagates input uncertainty through it by Monte Carlo sampling. This makes the analysis essentially free (milliseconds) once a surrogate is trained.

---

## Formula

Let $\hat{f}$ be the fitted surrogate of an objective, $\mathbf{x}_0$ the candidate design (e.g. a pinned or best trial), and let the input perturbation be Gaussian and independent per dimension:

$$
\mathbf{x}_i = \mathbf{x}_0 + \boldsymbol{\varepsilon}_i, \qquad
\varepsilon_{ij} \sim \mathcal{N}\!\left(0,\; \sigma_j^2\right), \qquad
\sigma_j = \delta \cdot (u_j - l_j)
$$

where $[l_j, u_j]$ is the declared range of parameter $j$ and $\delta$ is the user-chosen noise level (a fraction of each range, e.g. $\delta = 0.02$ for ±2 %-of-range scatter at 1σ). Samples are clipped to the declared bounds, mirroring the physical constraint that the variable cannot leave its feasible box.

For $N$ Monte Carlo samples the propagated output set is $\{y_i\}$ with

$$
y_i = \hat{f}(\mathbf{x}_i)
\quad\text{(aleatory only)}, \qquad
y_i \sim \mathcal{N}\!\left(\hat{f}(\mathbf{x}_i),\; \hat{s}^2(\mathbf{x}_i)\right)
\quad\text{(aleatory + epistemic)}
$$

The first form propagates only the **input (aleatory) uncertainty** through the surrogate mean. The second additionally draws from the GP posterior at each sample, folding in the **model (epistemic) uncertainty** $\hat{s}^2(\mathbf{x})$ — useful when the candidate lies in a sparsely sampled region where the surrogate itself is unsure.

From $\{y_i\}$ the analysis reports the empirical mean $\bar{y}$, standard deviation $s_y$, and the 5th / 50th / 95th percentiles. The **mean shift** $\bar{y} - \hat{f}(\mathbf{x}_0)$ exposes asymmetry: a design on a one-sided slope degrades on average even though the nominal prediction looks good.

With constraint surrogates $\hat{c}_k$ (trained on the values Optuna stores under the `constraints` convention, feasible iff $c_k \le 0$), the **feasibility rate** is

$$
P_\text{feas} \approx \frac{1}{N} \sum_{i=1}^{N} \mathbb{1}\!\left[\hat{c}_k(\mathbf{x}_i) \le 0 \;\; \forall k\right]
$$

i.e. the fraction of perturbed designs that remain feasible — a Monte Carlo counterpart of the reliability ("six-sigma") measures used in robust design optimization.

---

## Characteristics

- The analysis is only as good as the surrogate. Check the surrogate's cross-validation quality first; a poorly fitted model produces confident-looking but meaningless distributions.
- Clipping at the declared bounds truncates the noise distribution for candidates near a bound; the reported spread is then conditional on staying inside the box.
- Independent Gaussian noise per dimension is an assumption. Correlated tolerances or non-Gaussian scatter are not modeled.
- Gaussian samples are generated with the [Box-Muller transform](../statistics/box-muller.md) on a seeded RNG. Deterministic seeding makes the sample set reproducible: the same candidate, noise level, and sample count always produce the same statistics.
- Comparing the distributions of two candidates (e.g. two pinned trials) is the intended workflow: a slightly worse nominal value with a much tighter distribution is often the better engineering choice.

---

## Where It Is Used in the App

- **Robustness widget**: pick a candidate (best or pinned trial), set the noise level and sample count, and read the output histogram, statistics, and feasibility rate computed on the trained surrogate.
