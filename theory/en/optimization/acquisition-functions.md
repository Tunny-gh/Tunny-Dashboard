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

## References

- Jones, D. R., Schonlau, M., & Welch, W. J. (1998). Efficient global optimization of expensive black-box functions. *Journal of Global Optimization*, 13, 455–492.
- Srinivas, N., Krause, A., Kakade, S. M., & Seeger, M. (2010). Gaussian process optimization in the bandit setting: No regret and experimental design. *ICML*.
- Ginsbourger, D., Le Riche, R., & Carraro, L. (2010). Kriging is well-suited to parallelize optimization. *Computational Intelligence in Expensive Optimization Problems*, 131–162.
