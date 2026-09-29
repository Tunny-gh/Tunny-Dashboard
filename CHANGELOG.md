# Changelog

All user-facing changes to Tunny Dashboard — new features, behavior changes,
bug fixes — are documented in this file. Internal refactors, tests, and
doc-only changes are not (see [CONTRIBUTING.md](CONTRIBUTING.md#documentation)
for the full rule).

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Entries are grouped under `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`,
and `Security` as needed — omit headings that have nothing under them.

## [Unreleased]

### Fixed

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
