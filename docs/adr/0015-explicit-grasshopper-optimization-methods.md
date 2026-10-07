# ADR-0015: Explicit Grasshopper optimization methods

Status: Accepted
Date: 2026-10-07

## Context

[Issue #211](https://github.com/Tunny-gh/Tunny-Dashboard/issues/211) originally
proposed expanding Grasshopper adaptive sampling to non-GP models selected by
unrestricted Auto, including Ridge and LightGBM. These models do not provide the
predictive uncertainty required by the existing Expected Improvement (EI) and
Expected Hypervolume Improvement (EHVI) acquisitions. Supporting them would
require a separate proposal and exploration policy.

[ADR-0008](0008-auto-surrogate-selection-preference.md) originally included an
interim Grasshopper policy: Auto selected between GP-FITC and GP-VFE by
cross-validation score. The user-facing name "Adaptive (surrogate)" obscured the
actual optimization method and the GP selection policy. The agreed direction
for #211 is explicit method selection instead of non-GP adaptive Auto expansion.

## Decision

- Abandon #211's original non-GP adaptive Auto expansion. This decision does not
  change the general Surrogate Optimizer Auto policy in ADR-0008.
- Replace the vague user-facing "Adaptive" choice with explicit Grasshopper
  optimization methods. The agreed method labels and objective support are:

  | Method | Single objective | Multiple objectives |
  | --- | --- | --- |
  | NSGA-II | Supported | Supported |
  | CMA-ES | Supported | Unavailable, with an explanation that CMA-ES is single-objective only |
  | BO-GP-FITC | Supported | Supported |
  | BO-GP-VFE | Supported | Supported |
  | Random | Supported | Supported |

- Retain Random as a standalone user-facing optimization method for both
  single-objective and multi-objective runs, distinct from GP Bayesian
  optimization's random bootstrap.
- Preserve GP Bayesian optimization's candidate evaluation through Rhino.Compute
  and retraining loop. Use EI for single-objective runs and EHVI for
  multi-objective runs.
- Users choose the GP kind explicitly through the Bayesian optimization method
  choice. Do not use GP cross-validation-based Auto selection in Grasshopper.
  This replaces ADR-0008's Grasshopper interim policy as the accepted design,
  without replacing its general Surrogate Optimizer Auto decision.
- For CMA-ES constraint handling, reuse the existing NSGA-II/TrialRecorder
  violation-penalty fitness policy. Evaluate constraint-violating points and
  supply penalty fitness to the optimizer, but record their actual objective
  and constraint values in the journal, not the penalty. Do not introduce a
  dedicated CMA-ES constraint scheme.
- Expose only generations as a CMA-ES-specific setting, defaulting to 10.
  Derive the population size as `lambda = 4 + floor(3 ln d)`, where `d` is the
  parameter dimension. Use an initial normalized step size `sigma0 = 0.3` and
  start from the saved slider values.
- Display the CMA-ES upper evaluation budget as `1 + lambda * generations`;
  early termination may reduce the actual evaluation count. Reuse the existing
  common seed and parallelism settings, and apply the existing bound clamping
  and slider precision rounding on actual evaluations.

The methods above are implemented. BO means Bayesian optimization; hints explain
EI (Expected Improvement) for one objective and EHVI (Expected Hypervolume
Improvement) for multiple objectives. CMA-ES rejects multi-objective runs before
journal creation and reuses the existing optimizer with a stop check for
cancellation and journal I/O errors; surrogate-only callers remain unchanged.

## Alternatives

- **Expand adaptive Auto to Ridge and LightGBM.** Rejected in favor of explicit
  optimization methods: this would require a new non-GP proposal and exploration
  policy rather than reuse the existing GP acquisitions.
- **Keep "Adaptive" with GP-only Auto.** Rejected as the long-term Grasshopper
  design because the label hides the Bayesian optimization method and leaves GP
  kind selection implicit.
- **Expose Bayesian optimization but automatically choose its GP kind by CV.**
  Rejected in favor of user choice between GP-FITC and GP-VFE; explicit selection
  makes the selected method clear, at the cost of requiring that choice.

## Consequences

- Grasshopper method selection communicates the optimization algorithm and,
  for Bayesian optimization, the GP kind. Users must choose rather than rely on
  CV-based GP Auto selection.
- The GP evaluation/retraining workflow and EI/EHVI acquisitions remain part of
  the design; abandoning non-GP Auto expansion does not remove Bayesian
  optimization through Rhino.Compute.
- Multi-objective runs offer NSGA-II, both GP methods, and Random, while CMA-ES is
  unavailable with an explicit explanation instead of implying support.
- This implements the [approved replacement contract](https://github.com/Tunny-gh/Tunny-Dashboard/issues/211#issuecomment-6029219821),
  not the original non-GP Auto expansion criteria in #211.
