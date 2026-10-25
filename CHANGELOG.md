# Changelog

All user-facing changes to Tunny Dashboard — new features, behavior changes,
bug fixes — are documented in this file. Internal refactors, tests, and
doc-only changes are not (see [CONTRIBUTING.md](CONTRIBUTING.md#documentation)
for the full rule).

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Entries are grouped under `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`,
and `Security` as needed — omit headings that have nothing under them.

## [Unreleased]

### Added

- Generic-process and Grasshopper optimization can explicitly add new results to
  an existing Study through `load_if_exists` or the default-OFF GUI option. Each
  invocation starts a fresh optimizer; this is not history-aware resume.

- Artifact Animation is an independently configurable canvas widget for PNG/JPEG
  trial artifacts, with Study-local playback, optional trial values and cached
  trial-prefix Pareto/history plots on fixed axes. Export GIF captures the same
  content presentation through a protected snapshot, with responsive cancellation
  and background encoding; controls and surrounding panels are excluded.

### Fixed

- Parallel Coordinates rescales line colors to finite feasible values passing all
  active brushes, including all selected rows regardless of downsampling. Clearing
  brushes restores the full feasible color scale without changing axis ranges.

- Cluster Scatter 3D and Pareto Scatter 3D refresh axis ranges and projected points
  when the displayed input changes, including same-size updates and Study/source
  switches, while preserving camera and local view controls.

- Correlation Matrix refreshes coefficients and column labels when the displayed
  input or selected columns change, including same-size replacements and same-name
  Studies from different sources, while preserving method and column-group controls.

- Parallel Coordinates brushes exclude missing and nonfinite numeric observations
  on actively brushed axes, including axes with constant ranges. Inactive axes do
  not filter trials.

- MCDM 2D and 3D scatter plots refresh score coordinates, picking positions, and
  axis ranges after a new accepted result even when the ranking is unchanged,
  while preserving rank colors and widget controls/camera.

- Journal recording rejects duplicate Study names by default instead of returning
  an ID that cannot be read back. Explicit reuse requires matching objective
  counts and ordered directions, preserves existing objective names and trials,
  and continues global and Study-local numbering. Legitimate rejected Optuna
  creation attempts do not consume Study IDs. Unsafe append prefixes are rejected
  without automatic repair; no parallel-writer or complete corruption detection
  guarantee is added.

- Reports select finite objective rows before computing nondominance, so
  recorded infinities cannot eliminate finite front candidates. Constrained
  reports use clearly labeled objective-only candidates when no finite verified
  feasible rows exist, without changing recorded observations or feasible counts.

- Journal loading now preserves recorded numeric infinity and NaN in objectives,
  intermediate values and user attributes instead of conflating them with JSON
  null. CSV numeric user attributes retain nonfinite observations through the
  owned Study handoff; numerical analyses keep their existing finite-input policies.
- Late Journal metric names replace objective labels without losing streaming
  column layout, and repeated objective labels retain separate indexed values.
  CSV artifact resolution uses the shared root-containment check.

- PostgreSQL and MySQL Study loading now reads related metadata, trial data and
  supplementary information in one read-only repeatable-read transaction, avoiding
  mixed results during concurrent updates. Acquisition ends before DataFrame
  conversion or analysis; MySQL snapshot validation covers ordinary Optuna-created
  InnoDB tables.

- SQLite Study loading now reads metadata, trial data and supplementary information
  from one database snapshot, avoiding mixed results when a writer commits during
  acquisition. The read transaction ends before DataFrame conversion or analysis.

- The public RDB/SQLite fingerprint API now detects COMPLETE trial user-attribute
  insertions and updates, and constraint insertions and updates across all trial
  states, even when trial counts, states and intermediate-record counts do not change.

- Journal loading now retains user attributes and replacement constraint results
  recorded after COMPLETE, PRUNED or FAIL, correcting earlier displayed rows and
  all-state feasibility evidence in batches without delaying the initial results.

- Artifact Animation starts the next trial-prefix preparation as soon as a cached
  frame becomes ready, avoiding an extra loading wait during deadline-driven playback.

- Grasshopper BO-GP-FITC and BO-GP-VFE now report the original journal write
  failure instead of an insufficient-successful-evaluations or surrogate-fit
  error, preserving already written successful and started trial records.

- Rank Plot 3D now fills the remaining chart area and resizes with the widget,
  with its vertical Best/Worst legend overlaid inside the canvas's right edge.

### Changed

- Artifact Animation's Frame skip is shared by widget preview and GIF export.
  Playback, stepping, seeking, looping, and prefetch follow the same selected
  chronological sequence, including both endpoints. Changing skip preserves the
  current selected image or moves to its nearest selected predecessor. Trial
  numbers and full trial-prefix analysis are unchanged.

- Artifact Animation uses one live/GIF display duration (20–10000 ms in 10 ms
  steps, default 330 ms), with a logarithmic slider, numeric input, and reference
  FPS. Shared frame skipping preserves both endpoints and all trial-prefix
  observations, and shows frame count and duration. Display/DPI-aware images are
  reused across Animation widgets within a shared 256 MiB estimated retained
  resource budget, with chronological prefetch and safe seek/invalidation.

- Grasshopper/Rhino.Compute offers NSGA-II, CMA-ES, BO-GP-FITC, BO-GP-VFE, and
  Random. BO (Bayesian optimization) fits the explicitly chosen GP, with EI for
  one objective and EHVI for multiple objectives; GP Auto selection is removed
  from this workflow. Surrogate Optimizer Auto is unchanged. CMA-ES is
  single-objective only, starts from saved slider values, and exposes generations
  (default 10) with an upper evaluation budget of `1 + lambda * generations`.
  It reuses bound/precision handling, seed, parallelism, constraint penalties,
  actual-value journal recording, and cancellation.

- Box Plot and Violin Plot share a single Value, optional Group by, and global
  Normalize selection. Grouping supports categorical and explicitly declared
  Optuna integer/stepped parameters, not continuous parameters or CSV numeric
  parameters without distribution metadata. Missing/non-finite data is excluded;
  categorical identity and presence are retained, including empty-string choices.
  Box plots include singleton/constant groups; violins report omitted groups.
  CSV exports use the displayed groups and scale, and distribution caches refresh
  when the underlying study snapshot changes.

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

- `-a` / `--artifact <directory>` sets a session-wide artifact root independently
  of `--input`. It requires an existing directory; GUI Artifacts selections take
  priority across source changes, study switches, and reload. DesignExplorer-format
  CSV `img` paths resolve against the explicit root while preserving trial associations.
- **Rank Plot 3D** displays three distinct parameters with objective rank colors,
  shared 3D navigation, hover details, Trial Detail, and full-row CSV export.
  The two-parameter chart is named **Rank Plot 2D**; both use the same rank and
  Best/Worst color semantics.
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

- MCDM Ranking bar labels now show Study-local trial numbers for TOPSIS, VIKOR,
  PROMETHEE I, and PROMETHEE II, matching tables and trial details while preserving
  failed/pruned trial gaps. Ranking order and Top N do not renumber trials; an
  unavailable trial number falls back to the original Study-view row index.

- Artifact Gallery cards and Trial Table Cluster mode display Study-local trial
  numbers consistently, preserving gaps from failed or pruned trials. Trial Table
  headers use **Trial Number**, and related trial labels fall back to the row index
  when a trial number is unavailable. Artifact associations and trial interactions
  continue to use internal global IDs.

- Scatter Matrix histograms now place extreme finite observations in the correct
  bins even when the difference between their endpoints overflows.

- Scatter Matrix histograms now derive bin ranges and counts only from finite
  observations, excluding missing (NaN) and infinite values. Columns with no
  finite observations show no bars; constant columns count only finite values.

- Scatter Matrix row and column labels now refresh after font atlas recreation,
  DPI changes, and theme changes instead of retaining stale text layouts.

- Artifact Gallery scopes Artifact # to the current study in All, Cluster, and
  MCDM modes. The zero-based "of up to" bound now matches the selector maximum;
  switching studies clamps invalid indices while preserving valid selections,
  and studies with at most one artifact per trial use index 0 without a selector.

- Artifact Gallery now reads artifact metadata embedded in Journal trial creation
  records, associating images with the correct global trial IDs across studies,
  including trials with no artifacts and non-COMPLETE states.
  Local Journals automatically load an adjacent `artifacts` folder. Artifact
  folders and scan results survive Study switches within the same storage;
  opening a storage or choosing New clears storage results and automatically
  discovered folders and ignores older scans, while explicit GUI/CLI roots remain
  session-wide and take priority over adjacent-folder discovery.

- Parallel Coordinates Plot axis names now refresh with the font atlas and DPI,
  and reserve their full horizontal or rotated bounds to avoid clipping outer
  labels when chart space is sufficient (#216).

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
