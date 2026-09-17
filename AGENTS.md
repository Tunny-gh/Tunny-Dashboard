# Tunny Dashboard

A Rust egui desktop app for analyzing Optuna optimization results.

## Language

- Code comments and doc comments, commit messages, and PR titles/bodies are
  written in English.
- Project documents such as docs/planning/roadmap.md are written in English.
- UI text (labels, progress, error messages, and other user-facing strings) is
  written in English.

## Engineering Principles

- Do not preserve backward compatibility.
- Choose the simplest implementation that fully meets the current
  requirements.
- Prefer established, well-maintained libraries over custom implementations.

## Documentation

- A user-facing change (new feature, behavior change, bug fix) must add an
  entry under `[Unreleased]` in [CHANGELOG.md](CHANGELOG.md) in the same
  commit. Internal refactors, tests, and doc-only changes are exempt.
- Work that involved a durable design decision meeting at least one of these
  conditions (costly to reverse later, surprising without historical context, or
  chosen over a real alternative where a meaningful trade-off was made)
  must be recorded as an ADR under `docs/adr/` in the same commit. See
  [`docs/adr/README.md`](docs/adr/README.md) for the format and
  [CONTRIBUTING.md](CONTRIBUTING.md#documentation) for when an ADR is expected.

To understand why the project is the way it is, read in this order: this file,
then relevant ADRs under `docs/adr/`, then relevant `docs/reports/` and
`docs/planning/`, then the code.

## Development Commands

Run tests and formatting with the same settings as CI. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the exact commands.
