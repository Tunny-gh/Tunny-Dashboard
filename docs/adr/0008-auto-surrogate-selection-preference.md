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

Adaptive sampling is an explicitly approved interim exception: single-objective
EI and multi-objective EHVI require predictive variance, which Ridge and LightGBM
do not provide. Before validation and final fitting, restrict adaptive eligibility
to GP-FITC then GP-VFE. Compute the maximum finite score within that eligible
subset, then apply the same inclusive absolute 0.01 margin and preference. Each
objective is selected independently; mixed unrestricted Auto results do not cause
non-GP models to be fitted and then replaced. Reports contain only evaluated
eligible GPs. If neither GP has a finite validation score, fail clearly without a
fallback proposal strategy. Preserve acquisition guards and constraint handling.
Unrestricted adaptive Auto and its non-GP proposal policy are separate work in
[Issue #211](https://github.com/Tunny-gh/Tunny-Dashboard/issues/211).

## Alternatives

- Keep the 0.001 tolerance and the old order: fixes the stale comparison but does
  not implement the agreed wider near-best margin or LightGBM preference.
- Always select the maximum score: gives up the agreed preference for earlier
  candidates when score improvements are small.
- Compare to the currently selected score: rejected because intermediate scan
  decisions can violate the global-best policy.
- Rank by measured runtime or statistical significance: these are different,
  problem-dependent policies requiring additional evaluation, not this change.
- Keep unrestricted Auto for adaptive and coerce its winner to a GP, or remove
  EI/EHVI uncertainty guards: rejected because selection reports would no longer
  describe the deployed model, or acquisition requirements would be violated.
- Add a non-GP adaptive proposal strategy here: deferred to #211 because its
  exploration and failure policy require a separate design decision.

## Consequences

Auto may select a model with up to 0.01 lower mean CV R² than the best finite
candidate, including LightGBM over a GP. This deliberately trades a small score
gain for deterministic preference, without promising lower runtime or equivalent
predictive quality. Two passes over the recorded scores make the choice independent
of stale intermediate comparisons. Dashboard v0.2 documentation records this
behavior while v0.1 remains unchanged.
Adaptive runs may deploy a lower-scoring GP than unrestricted Auto would choose;
the margin is relative only to eligible GPs. This capability restriction keeps
EI/EHVI valid but is not the desired long-term unrestricted adaptive policy.
