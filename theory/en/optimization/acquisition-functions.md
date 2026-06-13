# Acquisition Functions

Acquisition functions are the core of Bayesian optimization: they combine a surrogate model's predictions with a measure of uncertainty to decide *where* to evaluate the expensive objective next.

In Tunny Dashboard, acquisition functions are available in the **Surrogate Optimizer** widget after a Gaussian Process surrogate (GP-FITC, GP-VFE, or GP-MOE) has been fitted. Click **Suggest next trials** to obtain recommended parameter settings.

---

## Requirement: Gaussian Process posterior variance

Acquisition functions require the surrogate to expose a posterior variance, i.e. to output not just a predicted mean μ(x) but also an uncertainty σ(x). Only the three GP variants satisfy this:

| Model | Supports acquisition functions |
|-------|-------------------------------|
| GP-FITC | Yes |
| GP-VFE | Yes |
| GP-MOE | Yes |
| Ridge | No |
| LightGBM | No |

---

## Implemented acquisition functions

All mathematics operates in **normalized space**: x ∈ [0, 1]^d and y in z-score units. Results are converted back to original units before display.

### Expected Improvement (EI)

EI measures the expected amount by which a new point x would improve on the current best observation f* (called the incumbent). It balances exploitation (μ close to f*) with exploration (large σ).

For a minimization problem:

$$
\text{EI}(x) = I \cdot \Phi(z) + \sigma(x) \cdot \phi(z)
$$

where:
- I = f\* − μ(x) − ξ  (improvement gap with exploration offset ξ = 0.01)
- z = I / σ(x)
- Φ = standard normal CDF, φ = standard normal PDF

When σ(x) < 10⁻¹² (deterministic region), EI = max(I, 0).

For a maximization problem, μ and f\* are sign-flipped so the formula is identical.

**Exploration offset**: ξ = **0.01** (in z-score units). Increasing ξ favours exploration; decreasing it increases exploitation.

### Lower Confidence Bound (LCB)

LCB (also called UCB — Upper Confidence Bound — in the maximization literature) selects the point with the lowest lower bound on the objective:

$$
\text{LCB}(x) = \mu(x) - \kappa \cdot \sigma(x)
$$

For maximization, the sign is flipped so the optimizer seeks the highest upper bound.

**Exploration weight**: κ = **2.0**. Larger κ encourages more exploration.

---

## Batch acquisition: Constant Liar strategy

When requesting n > 1 candidates simultaneously, Tunny uses the **Constant Liar** algorithm:

1. Optimize the acquisition function on the current surrogate → candidate c₁.
2. Append (c₁, y_lie) to the training data, where y_lie = best observed objective value so far (minimum when minimizing, maximum when maximizing).
3. Refit the GP surrogate on the augmented data.
4. Optimize the acquisition function on the new surrogate → candidate c₂.
5. Repeat until n candidates have been collected.

The "lie" makes the GP artificially confident around already-selected candidates, encouraging the next candidates to explore other regions. If a mid-batch refit fails, the candidates collected up to that point are returned.

**Diversity guard**: if a new candidate falls within L2 distance 10⁻⁶ of an existing one (in normalized space), the optimizer is restarted from a different random point.

---

## Using the exported JSON with Optuna

The **Copy enqueue JSON** button copies a JSON array to the clipboard in the format expected by `study.enqueue_trial()`:

```json
[
  {"x": 1.5, "y": 2.0},
  {"x": 0.8, "y": 3.1}
]
```

Each object maps parameter names to their suggested values. In Python:

```python
import json, optuna

study = optuna.load_study(...)
candidates = json.loads("<paste from clipboard>")
for params in candidates:
    study.enqueue_trial(params)
```

The enqueued trials will be sampled next by any Optuna sampler that respects the trial queue (all built-in samplers do).

---

## Constraint-aware acquisition functions

When the study has constraint columns and **Use constraints** is enabled, the acquisition functions are modified to account for feasibility.

### Feasibility probability P_feas(x)

Each constraint model predicts the constraint signal at the candidate point. When the model is a Gaussian Process (posterior mean μᵢ and standard deviation σᵢ, in normalized space), the feasibility probability is smooth:

$$
P(c_i \le 0 \mid x) = \Phi\!\left(\frac{z_{0,i} - \mu_i(x)}{\sigma_i(x)}\right), \qquad z_{0,i} = \frac{0 - \bar{c}_i}{s_{c_i}}
$$

where z₀ is the feasibility boundary (cᵢ = 0) expressed in the constraint's z-scored space (c̄ᵢ, s_cᵢ are its mean and standard deviation). For a deterministic model without posterior variance (Ridge, or a GP that fell back to Ridge — see below), a hard indicator is used instead:

$$
P(c_i \le 0 \mid x) = \begin{cases} 1 & \text{if } \tilde{c}_i(x) \le 0 \\ 0 & \text{otherwise} \end{cases}
$$

The overall feasibility probability assumes independence across constraints:

$$
P_\text{feas}(x) = \prod_i P(c_i \le 0 \mid x)
$$

### Constrained EI

$$
\text{EI}_c(x) = \text{EI}(x) \cdot P_\text{feas}(x)
$$

The incumbent f* is taken from the **best feasible trial** (all constraint values ≤ 0). If no feasible trial exists, the global best is used as fallback (Gardner et al., 2014).

### Constrained LCB

$$
\text{LCB}_c(x) = \text{LCB}(x) + \lambda \cdot (1 - P_\text{feas}(x))
$$

where λ = **10.0** is the infeasibility penalty. The penalty pushes the optimizer away from regions predicted to be infeasible.

### Constraint Constant Liar

In batch acquisition, the constraint models are also refitted at each Constant Liar iteration. The "lie" values for constraint columns are the predicted constraint means at the previously selected candidate:

$$
c_i^\text{lie} = \tilde{c}_i(\mathbf{x}_\text{prev})
$$

This preserves constraint information in the augmented training data.

### Constraint model

Constraint surrogates use the **same model kind as the objective**, so a GP objective yields a smooth feasibility probability that accounts for uncertainty near the constraint boundary. A perfectly linear, noise-free constraint (e.g. `c = 0.5 − x`) is a degenerate case for GP hyperparameter optimization (the optimal length scale tends to infinity), which can make the fit fail; in that case the affected constraint **falls back to Ridge regression** and uses the hard indicator above, while other constraints keep their smooth GP feasibility. Objectives without posterior variance (Ridge, LightGBM) use the hard indicator directly.

---

## References

- Jones, D. R., Schonlau, M., & Welch, W. J. (1998). Efficient global optimization of expensive black-box functions. *Journal of Global Optimization*, 13, 455–492.
- Srinivas, N., Krause, A., Kakade, S. M., & Seeger, M. (2010). Gaussian process optimization in the bandit setting: No regret and experimental design. *ICML*.
- Ginsbourger, D., Le Riche, R., & Carraro, L. (2010). Kriging is well-suited to parallelize optimization. *Computational Intelligence in Expensive Optimization Problems*, 131–162.
- Gardner, J. R., Kusner, M. J., Xu, Z. E., Weinberger, K. Q., & Cunningham, J. P. (2014). Bayesian optimization with inequality constraints. *ICML*.
