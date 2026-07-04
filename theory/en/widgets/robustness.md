# Robustness

The Robustness widget answers a question the optimizer itself cannot: **how stable is a candidate design's predicted performance when its input variables carry scatter?** It perturbs a chosen design with Gaussian input noise, propagates the perturbations through a surrogate model fitted to the observed trials, and shows the resulting output distribution — without running a single additional real evaluation. The underlying method is described in [Robustness analysis (Monte-Carlo noise propagation)](../optimization/robustness-analysis.md).

## Workflow

1. Select the **objective** and the **surrogate model**, then click **Fit Surrogate**. The surrogate is trained asynchronously on all completed trials (the same training pipeline as the Surrogate Optimizer; at least 10 trials are required). When the study has constraint columns, one constraint surrogate per column is trained alongside automatically.
2. Choose the **Center** — the candidate design to analyze: the *Best trial* (direction-aware best of the selected objective) or any **pinned** trial. Pin candidates in the Trial Table or detail modal first to compare specific designs. If a pinned trial disappears (e.g. after switching studies), the widget falls back to the best trial.
3. Set the **Noise %** — the 1σ of the Gaussian input scatter, as a percentage of each parameter's declared range — and the **sample count** (256 / 1024 / 4096).
4. Optionally enable **Model uncertainty** to additionally draw from the GP posterior at each sample, folding the surrogate's own (epistemic) uncertainty into the distribution. Ignored by models without predictive variance (Ridge, LightGBM).

The analysis itself runs instantly on any setting change — only the surrogate fit is a long-running step.

## Reading the results

- **Histogram** — the distribution of the predicted objective across all perturbed samples.
- **Nominal** (grey dashed line) — the surrogate prediction at the unperturbed center. **Mean** (red line) — the average over the perturbed samples. A gap between them (also printed as **Shift**) means the design sits on an asymmetric slope: scatter degrades it on average even though the nominal value looks fine.
- **P5 / P95** (dashed lines) — the empirical 5th/95th percentiles; for a minimized objective, P95 is a pessimistic "bad day" estimate.
- **Mean ± Std** — the spread is the primary robustness measure: between two candidates with similar nominal values, prefer the one with the smaller std.
- **P(feasible)** — shown when the study has constraints: the estimated probability that a perturbed design still satisfies all constraints (Optuna convention, feasible iff $c \le 0$).
- **Clipped %** (amber warning) — the fraction of samples that hit a declared parameter bound. When this is large, the center is near a bound and the reported distribution is truncated — interpret it as conditional on staying inside the range.

## Notes

- The analysis is a *model estimate*: its validity is bounded by the surrogate's quality. Fit quality can be assessed with the Surrogate Optimizer's validation plot using the same model kind; a poorly fitted surrogate produces confident-looking but meaningless distributions.
- Results are deterministic: a fixed seed means the same center, noise, and sample count always reproduce the same statistics — comparisons between candidates are never polluted by sampling luck. Gaussian draws use the [Box-Muller transform](../statistics/box-muller.md).
- The intended workflow is **comparison**: place two Robustness widgets side by side (each keeps its own settings), point them at two pinned candidates, and compare distributions. A slightly worse nominal with a much tighter spread is often the better engineering choice.
- Categorical parameters are excluded (numeric parameters only), matching the Surrogate Optimizer.
- The CSV export writes the raw output samples for external post-processing.
