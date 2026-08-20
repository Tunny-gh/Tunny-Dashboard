# Contributing to Tunny Dashboard

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable 1.97.1 or later — the `libsqlite3-sys` dependency requires Rust stable 1.97.1+, so run `rustup update` if your toolchain is older)

Install Rust if not already present:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### LightGBM Library Setup

The sensitivity analysis features (SHAP, MDI, RF-ANOVA) and 2D PDP use LightGBM via a pre-built native library. These binaries are **not included in the repository** and must be placed manually before building.

#### Required files

| File | Platform | Description |
|---|---|---|
| `libs/lib_lightgbm.dll` | Windows | Shared library |
| `libs/lib_lightgbm.lib` | Windows | Import library for the linker |
| `libs/lib_lightgbm.dylib` | macOS | Shared library |

Place the files directly under the `libs/` directory at the project root:

```
Tunny-Dashboard/
└── libs/
    ├── lib_lightgbm.dll   (Windows)
    ├── lib_lightgbm.lib   (Windows)
    └── lib_lightgbm.dylib (macOS)
```

#### Where to download

Download the pre-built binaries from the [LightGBM GitHub Releases](https://github.com/microsoft/LightGBM/releases):

**Windows**

1. From the release assets, download `lib_lightgbm.zip`
2. Extract and copy `lib_lightgbm.dll` and `lib_lightgbm.lib` into `libs/`

**macOS**

The recommended way is to install via Homebrew, which provides the correct arm64 binary:

```bash
brew install lightgbm
cp $(brew --prefix lightgbm)/lib/lib_lightgbm.dylib libs/
# Bundle libomp (a LightGBM runtime dependency) alongside it so libs/ is
# self-contained and does not depend on Homebrew staying installed.
cp $(brew --prefix libomp)/lib/libomp.dylib libs/
chmod u+w libs/lib_lightgbm.dylib libs/libomp.dylib
```

After copying, fix the install name so the dynamic linker can find the library relative to the binary (this step is required once per copy):

```bash
install_name_tool -id "@rpath/lib_lightgbm.dylib" libs/lib_lightgbm.dylib
# Point the libomp dependency at the sibling copy via @loader_path.
install_name_tool -change \
  "@rpath/libomp.dylib" \
  "@loader_path/libomp.dylib" \
  libs/lib_lightgbm.dylib

# install_name_tool invalidates the code signature; re-sign ad-hoc so macOS
# will load the dylibs (otherwise binaries linking them are killed with SIGKILL).
# The Homebrew bottles are already signed, so --force is needed to re-sign.
codesign --force -s - libs/libomp.dylib
codesign --force -s - libs/lib_lightgbm.dylib
```

Alternatively, download from [LightGBM GitHub Releases](https://github.com/microsoft/LightGBM/releases) — make sure to pick the **arm64** asset (e.g. `LightGBM-*-macos-arm64.tar.gz`) and apply the same commands above.

> **Why are these commands needed?**
> The Homebrew-installed dylib embeds its Homebrew install path as its install name; the build system looks for the library via `@rpath`, so the name must be rewritten once after copying into `libs/`. Rewriting the binary invalidates its code signature, and macOS refuses to load (and SIGKILLs) a process that links an improperly signed dylib — `codesign -s -` re-signs it ad-hoc to fix this.

> The `lib_lightgbm.def` and `lib_lightgbm.exp` files sometimes bundled in LightGBM releases are not required.

## Building from Source

### Development Build

```bash
cargo run -p tunny-desktop
```

Builds and runs the desktop application in debug mode.

### Release Build

```bash
cargo build --workspace --release
```

Builds an optimized release binary. The executable will be at:

- Windows: `target/release/TunnyDashboard.exe`
- Linux/macOS: `target/release/TunnyDashboard`

### Build Specific Workspace Members

```bash
# Build only the core library
cargo build -p tunny-core

# Build only the desktop app
cargo build -p tunny-desktop

# Build only the MCP server
cargo build -p tunny-mcp
```

### Runtime (Windows)

On Windows, `lib_lightgbm.dll` must be discoverable at runtime. The `build.rs` script automatically copies it into the Cargo target directory during build, so `cargo test` and `cargo run` work out of the box. For a standalone binary, place `lib_lightgbm.dll` alongside the executable.

## Development Commands

CI is defined in [`.github/workflows/ci.yml`](.github/workflows/ci.yml). Commands marked **CI** are run by CI with the exact flags shown below; unmarked commands are local-only conveniences.

### Run Tests

**CI** (see the `build-test` job; runs on Windows/macOS with the `ci-test` profile):

```bash
cargo test --workspace --locked --profile ci-test
```

Local only (quicker feedback, uses the default dev profile):

```bash
cargo test -p tunny-core
cargo test -p tunny-desktop
```

Run tests with output:

```bash
cargo test -- --nocapture
```

### Formatting

Always run these before committing. **CI** (see the `lint` job):

```bash
cargo fmt --manifest-path rust_core/Cargo.toml --all -- --check
cargo fmt --manifest-path egui-app/Cargo.toml --all -- --check
```

Local only:

```bash
cargo fmt -p tunny-mcp -- --check
```

To apply formatting instead of just checking it, drop `-- --check` from any of the above.

### Static Analysis

Always run this before committing. **CI** (see the `lint` job; runs on Linux, `--all-targets` is required so that test code is also covered by clippy):

```bash
cargo clippy --workspace --all-targets --locked -- -D warnings
```

### Security Audit

**CI** (see the `audit` job; requires `cargo-audit` to be installed locally):

```bash
cargo audit --ignore RUSTSEC-2026-0194 --ignore RUSTSEC-2026-0195
```

### Run the Application

```bash
cargo run -p tunny-desktop
```

### Benchmark

```bash
cargo bench -p tunny-core
```

### GUI Verification

The startup beta notice blocks the window on a machine that has not dismissed it
yet (including clean environments and CI). Pass the undocumented
`--no-beta-notice` flag when driving the app for screenshots or automated checks:

```bash
cargo run -p tunny-desktop -- --no-beta-notice -i path/to/study.db
```

## Documentation

`docs/` is organized by purpose, not by feature:

| Folder | Purpose |
| --- | --- |
| `docs/guides/` | User-facing how-to guides (process integration, Grasshopper/Tunny integration) |
| `docs/planning/` | Forward-looking strategy (`roadmap.md`) |
| `docs/reports/` | Dated, one-off audits and investigations (quality reviews, cross-validation, gap analyses) — filename or subfolder prefixed `YYYY-MM-DD_` |
| `docs/handoff/` | Dated implementation records of what was decided, what was done, and what's left — same `YYYY-MM-DD_short-topic.md` naming as `docs/reports/` |

Two things must be updated alongside the code change itself, not deferred to a
later cleanup pass:

### CHANGELOG.md

A user-facing change (new feature, behavior change, bug fix) adds an entry
under `[Unreleased]` in [`CHANGELOG.md`](CHANGELOG.md), in the same commit as
the change. Internal refactors, test-only changes, and doc-only changes don't
need an entry — the changelog is for people deciding whether to upgrade, not
a commit log.

### docs/handoff/

Work that involved a non-trivial implementation decision (not a one-line fix)
gets a note under `docs/handoff/`, in the same commit as the change. Each
handoff file covers three things:

1. **Decision** — what was decided and why, including alternatives that were
   rejected and the reason. This is the part CHANGELOG.md can't capture.
2. **What changed** — what was actually implemented.
3. **Open Items** — what's left, or `None.` if the work is fully wrapped up.

Adding a handoff file and updating the index table in
[`docs/handoff/README.md`](docs/handoff/README.md) is one edit, not two
separate steps — an entry missing from the index is as good as not existing.

## Releasing

Releases are built by [`.github/workflows/release.yml`](.github/workflows/release.yml),
which is triggered by pushing a `v*` tag.

1. Move the [`CHANGELOG.md`](CHANGELOG.md) `[Unreleased]` entries under a new
   `[<version>]` heading and add a fresh empty `[Unreleased]` section above it.
2. Bump `version` in `egui-app/Cargo.toml` and commit the updated `Cargo.lock`
   together with the changelog update.
3. Tag the commit as `v<version>` — the tag must match the crate version exactly.
4. Push the tag.

Step 2's version bump is not optional. The startup beta notice records the
version the user dismissed it for (`CARGO_PKG_VERSION`), so a release that
reuses the previous version number would never show the notice again to
existing users. The release workflow verifies the tag against
`egui-app/Cargo.toml` and fails on a mismatch.

### Code signing (Windows)

`TunnyDashboard.exe` is Authenticode-signed via the [SignPath Foundation OSS
program](https://signpath.io/solutions/open-source-community), using the
[`signpath/github-action-submit-signing-request`](https://github.com/SignPath/github-action-submit-signing-request)
action in the `package` job of `release.yml`. macOS is unaffected — it is
still signed ad-hoc as described in the README, since the project has no
Apple Developer Program membership.

Signing runs only for `v*` tag pushes on `Tunny-gh/Tunny-Dashboard`; forks and
`workflow_dispatch` test builds skip it and ship an unsigned exe. It needs, in
the repository's Actions settings:

| Name | Kind | Value |
| --- | --- | --- |
| `SIGNPATH_API_TOKEN` | Secret | API token for a SignPath user with the `Submitter` role on the `release-signing` policy |
| `SIGNPATH_ORGANIZATION_ID` | Variable | The SignPath organization ID (GUID) |

`project-slug` (`tunny-dashboard`), `signing-policy-slug` (`release-signing`),
and the artifact-configuration slug (`default`, from
[`.signpath/artifact-configurations/default.xml`](.signpath/artifact-configurations/default.xml))
are hardcoded in `release.yml` — they must match the names created in the
SignPath project settings when the OSS application is approved. See
[`docs/handoff/2026-08-20_signpath-code-signing.md`](docs/handoff/2026-08-20_signpath-code-signing.md)
for how the pieces fit together and what's still pending.
