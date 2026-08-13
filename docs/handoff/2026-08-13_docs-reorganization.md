# 2026-08-13: Documentation Reorganization

## Decision

The docs were scattered and partly stale: `README.md` mixed product marketing
with detailed build/test/format instructions that duplicated (and in one
Rust-version detail, contradicted) `CONTRIBUTING.md`; `ROADMAP.md` sat at the
repo root even though most of its items were done; there were two unrelated
`reports/` directories (root and `docs/reports/`) holding the same genre of
dated audit/investigation notes; and `docs/` had three loose `.md` files at
its root with no folder structure.

Decisions made, with rejected alternatives:

- **README vs. CONTRIBUTING split**: README keeps only the Installation
  section and a pointer to CONTRIBUTING; all build/test/format detail moved
  to CONTRIBUTING. Rejected: keeping a `cargo run --release` one-liner in
  README as a "Quick Start" — decided full separation was cleaner than a
  partial duplicate.
- **Stale CONTRIBUTING content**: while merging, found and removed dead
  `wasm-pack`/`frontend/`/Node.js references (leftover from before the egui
  migration — no `frontend/` directory exists) and a wrong workspace member
  name (`cargo build -p egui-wgpu`, not a real crate). Fixed the Rust version
  contradiction (README said 1.70+, CONTRIBUTING said 1.97.1+ with a reason)
  by keeping CONTRIBUTING's more specific claim.
- **CHANGELOG.md**: created new, Keep a Changelog format, `[Unreleased]`
  only. Explicitly did not backfill entries for the existing `v0.1.0`/`v0.1.1`
  tags, per instruction — starts clean going forward rather than
  reconstructing history from git log.
- **ROADMAP.md**: moved to `docs/planning/roadmap.md` with a freshness note
  at the top ("mostly `[x]` complete, treat as a decision record") rather
  than rewritten/trimmed down to only open items. Rejected the heavier-edit
  option because most doc-comment references in the Rust code
  (`rust_core/src/gh/*.rs`, `process/*.rs`) cite "ROADMAP item N" as a
  concept, not a path — moving and annotating was lower-risk than rewriting
  content those comments implicitly point at.
- **reports/ consolidation**: merged the root `reports/` (Python
  cross-validation, egui-app/rust_core quality reviews) into `docs/reports/`
  (upgrade plans, audits, gap analysis). No functional or naming reason was
  found for keeping them apart — same genre, same dated-prefix convention.
- **docs/ root folders**: split into `guides/` (user-facing how-tos),
  `planning/` (roadmap), `reports/` (existing), `handoff/` (new — this file
  is one). Named `guides/` over `integration/` since future guides may not
  all be integration-related.
- **CHANGELOG/handoff enforcement**: added rules to `AGENTS.md` (binding on
  agents) and `CONTRIBUTING.md` (detail, for human contributors) requiring a
  CHANGELOG entry for user-facing changes and a handoff note for non-trivial
  implementation decisions. These are **prose rules, not CI-enforced** — see
  Open Items.

## What changed

- `README.md`: removed Quick Start / Prerequisites / Build / Testing /
  Formatting sections; added a "Building from Source" pointer to
  CONTRIBUTING.md under Installation; updated the repo-structure comment and
  the two links into `docs/guides/`.
- `CONTRIBUTING.md`: merged README's build/test/format content, removed the
  stale wasm-pack/frontend/Node.js/wrong-member-name content, restructured
  into Prerequisites → Building from Source → Development Commands →
  Documentation → Releasing. Releasing now includes a CHANGELOG-update step.
- `CHANGELOG.md`: new file, Keep a Changelog format, empty `[Unreleased]`.
- `AGENTS.md`: added a `## Documentation` section with the two binding rules;
  updated the `ROADMAP.md` path reference.
- `docs/roadmap.md` (formerly `ROADMAP.md`) → `docs/planning/roadmap.md`,
  with a freshness note added; internal `../CHANGELOG.md` link fixed to
  `../../CHANGELOG.md` for the new depth.
- `docs/process-integration.md`, `docs/tunny-plugin-integration.md` →
  `docs/guides/`; fixed a dangling `(ROADMAP prerequisite ...)` cross-reference
  in `tunny-plugin-integration.md` to link to the roadmap's new path (found
  during review — the original text survived the move as prose, not a link,
  so it wasn't caught until content, not just links, was checked).
- Root `reports/` (3 subdirectories, 26 files) moved into `docs/reports/`.
- `docs/handoff/README.md`: new index file — lists the required 3-part
  handoff structure (Decision / What changed / Open Items) and an index
  table to be updated alongside every new handoff file.

## Open Items

- **Enforcement is prose-only.** Nothing in CI checks that a PR touching
  `egui-app/**` or `rust_core/**` actually added a `CHANGELOG.md` entry, or
  that a substantial change added a handoff note. The repo already has a
  precedent for this kind of check (`release.yml` fails the build if the tag
  doesn't match `egui-app/Cargo.toml`); an equivalent PR-time check was
  discussed but intentionally not built — the user hasn't asked for it yet.
- All of this session's changes are uncommitted in the working tree.
