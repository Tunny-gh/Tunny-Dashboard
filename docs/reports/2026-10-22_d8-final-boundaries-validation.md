# D8 final responsibility boundaries: validation

Date: 2026-10-22. Base: `081cc5a`; branch:
`refactor/256-remove-legacy-core`. Scope: [#256](https://github.com/Tunny-gh/Tunny-Dashboard/issues/256)
under [#248](https://github.com/Tunny-gh/Tunny-Dashboard/issues/248).
This is implementation-owner evidence, not independent acceptance, Issue closure,
or authorization to commit/push. D1–D7 were closed when inspected with `gh`.

## Removal and retained coverage

- Removed `rust_core`, its package, global/thread-local Study stores, active-Study
  accessors, entry facades and native build script. GUI test messages now carry
  local tables explicitly; their global-store serialization guard is gone.
- Removed I/O's temporary `DecodedTable`, Journal bulk table/finalizer APIs,
  SQLite aliases and RDB table conversion APIs. CSV returns decoded acquisition
  rows; live `into_batch`, Journal replay, database read sessions and analysis
  projections are unchanged. Writer readback now asserts domain records directly.
  Acquisition-only decoder rows/extras are not installed authorities or analysis
  tables. ADR-0021 now distinguishes prerequisite paired APIs from the final API.
- Moved cross-boundary D2–D4, ingestion, DataFrame, export and #258 regressions to
  `egui-app/tests/`, above I/O and analysis. Test-local managers and observations
  replace global installation; the actual extracted readers/projections execute.
  Journal ID extraction/count and storage-error tests moved directly to I/O.
  Checked-in Journal fixtures and producer probes moved with their consumers;
  real-file tests require the relocated files rather than silently skip them.
- Moved numerical examples and three benchmarks to analysis, RDB CLI to I/O,
  composed report CLI and ingestion benchmark to the GUI package. All-target
  Clippy compiles these targets. No benchmark measurement/performance claim.
- The 18-test difference from D7's 2868 passing cases is limited to retired APIs:
  11 store implementation tests, three core facade/compile tests, custom column-JSON
  parser and elapsed-time wrapper tests, and the missing-active-Study fixture test
  compiled in both GUI suites. Column/value behavior remains covered by DataFrame
  tests. Snapshot tests retain actual rows-API interleaving/error checks; duplicate
  invocations through removed table wrappers are gone. No unrelated test pruning.
- Updated manifests/lockfile, CI and CONTRIBUTING to `cargo fmt --all -- --check`,
  benchmark selectors, README layout, Zed fixture paths, release link comment and
  Git fixture inclusion. Existing dependency versions, native provider, binaries
  `TunnyDashboard`/`tunny-mcp` and entry directories remain unchanged. No intended
  user-facing change or new durable architecture decision: no CHANGELOG/new ADR.

## Parent criteria cross-check

| Parent criterion / child | Concrete final evidence |
| --- | --- |
| Design gate, D1/#249 | Accepted ADR-0019/0020/0021; no new ownership, algorithm, readiness or runner policy selected here. |
| Six packages and boundaries, D8 | `tests/entry_dependencies.rs` runs locked Cargo metadata, requires exactly the six approved workspace packages, every ADR-0021 normal/build edge, acyclicity, unchanged binary targets, no core in metadata/closures, no Study workspace dependency of any kind and no I/O–analysis test edge. |
| Explicit authority, revision and reload, D2/#250 | Study's 2 unit + 9 management tests; `study_slice` (9), `d3_io` (8), and live `d2_journal_live_open_select_reload_replace_and_headless_projection`, stale same-path loader, reload failure and comparison-order/sharing tests. |
| Initial/incremental and semantic equivalence, D3/#251 | `dataframe` (33), `journal_projection` (64), `csv_projection` (11), `sqlite_projection` (21), `d3_csv_attribute_boundary` (1) and #258 (5 passing/1 ignored). Includes all-state width growth, missing/nonfinite slots, category identity, terminal attributes, source order and retained inputs. SQLite interleaved snapshot/error tests and writer round trips run inside I/O. |
| Explicit computation and stale same-count results, D4/#252 | `d4_analysis` (8), `d4_preparation` (11); GUI `d4_entry_rejects_same_count_revision_load_epoch_and_source_results`, `d4_publication_invalidates_same_count_schema_results_without_resetting_controls`, fit provenance and cached-selection/rerun regressions. Analysis's 701 passing numerical/native tests remain active. |
| Conditions-specific sharing and trial identity, D5/#253 | GUI `d5_typed_completed_and_inflight_results_share_payload_and_trial_maps`, `d5_cluster_and_mcdm_reject_stale_success_error_and_cancellation`, last-computation coloring, table/cluster/MCDM filtering and rank-label regressions. Equal conditions share; unequal conditions and stale success/error/cancel remain distinct. |
| Artifact lifetimes, D3/D5/D7 | I/O scan tickets, path containment/MIME/CSV tests; GUI source/root scan races, reload/manual-root precedence, study-switch cards, Animation source/seek/cache tests. Explicit native check below also exercises migrated fixtures. |
| Both execution approaches, D6/#254 | Runner's 102 tests plus GUI `d6_process_launch_progress_cancel_completion_and_reload` and `d6_gh_launch_mock_compute_completion_and_reload`. Mock Compute only, not a licensed live solver. |
| Product entry behavior, D7/#255 | `d7_live_filter_session_and_csv_export_keep_trial_identity`, acquired-input report worker, MCP 16 unit + 4 normal stdio integration tests and explicit SQLite opt-in below. IDs, constraints, pagination/nonfinite JSON and fail-closed credential errors retained. See the [D7 report](2026-10-22_d7-entry-integration-validation.md) for scenario details and policy limits. |

Current implementation search found no old package consumer, store management,
ambient active-Study computation or compatibility re-export. The old package name
in the architecture rejection assertion and historical dated documents is not a
consumer. No functional integration gap requiring an owning child to reopen was
found. Parent security/platform validation still has the limitations below;
this report does not assert that every parent checkbox or remote CI is green.

## Commands and actual results

Windows, Rust/Cargo **1.99.1**, existing workspace-root LightGBM DLL/import library:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --profile ci-test
cargo audit --ignore RUSTSEC-2026-0194 --ignore RUSTSEC-2026-0195
cargo metadata --locked --format-version 1
git diff --check
```

- Formatting and all-target Clippy: **passed**. Workspace tests: **2850 passed,
  zero failed, 16 ignored**. Analysis 701/3 ignored; I/O 111/3; MCP 16+4/2;
  runner 102; Study 2+9; GUI 860/2 in each lib/bin suite; GUI boundary suites
  11+1+8+8+11+33+1+13+5+64+21+9; RDB integration 0/3 ignored. Doc tests: zero
  cases, successful. Native harness's default skip is not passing native evidence.
- Audit: **failed**, existing #269 `zip 2.4.2` advisories `RUSTSEC-2026-0367`,
  `RUSTSEC-2026-0368`, `RUSTSEC-2026-0369`, plus nine allowed warnings. No new ignore
  or unrelated dependency correction. Lockfile changes only workspace ownership.
- Initial workspace test attempt hit the tool's 120-second timeout during build;
  rerun used 600 seconds. A migrated writer assertion initially confused declared
  metadata with observed constraint evidence; corrected the test to assert the
  actual recorded constraint presence/values, not production semantics.
- Locked metadata independently printed all six packages and the exact approved
  normal/build adjacency list. `git diff --check`: **passed**. Production binaries
  built with `cargo build --locked --profile ci-test -p tunny-dashboard -p tdashboard-mcp --bins`;
  `target/ci-test/TunnyDashboard.exe --version` printed **TunnyDashboard 0.1.2**.
- Final workspace format/Clippy/test log retained locally as
  `C:\Users\dev\.local\share\opencode\tool-output\tool_1629265c2001NQ3NWzOw4DiJoG`.

Additional opt-ins (environment variables removed after each command):

```powershell
$env:TUNNY_NATIVE_CAPTURE_CHECK = "1"
cargo test -p tunny-dashboard --locked --profile ci-test --test artifact_animation_native
$env:TUNNY_FIXTURE_PYTHON = (Get-Command python).Source
cargo test -p tdashboard-mcp --locked --profile ci-test --test integration d7_sqlite -- --ignored --nocapture
```

Native full-app wgpu preview/GIF: **passed**, three selected frames, identical
live/GIF order, **684x544**, scale one, **330 ms**, full prefix observations, two
instances, seek/pause restoration and PNG/JPEG aspect ratios. Temporary GIF removed
by the harness; no manual screenshot approval claimed. MCP SQLite: **one passed**.

No local macOS/Linux build/lint/link/startup, release archive/signing, remote CI,
live PostgreSQL/MySQL success run, licensed Rhino.Compute or performance probe.
Ignored checks remain unverified, not passing evidence. Existing final-executable
link/rpath handling stays with entries/runner; analysis owns numerical native
linking and DLL deployment, including its relocated example/benchmark targets.

## External documentation (read-only)

Inspected `origin/dashboard-v0.2` at `89e05b1` in the external docs repository.
Its English/Japanese v0.2 startup instructions reference the unchanged
`TunnyDashboard.exe -i ... -a ...`; no old Cargo package, Rust API or fixture path
reference requiring D8 changes was found. **No external edits warranted**.
Checkout remains `docs/245-artifact-animation`; the pre-existing deletion of
`.claude/settings.local.json` was not touched. Historical internal reports retain
their original paths/commands as dated evidence, not current contributor guidance.
