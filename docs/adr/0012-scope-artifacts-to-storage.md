# ADR-0012: Scope artifacts to storage

Status: Accepted
Date: 2026-10-03

## Context

Journal artifact metadata uses global trial IDs across studies. Clearing the
artifact folder and map whenever a study is activated loses scans that completed
before selection, including manual scans. Users expect a local Journal's adjacent
`artifacts` directory to load without a separate folder operation.

## Decision

Keep the effective artifact folder and scan results for the lifetime of the open storage.
The gallery filters that shared map to the selected study. After opening a local
Journal, discover `<journal parent>/artifacts` if it exists and no folder has
already been explicitly selected. Do not discover folders for CSV, SQLite, or
remote RDB storage. Explicit folder selection remains authoritative.

The explicit GUI/CLI root is session-scoped as defined in
[ADR-0011](0011-session-artifact-root-default.md) and takes priority over adjacent
Journal discovery. An automatically discovered directory does not become an
explicit session root.

Opening a storage, even the same path again, or choosing New clears storage scan
results and the automatically discovered directory, but retains the explicit
session root and invalidates pending directory scans. Reload retains the effective directory,
invalidates older scans, and rescans after the study is reloaded.
Study-list completions must match the currently requested source before they
can update state or trigger discovery. Reload rescans the current folder choice,
not a captured earlier choice, so manual selection during reload wins.

## Alternatives

- Keep study-scoped artifacts and rescan on every selection: unnecessary I/O and
  asynchronous ordering hazards for metadata that already covers all studies.
- Require manual folder selection: avoids implicit discovery but fails the
  expected adjacent-folder workflow for local Journals.

## Consequences

Study switches no longer erase automatic or manual scans. Storage replacement
cannot attach an old directory scan to unrelated trials sharing the same IDs.
An adjacent folder is a convention, not proof that every referenced file exists;
missing files still follow the existing resolver behavior.
