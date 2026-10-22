# D6 optimization-runner extraction: implementation and verification

Date: 2026-10-22. Base: `8ba71cd`; branch: `refactor/254-optimization-runner`.
Scope: [#254](https://github.com/Tunny-gh/Tunny-Dashboard/issues/254) under
[#248](https://github.com/Tunny-gh/Tunny-Dashboard/issues/248), implementing
ADR-0002/0003/0015/0019/0020/0021. This is implementation-owner evidence, not
independent acceptance, Issue closure, a commit, push, or PR. D7/D8 are excluded.
The known #269 security audit failure remains unresolved.

## Changed files and public boundaries

| Files | Responsibility/change |
| --- | --- |
| `Cargo.toml`, `Cargo.lock`, `crates/optimization-runner/Cargo.toml` | Add workspace library `tdashboard-optimization-runner`; move execution dependencies out of core. External dependency versions are unchanged. |
| `crates/optimization-runner/src/runner/mod.rs` | Relocate the generic Random/NSGA-II loop, evaluator/problem/configuration types, recording and summary. Existing `prepare_run`/`run_prepared` API and behavior remain distinct from GH execution. |
| `crates/optimization-runner/src/process/{mod,definition,evaluator,extract,substitute}.rs` | Relocate process definitions, substitution, command execution/retries/timeouts/hooks, extraction and evaluator integration. |
| `crates/optimization-runner/src/gh/{mod,adaptive,compute,compute_def,compute_server,ghx,problem,runner}.rs`, `gh/runner/{config,recorder,summary,tests,validate}.rs` | Relocate GHX parsing/injection, Compute transport/process lifecycle, GH Random/NSGA-II/CMA-ES/BO loops, recording, diagnostics and regressions. |
| `crates/optimization-runner/src/gh/compute_run.rs` | Move the GUI's definition preparation/sidecar persistence and Compute-target lifecycle into execution: `prepare_compute_run`, `PreparedComputeRun::run`, `ComputeLaunchConfig`. Preserve synchronous preparation/error order, best-effort sidecar write, local-only definition pointer, startup timeout/cancellation and server handle lifetime through the run. |
| `crates/optimization-runner/src/{lib,test_support}.rs` | Document public execution modules; test-only recorded readback composes I/O batches, a caller-owned StudyManager and analysis projection, never core. |
| `crates/optimization-runner/build.rs` | Retain final execution-test binary Unix link/rpath handling. Analysis remains the sole numerical/native-library provider and Windows DLL-copy owner; resolve workspace-root `libs/`. |
| `crates/optimization-runner/src/process/windows_tests.rs` | Add Windows subprocess counterparts to retained Unix shell integrations. |
| `egui-app/Cargo.toml`, `src/app/{files,run,tests}.rs` | Import execution directly for definition opening, both live launches, and headless application smoke tests. GUI chooses settings, owns worker/progress handles and displays results; Compute orchestration no longer lives in GUI. |
| `egui-app/src/state/{app_state,messages}.rs`, `state/message_handler/{gh,study,tests}.rs`, `state/types/{gh_dialog,process_dialog,tests}.rs`, `ui/widgets/common/{ghx_opt_modal,process_def_modal}.rs` | Direct execution types for run settings/completions and analysis `FitProgress` for live run cancellation/progress. Preserve modal/overlay/completion policies and tests. |
| `egui-app/src/io/{journal,study}.rs` | Project recorded results into screen metadata directly from analysis, removing the old-core metadata conversion from live publication/loading. Actual recorded Journal acquisition/reload continues through I/O batches and caller-owned Study managers, not execution or legacy orchestration. |
| `rust_core/Cargo.toml`, `src/lib.rs`, `src/io/journal/mod.rs` | Remove execution modules/dependencies and the unused Journal writer facade. Original `src/{runner,process,gh}` files and `src/io/journal/writer.rs` are removed, not retained as delegates. Other legacy core facades remain for D7/D8. |
| `README.md`, `CONTRIBUTING.md`, `docs/guides/process-integration.md`, this report | Current source layout, runner build/native prerequisites, library import example and verification evidence. |

Execution calls public analysis RNG/optimizer/model/acquisition/`FitProgress` APIs
and the I/O Journal writer directly. There is no execution-to-core dependency,
analysis-to-I/O edge, duplicate optimizer implementation, new Python/Optuna runtime
dependency, merged runner, new sampler or storage-semantic change. Algorithms,
seed derivation, cancellation checks, counters, penalty tiers, bounds/rounding,
IDs, error text and supported objective combinations were retained. The new
Compute launch API relocates existing entry orchestration, not a new execution
policy. No entry package or executable is renamed.

Locked Cargo metadata was parsed and the direct workspace edges (including dev
dependencies) asserted:

```text
tdashboard-study -> none
tdashboard-io -> tdashboard-study
tdashboard-analysis -> tdashboard-study
tdashboard-optimization-runner -> tdashboard-analysis, tdashboard-io, tdashboard-study
```

Source searches find no `tunny_core` import or legacy numerical/I/O facade access
in the new crate, and no remaining `tunny_core::{gh,process,runner}` caller.
The CI workspace commands already cover the added member; no workflow/manifest
path or CI flag change is needed.

## Direct regression evidence

| Contract | Executed evidence |
| --- | --- |
| Generic seeds, combinations, bounds and precision | `runner::tests::seeds_bounds_rounding_and_method_objective_combinations` runs Random/NSGA-II with one/two objectives, same/different seeds, integer bounds and decimal precision. Point comparisons ignore parallel recording order, not parameter values. |
| Penalty versus recorded values and identity | `penalty_fitness_does_not_replace_recorded_values_or_global_ids` checks actual returned penalty fitness, readback of true negative objectives and positive constraints, and appended distinct Study IDs with global trial IDs unlike local Study numbers. Relocated GH penalty/CMA regressions retain true-value recording, direction, budgets, saved start, seed and precision checks. |
| Generic cancellation and Journal failures | `cancellation_and_journal_failures_stop_new_evaluations` covers both methods, pre-run/in-evaluation cancellation, prepare/open failure, injected begin/finish write failures and readback of the retained RUNNING prefix. A one-thread pool makes the permitted in-flight count deterministic. |
| GH combinations, bootstrap/retraining/convergence | Relocated `both_bo_methods_run_single_and_multi_real_evaluator_loops`, `bo_multi_deploys_explicit_gp_without_auto_reports`, single/multi adaptive recording, range and monotonic diagnostics tests; `adaptive_stops_early_on_convergence`; CMA single-objective/min/max/error/cancel tests and multi-objective rejection before Journal creation. The existing tests use Rust synthetic evaluators, not an external licensed solver. |
| GH cancellation/Journal failures | Relocated bootstrap failure, fit-boundary cancellation, begin/finish Journal error, pre-cancellation and CMA in-flight cancellation/error readback tests. |
| Process retry/timeout/hooks/extraction | Four new `windows_*` tests execute actual PowerShell children: three observed retry attempts and preserved stderr/exit status, timeout/kill, pre/main/post sequencing plus file JSON extraction and real recorded Journal readback, hook and missing-output failure. Existing pure substitution/CSV/JSON/regex/definition tests also run. |
| Compute integration/readback | `mocked_compute_run_records_success_and_http_failure` runs the GH loop through the real HTTP client against a local TCP mock, records one COMPLETE true-value constrained trial and one FAIL on HTTP 500, then reads both back. Existing request/pointer/Gene Pool/type/semaphore tests remain active. |
| Compute launch/preparation | New `preparation_preserves_error_order_and_best_effort_definition_write` and `local_launch_failure_preserves_prepared_journal` test invalid definition/config ordering, sidecar failure without preparation failure, remote pre-cancellation and missing local EXE without modifying prepared Journal records. |
| Live GUI process run | `d6_process_launch_progress_cancel_completion_and_reload` invokes production launch, an actual external command, shared progress/cancellation handle, message pump and automatic recorded-result reload. It exercises successful and cancelled runs, renders the production overlay headlessly before/after completion, and verifies accepted rows/progress and new acquisition generation. Cancellation uses the same handle as the UI button, not a simulated mouse click. |
| Live GUI GH run | `d6_gh_launch_mock_compute_completion_and_reload` invokes production preparation/launch and completion handling against an HTTP mock; verifies injected sidecar, actual request parameter name, recorded objective, progress and automatic generation-advancing reload. The minimal archive/mock does not establish Rhino solving validity. |

## Exact commands and results

Executed from the workspace root on Windows with Rust/Cargo **1.99.1** and the
existing LightGBM DLL/import-library prerequisites:

```text
cargo fmt --manifest-path rust_core/Cargo.toml --all -- --check
cargo fmt --manifest-path egui-app/Cargo.toml --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --profile ci-test
cargo audit --ignore RUSTSEC-2026-0194 --ignore RUSTSEC-2026-0195
```

- Both exact formatting checks: **passed**.
- Exact all-target workspace Clippy: **passed** (Windows, not Linux).
- Final workspace tests: **2857 passed, zero failed, 15 ignored**; doc tests
  passed with zero cases. Breakdown: analysis 701/3 ignored; I/O 96/3 ignored;
  optimization-runner 99/0 ignored; Study 2+9; core 170 plus integrations
  1+8+8+11+5+9 (one ignored probe); desktop 860/2 ignored in each library/binary
  suite; preseeded RDB integration 0/3 ignored; MCP 16+2/1 ignored.
- Audit: **failed**, reporting existing `zip 2.4.2` advisories
  `RUSTSEC-2026-0367`, `RUSTSEC-2026-0368`, `RUSTSEC-2026-0369`, with nine allowed
  warnings. This remains #269; no dependency version or additional ignore changed.

Additional executed commands:

```text
cargo check --workspace --all-targets
cargo test -p tdashboard-optimization-runner --locked --profile ci-test
cargo test -p tunny-desktop --lib --locked --profile ci-test d6_ -- --nocapture
cargo metadata --locked --format-version 1 --no-deps
git diff --check
```

The focused execution run passed 97 tests before the final two preparation/lifecycle
tests were added; all **99**, including those two, passed in the final workspace
run. The focused desktop run passed both D6 tests; both also passed in each final
workspace desktop suite. Metadata edge assertions and diff whitespace checks
passed. Windows execution tests, desktop and MCP binaries linked and ran with
the existing native provider. No Linux/macOS build or remote CI run is claimed.

Development attempts exposed fixture/compiler mistakes (direction defaults,
default NSGA-II versus a two-trial Random budget, private GUI view access, and
duplicate Study names described below); these were corrected before the final
passing baseline. They are not represented as passing attempts or production
algorithm fixes.

## Limitations and out-of-scope finding

- No licensed Rhino.Compute/Rhino/Tunny plugin instance was used in this task. Local
  EXE startup, real readiness, successful lifetime/termination and actual GHX
  solving are not certified by the HTTP mocks or missing-EXE regression.
- GUI checks are executable **headless** entry/message/egui-render smoke tests,
  not interactive visual approval or mouse-driven Run/Cancel/Reload verification.
  The opt-in native Artifact Animation check was not run; the workspace harness
  explicitly skipped it. No Animation behavior/resource path was changed.
- The seven retained Unix subprocess tests are `cfg(unix)` and did not execute
  on Windows; their exclusion is not a pass or part of the ignored count.
  Windows counterparts directly exercise the relevant real-process failure paths.
  Ignored live-database/performance tests remain unverified, not passes.
- A development append/readback fixture reproduced an existing duplicate-name
  mismatch: two `prepare_run` calls with the same Study name return Study IDs 0
  and 1, but reading ID 1 returns `study_id 1 not found`. The I/O writer allocates
  IDs without rejecting duplicate names; Journal parser `ParserState` deduplicates
  CREATE_STUDY names (`crates/io/src/io/journal/parser/state.rs`). The final
  identity test uses distinct names. This pre-existing writer/parser policy is
  **not fixed** or silently treated as intended semantics by this extraction;
  it needs a separately scoped follow-up.

## Documentation and handoff

No new durable architectural decision or domain term was needed: the new APIs
implement ADR-0021's accepted boundaries. No ADR/CONTEXT.md edit is required.
This is behavior-preserving internal extraction, so no CHANGELOG entry is added.

The external documentation repository received a read-only D6 inspection of
fetched `origin/dashboard-v0.2` at `8c8a549` (the v0.2 target is `dashboard-v0.2`,
not `v0.2`). The English/Japanese process pages contain GUI instructions and JSON
examples, not the local process guide's Rust-library example; v0.2 serves those
pages through its v0.1 fallback. The GH pages and all tracked target-branch
content contain no matching moved package/API/source-path references or Rust
examples requiring D6 updates. No external documentation changes are required
or were made. The existing checkout remains on `docs/245-artifact-animation`,
with its pre-existing deletion of `.claude/settings.local.json` preserved.
The local process guide's import example is updated here.

No unresolved important specification/design choice blocks D6. D7 still owns
remaining GUI/MCP import composition and package integration; D8 owns retirement
of the remaining old core. Neither is implemented here. Independent review,
cross-platform CI, any real Compute/interactive acceptance, and commit/PR work
remain with the lead.
