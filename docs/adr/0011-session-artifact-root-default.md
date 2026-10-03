# ADR-0011: Session artifact root default

Status: Accepted
Date: 2026-10-03

## Context

[Issue #218](https://github.com/Tunny-gh/Tunny-Dashboard/issues/218) introduces
an optional CLI artifact root directory independent of the optimization result
source. A session may start without a result source and open results later in
the GUI, so the directory's lifetime and priority need an explicit decision.

## Decision

`--artifact` supplies an optional session-wide default artifact root directory
and is accepted without `--input`. The default remains available for GUI-opened
sources, study switching, and reload. A directory explicitly selected through
the GUI Artifacts control takes priority over the CLI default and remains the
session root through those changes. Relative CLI paths use the launch working
directory.

When `--artifact` is omitted, existing behavior is preserved.

When supplied, `--artifact` must name an existing directory at startup.
Nonexistent paths and individual files produce a clear CLI error and abort
startup. This validation introduces no new GUI warning or directory monitoring.

For DesignExplorer-format CSV, `img` paths resolve relative to the explicit
artifact root while retaining their association with each trial. Without an
explicit artifact root, they continue to resolve relative to the
DesignExplorer-format CSV file's parent directory. Existing safe root-relative
path rules remain unchanged.

## Alternatives

- **Apply the CLI directory only to the initial result source.** Rejected
  because independent inputs should remain useful when opening results later,
  including sessions launched without `--input`.

## Consequences

- The CLI default is not discarded when the result source or selected study
  changes, or when results are reloaded.
- A session-wide default may be used with multiple result sources; an explicit
  GUI Artifacts directory selection takes precedence when a different root is
  needed.
