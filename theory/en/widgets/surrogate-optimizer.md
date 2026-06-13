# Surrogate Optimizer

The surrogate optimizer fits a response surface (surrogate model) to the sampled trials and then optimizes on that surface to estimate the optimal parameter values — without running any additional trials.

## Workflow

1. Select the objective, the surrogate model, and the optimization method.
2. Click **Run Optimization**. The surrogate is trained on all completed trials and the optimizer searches the surface within the sampled parameter ranges.
3. The estimated optimum is shown as a parameter table together with the predicted objective value, and is marked on a 2D slice of the response surface through the optimum.

## Parameter importance (ARD)

After fitting a GP surrogate (GP-FITC / GP-VFE), the validation panel shows a **Parameter importance (ARD)** bar list. Each bar is proportional to the normalized GP length-scale relevance of that parameter (the scores sum to 100%): a larger value means the response surface is more sensitive to that parameter. This is a free, global, smoothness-based sensitivity available straight from the fitted kernel. It is shown for GP models only (GP-MOE, Ridge and LightGBM do not report it).

See [ARD Parameter Importance](../sensitivity-analysis/ard-importance.md) for the full description and the larger-θ-means-more-sensitive convention.

## Slice uncertainty overlay

When a GP surrogate is used, the 2D response-surface slice can also show the model's predictive uncertainty. A **Show uncertainty (±σ)** toggle (off by default) appears below the slice; enabling it overlays a translucent grey tint that grows darker where the predicted standard deviation is high, fading out regions the surrogate is unsure about. Non-GP models have no posterior variance, so the toggle does not appear.

## Surrogate models

- **GP-FITC** — Gaussian process regression (Kriging) with ARD Matérn 5/2 kernel backed by egobox-gp. FITC sparse approximation with M = min(N, 100) inducing points; trains on all trials. Provides predictive uncertainty ($\pm 1.96\sigma$). Default GP choice.
- **GP-VFE** — Same architecture as GP-FITC but uses the Variational Free Energy bound instead of FITC likelihood. Produces a slightly smoother, more conservative fit; recommended when GP-FITC surface looks overfit or spiky.
- **GP-MOE** — Mixture-of-experts GP via egobox-moe. Clusters the input space with a Gaussian Mixture Model and trains one FITC expert per cluster (up to 3, selected by cross-validation on ≤ 500 points). Best for discontinuous or regime-switching objectives. If training fails, an error is reported rather than silently falling back.
- **Ridge** — Linear ridge regression. Fast baseline; the surface is a plane, so the optimum always lies on the boundary of the sampled ranges.

## Automatic model selection

Choosing **Auto (cross-validated)** in the model selector lets the tool pick the surrogate for you. Each candidate is cross-validated and the one with the highest mean CV R² is trained.

- **Candidates**: Ridge, GP-FITC, GP-VFE, LightGBM. GP-MOE is *excluded* from Auto — its cross-validated cluster search is expensive and it degenerates to a single GP on smooth or linear data, giving a poor cost/benefit ratio. Pick GP-MOE manually when you know the response is discontinuous or multi-modal.
- **Criterion**: mean k-fold CV R² (the same metric shown in the validation panel), which rewards generalization rather than in-sample fit.
- **Tie-break**: candidates whose CV R² is within 1e-3 of the best are treated as tied, and the *earlier* (simpler, cheaper) candidate in the order above is chosen. On perfectly linear data, where both Ridge and a GP reach R² ≈ 1, this keeps the simpler Ridge.

After an Auto fit, the widget shows which model was chosen and the ranked candidate CV R² scores. The chosen concrete model is used for everything downstream (ARD importance, acquisition suggestions, constraints).

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

## Constraint-aware optimization

When the study has constraint columns (Optuna convention: value ≤ 0 means feasible), a **Use constraints (N)** checkbox appears in the Fit section, where N is the number of constraint columns.

Enabling this checkbox makes the optimizer:

1. Fit a surrogate for each constraint using the **same model kind as the objective** (a GP yields a smooth feasibility probability; only a perfectly linear, noise-free constraint whose GP fit degenerates falls back to Ridge for that constraint).
2. Add a constraint penalty to the cost function during optimization:
   $$\text{cost}(x) = \text{sign} \cdot \hat{\mu}_y(x) + 100 \cdot \sum_i \max(0, \hat{c}_i(x) - z_{0,i})$$
   where $z_{0,i}$ is the feasibility boundary in the normalized constraint space and $\text{sign} = 1$ for minimization, $-1$ for maximization.
3. Display the **P(feasible)** percentage and per-constraint predicted values in the results.

The feasibility probability shown is the product over all constraints of $P(c_i \le 0)$ (a smooth $\Phi$-based probability for GP constraints, or a hard indicator when a constraint fell back to Ridge):

$$P_\text{feas}(x) = \prod_i P(c_i \le 0 \mid x)$$

In the **Suggest next trials** section, the **P(feas)** column in the candidate table shows the feasibility probability for each suggested candidate; values below 0.5 are highlighted in orange/red as a warning.

See [Acquisition Functions — Constraint-aware acquisition functions](../optimization/acquisition-functions.md#constraint-aware-acquisition-functions) for the full mathematical description.

## Notes

- The search is constrained to the hyper-box spanned by the sampled parameter values; the surrogate is not trusted to extrapolate beyond the data.
- The objective direction (minimize / maximize) is taken from the study metadata.
- $R^2$ reports how well the surrogate fits the training data. A low $R^2$ means the estimated optimum should not be trusted.
- The predicted optimum is a model estimate — validate it with a real evaluation before relying on it.
- Categorical parameters are excluded; only numeric parameters are optimized.
- The constraint checkbox is only shown for the single-objective path. Multi-objective mode does not currently incorporate constraints.
