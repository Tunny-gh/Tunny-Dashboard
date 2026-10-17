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

## Reference Implementations

When implementation details or expected behavior are unclear,
consult the relevant upstream projects:

- [Optuna](https://github.com/optuna/optuna)
- [Optuna Dashboard](https://github.com/optuna/optuna-dashboard):
- [Rustuna](https://github.com/optuna/rustuna):

Use these projects as references, not requirements.
Prioritize this project's requirements and existing decisions.
Do not add features solely for parity with upstream projects.

## Code, Tests, Commits, and Comments

- **Code — How:** Make the implementation clear from the code itself.
- **Tests — What:** Specify the expected behavior, not the implementation details.
- **Commit messages — Why:** Explain the reason for the change, not just what changed.
- **Code comments — Why not:** When needed, explain non-obvious constraints or why
  an otherwise plausible approach was not taken. Do not restate the code.

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
