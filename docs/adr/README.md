# Architecture Decision Records

An Architecture Decision Record (ADR) captures a durable design decision and,
more importantly, the "why" behind it — the reasoning that code alone does not
show. An ADR states what was chosen, what alternatives were considered, why the
alternatives were rejected, and what the choice commits the project to.

This directory is the stable source of truth for those decisions. Implementation
logs, task status, and TODOs do not belong here; they live in Issues, and
user-facing changes live in [CHANGELOG.md](../../CHANGELOG.md).

## When an ADR is required

Write an ADR only when **all three** of these hold:

1. The decision is costly to reverse later.
2. The decision would be surprising without the historical context.
3. A real alternative existed and a trade-off was made.

If any one of the three is missing, do not write an ADR. Routine fixes, small
refactors, dependency updates, UI or documentation tweaks, and ordinary tests
do **not** get ADRs. `CONTRIBUTING.md#documentation` states the same rule for
contributors.

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
- `Date` is the date the ADR was accepted, in `YYYY-MM-DD`. If the decision
  predates the ADR that records it, keep the acceptance date here and note the
  original timing in `Context`.
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
| 0002 | [Phase 2 scope — extend into the execution loop](0002-phase-2-scope.md) | Accepted | 2026-09-17 |
| 0003 | [Parse .ghx directly for Grasshopper integration](0003-ghx-direct-parsing.md) | Accepted | 2026-09-17 |
