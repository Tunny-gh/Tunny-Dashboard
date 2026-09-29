# ADR-0004: Display mixed-type trial user attributes separately

Status: Accepted
Date: 2026-09-29

## Context

Optuna trial user attributes can use the same key with a numeric value in one
trial and a string value in another. The Dashboard stores these as separate
numeric and string columns. Issue #199 exposes the stored values in the trial
detail modal, All Trials table, and its CSV export. A shared key would otherwise
produce indistinguishable columns.

## Decision

Keep the numeric and string values in separate columns. When both types use the
same key, distinguish them with `(numeric)` and `(text)` suffixes in the table
and CSV headings, and in the modal labels. Resolve each value from its own
attribute category rather than a name-only lookup.

## Alternatives

- **Combine both types into one text column.** This would simplify the visible
  table but discard numeric typing for display and export, and require merging
  the existing category-specific columns.
- **Show separate columns with identical headings.** This would retain the data
  but make table and CSV values ambiguous to readers and downstream tools.

## Consequences

Readers can identify the original value type for mixed-type keys. Such keys use
two columns, so a trial has a value in at most one of those cells. Keys that
occur in only one type retain the `User attr: <key>` heading.
