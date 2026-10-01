# ADR-0009: Exclude nominal categorical parameters from Spearman and Ridge

Status: Accepted
Date: 2026-10-01

## Context

[Issue #196](https://github.com/Tunny-gh/Tunny-Dashboard/issues/196) identifies
that sensitivity analysis label-encodes categories in first-appearance order
and passes those numeric IDs to Spearman and Ridge. This invents an order for
Spearman and numerical distances for Ridge: changing category order can change
results without changing the underlying category identities and outcomes.

A [nominal categorical parameter](../../CONTEXT.md) has no inherent numerical
order or distance. Optuna categorical choices remain nominal even when their
labels look numeric. The agreed policy must avoid arbitrary ordinal assumptions
without changing the meaning of Spearman or Ridge or introducing a general
categorical-analysis framework.

## Decision

Exclude nominal categorical parameters from both Spearman and Ridge sensitivity
calculations. Explicitly mark them as unsupported in analysis output and the UI,
rather than presenting a plausible-looking numerical importance score.
Numerical parameters continue to be evaluated normally; results on complete
numerical data remain unchanged.

Ridge fits only numerical parameters, and its R² describes that numerical-only
model, not a model that includes excluded categorical parameters. Spearman
remains a rank correlation for numerical parameters, and Ridge remains the
existing numerical Ridge sensitivity method.

Results retain only numerical parameter names and scores, with excluded names in
an explicit `unsupported_categorical` list. UI and CSV display these separately
as `Unsupported (categorical)`, without a score or ranking. All-categorical
inputs have no supported numerical parameters and no Ridge model or R². This is
distinct from numerical NaN missingness (ADR-0006), whose existing finite-pair
and complete-row policies remain unchanged. Other methods and generic Ridge
fitting outside sensitivity analysis are not changed.

## Alternatives

- **One-hot coding for Ridge and a distinct categorical association metric for
  categorical inputs instead of Spearman.** This could evaluate categorical
  effects without arbitrary numeric IDs, but requires encoding and per-parameter
  aggregation decisions for Ridge and a separately named association measure.
  Rejected in favor of minimal exclusion that preserves the current methods'
  meaning and avoids expanding this issue into categorical analysis.
- **Keep label-encoded category IDs as numerical inputs.** Rejected because
  these IDs impose order and distance that nominal categories do not have;
  making the encoding stable would not make those assumptions meaningful.

## Consequences

Spearman and Ridge no longer quantify nominal categorical effects. Unsupported
must be distinguishable from a measured zero effect, and users must be able to
recognize that Ridge R² assesses only the numerical-only model. This accepts
reduced input coverage in exchange for avoiding fabricated categorical scores.

Changing category labels or their first-appearance order must not create a
different sensitivity magnitude or ordering solely through arbitrary numeric
IDs. Numerical-only analyses retain their existing semantics. The output
representation never substitutes zero or numerical missingness for unsupported.

The remaining `get_param_numeric_values` callers are tree sensitivity extraction
(`metrics.rs`: RF-ANOVA, MDI, SHAP, Permutation), Sobol (`sobol.rs`, quadratic
Ridge surrogate), and GP-FITC ARD (`surrogate_opt/ard.rs`). All still encode labels
as first-appearance integer IDs. Tree split thresholds impose an arbitrary order;
RF-ANOVA additionally integrates numeric leaf boxes. Sobol treats these codes as
continuous ranges with arbitrary distances, including intermediate non-category
points. ARD's GP kernel interprets code distances as meaningful. Thus none of
these categorical results is guaranteed invariant to code assignment or trial
order. A bijective relabeling with identical equality/appearance patterns keeps
the codes, but does not validate the assumed order/distances. No algorithm in
these callers is changed by this decision. Their limitations are documented in
the public sensitivity overview.
