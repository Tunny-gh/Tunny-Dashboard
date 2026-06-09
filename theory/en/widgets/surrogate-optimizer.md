# Surrogate Optimizer

The surrogate optimizer fits a response surface (surrogate model) to the sampled trials and then optimizes on that surface to estimate the optimal parameter values — without running any additional trials.

## Workflow

1. Select the objective, the surrogate model, and the optimization method.
2. Click **Run Optimization**. The surrogate is trained on all completed trials and the optimizer searches the surface within the sampled parameter ranges.
3. The estimated optimum is shown as a parameter table together with the predicted objective value, and is marked on a 2D slice of the response surface through the optimum.

## Surrogate models

- **Kriging** — Gaussian process regression with an ARD Matérn 5/2 kernel. Provides a predictive uncertainty ($\pm 1.96\sigma$) alongside the mean. Trained on a subsample of up to 100 trials.
- **Sparse Kriging** — FITC approximation of the Gaussian process using inducing points. Faster for large studies while retaining uncertainty estimates.
- **Ridge** — Linear ridge regression. Fast baseline; the surface is a plane, so the optimum always lies on the boundary of the sampled ranges.

## Optimization methods

- **Multi-start L-BFGS** — Gradient-based local search (numerical gradients) started from the best observed trial and several random points; the best converged point is reported.
- **NSGA-II** — Genetic algorithm with SBX crossover, polynomial mutation and binary tournament selection (crowded comparison). Population-based and derivative-free, robust on multimodal surfaces. Currently applied to the single selected objective; the implementation supports multiple objectives for future Pareto-front optimization.
- **CMA-ES** — Covariance Matrix Adaptation Evolution Strategy. Derivative-free search that adapts the sampling distribution to the local curvature of the surface; a strong default for continuous problems.
- **Random Search** — Evaluates the surrogate at thousands of random points and picks the best. A robust baseline.

## Notes

- The search is constrained to the hyper-box spanned by the sampled parameter values; the surrogate is not trusted to extrapolate beyond the data.
- The objective direction (minimize / maximize) is taken from the study metadata.
- $R^2$ reports how well the surrogate fits the training data. A low $R^2$ means the estimated optimum should not be trusted.
- The predicted optimum is a model estimate — validate it with a real evaluation before relying on it.
- Categorical parameters are excluded; only numeric parameters are optimized.
