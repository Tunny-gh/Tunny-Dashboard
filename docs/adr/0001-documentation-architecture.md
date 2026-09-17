# ADR-0001: Documentation architecture

Status: Accepted
Date: 2026-09-17

## Context

The project's documentation had become scattered and partly stale. `README.md`
mixed product marketing with detailed build, test, and format instructions that
duplicated — and in one Rust-version detail contradicted — `CONTRIBUTING.md`.
`ROADMAP.md` sat at the repository root even though most of its items were done.
There were two unrelated `reports/` directories (one at the root, one under
`docs/`) holding the same genre of dated audit and investigation notes, and
`docs/` had loose `.md` files at its root with no folder structure.

On 2026-08-13 the documentation was reorganized into `docs/guides/`,
`docs/planning/`, `docs/reports/`, and `docs/handoff/`. The handoff folder was
new: a dated implementation record of what was decided, what was done, and what
was left. In practice it was later found, in 2026-09, to combine three different
kinds of content — durable design decisions, implementation logs, and TODOs —
which meant the project had no stable, single source of truth for durable
decisions (issue #176). A decision could only be found by reading through dated
session notes.

## Decision

The documentation taxonomy is organized by purpose:

- `docs/adr/` — durable architectural and design decisions (this directory).
- `docs/guides/` — user-facing how-to guides.
- `docs/planning/` — forward-looking strategy and plans.
- `docs/reports/` — dated investigations, audits, and evidence.

Durable design decisions are recorded as ADRs. User-facing changes are recorded
in `CHANGELOG.md`. Implementation status and remaining work live in Issues.
`README.md` keeps only Installation and points to `CONTRIBUTING.md` for build,
test, and format detail. The two dated reports directories are consolidated into
`docs/reports/`. `docs/handoff/` is retired.

## Alternatives

- **Keep `docs/handoff/` as the home for durable decisions.** Rejected because
  the format mixes durable decisions with implementation logs and TODOs, so
  there is no stable place to look for a decision, and no stable place to point
  when a decision changes.
- **Keep `docs/handoff/` and `docs/adr/` in parallel.** Rejected: two parallel
  locations for the same decision create two canonical sources that inevitably
  drift, and leave readers unsure which one is authoritative.
- **Adopt a heavier ADR framework, or enforce ADR creation in CI.** Rejected:
  the project is small and the process overhead and tooling are not justified at
  this scale. A lightweight convention is enough.

## Consequences

- Agents and contributors follow the discovery order in
  [`docs/adr/README.md`](README.md): `AGENTS.md`, then relevant ADRs, then
  relevant `docs/reports/` and `docs/planning/`, then code.
- ADR creation stays selective. Only decisions meeting all three conditions in
  [`docs/adr/README.md`](README.md) get an ADR, keeping the directory readable.
- Enforcement is prose-only, stated in `AGENTS.md` and `CONTRIBUTING.md`, not
  checked by CI. A missing ADR is caught in review, not by automation.
