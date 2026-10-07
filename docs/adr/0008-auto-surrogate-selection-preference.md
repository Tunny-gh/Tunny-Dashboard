# ADR-0008: Auto surrogate selection preference

Status: Accepted
Date: 2026-10-01

## Context

Issue #195 exposed a stale comparison in Auto selection: comparing each candidate
against the currently preferred score can skip an earlier candidate close to the
global best. The user approved expanding the fix to a tolerance of 0.01 and a
preference order of Ridge, LightGBM, GP-FITC, then GP-VFE.

## Decision

Determine the maximum finite mean cross-validation R² first. Select the earliest
candidate in that preference order whose absolute R² gap from the maximum is
less than or equal to 0.01. Failed validation and all non-finite scores are
excluded; no finite scores remains an error. Cancellation is propagated.

The order is an explicit product policy, not a universal computational cost
ranking: cost depends on the problem. The tolerance is an absolute score margin,
not statistical equivalence, a relative percentage, or a guarantee of 1% accuracy.
Candidate models, training methods, and validation folds remain unchanged.

This decision applies to general Surrogate Optimizer Auto. Grasshopper Bayesian
optimization uses explicit BO-GP-FITC or BO-GP-VFE methods with no GP Auto
selection, as implemented in
[ADR-0015](0015-explicit-grasshopper-optimization-methods.md). That decision
supersedes the Grasshopper GP-only Auto exception; general Surrogate Optimizer
Auto remains unchanged and this ADR remains Accepted for that policy.

## Alternatives

- Keep the 0.001 tolerance and the old order: fixes the stale comparison but does
  not implement the agreed wider near-best margin or LightGBM preference.
- Always select the maximum score: gives up the agreed preference for earlier
  candidates when score improvements are small.
- Compare to the currently selected score: rejected because intermediate scan
  decisions can violate the global-best policy.
- Rank by measured runtime or statistical significance: these are different,
  problem-dependent policies requiring additional evaluation, not this change.

## Consequences

Auto may select a model with up to 0.01 lower mean CV R² than the best finite
candidate, including LightGBM over a GP. This deliberately trades a small score
gain for deterministic preference, without promising lower runtime or equivalent
predictive quality. Two passes over the recorded scores make the choice independent
of stale intermediate comparisons. Dashboard v0.2 documentation records this
behavior while v0.1 remains unchanged.
