# ADR-0014: Preserve category identity and presence

Status: Accepted
Date: 2026-10-04

## Context

The distribution grouping contract in [Issue #227](https://github.com/Tunny-gh/Tunny-Dashboard/issues/227)
excludes missing parameters but includes actual categorical values. Existing
display labels lose the distinction between an empty-string choice and an absent
parameter, and between differently typed choices such as number `1` and string
`"1"` that have the same display label.

## Decision

Preserve true category identity and presence from ingestion through distribution
grouping, including incremental updates. Box Plot and Violin Plot use that
information for grouping and consistent, unambiguous chart and CSV labels.
Display actual empty-string categories with an intelligible English label and
disambiguate colliding labels deterministically, without cluttering ordinary
noncolliding labels. CSV categories retain their parsed types; do not invent
Optuna metadata for CSV inputs.

Keep unrelated charts' behavior unchanged, preferring additive metadata over
changes to existing display-label conventions.

## Alternatives

- **Group by existing display labels only.** Rejected because it merges distinct
  choices and cannot distinguish absence from a real empty-string category.
- **Change category display conventions throughout the application.** Rejected
  because unrelated chart behavior is outside this distribution-analysis scope.

## Consequences

The parser, row, DataFrame, and chart-input paths must carry category identity
and presence in addition to existing display labels. This adds metadata storage
and propagation work but avoids fabricated groups, lost choices, and unrelated
UI changes.
