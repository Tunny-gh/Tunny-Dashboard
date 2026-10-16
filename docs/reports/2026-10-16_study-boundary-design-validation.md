# Study boundary design: evidence and validation handoff

Investigation date: 2026-10-16. Source baseline: `1aacc58` (PR #257 merged).
Scope: [#249](https://github.com/Tunny-gh/Tunny-Dashboard/issues/249), documents
only. No Rust, native, GUI, model fit, or concurrent-writer test was executed for
this report. Existing source/test anchors are not evidence of a newly passing
test. This is a dated evidence/review handoff, not a second decision authority
or a live task-status document. Work status belongs in the linked Issues.

The policy is [ADR-0019](../adr/0019-responsibility-refactoring-goal-and-scope.md)
and [ADR-0020](../adr/0020-commit-study-updates-by-validated-batch.md); the concrete
independently reviewed and adopted concrete design is
[ADR-0021](../adr/0021-study-analysis-and-io-boundary-contracts.md). The earlier
temporary inventory informed this report; its pre-Q7 ambiguity about provisional
fronts is resolved by ADR-0020, not retained here as an open question.

## Model coverage evidence

All paths in code columns are repository-relative. These identify implementation
locations, not an assertion that the new domain structures exist.

| Information | Existing source anchor | Required model/projection distinction |
| --- | --- | --- |
| Trial identity/number | `rust_core/src/data/dataframe/types.rs`, `egui-app/src/state/types/study.rs` | TrialKey retains stored global ID within source; display number stays separate from analysis row. |
| State/time/intermediate | `rust_core/src/data/extras.rs`, `io/rdb/generic/study_rows.rs` under `rust_core/src/` | All states belong in the same trial authority, not an independently versioned extras slot. |
| Non-COMPLETE params/results/attrs | `rust_core/src/io/journal/parser/finalize.rs:44-60`, `rust_core/src/io/rdb/generic/study_rows.rs:94-224` | Current finalize discards non-COMPLETE payloads; RDB payload queries filter COMPLETE. New ingestion must retain available all-state fields before analysis selects COMPLETE rows. |
| Category identity/distributions | `rust_core/src/io/journal/parser/distribution.rs`, `rust_core/src/data/dataframe/model.rs` | JSON type/presence, numeric kind/discrete eligibility and declared bounds survive. Labels/feature encodings remain method-specific; do not fabricate distributions for CSV. |
| Missing/objective index | `rust_core/src/io/rdb/generic/study_rows.rs:132-159,301-309`, `rust_core/src/io/flat_csv.rs` | Store indexed observations without compacting holes. Preserve current supported-input outputs; malformed/sparse-source discrepancies need isolated evidence, not silent imputation. |
| Constraints | `rust_core/src/data/dataframe/feasibility.rs`, Journal/RDB readers | Presence, invalid slots and all-state expected width are neutral domain evidence. Derived numeric feasibility/violation columns are analysis projections. |
| JSON attributes | `rust_core/src/io/user_attrs.rs`, `rust_core/src/data/dataframe/types.rs` | Original JSON is authoritative; numeric/string maps and array display columns are derived. Missing is not JSON null. |
| Artifacts | `rust_core/src/io/artifacts.rs`, `egui-app/src/io/flat_csv.rs` | Preserve association order/IDs separately from root-relative resolution and GUI texture loading. |

Do not extend unknown system-attribute retention, add geometry capture, or expose
new non-COMPLETE analysis rows. Data not recorded by a source remains unavailable;
the new all-state type cannot recover absent observations. Parser builders and
RDB query accumulators are transient acquisition work, not extra model authorities.

## Readiness contracts

These contracts distinguish data eligibility, load/dependency eligibility,
existing computation fallback, and presentation. Widgets retain busy/pending
behavior but query analysis preparation instead of implementing eligibility
again. `Complete` refers to the input's captured load state, not a global flag.
Every request/result uses the full provenance contract in ADR-0021. Unless the
table requires Complete/finalized dependency, no new completion-only restriction
is introduced. A row's existing fallback is not automatically a new UI error.

| Path | Load/dependency rule | Data/subset and preserved computation behavior | Reuse/trigger |
| --- | --- | --- | --- |
| Shared full-Study Pareto | Complete input; finalize at existing final-chunk/activation timing | COMPLETE projection, directions, finite objectives, existing three-state constraint rank order. Empty rows/objectives yield existing empty result. Hypervolume only when supported by objective/front counts. | One finalized result for matching input/conditions; table/plots consume it. Temporary rank-zero padding is display compatibility, never a dependency. |
| Best-so-far history | Preserve existing final-load/activation refresh | Exactly one objective/direction; current trial-number and value policies. Partial history drawing is not shared-front finalization. | Refresh on the existing events, not a new automatic-analysis schedule. |
| Cluster (2D/3D/table/Gallery) | Matching finalized Pareto result | Select rank-zero rows, then finite required features. At least two retained rows and one feature; manual k in range, automatic elbow uses actual retained count. Preserve categorical zero representation in this path and -1 labels outside the target. | Manual Run; key includes space/k-mode/k/init/elbow settings. Same key shares; display order is excluded. |
| MCDM ranking | Matching finalized Pareto result | Nonempty front; method weights/direction/shape checks, negative-weight rejection and current per-method finite-row behavior. Map subset scores and ranks back to TrialKeys; preserve outside-front output/UI. | Manual Run; existing normalized, six-decimal quantized weights/mode/method/v semantics. Top N is display-only. GUI most-recent coloring is separate. |
| Entropy weights | Does not require finalized Pareto | All COMPLETE objective rows; at least one row/objective, correct matrix shape and at least one fully finite objective row. Preserve constant-objective/zero-weight policies. | Existing manual/weight-mode trigger. Do not wait for a front merely because ranking uses one. |
| Convergence indicators/comparison | Complete base and individually acquired comparison inputs | Existing sampled feasible series/placeholders, all objective directions, indicator/ref-point conditions. No new universal minimum count beyond existing indicator behavior. Comparison uses every input stamp. | Automatic when no matching result and not already computing; preserve reference controls. Cache invalidates on any dependency version, not just base Arc identity. |
| Spearman importance/heatmap | No additional completion gate | Valid objective, at least two COMPLETE projection rows and some declared parameter. Numeric parameters only; pairwise finite pairs per parameter. Fewer than two usable pairs yield existing NaN; raw short-input zero fallback remains distinct. Unsupported categorical list remains displayable. | Importance manual; low-cost heatmap auto on miss. Include objective/metric/feasible policy; all-objective heatmap differs from single-objective view. |
| Ridge importance/heatmap | No additional completion gate | Same initial count/objective/parameter check; numeric features only, complete-case X/y for fitting. Preserve all-categorical unsupported output, insufficient-fitting-row and numerical fallback rather than demand a model in every displayable result. | Existing triggers and method/objective/feasible keys, now input-stamped. |
| Tree importance/heatmap | No additional completion gate | Parameters/objective and initial >=2 rows; common preparation needs >=2 finite complete-case feature/target rows, existing label encoding, method sampling caps/seeds and holdout split. Preserve None/error versus public low-level zero fallback per entry path. | Expensive methods manual; same input/method/objective/feasible settings reuse. |
| Sobol | No additional completion gate | >=2 rows, parameters/objectives, positive sample count, >=2 rows finite in all required parameters and objectives; existing label encoding and surrogate construction. Computes all objectives. | Manual; single-objective display and all-objective heatmap can derive from same stamped full Sobol result without changing outputs. |
| ARD importance | No additional completion gate | Parameters and selected objective; existing label encoding. Delegate raw X/y to fit validation (>=10 and finite); do not silently switch this path to GUI finite-row exclusion. | Manual, metric/objective/feasible input key. |
| PDP 1D | No additional completion gate | Resolve selected parameter/objective, optional feasible subset, existing GUI finite extraction and categorical fallback. Ridge needs >=2 shaped finite rows/grid/valid index; GP/RF failures retain existing Ridge fallback and empty results. | Manual; parameter/objective/model/feasible/grid key. Current 1D and 2D missing-column extraction differences are preserved until isolated evidence justifies a correction. |
| PDP 2D | No additional completion gate | Distinct nonempty parameters/objective, optional feasible subset, current core finite extraction. Preserve model-dependent None/empty/fallback outputs. Replace active-store access with the acquired input, not a new readiness rule. | Manual, last-result behavior; analysis object owns result. |
| Single/multi surrogate fit | No additional completion gate; multi needs >=2 objectives | Numeric features, finite GUI training rows (single selected objective, multi all objectives), aligned constraints only where current request uses them; >=10 rows, equal shaped X/y, positive dimension, all finite at fitting boundary. Preserve backend/CV failures as computation errors. | Manual Fit; retain stamped trained handle. Existing explicit cancellation via FitProgress remains. |
| Optimize/suggest from trained model | Matching trained dependency and current target provenance | Keep existing method/acquisition objective-support and spec checks; no implicit refit when input changes. Model-only evaluation has fit provenance, not a made-up current Study revision. | Existing Run/Suggest stages; fit ResultKey and actual settings enter key. |
| Robustness/surface/compare | No additional completion gate | Existing numeric-fit checks. Robustness aligns constraints; surface/compare omit them. Compare also requires valid slice and a best-row anchor surviving finite selection; show partial model failures, overall error only if all fail. | Existing fit/run controls and retained handles. No new universal fit cache or feasible-only policy. |
| GUI report | No additional completion gate beyond existing current Study/formats/generating checks | Capture one coherent model input, not view DF plus independent active extras. Preserve section-specific omission, numeric-only Spearman importance and labeled objective-only candidate fallback. It computes its existing report-local selection, not a shared finalized front dependency. | Request-local report/options/source presentation; no cross-invocation timestamp cache. |
| MCP summary/report/trials | Existing synchronous acquisition succeeds | Same domain semantics and report builder; summary skips decision sections; trials paginates COMPLETE rows. Do not add GUI-only tools or runtime cache sharing. | Request-local ownership/session. |
| PCA/SOM/dendrogram | No new completion gate | Existing feature-space numeric selection, finite-row mapping; PCA >=2 usable rows/components, SOM >=3 with valid grid/epochs, linkage >=2 finite rows. Preserve sampling and empty outputs. | Synchronous computation; replace name/row-count-only or pointer keys with explicit input/settings keys. |
| Observed contour | No new completion gate | Selected X/Y/value columns, finite rows, optional feasible subset, >=3 points and nondegenerate geometry. | Existing manual request/last result; row mapping includes TrialKeys. |
| Distributions/table/timeline/intermediate/Animation | Existing display and resource requirements, not one universal analysis gate | Preserve COMPLETE versus all-state extras selection, typed categories, finite handling, trial labels and prefix observations. Artifactless trials still contribute to prefix analysis. | Do not replace Animation's independent cache/export policy (ADR-0018) with generic analysis policy. |

### Evidence anchors and preserved irregularities

- `egui-app/src/ui/chart/poll_chart.rs:24-97,245-749`: shared dispatcher lacks a
  global completion gate; convergence has one. `poll_chart/compute.rs` contains
  method-specific extraction, `poll_chart/surrogate.rs` fit/stage checks.
- `egui-app/src/state/message_handler/study.rs:65-147`: partial rank padding,
  first-chunk cache clearing and per-chunk convergence invalidation. The design
  adds required provenance/Q7 front checks, not a general all-analysis ban.
- `rust_core/src/sensitivity/spearman.rs`, `ridge.rs`, `metrics.rs`, `sobol.rs`,
  `tree/common.rs` and `rust_core/src/surrogate_opt/ard.rs`: pairwise versus
  complete-case selection, label encoding, insufficient data and fallback differ.
  Heatmap may keep zero entries where individual Importance reports failure;
  preserve that current result contract, not invent a new global failure UI.
- `rust_core/src/pdp/api.rs`, `ridge.rs`, `utils.rs` and GUI extraction differ in
  missing-column fallback. Tests must preserve supported-input behavior; do not
  silently 'fix' it while extracting APIs.
- `rust_core/src/surrogate_opt/fit.rs:14-41`, `mod.rs:63`: common fitting minimum
  10 and explicit shape/finite validation, independent of whether UI queues Run.
- `egui-app/src/app/dialogs.rs:43-61` acquires DF and extras separately today;
  `rust_core/src/report/builder.rs:51-138` accepts optional extras and handles
  finite/feasible fronts locally. `mcp-server/src/tools.rs:221-284` calls it after
  synchronous storage load; neither implies a new partial-front feature.

The table selects preparation contracts from these existing branches, preserving
their result/error adapters. It is not a claim that every existing UI action was
exercised interactively. If a supported fixture contradicts a stated existing
predicate, resolve that specific discrepancy in review before migrating the path;
do not treat a guessed strict validator as permission to change outputs.

## Fixture and validation matrix

`Existing anchor` means reusable tests/source fixtures, not newly passing tests.
`New case` is a precise input/expected check to implement with the owning Issue.
This matrix contains no execution status or checkbox task tracker.

| Case | Inputs and expected result | Existing anchor / new coverage | Work owner |
| --- | --- | --- | --- |
| M1 all-state authority | COMPLETE/RUNNING/PRUNED/FAIL/WAITING with distinct params, missing/results, JSON attrs/time/intermediate. Acquired records retain all recorded information; analysis rows remain COMPLETE-only. | Existing Journal/RDB fixtures in `rust_core/src/io/journal/parser/tests.rs`, `rust_core/src/io/sqlite/tests.rs`; add non-COMPLETE payload retention assertions. | #250 model, #251 adapters |
| M2 identity | Two sources reuse Study/trial IDs and row counts; one Study has global IDs unlike gapped trial numbers. Complete trials finish out of creation order: preserve streaming emission order versus full-read row order without confusing either with identity. TrialKey lookup/labels/subset results stay distinct; replace source rejects old work. | Existing Artifact/trial-number fixtures; new namespace/manager and completion-order fixtures. | #250, #253 |
| M3 batch integrity | Valid trial+metadata batch followed by invalid identity/base/sequence batch. Second has no effect; first remains, sequence not consumed. Duplicate/out-of-order ticket cannot append twice. | New manager contract tests; no executable manager exists yet. | #250 |
| M4 metadata-only | PRUNED/RUNNING trial grows constraint array or establishes empty constraint attribute; no new COMPLETE rows. Revision advances, old rows reclassify; bulk/incremental final evidence agrees. | Existing schema/invalid-position tests in Journal parser and SQLite tests; new manager/projection assertions. | #250, #251, #252 |
| M5 completion-only/empty | Empty authoritative batch then finish publishes new generation with zero rows. Finish with unchanged contents changes load epoch, not revision; finalized-front readiness can advance without fake rows. | New protocol/readiness cases; existing streaming final-empty-batch tests seed inputs. | #250, #252 |
| M6 reload failures | Fail before first new batch: old display stays old-stamped. Accept new batch then fail: new partial stays, no old rollback. New generation rejects late old job even with equal contents. | Existing `egui-app/src/app/reload.rs` and `app/tests.rs`; add ticket/failure boundaries. | #250, #253, #255 |
| M7 retain/release | Hold input/job over update; old values unchanged, new input updated. Drop job/old display/interests; Weak handles show obsolete roots/records can free. Metadata-only shares trial directory; single-trial update shares unchanged payloads. | Existing Arc snapshot tests in `rust_core/src/data/dataframe/state.rs`; new Weak/COW tests. No memory/time number is a pass criterion. | #250, #252 |
| M8 finalized dependency | During partial load padded ranks would be zero. Cluster/ranking preparation waits; entropy and eligible non-front methods keep their existing conditions. Front/result from different revision/generation is rejected. | Existing `egui-app/src/ui/chart/poll_chart/tests.rs`, message-handler streaming tests; new dependency-key cases. | #252, #253 |
| M9 numeric policy | Pairwise versus complete-case NaN/inf, real zero, unsupported typed categories, insufficient rows, Ridge/PDP/heatmap fallback. Results/errors match existing path, mappings remain exact. | Existing `rust_core/src/data/finite_rows.rs`, sensitivity/PDP tests; add extracted-API equivalence fixture pairs. | #252 |
| M10 shared widgets | Table, Pareto, 2D/3D cluster and MCDM select identical key: one result/row map. Different weights/features stay isolated; sorted/filter/pinned/Top-N display still targets correct trials. Most-recent coloring remains GUI selection. | Existing cluster/mcdm/table tests, `egui-app/src/state/message_handler/tests.rs`; add shared-session integration. | #253 |
| M11 late same-size results | Switch to equal-row-count Study/source or reload; let cluster/MCDM/fit/compare old workers finish. Do not change current slot/cache; explicit rerun also rejects earlier run serial. | New provenance tests augment label-length-only cluster and unconditional MCDM completion tests. | #253, #255 |
| M12 comparison dependencies | A/B histories acquired at different generations. B changes, A-only fit remains reusable; A+B result invalidates. Old base-Arc match cannot excuse stale B. | Existing convergence/comparison tests; new all-input key test. | #252, #253 |
| M13 Artifact lifetime | Switch Study, reload, replace source with reused IDs, manual root during reload, stale scan, extensionless MIME, CSV rebasing/traversal. Keep storage map/session root and reject wrong scan/root/source; texture invalidation stays separate. | Existing `rust_core/src/io/artifacts.rs`, `egui-app/src/app/tests.rs`, reload and CSV/URI tests; native Animation harness when affected. | #251, #255 |
| M14 late Journal attrs | Terminal state then supported JSON/constraint records, batch size 1 and large; compare full and streaming, including non-COMPLETE schema. | #258 reproduction first; parser fixtures are anchors, not proof of reproduction. | #258; affected #251 path gated by findings |
| M15 fingerprint-only change | Update attrs/constraints without counts/state changes, compare fingerprint and direct load; locate real refresh consumer before claiming UI failure. | Existing SQLite fingerprint tests / `egui-app/tests/rdb_integration.rs`; new mutation cases. | #259; affected #251 detection path gated by findings |
| M16 external snapshot | Barrier between queries, writer commits state/value/attrs, compare old/new view; rollback/WAL, PostgreSQL and MySQL transactional prerequisites, wait/resource/error cleanup. No analysis-retained transaction. | #260 investigation; existing SQLite schema and opt-in RDB integration harness are starting points, not completed concurrency tests. | #260; DB I/O session path gate |
| M17 run equivalence | Fixed seeds, rounding/bounds, each supported method/objective count, penalties versus recorded values, bootstrap/refit/stop/cancel/I/O failure, process timeout/hooks and Compute mock. Journal readback equal. | Existing `rust_core/src/runner/mod.rs` tests, `gh/runner/tests.rs`, `gh/adaptive.rs`, process tests. | #254 |
| M18 entry/parity/link | GUI open/select/compare/reload/report/export/run, MCP stdio/list/summary/report/trials/pagination/masking, equivalent supported semantics, Windows/macOS LightGBM startup/test link. No new MCP tools. | Existing `mcp-server/tests`, `egui-app/tests/rdb_integration.rs`, native capture harness, existing CI. Add new-package startup invocations. | #255, final audit #256 |

## Vertical slices and bridge ownership

These are concrete handoff boundaries for existing Issues, not new Issue scope or
permission to mark them complete. Every row requires its tests and a working
caller path at that stage. The durable dependency direction is in ADR-0021;
temporary legacy-core adapters do not become reverse dependencies of new crates.
For a migrated Study, the explicit manager is the only writer/authority; any
legacy DataFrame slot is a derived read projection published from that manager,
never a second ingestion destination. Unmigrated formats can use their old path
until their slice switches ownership, but cannot install a second authority for
the same Study. All such compatibility projections disappear with their owner
slice or the mechanical #256 facade deletion.

| Owner | Working acceptance boundary | Temporary bridge and final removal |
| --- | --- | --- |
| #250 | Journal byte fixture -> all-state StudyBatch -> explicit manager -> acquired input -> existing history/table numeric projection in GUI and a headless harness. Include batch errors, reload generation and retained-input tests now, not at #255. Existing worker command delivery can remain. | Legacy Journal builder-to-batch adapter and analysis-table projection sit in legacy core/caller, not Study. #251 replaces parser delivery; #252 replaces projection; #253 replaces remaining GUI store lookups. First slice uses supported before-terminal attrs; #258 governs later-record correctness, not model/API shape. |
| #251 | Replace each Journal, CSV, SQLite/RDB ingestion adapter in turn with I/O-produced owned data; existing entry consumers apply through managers. Migrate writer/resolver consumers in the same change boundary, preserving streaming/reload/errors/root behavior. | Legacy facade delegates to I/O during consumer transition. #254 removes run-writer facade use; #255 removes remaining export/session entry glue. #258/#259 findings gate only affected ordering/detection paths; #260 validates DB session configuration before that path is accepted. Do not build a new poller to satisfy an obsolete comment. |
| #252 | Explicit Pareto/PCA/PDP/Sobol/sensitivity/surrogate/report APIs and derived input object, with each existing caller converted alongside its API. All numerical helpers move once, existing execution callers use the extracted shared APIs without merging loops. | Legacy `tunny-core` re-exports the new analysis APIs temporarily for unmigrated entry/execution imports; analysis never imports legacy core. No ambient helper survives in a migrated path. #253 removes GUI result/cache shims, #254 execution imports, #255 remaining entry imports. |
| #253 | One shared analysis session used by table/plots/Gallery and fit consumers; typed worker completion/provenance and Interest release work end-to-end. Settings/busy/most-recent coloring stay GUI-owned. | AppState caches become temporary presentation handles, not copied authorities. Replace their lookups with session lookup in this slice; keep no functional cache migration for #256. |
| #254 | Both existing real-evaluation loops use `tdashboard-optimization-runner`, public analysis optimizer/progress and I/O writer. GUI launch/progress/cancel/readback works now. | Legacy execution re-exports only for remaining launch imports; remove actual old loops here. #255 removes last import glue; #256 removes empty facade. |
| #255 | Remove remaining report/export/session/comparison/protocol/launch glue, rename packages and update license/build/release/launcher references, exercise current binaries. | No production consumer still depends on `tunny-core`; any discovered functional gap returns to its owning slice, not hidden in cleanup. No directory rename campaign or user executable rename. |
| #256 | Remove legacy `rust_core` package, globals, compatibility exports and abandoned build/test paths; inspect final metadata/DAG and run full regressions. | Delete the adapter inventory only after proving all consumers migrated. This is mechanical closure and cross-check, not postponed model/I/O/GUI integration. |

The existing sequential #250 -> #251 -> #252 ordering, parallel #253/#254 after
#252, then #255/#256 remains feasible with these adapters. Each Issue remains one
PR. Independent review can require implementation slices inside that PR, but
does not authorize extra PRs, widened scope or automatic merge. A backend gate
failure is an I/O compatibility decision to surface, not permission to redesign
the Study input type. No code work on #250 starts until #249 review is complete.

## Issue #249 completion mapping

| Published Done requirement | Review artifact |
| --- | --- |
| Concrete contracts and durable decisions | ADR-0020 accepted Q1-Q9 policies; ADR-0021 independently reviewed and accepted concrete model, identity, update, retention, API and read-session design. |
| Ownership/dependency diagram and API sketches | ADR-0021 dependency graph, per-Study manager/immutable input protocol, analysis session/result and backend-private read interface. |
| Fixtures, outcomes, owners | M1-M18 above, including initial/incremental equivalence, same-size old-result rejection and #258-#260 independent investigations. |
| Executable vertical-slice boundaries | Bridge/owner table above; first #250 Journal slice includes caller and retained-input verification, not an isolated data-type crate. |
| No unresolved decision affecting first slice | Standard Arc/COW model and keys/update protocol are specified. Later backend configuration validation is encapsulated by I/O; #258 late-record investigation uses the same upsert contract. Rust implementation details remain to be tested, not architectural choices silently delegated to coding. |

Independent review confirmed the concrete contracts and this completion mapping.
Issue #249 remains open until its design PR is merged; no downstream test or
implementation is claimed complete. The adopted designer choices and their
alternatives remain within the approved policies, not new product scope.

## Verification commands and limitations

Use [CONTRIBUTING](../../CONTRIBUTING.md#development-commands) as the command
authority. The current exact CI commands are:

```text
cargo fmt --manifest-path rust_core/Cargo.toml --all -- --check
cargo fmt --manifest-path egui-app/Cargo.toml --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --profile ci-test
cargo audit --ignore RUSTSEC-2026-0194 --ignore RUSTSEC-2026-0195
```

When legacy core is retired, change the manifest path/coverage in CI and
CONTRIBUTING together, keeping `--locked`, `ci-test`, all-target lint and complete
workspace coverage. This report does not change those files or run these commands.
Build/test CI is Windows/macOS with native LightGBM; Linux clippy is check-only.
Run the existing opt-in Artifact Animation native harness on an interactive
desktop when affected; real RDB/Compute environments must be explicitly reported
as tested or unavailable. Document-only checks cover document structure, links,
names, scope and whitespace, not Rust compilation of the sketches.
