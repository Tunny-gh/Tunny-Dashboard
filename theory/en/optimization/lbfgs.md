# L-BFGS — Limited-memory BFGS

## Overview

L-BFGS (Limited-memory Broyden–Fletcher–Goldfarb–Shanno) is a quasi-Newton optimizer that approximates the inverse Hessian from the last m gradient/parameter difference pairs rather than storing the full matrix. In Tunny Dashboard it is used in the **surrogate optimizer** stage (searching the fitted GP surface for the optimal parameter values).

| Method        | Memory  | Convergence        | Hessian info               |
| ------------- | ------- | ------------------ | -------------------------- |
| Gradient descent | O(p) | Slow (1000+ iters) | None                       |
| BFGS          | O(p²)   | Fast (30–100 iters)| Full inverse Hessian       |
| **L-BFGS**    | **O(mp)**| Fast (30–100 iters)| Last m steps only (m = 5)  |

p = number of parameters, m = history size.

## Quasi-Newton Foundation

Instead of computing the true Hessian, use gradient differences to approximate it:

$$
s_k = x_{k+1} - x_k \quad \text{(parameter step)}
$$

$$
y_k = \nabla f_{k+1} - \nabla f_k \quad \text{(gradient step)}
$$

$$
\rho_k = \frac{1}{y_k^\top s_k}
$$

This secant condition approximates curvature without forming H explicitly.

## Two-Loop Recursion

Computes the search direction d = −H_k⁻¹ · ∇f_k using only {s_i, y_i, ρ_i}:

```
q ← ∇f_k

// First loop (newest first)
for i = k−1, k−2, ..., k−m:
    α_i = ρ_i · (s_iᵀ q)
    q   = q − α_i · y_i

// Initial scaling
γ = (s_{k-1}ᵀ y_{k-1}) / (y_{k-1}ᵀ y_{k-1})
r ← γ · q

// Second loop (oldest first)
for i = k−m, ..., k−1:
    β_i = ρ_i · (y_iᵀ r)
    r   = r + (α_i − β_i) · s_i

d = −r
```

Cost: O(mp) vs O(p²) for full BFGS.

## Line Search: Moré-Thuente (Strong Wolfe)

Find step size α satisfying both the sufficient decrease (Armijo) condition and the curvature condition (strong Wolfe conditions):

$$
f(x_k + \alpha \cdot d_k) \leq f(x_k) + c_1 \cdot \alpha \cdot \nabla f_k^\top \cdot d_k \quad (c_1 = 10^{-4})
$$

$$
|\nabla f(x_k + \alpha \cdot d_k)^\top d_k| \leq c_2 \cdot |\nabla f_k^\top d_k|
$$

The argmin library uses the Moré-Thuente algorithm, which satisfies both conditions via a cubic interpolation bracket-and-zoom scheme rather than simple backtracking.

## Application in the Surrogate Optimizer

L-BFGS is used in the **surrogate optimizer** stage: it searches the fitted response surface (GP-FITC, GP-VFE, GP-MOE, or Ridge) for the parameter values that minimize or maximize the predicted objective. Numerical gradients are used (central finite differences) so the same optimizer works for any surrogate model.

Note: GP hyperparameter optimization (fitting σ_f, l_d, σ_n) is handled internally by egobox-gp using COBYLA, not L-BFGS.

**Convergence**: gradient norm < 1e-5 or max iterations reached (max_iters = 100).

**Bounds handling**: box constraints [0,1]^d are enforced via a quadratic exterior penalty added to the cost function (weight = 1e3), not via projection. Numerical gradients (central finite differences, step = 1e-4) are used since the surrogate is a black box.

## References

- Liu & Nocedal (1989). On the limited memory BFGS method for large scale optimization. *Mathematical Programming*, 45, 503–528.
- Nocedal & Wright (2006). *Numerical Optimization* (2nd ed.), Chapter 7. Springer.
