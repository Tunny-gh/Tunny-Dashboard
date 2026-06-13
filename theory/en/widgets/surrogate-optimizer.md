# Surrogate Optimizer

The surrogate optimizer fits a response surface (surrogate model) to the sampled trials and then optimizes on that surface to estimate the optimal parameter values — without running any additional trials.

## Workflow

1. Select the objective, the surrogate model, and the optimization method.
2. Click **Run Optimization**. The surrogate is trained on all completed trials and the optimizer searches the surface within the sampled parameter ranges.
3. The estimated optimum is shown as a parameter table together with the predicted objective value, and is marked on a 2D slice of the response surface through the optimum.

## Surrogate models

- **GP-FITC** — Gaussian process regression (Kriging) with ARD Matérn 5/2 kernel backed by egobox-gp. FITC sparse approximation with M = min(N, 100) inducing points; trains on all trials. Provides predictive uncertainty ($\pm 1.96\sigma$). Default GP choice.
- **GP-VFE** — Same architecture as GP-FITC but uses the Variational Free Energy bound instead of FITC likelihood. Produces a slightly smoother, more conservative fit; recommended when GP-FITC surface looks overfit or spiky.
- **GP-MOE** — Mixture-of-experts GP via egobox-moe. Clusters the input space with a Gaussian Mixture Model and trains one FITC expert per cluster (up to 3, selected by cross-validation on ≤ 500 points). Best for discontinuous or regime-switching objectives. If training fails, an error is reported rather than silently falling back.
- **Ridge** — Linear ridge regression. Fast baseline; the surface is a plane, so the optimum always lies on the boundary of the sampled ranges.

## Optimization methods

- **Multi-start L-BFGS** — Gradient-based local search (numerical gradients) started from the best observed trial and several random points; the best converged point is reported.
- **NSGA-II** — Genetic algorithm with SBX crossover, polynomial mutation and binary tournament selection (crowded comparison). Population-based and derivative-free, robust on multimodal surfaces. Currently applied to the single selected objective; the implementation supports multiple objectives for future Pareto-front optimization.
- **CMA-ES** — Covariance Matrix Adaptation Evolution Strategy. Derivative-free search that adapts the sampling distribution to the local curvature of the surface; a strong default for continuous problems.
- **Random Search** — Evaluates the surrogate at thousands of random points and picks the best. A robust baseline.

## Suggesting next trials (acquisition functions)

After fitting a GP surrogate, the **Suggest next trials** section appears below the optimization results. It uses an acquisition function to recommend parameter settings for the next real evaluations.

- **EI (Expected Improvement)** — balances predicted improvement and uncertainty (default).
- **LCB (Lower Confidence Bound)** — selects points where the lower bound on the objective is smallest.

Batch candidates (up to 10) are generated with the **Constant Liar** strategy: each selected candidate is temporarily added as a "lie" observation before refitting and selecting the next one.

The **Copy enqueue JSON** button copies the suggested parameters as a JSON array for use with Optuna's `study.enqueue_trial()`.

See [Acquisition Functions](../optimization/acquisition-functions.md) for the full mathematical description.

## Notes

- The search is constrained to the hyper-box spanned by the sampled parameter values; the surrogate is not trusted to extrapolate beyond the data.
- The objective direction (minimize / maximize) is taken from the study metadata.
- $R^2$ reports how well the surrogate fits the training data. A low $R^2$ means the estimated optimum should not be trusted.
- The predicted optimum is a model estimate — validate it with a real evaluation before relying on it.
- Categorical parameters are excluded; only numeric parameters are optimized.
