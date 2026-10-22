# D7 entry integration: implementation and validation

Date: 2026-10-22. Base: `e8dc10d`; branch: `refactor/255-entry-integration`.
Scope: [#255](https://github.com/Tunny-gh/Tunny-Dashboard/issues/255) under
[#248](https://github.com/Tunny-gh/Tunny-Dashboard/issues/248), following
ADR-0019/0020/0021. Implementation-owner evidence, not independent acceptance,
Issue closure or authorization to commit/push. D8 removal is excluded.

## Changed boundaries

| Files / area | Result |
| --- | --- |
| `egui-app/src/state/{app_state,filter,messages,types/study}.rs`, `ui/widget_states.rs` | Direct analysis DataFrame, feasibility, filter, indicator, result and surrogate types. Managers/inputs remain explicitly owned by the entry; no replacement store or compatibility alias. |
| `ui/chart/{poll_chart,render_chart}.rs`, `poll_chart/{cluster,compute,mcdm,surrogate}.rs`, affected widgets | Direct extracted analysis calls/types for numerical dispatch, statistics, projections and presentation. Existing preparations, acquired stamps, sharing, worker adoption and model dependencies are unchanged. Animation uses analysis helpers without changing its resource policy. |
| `io/csv_export/{clustering,distribution,history,importance,mcdm,surrogate}.rs` | Neutral CSV encoding comes from `tdashboard_io::csv`; numerical helpers/types come from analysis. Row/result selection remains caller composition. |
| `io/{report_export,session}.rs`, report/URL dialogs | Direct analysis report rendering/language and indicator types; URL/TLS helpers come from I/O. Reports still retain one input/projection, render pure content, then atomically write chosen destinations. Session JSON is unchanged. |
| `mcp-server/src/tools.rs` | Direct I/O scan-direction type and analysis renderer/Outcome. Existing request-local manager acquisition and report/trial computation already used extracted APIs. Tools, wire identity and selection policies are unchanged. |
| Entry manifests, `Cargo.lock` | Packages are `tunny-dashboard` and `tdashboard-mcp`. Executables remain `TunnyDashboard` and `tunny-mcp`, directories remain `egui-app` and `mcp-server`. No external dependency/version change. GUI core dependency is dev-only; MCP has none. |
| `egui-app/build.rs`, release workflow, Zed tasks/debug, README/CONTRIBUTING, process guide/roadmap | Correct package selectors and GUI license-metadata root. Preserve existing final-binary link/rpath, analysis-owned LightGBM/DLL copying, release layout and launch paths. CI manifest paths/flags need no change. |

Remaining GUI `tunny_core` references are test-only fixtures, including
`io/study.rs` (module gated by `cfg(test)`), legacy scan/chunk handlers, report
fixture construction, widget tests and the opt-in performance harness. Legacy
test/example scaffolding and `rust_core` facades remain for D8, not production
integration. Production-only entry compilation succeeds without a normal core
dependency. No new architecture, domain term, algorithm or execution policy was
introduced; no new ADR or internal-refactor CHANGELOG entry is needed.

## Direct evidence

- `egui-app/tests/entry_dependencies.rs` runs locked Cargo metadata and asserts
  all six approved normal/build workspace edges, traverses both production
  dependency closures to exclude core, checks unchanged binary target names,
  and distinguishes the temporary GUI dev dependency. Passed standalone and in
  the final workspace run. License collection tests also pass after the rename.
- New `d7_live_filter_session_and_csv_export_keep_trial_identity` opens a real
  Journal through the GUI entry, filters the acquired view, saves/restores filter,
  pin and indicator settings, then exports on the production worker. The retained
  selected row exports global ID **3**, Study number **2**, x **3**, objective **30**.
- The existing acquired-input report worker regression now writes **HTML,
  Markdown and JSON** while its manager advances to empty data. All formats are
  produced; JSON retains the original two COMPLETE/three total trials and finite
  front size one. This is not the legacy report fixture path.
- Retained D2/D3 live open/select/compare/reload/replacement/CSV/SQLite regressions,
  D4/D5 same-count stale success/error/cancellation and result-sharing/identity
  regressions, Artifact lifecycle/race tests, table/pin/filter tests and all CSV
  regressions pass. D6 process launch/progress/cancel/reload and mocked GH
  launch/completion/reload regressions pass through the current entry.
- New MCP Journal and opt-in SQLite tests launch the actual unchanged
  `tunny-mcp` executable and share nine-request protocol assertions: list,
  summary, JSON report, Japanese Markdown report, full/page/empty/zero-limit
  trials and unknown Study. They distinguish global IDs from local numbers,
  retain five COMPLETE of six target trials, use RUNNING constraint width three
  to reclassify historical rows, preserve a confirmed violation despite missing
  slots, verify objective-only report fallback and exclude infinite candidates.
  Nonfinite objectives/parameters/constraints appear as JSON null, and offset
  pages retain original trial correspondence. Existing initialization/tools-list,
  categorical report and limit-cap regressions pass.
- New rejected PostgreSQL/MySQL stdio requests cover all four tools and contain
  no raw password/URL. These are fail-closed TLS-policy errors, **not** successful
  live-server report/source masking evidence; the existing opt-in live-server
  test is retained but was not executed.

The SQLite fixture writer uses only Python 3's standard library, not a new Cargo
or Python dependency. Its reduced Optuna-shaped schema verifies composition and
wire behavior, not genuine Optuna producer compatibility. Journal scan-list count
metadata is not assumed identical to loaded/SQLite counts; loaded summary/report
assertions verify actual all-state counts without changing scan policy.

## Commands and results

Windows, Rust/Cargo **1.99.1**, existing workspace-root LightGBM DLL/import library:

```text
cargo fmt --manifest-path rust_core/Cargo.toml --all -- --check
cargo fmt --manifest-path egui-app/Cargo.toml --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --profile ci-test
cargo audit --ignore RUSTSEC-2026-0194 --ignore RUSTSEC-2026-0195
```

- Both formatting checks and all-target workspace Clippy: **passed**.
- Final workspace tests: **2868 passed, zero failed, 16 ignored**. Analysis
  701/3 ignored; I/O 99/3; MCP 16 unit + 4 integration/2 ignored; runner 102;
  Study 2+9; core 170+1+8+8+11+5+9/one ignored; GUI 861/2 ignored in each
  library/binary suite + one metadata integration; RDB integration 0/3 ignored.
  Doc tests passed (zero cases). Native harness's default skip is not a pass.
- Audit: **failed** on existing #269 `zip 2.4.2` advisories
  `RUSTSEC-2026-0367`, `RUSTSEC-2026-0368`, `RUSTSEC-2026-0369`, plus nine allowed
  warnings. No additional ignore or dependency correction.
- `cargo check -p tunny-dashboard -p tdashboard-mcp --bins`,
  `cargo build --locked --profile ci-test -p tunny-dashboard -p tdashboard-mcp --bins`
  and `target/ci-test/TunnyDashboard.exe --version`: **passed**; startup prints
  `TunnyDashboard 0.1.2`. Standalone metadata, new GUI filter/session/export and
  acquired report focused tests also passed. `git diff --check` passed.

Additional explicit opt-ins (PowerShell):

```powershell
$env:TUNNY_FIXTURE_PYTHON = (Get-Command python).Source
cargo test -p tdashboard-mcp --locked --profile ci-test --test integration d7_sqlite -- --ignored --nocapture
Remove-Item Env:\TUNNY_FIXTURE_PYTHON
$env:TUNNY_NATIVE_CAPTURE_CHECK = "1"
cargo test -p tunny-dashboard --locked --profile ci-test --test artifact_animation_native
Remove-Item Env:\TUNNY_NATIVE_CAPTURE_CHECK
```

SQLite stdio: **one passed**, Python **3.11.0**. Native full-app wgpu preview/GIF:
**passed**, three selected frames, matching live/GIF order, **684x544** content-only
capture, scale one, **330 ms**, full prefix observations, two independent instances,
seek/pause restoration and PNG/JPEG aspect ratios. The harness validates captured
frame colors/order and removes its temporary GIF; no manual screenshot approval
or checked-in screenshot artifact is claimed.

## Limits and documentation assessment

- No local macOS build/link/startup, Linux lint, release archive/signing or remote
  CI run. Commands above ran on Windows, not all CI platforms. Existing native
  provider/entry rpaths and unchanged release artifact paths were inspected;
  Windows production binaries and test binaries linked/started successfully.
- GUI smoke evidence is executable headless entry/egui checks plus the opt-in
  native capture, not manual file-dialog/mouse-driven visual approval. No licensed
  Rhino.Compute solver was used. Live PostgreSQL/MySQL success tests were not run:
  Docker's Linux engine pipe was unavailable. Other ignored/performance tests
  remain unverified, not passing evidence. No performance/parity claim.
- Development fixture assertions initially expected loaded counts from Journal's
  scan-only list metadata; corrected the test, not production scan semantics.
- External docs were checked read-only on `origin/dashboard-v0.2` (`89e05b1`),
  including v0.2 startup instructions and v0.1 fallbacks. They reference the
  preserved executable and no affected Cargo package/API; no edits warranted.
  Checkout remains `docs/245-artifact-animation`, with the pre-existing deletion
  `.claude/settings.local.json` unchanged.
