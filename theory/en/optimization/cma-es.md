# CMA-ES (Covariance Matrix Adaptation Evolution Strategy)

## Overview

CMA-ES (Covariance Matrix Adaptation Evolution Strategy) is a derivative-free method for single-objective, continuous optimization. Candidate solutions are sampled from a multivariate normal distribution $\mathcal{N}(m, \sigma^2 C)$, and the distribution's mean $m$, step size $\sigma$, and covariance matrix $C$ are adapted generation by generation based on the evaluation results. Because the covariance matrix learns the local curvature of the problem (correlations and scale differences between variables), it is robust on ill-conditioned (axis-skewed) objective functions.

Tunny Dashboard's implementation follows the standard form from Hansen's tutorial (*The CMA Evolution Strategy: A Tutorial*).

| Method | Gradient info | Unit of search | Robustness to conditioning |
| --- | --- | --- | --- |
| L-BFGS | Uses numerical gradient | Single point (multi-start) | Exploits curvature (Hessian approximation) |
| NSGA-II | Not required | Population ($n$ individuals, genetic operators) | Axis-wise perturbation only (does not learn correlations) |
| **CMA-ES** | **Not required** | **A single adaptive distribution** ($\lambda$ samples/generation) | **Covariance matrix learns correlations and scale** |

---

## Sampling and Eigendecomposition

At each generation, the covariance matrix $C$ is eigendecomposed:

$$
C = B D^2 B^\top
$$

$B$ is the orthogonal matrix of eigenvectors, and $D = \mathrm{diag}(d_1, \dots, d_n)$ holds the square roots of the eigenvalues. Candidates are sampled as:

$$
z \sim \mathcal{N}(0, I), \qquad y = B D z, \qquad x = m + \sigma y
$$

Standard normal samples $z$ are generated via the Box-Muller method (see [Box-Muller Method](../statistics/box-muller.md) for details).

Tunny's implementation uses faer's **symmetric (self-adjoint) eigendecomposition**. Since numerical error can make $C$ not exactly symmetric, $(C + C^\top)/2$ is taken before decomposition to guarantee symmetry, and eigenvalues are clamped to $\max(\lambda, 10^{-20})$ before taking the square root, to guard against numerical error producing negative eigenvalues.

---

## Strategy Parameters (Hansen's Recommended Values)

Let $n$ be the number of dimensions.

**Population size and weights**:

$$
\lambda = 4 + \lfloor 3 \ln n \rfloor, \qquad \mu = \left\lfloor \frac{\lambda}{2} \right\rfloor
$$

The top $\mu$ individuals receive logarithmic weights (normalized to sum to 1):

$$
w_i \propto \ln\!\left(\frac{\lambda+1}{2}\right) - \ln(i), \qquad i = 1, \dots, \mu
$$

Effective sample size:

$$
\mu_{\mathrm{eff}} = \frac{1}{\sum_i w_i^2}
$$

**Step-size adaptation (CSA) parameters**:

$$
c_\sigma = \frac{\mu_{\mathrm{eff}} + 2}{n + \mu_{\mathrm{eff}} + 5}, \qquad
d_\sigma = 1 + 2\max\!\left(0,\ \sqrt{\frac{\mu_{\mathrm{eff}}-1}{n+1}} - 1\right) + c_\sigma
$$

**Covariance adaptation parameters**:

$$
c_c = \frac{4 + \mu_{\mathrm{eff}}/n}{n + 4 + 2\mu_{\mathrm{eff}}/n}, \qquad
c_1 = \frac{2}{(n+1.3)^2 + \mu_{\mathrm{eff}}}, \qquad
c_\mu = \min\!\left(1 - c_1,\ \frac{2(\mu_{\mathrm{eff}} - 2 + 1/\mu_{\mathrm{eff}})}{(n+2)^2 + \mu_{\mathrm{eff}}}\right)
$$

All of these are Hansen's tutorial recommendations, and Tunny's implementation (`cma_es.rs`) adopts them directly.

---

## Mean Update

$\lambda$ candidates are evaluated and sorted by ascending cost, and the top $\mu$ values of $y_k$ (the $y_k$ in $x_k = m + \sigma y_k$) are combined with a weighted average:

$$
y_w = \sum_{i=1}^{\mu} w_i\, y_{(i)}, \qquad m \leftarrow m + \sigma\, y_w
$$

$y_{(i)}$ is the $y$ of the individual ranked $i$-th by cost.

---

## Step-size Path and CSA Update

The step-size path $p_\sigma$ is updated using $C^{-1/2} y_w = B D^{-1} B^\top y_w$:

$$
p_\sigma \leftarrow (1 - c_\sigma)\, p_\sigma + \sqrt{c_\sigma(2-c_\sigma)\mu_{\mathrm{eff}}}\ \, C^{-1/2} y_w
$$

Step-size update (CSA, Cumulative Step-size Adaptation):

$$
\sigma \leftarrow \sigma \cdot \exp\!\left(\frac{c_\sigma}{d_\sigma}\left(\frac{\lVert p_\sigma \rVert}{E\lVert \mathcal{N}(0,I) \rVert} - 1\right)\right)
$$

$E\lVert \mathcal{N}(0,I) \rVert$ (the expected norm of a standard normal vector) uses the following approximation:

$$
E\lVert \mathcal{N}(0,I) \rVert \approx \sqrt{n}\left(1 - \frac{1}{4n} + \frac{1}{21n^2}\right)
$$

When $\lVert p_\sigma \rVert$ exceeds this expected value (i.e., steps are cumulatively moving in a consistent direction), the step size is increased; when it is smaller (i.e., steps are zigzagging and canceling out), the step size is decreased.

---

## Covariance Path and Rank-one + Rank-$\mu$ Update

The covariance path $p_c$ update uses a Heaviside-type stall indicator $h_\sigma$:

$$
h_\sigma = \begin{cases} 1 & \dfrac{\lVert p_\sigma \rVert}{\sqrt{1-(1-c_\sigma)^{2(g+1)}}} < \left(1.4 + \dfrac{2}{n+1}\right) E\lVert \mathcal{N}(0,I) \rVert \\[6pt] 0 & \text{otherwise} \end{cases}
$$

($g$ is the current generation number; the denominator is a normalization term correcting for the initialization bias of $p_\sigma$.) $h_\sigma$ stops the covariance path update during phases where the step size $\sigma$ is increasing rapidly, preventing excessive inflation of $C$.

$$
p_c \leftarrow (1 - c_c)\, p_c + h_\sigma \sqrt{c_c(2-c_c)\mu_{\mathrm{eff}}}\ \, y_w
$$

The covariance matrix $C$ is updated by combining a rank-one update (along the $p_c$ direction) with a rank-$\mu$ update (variance of the top $\mu$ individuals):

$$
C \leftarrow (1 - c_1 - c_\mu)\, C + c_1\left(p_c p_c^\top + \delta(h_\sigma)\, C\right) + c_\mu \sum_{i=1}^{\mu} w_i\, y_{(i)} y_{(i)}^\top
$$

$\delta(h_\sigma) = (1 - h_\sigma)\, c_c (2 - c_c)$ is the correction term (a standard correction from Hansen's tutorial) that compensates for the variance lost by the rank-one update when $h_\sigma = 0$.

Tunny's implementation computes only the upper triangle and mirrors it to the lower triangle, strictly preventing asymmetry from numerical error.

---

## Application in the Surrogate Optimizer

CMA-ES is used in the **surrogate optimizer** stage for single-objective optimization on the fitted response surface. Since it requires no numerical gradient and adapts the entire sampling distribution as it searches, it depends less on the initial point than L-BFGS's local gradient following, and converges reliably even when the response surface has mild multimodality.

- **Versus L-BFGS**: L-BFGS performs local search via numerical gradients and multi-start, converging in fewer evaluations when the surface is smooth and strongly unimodal. CMA-ES needs somewhat more evaluations since it adapts an entire distribution, but is robust when the surface has mild multimodality or skewed scale/correlation.
- **Versus NSGA-II**: NSGA-II lets individuals in a population search independently, making it suited to strong multimodality or multi-objective front computation, whereas CMA-ES concentrates information into a single adaptive distribution and tends to converge faster on unimodal-to-mildly-multimodal continuous optimization.
- **Versus Random Search**: Random Search is a robust baseline that always works but never adapts its sampling distribution. CMA-ES tends to reach a higher-precision solution within the same evaluation budget.

CMA-ES currently does not support multi-objective mode (single-objective only).

### Implementation Parameters

| Item | Value |
| --- | --- |
| Initial step size $\sigma_0$ | 0.3 (standard value assuming a $[0,1]^d$ box) |
| Maximum generations | $\min(100 + 20n,\ 500)$ when configured as 0 ($n$ = dimensions) |
| Random seed | 42 (deterministic) |
| Standard normal generation | Box-Muller method (see [Box-Muller Method](../statistics/box-muller.md)) |
| Stopping condition | $\sigma$ becomes non-finite, or $\sigma \cdot \max_i d_i < 10^{-9}$ |
| Eigenvalue clamping | $\max(\lambda, 10^{-20})$ (numerical safeguard) |
| Return value | Best-ever evaluated point (the minimum-cost point across all samples in all generations) |

The initial mean $m$ is the observed best point (normalized coordinates).

---

## References

- Hansen, N. (2016). The CMA Evolution Strategy: A Tutorial. *arXiv:1604.00772*.
- Hansen, N., & Ostermeier, A. (2001). Completely derandomized self-adaptation in evolution strategies. *Evolutionary Computation*, 9(2), 159–195.

## Related Documents

- [Box-Muller Method](../statistics/box-muller.md) — how standard normal samples are generated.
- [L-BFGS](lbfgs.md) — the gradient-based method in the surrogate optimizer.
- [NSGA-II](nsga2.md) — the population-based method in the surrogate optimizer.
- [Surrogate Optimizer (widget)](../widgets/surrogate-optimizer.md) — the usage context for this method.
