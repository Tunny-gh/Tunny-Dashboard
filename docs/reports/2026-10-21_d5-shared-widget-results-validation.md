# D5 shared widget results: implementation and verification

Date: 2026-10-21. Base: `5b349c9` (main including PR #273).
Branch: `refactor/253-shared-widget-results`.
Scope: [#253](https://github.com/Tunny-gh/Tunny-Dashboard/issues/253) under
[#248](https://github.com/Tunny-gh/Tunny-Dashboard/issues/248), implementing
ADR-0021's accepted ownership, provenance, correspondence and lifetime contracts.
This is implementation-owner evidence, not independent review, Issue closure,
commit, push, PR creation or merge. The existing security audit is **not passing**.

## Changed boundaries

| Area | Working boundary |
| --- | --- |
| `crates/analysis/src/{results,shared}.rs` | Shared payload definitions live in analysis. Typed retained results own a session handle, shared payload and explicit full-view TrialKey correspondence. The handle also retains the preparation's subset maps. Caller-selected entries contain handles, not independently copied cluster/MCDM/sensitivity results. |
| `crates/analysis/src/{projection,preparation}.rs` | Projection DataFrames are Arc-owned; the live GUI and session share the same immutable columns. Feasible sensitivity selection still constructs its existing filtered table; the unfiltered path shares the projection. |
| `egui-app/src/state/analysis_job.rs` | Workers publish typed shared cluster, MCDM, sensitivity, heatmap, Sobol and trained-model payloads. Adoption checks every acquired input stamp and current run key before publication. One-shot entry notifications still use the checked delivery envelope; they do not become another shared result cache. |
| `state/app_state.rs`, `state/message_handler.rs` | AppState's shared-result maps are selections of typed analysis handles. Cluster/MCDM adoption additionally checks the full-view TrialKey order. Most-recent MCDM coloring retains its own handle separately from condition-specific selections. Raw shared-result fixture deliveries are not a production fallback. |
| `state/types/study.rs`, `state/message_handler/study.rs` | A live view retains its analysis projection and finalized Pareto handle. Final ranks borrow that payload instead of copying it; loading padding remains display-only. Production publication no longer writes a global DataFrame/extras/active-selection mirror. |
| `ui/chart/render_chart.rs`, `io/csv_export{,/history}.rs` | Timeline, Intermediate Values and their CSV availability/output read the displayed view's explicit extras, not ambient active extras. Encoding and output destinations remain entry/I/O work. |
| Canvas orchestration | Pending interests are keyed by consumer instance and delivery slot, not only chart kind. Equal-condition requests share a computation; unequal requests from two instances of the same chart cannot replace each other's adoption target. Runtime completions update only their requesting instance. Established surrogate model/output reuse for Pareto overlays and model-only stages is preserved separately, without broadcasting busy/error/settings state. Input changes invalidate each canvas instance's derived runtime state without synchronizing settings. |
| Interest release | Live widget selections, displayed inputs/fronts, retained fits, most-recent coloring and pending consumers supply session interests. Removed consumers and obsolete settings release lookup entries. An abandoned late completion cannot leave an indefinitely pending cache entry. External handles remain stamped and usable until dropped. |

The numerical algorithms, constraint/category/missingness policies, MCDM weight
normalization/quantization, display sorting and outside-target behavior are
unchanged. Display lookup reuses completed handles without starting work. Existing
manual cluster/MCDM requests can rerun; joining matching in-flight work shares its
run. This does not add automatic recomputation, a universal fit cache or automatic
settings synchronization. No dependency versions, manifests, package names or
executables changed.

The legacy format facades and memory-probe example were adjusted only for the
projection's Arc<DataFrame> boundary. Owned legacy return APIs unwrap or clone at
that boundary; live GUI views do not deep-copy the table. Execution extraction,
remaining entry imports/package renames and mechanical legacy-package removal
remain D6, D7 and D8 respectively. Test-only global-store fixtures remain test-only.

## Direct regression evidence

| Contract | Evidence |
| --- | --- |
| Equal-key in-flight/completed sharing | `d5_typed_completed_and_inflight_results_share_payload_and_trial_maps` exercises both cluster and MCDM, verifies identical handles for two consumers, session lookup identity and retained source/subset maps. `d5_canvas_consumers_share_equal_conditions_and_keep_unequal_jobs_independent` dispatches real cluster workers through production poll/adoption paths. |
| Different conditions and per-instance runtime | The canvas regression dispatches equal and unequal k settings from two instances of the same chart. For unequal settings it holds one completion, verifies the other consumer alone finishes, then delivers the second. Plot rendering and consumer removal/lookup release are exercised. |
| Table/plot/Gallery reuse | `d5_mcdm_rank_table_and_gallery_share_the_retained_result` drives production Rank, table and Gallery request paths, proves one pending run key and shared completed/coloring handle, checks all requesting busy states and renders the three live consumers with egui. |
| Same-size stale success/failure/cancellation | `d5_cluster_and_mcdm_reject_stale_success_error_and_cancellation` covers 30 combinations: both operations, success/error/cancelled completion, and same-count revision, reload, source replacement, same-source different Study and rerun. Rejected work cannot populate either result selection. Cancellation uses the analysis failure-completion protocol; no new Cluster/MCDM Cancel UI is added. |
| Full/subset/trial correspondence | New fixtures distinguish global IDs 90+ from gapped Study numbers 5, 8, 11, etc. Missing features exclude ID 92 from the prepared cluster subset while its full-view position retains label -1. Full-view maps are checked at adoption. Existing cluster-matrix, table filter/pin/order, multi-Study Gallery/card and MCDM Top-N/ranking/label regressions remain active. |
| Pareto ownership | Existing D2 primary/comparison handoff regressions now assert shared session DataFrame and finalized front identity, including rank-buffer identity. Legacy global-publication assertions were replaced with explicit session/view assertions, not removed without replacement. |
| Coloring/condition isolation and release | `d5_coloring_keeps_the_last_computation_not_the_table_selection` retains distinct condition results, checks the table can still select the earlier result while coloring uses the later one, then uses Weak observation to prove an uninterested payload frees after its external handle drops. |
| Fits and stale-model rejection | Extended `d4_entry_fit_retains_actual_input_and_condition_provenance` runs a real Ridge fit, verifies retained typed session payload, finite-row TrialKeys and optimize dependency key, then advances the same-count input and proves the live model-request adapter rejects the stale fit without executing optimization. `d5_trained_model_reuse_does_not_finish_another_instances_fit` verifies production fit delivery shares the actual model Arc with existing consumers while preserving an unrelated instance's busy state and diagnostic. |
| No unrelated broadcast | `d5_global_deliveries_do_not_finish_unrelated_canvas_requests` verifies current global success and caught panic do not finish an unrelated canvas request. Existing stale busy-slot, cache-selection, rejection fanout and fit-cancellation regressions still run. |

These are executable headless egui smoke checks, not a claim of manual visual
inspection on every platform. Native Animation has separate evidence below.

## Review remediation: one-shot notification fanout (2026-10-22)

Independent review found a P2 gap in the one-shot notification path. Selecting a
new run rebinds every pending consumer of that AnalysisKey to its latest run key,
but `share_delivery` only supported part of the worker notification set. Its
optional return and `filter_map` silently omitted unsupported success deliveries.
With two same-condition PDP requests, the older completion was correctly rejected
but the latest success only reached one canvas consumer. The other retained its
busy state and never received its result. The direct 1D and 2D PDP regressions
both **failed before remediation** on that missing second completion.

The correction retains the existing rerun/rebinding and stale-result checks:

- Every currently produced one-shot analysis notification now has a fanout path:
  1D/2D PDP, convergence, observed contour, single/multi surrogate optimization,
  single/multi suggestion, comparison, their failure notifications and
  `AnalysisUnavailable`. Existing cluster/MCDM/entropy notifications and fit
  failure/cancellation paths remain covered. Typed shared successes still use
  their retained-handle path rather than this notification adapter.
- Fanout is non-optional: it no longer uses `filter_map` to discard an interested
  consumer. An unsupported protocol variant is an explicit invariant failure,
  not a silently missing delivery.
- A consumer delivery carries the selected delivery slot as well as its instance
  ID. After adoption has accepted the current run/input, the entry clears that
  consumer's slot before applying the notification. Generic errors therefore
  finish their PDP 2D busy state too, without clearing another slot's pending
  work. No stale completion is made adoptable to clear a spinner.

`d5_pdp_1d_rerun_delivers_success_to_both_rebound_canvas_consumers` and its 2D
counterpart dispatch two actual PDP workers through the production poll/entry
path, then deliver old/latest completions in both orders. Both consumers must
receive the latest numerical payload; 1D consumers also receive their cache
entries. Stale completion delivery cannot clear pending work or overwrite the
adopted result.

The analogous-operation audit adds direct notification/payload regressions for
convergence, contour, optimize, multi-optimize, suggest, multi-suggest and compare,
plus failure-text preservation and retargeted unavailable slots. These test the
notification adapter and live message handlers, not a rerun of every numerical
algorithm. A separate keyed generic-error regression uses two canvas consumers
and verifies the unrelated Importance slot stays busy. All five additional tests
run alongside the existing D5 stale-result and sharing regressions.

## Commands and outcomes

Windows, existing native LightGBM prerequisites, Rust/Cargo 1.99.1. Exact parent
checks:

```text
cargo fmt --manifest-path rust_core/Cargo.toml --all -- --check
cargo fmt --manifest-path egui-app/Cargo.toml --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --profile ci-test
cargo audit --ignore RUSTSEC-2026-0194 --ignore RUSTSEC-2026-0195
```

- Both formatting checks and all-target workspace Clippy: **passed**.
- Workspace tests: **2843 passed, zero failed, 15 ignored**; zero-case doc tests
  passed. Breakdown: analysis 701/3 ignored; I/O 96/3 ignored; Study 2+9; core
  259 plus integrations 1+8+8+11+5+9 (one ignored probe); desktop 858/2 ignored in
  each library/binary suite; preseeded RDB integrations 0/3 ignored; MCP 16+2/1
  ignored. Moving 31 payload tests from each desktop suite into analysis changes
  the aggregate count; tests were relocated rather than pruned.
- Audit: **failed** on the known `zip 2.4.2` advisories
  `RUSTSEC-2026-0367`, `RUSTSEC-2026-0368`, `RUSTSEC-2026-0369`, with nine allowed
  warnings. This remains #269; no dependency or additional ignore was changed.

Focused commands:

```text
cargo test -p tunny-desktop --lib --locked --profile ci-test d5_ -- --nocapture
cargo test -p tunny-desktop --lib --locked --profile ci-test d4_entry_fit -- --nocapture
cargo check --workspace --all-targets --locked
git diff --check
```

The focused desktop runs passed twelve and one tests respectively. The intentional
caught-panic diagnostic in the global-delivery test is not a test failure.
Development compilation/lint failures and three old global-publication assertions
were corrected before the passing runs; they are not presented as passing evidence.

Native PowerShell verification:

```powershell
$env:TUNNY_NATIVE_CAPTURE_CHECK = "1"
cargo test -p tunny-desktop --locked --profile ci-test --test artifact_animation_native
Remove-Item Env:\TUNNY_NATIVE_CAPTURE_CHECK
```

The opt-in full-app wgpu preview/GIF check **passed**: three selected frames,
identical live/GIF order, 684x544 content-only capture, scale 1, 330 ms timing,
full prefix observations, two independent instances, seek/pause restoration and
PNG/JPEG aspect ratios. The normal workspace native skip is not counted as this
pass. No Animation resource policy was changed.

## Limits and documentation assessment

- No local Linux Clippy or macOS build/native run; these exact commands ran on
  Windows. No new CI/PR run was requested. Ignored live-server/performance checks
  were not rerun or counted as passing; no Rhino.Compute instance was used.
- No latency, memory-limit, lock-free, database or numerical-accuracy claim.
- This implements ADR-0021, without a new durable design choice or new domain
  term. No ADR or CONTEXT.md change is needed.
- Behavior-preserving ownership migration does not need a CHANGELOG entry or
  external user-manual change. The external docs target is
  `origin/dashboard-v0.2`, not a nonexistent `origin/v0.2`; its v0.2 changed-page
  overlays and v0.1 fallback remain applicable. Existing stable-number/ranking
  descriptions remain correct. No external files were edited, including the
  preexisting deletion `.claude/settings.local.json`.
- Independent review is still required. This report does not close #253 or
  authorize committing/pushing the working tree.
