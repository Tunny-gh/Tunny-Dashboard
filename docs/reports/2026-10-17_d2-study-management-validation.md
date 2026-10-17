# D2 Study management: implementation verification

Date: 2026-10-17. Scope: [#250](https://github.com/Tunny-gh/Tunny-Dashboard/issues/250)
under [#248](https://github.com/Tunny-gh/Tunny-Dashboard/issues/248), based on
`ab71b8e`. This records local implementation evidence, not Issue closure or an
independent review. Architecture remains governed by ADR-0019/0020/0021 and the
[design validation handoff](2026-10-16_study-boundary-design-validation.md).

## Working boundary and changed files

- `crates/study/Cargo.toml`, `src/{lib,model,manager}.rs` and
  `tests/management.rs`: all-state records, source-scoped typed identity, shared
  feasibility/discrete-eligibility rules, validated single-writer management,
  checked counters, immutable acquired headers/roots and Arc/COW retention.
- Root `Cargo.toml`/`Cargo.lock`, `rust_core/Cargo.toml`, `egui-app/Cargo.toml`:
  add the Study workspace member and its consumers; no external dependency
  versions change. `cargo tree -p tdashboard-study --locked --edges normal
  --depth 1` shows only serde and serde_json, no workspace dependency.
- `rust_core/src/io/journal/parser.rs`, `parser/{study,study_projection}.rs`,
  `parser/distribution.rs`, `src/data/dataframe/{model,feasibility}.rs`,
  `src/multi_objective/pareto.rs` and `pareto/ranking.rs`: owned Journal batches,
  COMPLETE-only derived tables/history, shared domain rules and an explicit-input
  wrapper around the unchanged Pareto implementation. Tests are in
  `rust_core/tests/study_slice.rs`.
- `egui-app/src/state/{mod,app_state,journal_studies,messages,message_handler}.rs`,
  `state/message_handler/study.rs`, `src/app.rs`,
  `app/{files,reload,dialogs,tests}.rs`, `src/io/{journal,study_worker}.rs`:
  caller-owned Journal managers, source/scan rejection, acknowledged batch
  delivery, explicit projection consumption, and live open/select/reload paths.
  Remove the legacy Journal ingestion/installation entry; retain a bytes-only
  serial scan barrier while unmigrated storage loaders can still write globals.

The GUI owns the Journal bytes and per-Study managers; replay workers emit owned
batches and wait for application receipts. Initial publication is an authoritative
prefix (including the empty case). Later batches are coherent full-record upserts.
The first reload publication starts from an empty candidate, not the old display.
Read-only acquired inputs survive later updates without holding a lock or a
database transaction. Active selection remains GUI state.

The table/history path derives from the selected manager input, not a global
snapshot. The GUI temporarily publishes DataFrame/extras mirrors for unmigrated
consumers only. A regression replaces the global mirror with an empty frame and
then reselects the Study successfully from its manager.

## Regression evidence

Initial D2 verification used Windows with Rust 1.97.1 and the local LightGBM
DLL/import library. The PR #262 section below records the later Rust 1.99.1 rerun.
Focused suites contain 10 Study tests, nine headless
Journal tests and eleven `d2_` GUI orchestration tests, plus a comparison-handoff
unit test for failure/reset cleanup.

| Design case | Direct coverage |
| --- | --- |
| M1 all-state authority | All five states retain recorded parameters, indexed objectives, JSON attributes, constraints, times and intermediate values. Non-COMPLETE fields do not add table columns. Typed categories, null versus absence, unavailable values and numeric nonfinite values remain distinct. |
| M2 identity/order | Distinct source namespaces, reused IDs/equal row counts, global IDs unlike gapped Study numbers, out-of-creation-order completion, and separate full-read versus streaming order. Replaced-source scans/batches/errors cannot populate the current GUI slot. |
| M3 batch integrity | Wrong ticket/source/membership, stale base, duplicate/future sequence, duplicate IDs/numbers/ordinals and invalid record shapes leave the published root and expected sequence unchanged. Checked counter exhaustion cannot publish or wrap. |
| M4 schema updates | Non-COMPLETE constraint growth with unchanged COMPLETE row count reclassifies historical rows. Updating an existing non-COMPLETE record leaves total record count unchanged. Metadata-only constrained presence advances revision while sharing the directory. Incremental/bulk batches agree. |
| M5 no-op/completion/empty | Stable NaN representation does not force endless revisions. No-op batches consume sequence but not data revision; finish changes load epoch. Valid empty/attribute-only Studies preserve existing column discovery. Empty reload removes old rows. |
| M6 reload failures | Manager and GUI tests retain the old-stamped display before first acceptance, and the new accepted prefix after later failure. Old generation work cannot be relabeled or accepted into a replacement. |
| M7 retention/release | Held inputs retain their values across updates; later acquisitions see new revision data. Unchanged records/maps/JSON values share Arc handles. Weak root/directory/record references expire after old consumers drop. Independent Study inputs remain valid; a later activation after storage reload rereads new bytes rather than treating an old cached input as refreshed. |

The headless harness uses exactly the owned-batch/manager/projection interfaces
used by the GUI. Both shipped Journal fixtures additionally compare numeric
tables, labels, identity/order and feasibility with the legacy streaming output.
GUI tests drive actual open/select/reload actions and the production message pump;
the loaded table and optimization-history widgets render and tessellate in egui.

### Comparison-order review corrections

Review identified two existing-behavior regressions that the initial tests missed.
Both are corrected within D2, without changing the accepted per-path ordering:

- **Request/display order and colors:** GUI comparison requests reserve an ordered
  presentation handoff. A later worker may finish first, but its derived view waits
  until earlier requests complete or fail. Publication assigns colors in request
  order, as the former serial worker did. Failure releases a slot without adding
  a view/color; source reload and comparison-session reset discard pending views.
- **Rows within a comparison:** `parser/study.rs::read_full` uses the same supported
  before-terminal replay, returns one authoritative all-state batch, and assigns
  COMPLETE acquisition ordinals in the legacy full-reader's global-trial-ID order.
  Only unloaded/forced comparison acquisitions use it. Primary acquisition remains
  streaming. A comparison reusing an already acquired streaming input retains its
  existing row order and stamp rather than relabeling or sorting that input.

`d2_comparison_reload_preserves_request_order_and_full_read_rows_on_reverse_completion`
opens a real three-Study Journal, requests a 1,001-row B followed by a two-row C,
then reloads through the production GUI actions/message pump. The test intercepts
real worker messages and withholds B's finish until C has actually completed;
it does not rely on sleeps or relative worker speed. C cannot take B's display or
color slot. After B's finish, display order/colors remain B/C and both comparison
tables match legacy full-read trial IDs, Study numbers and objective values.

`d2_comparison_cached_streaming_input_is_reused_but_forced_reload_uses_full_read_order`
first loads B as the primary Study with reversed completion order, then checks
that adding it as a cached comparison preserves that streaming order/stamp.
Reload must reacquire it in full-read order with a new generation. The headless
`full_read_retains_all_state_authority_and_legacy_returned_row_order` additionally
checks all-state retention and schema alongside the full-read projection.

### Source replacement, column discovery and final-artifact review corrections

Subsequent review found that independent Journal scanning could race with the
unmigrated storage worker, and that rebuilding a sorted table lost append-time
column order. It also found `items_after_test_module` in `journal_studies.rs`;
the earlier report's all-target clippy success claim did not match that artifact.
The final checks below supersede that claim.

- **Storage replacement:** every legacy worker delivery now carries its dispatch
  `source_generation`. The entry rejects old generations before handling load
  chunks, cached activations, comparison results, scans, CSV associations or errors.
  Filtering messages alone would not protect mirrors: SQLite/RDB extras and
  comparison loads, CSV installation and cached activation still mutate the legacy
  global store inside the serial worker. Journal's bytes-only scan therefore uses
  that same worker as a barrier behind earlier storage writes. Journal replay and
  Study authority remain caller-owned, not installed by that worker. #251 owns
  removal of this temporary acquisition barrier with the legacy write boundary.
- **Streaming column discovery:** the caller owns a `TableProjection` per Study.
  It retains only an empty derived schema, then rebuilds values from the acquired
  input using existing `DataFrame::append_trials`. This preserves parameter,
  numeric/string attribute and physical column discovery order across batches.
  A different Study/generation or older input revision starts a fresh layout;
  future columns cannot leak into a retained old input. Full-read acquisition
  starts with its existing sorted layout. No trial values become a second source
  of authority, and the projection bridge is still assigned to #252.
- **Final lint/test isolation:** the test module is now after the implementation,
  without a lint suppression. A focused run also exposed interference between D2
  GUI tests sharing the production core store. Those tests now use the existing
  common desktop-test guard, rather than weakening mirror assertions or disabling
  test parallelism for the workspace.

`d2_replaced_sqlite_callbacks_cannot_overwrite_a_completed_journal_load` obtains
real SQLite worker chunks, cached selections, forced comparison results and errors,
holds their delivery, completes a new Journal load with reused Study/trial IDs,
then delivers the old callbacks. The current view, input stamp, DataFrame/extras
mirrors, comparison list, loading flag and error presentation remain on the Journal.
The same dispatch boundary was traced for RDB and CSV; no live server database
race or backend snapshot guarantee is claimed from the SQLite fixture.

`d2_streaming_gui_preserves_column_discovery_order_across_the_batch_boundary`
loads 1,000 COMPLETE `z` rows followed by the first `a` row through actual GUI
delivery. Parameters remain `[z, a]`, and all physical columns, numeric/string
attribute order and values match the legacy streaming frame. The headless
`streaming_projection_retains_discovery_order_but_resets_layout_on_full_reload`
also checks metadata-only final delivery, old-input isolation and full-reload
layout reset.

### Overlapping primary/comparison acquisition lifecycle

Review found that selecting B while its comparison load was in flight superseded
that operation without resolving its reserved comparison slot. Later comparisons
could then wait indefinitely. The reverse overlap could also supersede a primary
load and leave its loading state tied to the worker's original role.

Normal requests now share the same Study's current-scan Loading acquisition rather
than calling `begin` again. They retain its ticket and original full-read/streaming
order. Completion has no immutable primary/comparison role flag: it publishes to
the current selected slot and resolves a pending comparison interest as applicable.
Both views share the same derived DataFrame. Primary invalidation happens before
comparison handoff so it cannot erase the just-appended comparison history slot.

The related failure/supersede paths were checked as part of this correction:

- A valid failure resolves any comparison reservation and clears the selected
  load's busy flag, regardless of which interest started the worker. Partial or
  retained old display data is not adopted as a successful comparison.
- A forced acquisition still creates a new ticket; it transfers/reset any existing
  comparison reservation in place, including one started by a primary request.
  Rejected old batches and stale finish/error delivery cannot resolve the new slot.
- Cached activation requires the manager's current operation to be Complete. The
  subsequent correction below applies it locally, without queueing a GUI message.
- Existing comparison reset and source invalidation clear pending handoffs; source
  invalidation also supersedes loader tickets. No new cancellation policy is added.

Two production-path regressions hold B's real first batch before acknowledgement:
`d2_comparison_in_flight_then_primary_selection_shares_the_study_load` and
`d2_primary_in_flight_then_comparison_request_shares_the_study_load`. They add the
opposite interest, assert that the ticket is unchanged and no second worker starts,
then deliver through the production message pump. They check loading recovery,
both views' shared DataFrame and original row order, reservation release, addition
of C and re-adding B after returning to A.

`d2_shared_journal_failure_retry_and_forced_supersede_resolve_comparison_interests`
exercises both overlap orders. A real batch rejection propagates through the
worker's fallible sink into failure and releases an already-ready C. Retrying B
then forcing a primary replacement transfers B's reservation; the old batch and
finish are rejected while the new ticket completes and the queue remains usable.
No sleeps or assumed worker completion order establish these cases.

### GUI-local cached activation and bounded-channel send audit

Review identified a blocking self-send: the GUI called `JournalStudies::request`,
whose cached branch synchronously sent `JournalActivate` into the GUI's own
capacity-32 channel. A full backlog prevented that same thread from draining it.

The cached branch now returns owned local activation work to its caller. The
GUI applies it through the existing message handler and reload handoff directly;
it does not send, discard, defer or offload that activation. Loading and shared
comparison reservation handling remain the same, and cached inputs retain their
stamps. New/in-flight requests still return without waiting for worker delivery.

The other newly introduced GUI-thread sends were inspected:

- Scan startup's allocator-error branch also sent an error to the same bounded
  channel. `dispatch_scan` now returns that error directly; initial startup, Open
  and Reload set their existing error/loading presentation locally.
- Journal batch/finish sends and legacy storage-result/scan sends run in worker
  threads, where bounded backpressure can be released by the GUI's message pump.
- GUI storage-command dispatch uses the existing unbounded command channel, not
  the bounded GUI delivery channel. The GUI's per-batch acknowledgement uses a
  separate fresh capacity-one channel, with one response per delivered batch;
  it cannot wait for space in the GUI's own backlog. That protocol is unchanged.

`d2_cached_journal_selection_and_comparison_complete_with_a_full_gui_channel`
loads Studies through the real GUI path, fills all 32 delivery slots, and keeps
the receiver undrained while invoking cached primary selection and cached
comparison actions. Both views are adopted immediately, the channel remains full,
stamps stay unchanged, loading clears and the comparison reservation resolves.
Draining the original backlog afterward does not change those results. A test-only
watchdog frees a slot only on regression, allowing a failed blocking-send test to
terminate rather than hang the suite; its timeout is not a product latency target.

After the final cached-activation code and test edits, both exact format checks, the exact
all-target workspace clippy command and the full locked `ci-test` workspace run
succeeded on Windows. The desktop library and binary suites each report 861 passed
/ 1 ignored; the headless Journal slice reports nine passed; all eleven `d2_` GUI tests
pass in the focused run. Existing environment-gated tests remain ignored. Native
capture and standalone startup below are prior source/column-review evidence, not
rerun evidence for the lifecycle or cached-activation follow-ups. The audit remains unavailable.

### PR #262: CI stable-toolchain correction

The lint log for run `39023091705`, job `117368650833`, uses Rust 1.99.1
(`58cf8f9c3`) and rejects `AtomicU64::fetch_update` as deprecated under
`-D warnings`. The same failure was reproduced locally after aligning default
stable to that version. The sole production change is the method-name replacement
with `try_update`: the standard library implements the old name as a direct alias
to it. The `checked_add`, both Relaxed orderings, returned previous namespace and
`CounterExhausted` mapping are unchanged, preserving atomic uniqueness and no-wrap
failure semantics without a custom CAS loop or lint suppression.

`try_update` has been stable since Rust 1.95.0. The documented 1.97.1 baseline
therefore needs no increase. A named 1.97.1 installation was used for verification;
default stable remains 1.99.1 and no repository toolchain configuration was added.

Final local verification on Rust 1.99.1 / Windows:

- Both exact manifest-based formatting checks in the table below passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed.
- `cargo test -p tdashboard-study --locked --profile ci-test` passed: 10 tests.
- `cargo test --workspace --locked --profile ci-test` passed, retaining the
  existing ignored/opt-in tests. Each desktop suite reports 861 passed / 1 ignored.
- `cargo +1.97.1 test -p tdashboard-study --locked --profile ci-test` and
  `cargo +1.97.1 clippy -p tdashboard-study --all-targets --locked -- -D warnings`
  passed on the documented baseline.

Ubuntu lint was not rerun locally, and this unpublished correction cannot change
the existing remote lint conclusion. The final inspection of run `39023091705`
showed both macOS and Windows build/test jobs successful, with no other non-audit
failure beyond the reproduced lint error. Security audit, dependencies and workflows were
explicitly excluded from this repair and were not changed or investigated.

## Commands and outcomes

| Command | Outcome |
| --- | --- |
| `cargo fmt --manifest-path rust_core/Cargo.toml --all -- --check` | Passed. |
| `cargo fmt --manifest-path egui-app/Cargo.toml --all -- --check` | Passed. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed on Windows; Linux CI was not run locally. |
| `cargo test --workspace --locked --profile ci-test` | Passed. Existing opt-in/ignored tests remain ignored; this is not evidence of real RDB/Compute environments. |
| `cargo audit --ignore RUSTSEC-2026-0194 --ignore RUSTSEC-2026-0195` | Attempted; unavailable (`cargo-audit` is not installed). Not a passing audit. |
| `cargo test -p tdashboard-study --locked --profile ci-test` | Passed. |
| `cargo test -p tunny-core --locked --profile ci-test --test study_slice` | Passed. |
| `cargo test -p tunny-desktop --locked --profile ci-test --lib d2_` | Passed, also exercised in the workspace run. |
| `cargo test -p tunny-desktop --locked --profile ci-test --lib d2_comparison` | Passed: two actual GUI comparison-path regressions, including forced reverse delivery. |
| `cargo test -p tunny-desktop --locked --profile ci-test --lib comparison_failure_releases` | Passed: one request-handoff failure/reset regression. |
| `cargo test -p tunny-desktop --locked --profile ci-test --lib in_flight_then` | Passed: both overlapping-interest orders. Also rerun by the final `d2_` and workspace suites. |
| `cargo test -p tunny-desktop --locked --profile ci-test --lib d2_shared_journal_failure` | Passed: failure/retry/forced replacement in both overlap orders. Also rerun by the final `d2_` and workspace suites. |
| `cargo test -p tunny-desktop --locked --profile ci-test --lib d2_cached_journal_selection` | Passed: cached primary/comparison activation with all 32 GUI delivery slots occupied. |
| `cargo build -p tunny-desktop --locked --profile ci-test` | Passed in the prior source/column-review iteration; not rerun for the lifecycle or cached-activation follow-ups. |
| `git diff --check` | Passed. |

In the prior source/column-review iteration, the opt-in command from CONTRIBUTING
ran on the interactive Windows RDP desktop. It was not rerun for the lifecycle
or cached-activation follow-ups:

```powershell
$env:TUNNY_NATIVE_CAPTURE_CHECK = "1"
cargo test -p tunny-desktop --locked --profile ci-test --test artifact_animation_native
Remove-Item Env:\TUNNY_NATIVE_CAPTURE_CHECK
```

The native wgpu preview/GIF check passed: three frames in matching order,
684 x 544 content-only capture, 330 ms delay, full prefix observations and two
independent instances. This checks the existing Animation capture integration;
it is not a claim of manual visual review of every D2 workflow.

The built `target/ci-test/TunnyDashboard.exe --no-beta-notice -i
rust_core/fixtures/test.log` also opened a responsive native window with the
requested source in its title and remained alive after 12 seconds. The smoke
process was terminated afterward rather than saving session preferences. The
automated GUI tests above provide the load/result assertions, not window-title
inspection alone. macOS linking/startup and Linux execution were not verified.

## Temporary bridge/removal inventory and limits

| Remaining boundary | Owner from the accepted handoff |
| --- | --- |
| Legacy-core `parser/study.rs` and GUI batch delivery adapter | #251: replace parser delivery with extracted I/O; retain fallible backpressure. |
| Legacy-core `parser/study_projection.rs`, explicit Pareto wrapper and analysis callers | #252: replace with extracted explicit-input analysis objects/APIs. |
| GUI DataFrame/extras mirrors and active-store selection for unmigrated widgets | #253: remove remaining widget store lookups/result ownership shims. They are not ingestion targets in this Journal path. |
| CSV/SQLite/RDB ingestion and Artifact resolution | #251: still use their existing paths pending their own migration and investigation gates. |
| Remaining MCP/report/export/analysis callers | #251/#252/#255 as assigned by the design handoff. MCP is not migrated by the D2 headless harness. |
| Existing runner/writer callers and entry imports | #254/#255; #256 removes only mechanically obsolete facades/globals after their consumers migrate. |

Only before-terminal Journal attributes are within this first acquisition slice.
#258/#259/#260 remain separate investigations; this implementation neither fixes
their candidates nor establishes database snapshot guarantees. No performance or
RSS bound is asserted. No intended user-facing behavior change, package/executable
rename, new external dependency, or new durable architectural decision is made.
