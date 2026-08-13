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
- Work that involved a non-trivial implementation decision must leave a note
  under `docs/handoff/` in the same commit, and update the index in
  [`docs/handoff/README.md`](docs/handoff/README.md). See
  [CONTRIBUTING.md](CONTRIBUTING.md#documentation) for the required format.

## Development Commands

Run tests and formatting with the same settings as CI. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the exact commands.
