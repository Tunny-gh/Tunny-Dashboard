# Changelog

All user-facing changes to Tunny Dashboard — new features, behavior changes,
bug fixes — are documented in this file. Internal refactors, tests, and
doc-only changes are not (see [CONTRIBUTING.md](CONTRIBUTING.md#documentation)
for the full rule).

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Entries are grouped under `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`,
and `Security` as needed — omit headings that have nothing under them.

## [Unreleased]

### Changed

- Spearman and Ridge sensitivity now evaluate numerical parameters only. Nominal
  categorical parameters (including numeric-looking Optuna choices) are explicitly
  marked `Unsupported (categorical)` in charts, heatmaps, CSV and reports, never
  scored or ranked. Ridge R² is labeled numerical-only and is absent when no
  numerical parameters are supported. Existing numerical missingness handling is unchanged.

- Auto surrogate selection now prefers Ridge, LightGBM, GP-FITC, then GP-VFE
  among candidates within an inclusive absolute mean CV R² gap of 0.01 from
  the maximum finite score. This explicit preference is not a universal cost
  ranking; failed and non-finite candidates remain excluded.
- Adaptive sampling now selects only GP-FITC then GP-VFE before final fitting,
  because both single-objective EI and multi-objective EHVI require predictive
  variance. The same inclusive 0.01 tolerance is measured against the best finite
  eligible GP score, not unrestricted Auto's best score. No finite eligible GP
  remains an error; reports list only evaluated GPs. This is an interim exception;
  unrestricted adaptive Auto is tracked in #211. Adaptive fitting shares run
  cancellation while keeping trial progress counters separate.
- Trial details now use aligned key/value rows with subtle stripes and separators,
  clearer section boundaries, and indented array elements. Expanded arrays show
  an item count in the parent row.

### Added

- Trial User Attributes now retain and display every JSON value, including
  booleans, null, arrays, and objects, in one column per attribute key by default across
  trial details, the All Trials table, CSV export, and HTML reports.
- An **Expand lists** checkbox in All Trials adds `key[0]`, `key[1]`, and later
  array element columns. Trial details can also expand array attributes into
  indexed rows. The All Trials CSV export follows the checkbox setting.
- Trial details now show numeric and text User Attributes. The All Trials table
  and its CSV export include attribute columns, controlled by a visible-on-default
  **User attrs** checkbox.

### Fixed

- MCDM rankings reject negative criterion weights instead of accepting them or
  silently replacing them with uniform weights. Nonnegative weights retain their
  existing normalization, including uniform weights for a zero or nonfinite sum.

- Auto surrogate selection now compares tolerance against the global best finite
  CV R², rather than a stale intermediate selection (#195).
- Constraint feasibility now distinguishes **Feasible**, **Infeasible**, and
  **Feasibility unverified**. Missing, partial, invalid, or non-finite evaluations
  no longer become satisfied zeros; finite positive violations still establish
  infeasibility. Incremental constraint-schema growth reclassifies historical
  trials consistently with initial loading, including schema evidence from
  non-COMPLETE trials and metadata-only batches. Optuna's Python JSON non-finite
  tokens retain their constraint positions instead of dropping arrays or records.
  Journal objective arrays also preserve non-finite or invalid positions as NaN,
  so later finite objectives never shift into earlier columns; existing finite-data
  report and analysis filters continue to exclude invalid observations.
  Only verified feasible trials enter
  constrained Pareto fronts and feasible-only analyses. GUI convergence indicators
  also exclude unverified and infeasible observations from base/comparison histories
  and their shared reference set, preserving original trial coordinates. GUI
  convergence computation waits for the final streaming chunk, and results from
  replaced snapshots are rejected so constraint-schema growth cannot leave stale
  indicator histories. Scatter
  plots, details, and reports distinguish unverified trials. Reports with no verified feasible
  trials retain clearly labeled objective-only candidates, excluded from verified
  feasible/front counts. Genuinely unconstrained studies retain their behavior.
- IGD+ histories now give each exactly distinct finite reference objective vector
  equal weight, ignoring duplicate vectors (including signed-zero variants) within
  a study or across comparison studies while preserving every trial history entry.
- Absent numeric parameters now remain missing (NaN), rather than becoming zero,
  in initial and incremental loads. PCA excludes incomplete rows before scaling
  and keeps trial coloring aligned, while preserving the feature list and existing
  constant-zero categorical features. PCA CSV exports include the original row index;
  numerical fitting boundaries no longer silently consume missing parameter measurements.
- Entropy weighting now treats every finite constant objective, including all-zero
  columns, as zero-information when multiple valid trials exist. Single-valid-trial
  and all-constant data retain equal weights.
- Reload now refreshes artifacts from the selected folder along with trial data,
  including artifacts added to new or existing trials and removal of missing files.
- Fixed image artifacts failing to load from Windows file paths in the gallery
  and trial details.

## [0.1.2] - 2026-09-27

### Added

- New **Violin Plot** statistics chart. It estimates Gaussian kernel density
  curves for numeric objectives/parameters side by side, or for the levels of a
  categorical parameter, with an optional [0,1] normalization and a median
  marker per violin. Non-constant columns whose interquartile range is zero are
  still rendered, falling back to the standard deviation for the bandwidth.

### Fixed

- Fixed the Rank Plot shrinking to the toolbar row height instead of using the
  remaining vertical space.

### Changed

- Reversed the Rank Plot colormap display so Best (rank 0) uses the high end and
  Worst (rank 1) uses the low end, matching the color-bar labels.

[Unreleased]: https://github.com/Tunny-gh/Tunny-Dashboard/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/Tunny-gh/Tunny-Dashboard/compare/v0.1.1...v0.1.2
