# Changelog

All user-facing changes to Tunny Dashboard — new features, behavior changes,
bug fixes — are documented in this file. Internal refactors, tests, and
doc-only changes are not (see [CONTRIBUTING.md](CONTRIBUTING.md#documentation)
for the full rule).

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Entries are grouped under `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`,
and `Security` as needed — omit headings that have nothing under them.

## [Unreleased]

### Added

- CI infrastructure to Authenticode-sign Windows release builds via the
  SignPath Foundation OSS program. Inactive until the SignPath project is
  approved and its secrets/variables are configured (see
  CONTRIBUTING.md#code-signing-windows); until then, Windows builds remain
  unsigned as before.

[Unreleased]: https://github.com/Tunny-gh/Tunny-Dashboard/compare/v0.1.1...HEAD
