# Dependency security exposure and release assessment

Date: 2026-11-03

## Scope and conclusion

Source review covers the workspace, the exact locked registry sources, and the
`v0.1.2` release tag. No exploit execution, release-binary disassembly, or review
of all unrelated dependency advisories was performed.

The four reported ZIP extraction vulnerabilities have no identified reachable
call path in this application. They warrant dependency cleanup in #269, but do
not, by themselves, justify an emergency application release. This is a source
reachability assessment, not a claim that the old ZIP crate is patched or that
the complete application has no security issues.

The XML update has a stronger user-facing release rationale: the application
parses user-supplied GHX files with the affected attribute iterator. Ship that
fix in a normal near-term patch, especially when users exchange GHX files with
others. The ZIP dependency need not delay that release.

## ZIP dependency and API reachability

The locked dependency path is:

```text
tdashboard-analysis
  -> egobox-gp 0.36.4 / egobox-moe 0.35.5
  -> ndarray-npy 0.9.1 (default NPZ features)
  -> zip 2.4.2
```

- No workspace Rust source imports `zip`, `ndarray_npy`, `NpzReader`, or
  `ZipArchive`, or calls the ZIP extraction APIs.
- In the locked egobox sources, active `ndarray_npy` imports occur in
  `#[cfg(test)]` modules, examples, and benchmarks. Dependency test modules are
  not enabled by testing or building this workspace. The application uses
  egobox for in-memory GP training and prediction in
  `crates/analysis/src/gaussian_process.rs`.
- Even ndarray-npy's NPZ reader uses `ZipArchive::new`, `by_name`, and `by_index`
  to read array data. It does not call filesystem extraction APIs. This is an
  additional distinction, not an application feature: the application does not
  expose NPZ reading in the first place.
- Searches at `v0.1.2` likewise find no application ZIP/NPZ reader or extractor;
  the same egobox dependency versions are declared there.
- Distributing a Windows ZIP package is separate from runtime ZIP extraction
  by the Rust application; the user's archive utility is outside this call path.

| Advisory | Required behavior | Application assessment |
| --- | --- | --- |
| [RUSTSEC-2026-0367](https://rustsec.org/advisories/RUSTSEC-2026-0367.html) | Extract attacker-controlled Unix permission bits, particularly in a privileged process. | No ZIP extraction call identified. |
| [RUSTSEC-2026-0368](https://rustsec.org/advisories/RUSTSEC-2026-0368.html) | Extract a symlink targeting outside the destination and later access it. | No ZIP extraction call identified. |
| [RUSTSEC-2026-0369](https://rustsec.org/advisories/RUSTSEC-2026-0369.html) | Apply Unix permissions through an extracted symlink. | No ZIP extraction call identified. |
| [RUSTSEC-2026-0429](https://rustsec.org/advisories/RUSTSEC-2026-0429.html) | Extract a symlink with an attacker-declared huge size, causing allocation failure and process abort. | No ZIP extraction call identified, on Windows or macOS. |

All four advisories require ZIP >=9.0.0. Updating only the application's egobox
version does not currently provide that dependency path. Reassess reachability
if archive import, NPZ import, model loading, or dependency behavior changes.
Do not add audit exclusions based solely on this assessment.

## Other updates and release relevance

- `quick-xml` 0.38.4 -> 0.42.0 for GHX parsing. The parser calls
  `BytesStart::attributes()` with default duplicate checks. A crafted tag with
  many attributes before a requested attribute, or without it, reaches the
  quadratic iterator behavior described in
  [RUSTSEC-2026-0194](https://rustsec.org/advisories/RUSTSEC-2026-0194.html).
  The attack requires the application to parse the supplied GHX file; it is a
  CPU denial of service, not evidence of code execution or data disclosure.
  The same parser and dependency are present in `v0.1.2` under `rust_core/`.
- The GHX parser uses plain `Reader`, so the `NsReader` allocation behavior in
  [RUSTSEC-2026-0195](https://rustsec.org/advisories/RUSTSEC-2026-0195.html)
  is not an identified GHX call path. Updating `wayland-scanner` to 0.31.11
  and `zbus_xml` to 5.2.1 also removes the old transitive XML versions, allowing
  both historical audit exclusions to be removed.
- `libz-sys` 1.1.29 -> 1.1.31 addresses
  [RUSTSEC-2026-0418](https://rustsec.org/advisories/RUSTSEC-2026-0418.html).
  This concerns Unix build-script environment logging, not ZIP runtime input.
  The advisory requires `cc` >=1.6.0; both the reviewed lockfile and `v0.1.2`
  lock `cc` 1.2.65, so that prerequisite is absent in these locked builds.
  No secret exposure is inferred from the audit finding alone.

## Validation

- `cargo test --workspace --locked --profile ci-test`: 1,943 passed,
  16 ignored, zero failures on Windows.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed
  on Windows; Linux/macOS compilation is left to CI.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- `cargo audit` without exclusions: still fails on the four ZIP advisories,
  with 12 allowed warnings. XML and libz-sys vulnerability findings are removed.
- GHX coverage includes Unicode, escaped attribute names and text, numeric
  character references, and empty items. No malicious ZIP regression test was
  added because no application ZIP extraction behavior exists to exercise.
