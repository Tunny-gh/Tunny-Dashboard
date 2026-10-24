# ADR-0025: Consolidate workspace crate placement

Status: Accepted
Date: 2026-10-24

## Context

[Issue #281](https://github.com/Tunny-gh/Tunny-Dashboard/issues/281) follows the
completed responsibility separation. Four libraries already live directly under
`crates/`, but the GUI and MCP entries remain at the root with inconsistent,
implementation-oriented directory names. ADR-0021 deliberately retained those
entry paths during extraction; the extraction no longer needs that staging rule.

## Decision

Place all six existing workspace crates directly under `crates/`. Move the GUI
entry to `crates/desktop/` and MCP entry to `crates/mcp/`; retain `study/`, `io/`,
`analysis/`, and `optimization-runner/`. Directory names describe responsibilities,
not the GUI framework or an inconsistent server suffix.

Use the conventional Rust library name `tunny_dashboard` for package
`tunny-dashboard`, without a compatibility alias for `tunny_desktop`. Preserve all
six package names and versions, executable names `TunnyDashboard` and `tunny-mcp`,
and ADR-0019's product-scoped naming rationale. Preserve ADR-0021's production
dependency graph, responsibility contracts, and native-library ownership.

Only ADR-0021's original entry-placement rule is superseded. Keep `libs/`, `docs/`
and repository-level tooling at the root. This changes neither supported behavior
nor distribution formats.

## Alternatives

- Keep the extraction-era root entries: avoids path migration but leaves the
  physical layout inconsistent after responsibility separation is complete.
- Retain framework/server directory names under `crates/`: unifies placement but
  keeps names that do not consistently describe entry responsibilities.
- Retain the custom GUI library name: avoids import updates but differs from
  Cargo's package-derived convention without a current requirement for it.

## Consequences

Contributor, editor, build, test and release paths must follow the new placement.
Entry scripts must still resolve root native libraries and the workspace lockfile;
embedded root resources and cross-crate fixtures need adjusted relative paths.
Historical reports and completed roadmap evidence retain their original paths.
No new domain terminology or package/dependency migration is introduced.
