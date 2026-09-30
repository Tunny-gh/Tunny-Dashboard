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
