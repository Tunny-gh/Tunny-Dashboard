# Architecture Decision Records

An Architecture Decision Record (ADR) captures a durable design decision and,
more importantly, the "why" behind it — the reasoning that code alone does not
show. An ADR states what was chosen, what alternatives were considered, why the
alternatives were rejected, and what the choice commits the project to.

This directory is the stable source of truth for those decisions. Implementation
logs, task status, and TODOs do not belong here; they live in Issues, and
user-facing changes live in [CHANGELOG.md](../../CHANGELOG.md).

## When an ADR is required

Write an ADR when **at least one** of these holds:

1. The decision is costly to reverse later.
2. The decision would be surprising without the historical context.
3. A real alternative existed and a trade-off was made.

Routine fixes, small refactors, dependency updates, UI or documentation
tweaks, and ordinary tests do **not** get ADRs.
`CONTRIBUTING.md#documentation` states the same rule for contributors.

## Format

- File name: `NNNN-kebab-title.md` — a 4-digit, zero-padded, sequential number
  plus a short kebab-case title. Numbers are never reused or renumbered.
- The body starts with the title as `# ADR-NNNN: Title`.
- Two meta lines follow, `Status:` and `Date:`, then the sections below, in
  order.

```
# ADR-NNNN: Title

Status: Accepted
Date: YYYY-MM-DD

## Context

## Decision

## Alternatives

## Consequences
```

- `Status` is one of:
  - `Accepted` — the default; the decision is in force.
  - `Proposed` — written down but not yet agreed.
  - `Superseded by ADR-NNNN` — a later ADR replaced it.
  - `Deprecated` — no longer in force, with no direct replacement.
- `Date` is the date the decision was accepted for an `Accepted` ADR, or the
  date it was proposed for a `Proposed` ADR (which has no acceptance date yet),
  in `YYYY-MM-DD`. If the decision predates the ADR that records it, keep this
  acceptance or proposal date here and note the original timing in `Context`.
- `Context` gives the background and the forces at play. `Decision` states the
  choice. `Alternatives` lists the options that were rejected and why.
  `Consequences` states what follows, including the downsides accepted.

## Discovery order

When you need to understand why the project is the way it is, read in this
order:

1. [`AGENTS.md`](../../AGENTS.md) — binding project conventions.
2. Relevant ADRs in this directory — durable design decisions.
3. Relevant [`docs/reports/`](../reports/) and [`docs/planning/`](../planning/)
   — dated investigations and forward-looking strategy.
4. The code itself.

## Index

| # | Title | Status | Date |
| --- | --- | --- | --- |
| 0001 | [Documentation architecture](0001-documentation-architecture.md) | Accepted | 2026-09-17 |
| 0002 | [Phase 2 scope — extend into the execution loop](0002-phase-2-scope.md) | Accepted | 2026-07-16 |
| 0003 | [Parse .ghx directly for Grasshopper integration](0003-ghx-direct-parsing.md) | Accepted | 2026-07-16 |
| 0004 | [Display mixed-type trial user attributes separately](0004-display-mixed-type-user-attributes-separately.md) | Superseded by ADR-0005 | 2026-09-29 |
| 0005 | [Present trial user attributes by key](0005-present-user-attributes-by-key.md) | Accepted | 2026-09-29 |
| 0006 | [Represent missing numeric parameters as NaN](0006-represent-missing-numeric-parameters-as-nan.md) | Accepted | 2026-09-30 |
| 0007 | [Distinguish three constraint feasibility states](0007-distinguish-three-constraint-feasibility-states.md) | Accepted | 2026-09-30 |
| 0008 | [Auto surrogate selection preference](0008-auto-surrogate-selection-preference.md) | Accepted | 2026-10-01 |
| 0009 | [Exclude nominal categorical parameters from Spearman and Ridge](0009-exclude-nominal-categorical-parameters-from-spearman-and-ridge.md) | Accepted | 2026-10-01 |
| 0010 | [Reject negative MCDM weights](0010-reject-negative-mcdm-weights.md) | Accepted | 2026-10-01 |
| 0011 | [Session artifact root default](0011-session-artifact-root-default.md) | Accepted | 2026-10-03 |
| 0012 | [Scope artifacts to storage](0012-scope-artifacts-to-storage.md) | Accepted | 2026-10-03 |
| 0013 | [Preserve parameter distributions for grouping](0013-preserve-parameter-distributions-for-grouping.md) | Accepted | 2026-10-04 |
| 0014 | [Preserve category identity and presence](0014-preserve-category-identity-and-presence.md) | Accepted | 2026-10-04 |
| 0015 | [Explicit Grasshopper optimization methods](0015-explicit-grasshopper-optimization-methods.md) | Accepted | 2026-10-07 |
| 0016 | [Artifact Animation export interaction](0016-artifact-animation-export-interaction.md) | Accepted | 2026-10-08 |
| 0017 | [Artifact Animation frame resource ownership](0017-artifact-animation-frame-resource-ownership.md) | Superseded by ADR-0018 | 2026-10-08 |
| 0018 | [Artifact Animation timing, export sampling, and cache policy](0018-artifact-animation-timing-export-and-cache-policy.md) | Accepted | 2026-10-09 |
| 0019 | [Responsibility refactoring goal and scope](0019-responsibility-refactoring-goal-and-scope.md) | Accepted | 2026-10-16 |
| 0020 | [Commit Study updates by validated batch](0020-commit-study-updates-by-validated-batch.md) | Accepted | 2026-10-16 |
| 0021 | [Study, analysis, and I/O boundary contracts](0021-study-analysis-and-io-boundary-contracts.md) | Accepted | 2026-10-16 |
| 0022 | [Retain recorded Journal attributes for postprocessing](0022-retain-recorded-journal-attributes-for-postprocessing.md) | Accepted | 2026-10-18 |
| 0023 | [Include supported attribute changes in RDB fingerprints](0023-include-supported-attribute-changes-in-rdb-fingerprints.md) | Accepted | 2026-10-18 |
| 0024 | [Require explicit opt-in to reuse Journal Studies](0024-require-explicit-opt-in-to-reuse-journal-studies.md) | Accepted | 2026-10-22 |
| 0025 | [Consolidate workspace crate placement](0025-consolidate-workspace-crate-placement.md) | Accepted | 2026-10-24 |
