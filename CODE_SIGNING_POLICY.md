# Code signing policy

This project signs release artifacts. This page documents how, following the
requirements described in the
[SignPath Foundation Code of Conduct for Open Source projects](https://github.com/SignPath/Website-old/blob/v2/src/drafts/oss_policy.md).

## Windows — SignPath Foundation (application pending)

Tunny Dashboard has applied for a free code signing certificate through the
[SignPath Foundation](https://signpath.org) open-source program.

**Status:** Application pending. No certificate has been issued yet, so
Windows builds are **not currently signed**. The build pipeline
([`.github/workflows/release.yml`](.github/workflows/release.yml) and
[`.signpath/artifact-configurations/default.xml`](.signpath/artifact-configurations/default.xml))
is already in place; signing itself stays inactive until the SignPath project
is approved and its secrets/variables are configured — see
[CONTRIBUTING.md](CONTRIBUTING.md#code-signing-windows).

Once approved, this page will carry SignPath's required attribution
statement:

> Free code signing provided by [SignPath.io](https://signpath.io),
> certificate by [SignPath Foundation](https://signpath.org).

### What is signed

- `TunnyDashboard.exe`, distributed inside the Windows `.zip` package on
  [GitHub Releases](https://github.com/Tunny-gh/Tunny-Dashboard/releases).

### Build and signing process

- Artifacts are built from this repository's source using GitHub Actions
  ([`release.yml`](.github/workflows/release.yml)); no locally or manually
  built binary is ever submitted for signing.
- Only CI-built artifacts are submitted to SignPath for signing.
- The private key is held by SignPath (HSM-backed); this project never has
  access to it.

### Team roles (single-maintainer project)

- **Authors** (commit access, can modify the repository without additional
  review):
  - [@hrntsm](https://github.com/hrntsm)
- **Reviewers** (review required for changes proposed by non-authors, e.g.
  pull requests):
  - [@hrntsm](https://github.com/hrntsm)
  - Policy: all external pull requests are reviewed by the maintainer before
    merge.
- **Approvers** (approve each signing request):
  - [@hrntsm](https://github.com/hrntsm)
  - Policy: each signing request requires explicit approval by the
    maintainer.

## macOS

Signed ad-hoc (`codesign -s -`), not with an Apple Developer ID, and not
notarized — this project does not hold an Apple Developer Program membership.
See the [Installation section of the README](README.md#macos-apple-silicon-only)
for what this means for users.

## Linux

No Linux artifacts are currently built or distributed.

## Distribution locations

- <https://github.com/Tunny-gh/Tunny-Dashboard/releases>

## Privacy policy

This program will not transfer any information to other networked systems
unless specifically requested by the user or the person installing or
operating it.
