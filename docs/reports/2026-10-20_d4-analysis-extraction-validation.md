# D4 analysis extraction: implementation and verification

Date: 2026-10-20. Base: `30b633c` (main including PR #272).
Branch: `refactor/252-extract-analysis`.
Scope: [#252](https://github.com/Tunny-gh/Tunny-Dashboard/issues/252) under
[#248](https://github.com/Tunny-gh/Tunny-Dashboard/issues/248), following
ADR-0019/0020/0021 and the D1/D3 validation handoffs.
This records implementation-owner verification, not acceptance of the review
remediation, Issue closure, commit, push, PR creation, or merge. Prior independent
reviews found the P1 preparation and P2 concurrent-rejection gaps described below.
Verification and cache-selection remediation were updated on 2026-10-21.
The security audit is **not passing**.

## Changed boundaries

| Area | Implemented boundary |
| --- | --- |
| `crates/analysis/` | `tdashboard-analysis` owns numerical/model code, optimizers, `FitProgress`, derived tables/training selections, analysis sessions/provenance, report computation and pure HTML/Markdown/JSON/SVG rendering. Its only workspace dependency is Study, including test dependencies. |
| `crates/analysis/src/{projection,session,preparation,training,export}.rs` | Immutable acquired inputs produce COMPLETE tables, all-state presentation extras, typed method preparations and stable TrialKey row maps. Sessions reuse prepared tables and matching results/in-flight handles; different conditions remain separate. Revisions, generations and load epochs invalidate lookup entries. Completed payloads do not retain Study roots. |
| `crates/analysis/src/sensitivity/heatmap.rs` | All-objective aggregation is analysis code. Missing individual metrics retain zero cells; this is not replaced with blanket single-metric rejection. Existing pairwise, complete-case, categorical and runtime fallback policies remain in their numerical APIs. |
| `egui-app/src/state/analysis_job.rs` | GUI workers receive method-prepared data, not independently captured matrices or selections, with checked run serials and keyed success/error/cancellation delivery. Unavailable preparation never invokes the numerical callback; rejected preparations use the same stamped completion protocol. Matching cluster/MCDM/entropy requests share in-flight computation across presentation slots. Only matching displayed inputs/current requests adopt results. |
| GUI publication and widget adapters | Input changes detach completed legacy cache payloads without resetting selection/reference controls. PCA/SOM/dendrogram cache keys use acquired stamps rather than names/counts/pointers. No automatic refit or worker cancellation is added. |
| GUI/MCP reports | Reports acquire one coherent input, not a view table plus ambient extras. GUI can reuse the owner's prepared schema. Entry code chooses options, calls pure analysis and passes rendered bytes to existing I/O writers. MCP remains request-local and preserves its tools/wire format. |
| Surrogate handoff | Retained models carry their fit ResultKey and TrialKeys. Finite training selections and single/joint multi-objective subsampling keep their original row correspondence. Optimize/suggest requests include fit dependencies and cannot use an old model as a new input's fit. Matrix-only execution APIs remain explicit and do not fabricate Study provenance. |
| Native builds | Analysis owns LightGBM FFI/link metadata and Windows DLL copying, resolving workspace-root `libs/` from its new directory. Entry/test/example targets retain required final-binary link/rpath handling. |
| Legacy I/O facades | Core's remaining format facades compose owned batches, caller-owned managers and analysis projection. Production decoder-row-to-analysis conversion was removed; the remaining row-view conversion is a test fixture. I/O and Study do not import analysis. |

Numerical implementations and regression tests were relocated rather than
replaced. No external dependency versions, algorithms, source ID meanings,
executable names, storage formats or execution-loop approaches were changed.
`Cargo.lock` changes workspace package/dependency edges only. The workspace
retains legacy core during staging; it is not the final six-package layout.

## Review remediation: method-specific preparation (P1)

The original working implementation's generic preparation was insufficient: it
checked input/dependency stamps and returned COMPLETE projections, while GUI code
still decided eligibility and captured its own calculation data. Passing numerical
regressions did not prove the D1 preparation contract. This gap is fixed in
`preparation.rs`, `session.rs`, and the live poll/worker adapters:

- `prepare_for` takes a typed `Preparation` selector. Parameterized operations such
  as Fit cannot become Ready through a kind plus free-form conditions; supplying
  the wrong preparation kind or multiple inputs to a single-Study method fails.
- `PreparedAnalysis` owns the selected data and per-input TrialKey maps. Workers
  read `data()`; fitting consumes prepared X/y/bounds/constraints, Compare consumes
  its prepared anchor/observations, and model-only work consumes `models()` and
  prepared objective directions. Conditions remain provenance/cache settings,
  not a substitute for preparation. Lower numerical kernels may retain their
  existing method-internal matrix, encoding and sampling representations.
- `prepare_models` owns immutable trained-model handles and derives dependency
  keys from them; a bare operation without a model waits, and a model from another
  input cannot be retargeted. No unversioned model is assigned a fabricated stamp.
- Preparation failures are input-stamped and run-keyed, clear only the appropriate
  current busy slots, and never run the callback. GUI admission flags may retain
  pending work during loading; authoritative readiness uses the acquired inputs'
  captured phases and dependency keys, not those presentation flags.

The audit covered the D1 table, not only the three review examples:

| D1 path | Preparation / preserved execution boundary |
| --- | --- |
| Pareto / history | Complete-input gate; finalized front dependencies match the acquired input. History additionally requires one objective/direction and live publication queries `AnalysisSession::history`. Empty Pareto/history numerical results remain allowed. |
| Cluster | Matching finalized front, rank-zero subset, finite selected feature rows, categorical constant-zero handling, minimum rows/features and actual retained-count manual-k validation. Live adapters only transport the prepared matrix/map to the existing k-means/elbow kernels. The old explicit-table test adapter now delegates to the same selection implementation. |
| MCDM / Entropy | MCDM owns its front subset, objective matrix, directions and row map in preparation. Entropy independently requires nonempty objectives/rows and retains fully finite objective observations; it never waits for a front. Existing weight validation still precedes the empty-front diagnostic, and method-specific ranking/normalization policies remain numerical-kernel behavior. |
| Convergence / comparisons | Each input owns its sampled slots/TrialKeys, feasibility placeholders and sample step; every input stamp participates in the key. Calculation consumes prepared series. The legacy target-view field is only a presentation adoption guard, not numerical input, and remains a D5 adapter. |
| Spearman / Ridge | Prepared feasible subset, valid objective/parameter/count eligibility and pairwise versus complete-case feature maps. Too few usable pairs may still yield NaN; Ridge's insufficient-fitting-row zero fallback and all-categorical unsupported result remain displayable. |
| Tree / Sobol / ARD | Existing label encoding and finite-row eligibility are evaluated in analysis preparation. Tree/Sobol require sufficient finite required rows; Sobol requires positive samples and all required objectives. ARD validates raw encoded X/y rather than adopting GUI fit's finite exclusion. Existing numerical caps, seeds, splits and backend fallbacks stay unchanged. |
| Heatmaps | Separate typed request: remains Ready when an individual metric is unavailable, including empty/all-skipped cases, so existing zero-cell output is preserved rather than blanket-rejected. |
| PDP 1D / 2D | Resolved fields, feasible subset, the distinct existing missing-column policies, selected finite arrays and original-row maps. Backend-dependent empty/None/Ridge fallbacks remain execution outcomes. |
| Single / multi fit | Numeric features, selected objective(s), finite training rows, ten-row/shape/finite fitting validation, bounds and aligned constraints. Multi fit selects all required objectives jointly. No additional load-completion gate. |
| Robustness / surface / Compare | Shared fit preparation, with constraints only for Robustness; Compare additionally owns feature/slice validation, direction-aware best-row anchoring after finite selection, and observed slice points. Individual fitting failures and partial/all-model failure output remain execution behavior. |
| Optimize / suggest | Prepared immutable model handles, fit keys and objective directions; existing algorithm/acquisition support checks remain numerical-method validation. No implicit refit. |
| PCA / SOM / linkage | Shared analysis-side finite feature preparation and source maps, existing categorical policy, fixed per-method minima, PCA component and SOM grid/epoch validation. Live synchronous paths consume these prepared features; sampling and result remapping are unchanged. |
| Observed contour | Selected columns, feasible finite points, trial maps and cleaned geometry/bounds are prepared in analysis. **Do not blanket-reject collinear points:** the existing kernel intentionally returns axes with all cells masked when triangulation is impossible. Degenerate bounds/too few unique observations remain unavailable. This source behavior, not a new geometry policy, resolves the broad wording in the D1 table. |
| GUI/MCP reports and display/resource paths | Reports retain coherent request-local inputs and section-specific fallbacks, including empty reports and #272. Display-only tables/distributions/timelines/intermediate values and Animation keep their existing COMPLETE/all-state/resource rules; no universal readiness gate or Animation policy change was introduced. |

`rust_core/tests/d4_preparation.rs` adds eleven boundary regressions across these
policies. The GUI rejection test proves that a featureless Fit produces a keyed
failure without executing math. It also exposed a retained-schema invariant:
when an update removes the final parameter observation, old display columns must
still receive missing cells for every new projected row. Projection now fills
those cells without turning them into current observed features. Existing
convergence stale-target regression coverage is retained, not weakened. Full-
objective preparations preserve descriptor/direction order even when objective
display labels repeat; they do not resolve every direction through the first
matching label.

## Review remediation: concurrent rejected requesters (P2)

Two widgets could request the same unavailable calculation before either queued
completion was handled. Each rejection allocated a new run key, but registration
updated only the initiating slot. The session correctly rejected the obsolete
completion; the first widget therefore lost its error delivery. The new
two-requester regression reproduced **one delivered failure instead of two**
before the fix, and the supersession regression also failed before remediation.

`analysis_job.rs::select_job` now applies the existing successful-work interest
rule to both preparation rejection and settings-validation rejection: every
current slot selecting that AnalysisKey follows its latest checked ResultKey.
The latest completion fans out its failure to all those current requesters. A
slot changing conditions does not release the previous calculation while
another slot still selects it. `AnalysisSession::reject` and `complete` retain
their run-key/slot-identity supersession checks; obsolete errors were **not** made
adoptable to work around the missing delivery.

Two GUI regressions exercise the production protocol, including both completion
orders, the empty-finalized-front MCDM example, identical invalid settings,
same-key reruns, and one requester moving to different conditions while another
still awaits the prior calculation. They assert actual failure messages for
each current source, busy-slot protection, and final delivery through the normal
message handler. Numerical preparation, algorithms, public library APIs,
dependencies and final widget-cache ownership are unchanged by this remediation.

## Review remediation: cached and local selections supersede pending work (P2)

A cached B selection could return while the slot still selected pending A. A's
queued success, error or caught worker panic could then be adopted after B. The
new regression reproduced this before remediation: "A cache hit must replace
the old pending adoption interest".

`select_job` now also accepts no result key, and `supersede` uses that path for a
consumed selection that adopts a cache or finishes locally without starting a
worker. Only that slot's interest is removed. The previous session result is
released only when no other slot selects it; retained work is not cancelled.
Run-key validation and stale-completion rejection remain intact.

The analogous live-path audit and fixes cover:

- Regular and Sobol importance, Heatmap, and PDP cached selections. PDP now queues
  a cache-hit selection so its poll adapter can supersede the previous interest.
- Consumed Optimize, MultiOptimize, Suggest and MultiSuggest requests with no
  model: detach old interests and finish the corresponding busy state.
- Convergence indicator/reference edits: supersede before the busy early return.
- Loading/no-new-request waits: retain interests because these are not consumed
  replacement requests, rather than indiscriminately detaching pending work.

Six added desktop regressions cover cached B followed by queued A success,
generic error and caught panic across the cached paths; preservation and failure
delivery for another requester of shared MCDM work; all four missing-model paths;
PDP cache-hit request queuing; Convergence edits; and current canvas propagation.
The panic-hook output is intentional and caught, not a test failure.

The app message adapter now unwraps accepted `AnalysisCompleted` deliveries before
calling the existing message handler and `ComputeSyncKind` canvas propagation.
Rejected envelopes never propagate. A slot-based fallback supplies the existing
propagation kind for current caught panic/error deliveries. This preserves the
existing canvas update behavior through the keyed envelope; it does not migrate
widget/cache ownership or introduce a D5 presentation redesign.

## Evidence against #252 Done

| Requirement | Direct evidence |
| --- | --- |
| Explicit computations, no ambient analysis API | The new crate has no active/global store or I/O/core import. Pareto/PCA/PDP/Sobol APIs take explicit tables/data. GUI callers capture their displayed immutable input; MCP acquires request-local inputs. Core's remaining ambient store is only a presentation bridge. |
| Numerical and source-row semantics | Relocated finite-row, sensitivity, PCA/PDP, category, constraint, direction, optimizer and report regressions pass. Cross-boundary Journal tests retain nonfinite objective positions and nominal numeric-looking categories. Report tests retain #272's finite filtering before nondominance and labeled objective-only fallback. |
| Reuse, invalidation and consistency | `rust_core/tests/d4_analysis.rs` verifies same-input table/result sharing, condition/source isolation, same-count changes, all-state schema growth, completion-only load epochs, reloads, old errors/reruns, comparison dependencies and retained-root release. `d4_preparation.rs` verifies method eligibility, selections/maps and actual fitted-model dependency rejection. Equal-numbered slots from different sessions cannot accept each other's completions. |
| Live consumers | Desktop D4 tests run the production worker/publication/report paths: stale input rejection, current busy-slot protection, in-flight cross-widget sharing, missing-feature fit row maps and report generation while management advances. Existing GUI ingestion, selection/reload, exports and both generic/GH recording regressions pass. MCP integration launches the real stdio binary for its full session and categorical summary/report checks. |
| Cargo boundaries and helper placement | Locked Cargo metadata was parsed and asserted: analysis -> Study; I/O -> Study; Study -> no workspace crate. This also checks development dependencies. Numerical dependencies are on analysis; execution reuses its optimizer/progress APIs through the temporary facade, without a cycle. |
| Native and parent verification | Exact command results below. Windows native linking/tests and full-app native capture passed. Linux/macOS checks were not executed locally. The existing security-audit failure is still #269, not a passing or suppressed check. |

### Remaining bridges and removal owners

These are integration adapters, not additional trial authorities or deferred
analysis implementations:

| Remaining bridge | Removal owner |
| --- | --- |
| Legacy presentation store, AppState/widget payload copies and settings-only presentation caches | D5/#253. Current invalidation and stamped delivery protect the converted computation paths; D5 replaces these copies with selected typed result handles and finishes shared-widget ownership. |
| GUI-shaped consumed job deliveries and per-widget busy slots | D5/#253. Calculation keys exclude delivery destinations; sharing fans the same computed result out to the existing presentation callbacks. |
| Core's optimizer/progress/numerical re-exports used by existing real-evaluation loops | D6/#254. The actual algorithms/progress implementation already live in analysis; no loop extraction or unification was performed. |
| Legacy storage/report/export composition imports and analysis type re-exports | D7/#255. Actual projection/statistics/numerical/report implementation is in analysis; encoding/writes remain I/O or entry composition. Package names remain unchanged at this stage. |
| Mechanical removal of empty legacy package/facades and abandoned fixture paths | D8/#256, after their live consumers migrate. No functional D4 caller migration is assigned to this cleanup. |

## Commands and outcomes

Executed from the workspace root on Windows, Rust/Cargo **1.99.1**, with the
existing LightGBM DLL/import-library prerequisites. Final exact parent checks:

```text
cargo fmt --manifest-path rust_core/Cargo.toml --all -- --check
cargo fmt --manifest-path egui-app/Cargo.toml --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --profile ci-test
cargo audit --ignore RUSTSEC-2026-0194 --ignore RUSTSEC-2026-0195
```

- Both formatting checks and all-target workspace Clippy: **passed**.
- Full workspace tests: **2852 passed, 0 failed, 15 ignored**; doc tests passed
  (zero cases). Breakdown: analysis 670/3 ignored; I/O 96/3 ignored; Study 2+9;
  core 259 plus integrations 1+8+8+11+5+9 (one ignored probe); desktop 878 passed/2
  ignored for each library/binary suite; preseeded RDB integration 0 passed/3
  ignored; MCP 16+2 passed/1 ignored.
- Audit: **failed**, reporting `zip 2.4.2` advisories
  `RUSTSEC-2026-0367`, `RUSTSEC-2026-0368`, `RUSTSEC-2026-0369`, plus nine allowed
  warnings. This is the existing separate #269 failure; no new ignore or
  dependency-security change was made.

Additional focused commands:

```text
cargo test -p tunny-core --locked --profile ci-test --test d4_analysis
cargo test -p tunny-core --locked --profile ci-test --test d4_preparation
cargo test -p tunny-desktop --lib --locked --profile ci-test d4_ -- --nocapture
cargo test -p tunny-desktop --lib --locked --profile ci-test d4_rejection -- --nocapture
cargo test -p tdashboard-analysis --lib --locked --profile ci-test multi_fit_keeps_shared_source_rows_through_subsampling
cargo metadata --locked --format-version 1 --no-deps
git diff --check
```

The focused runs passed **8**, **11**, **15**, **2**, and **1** tests respectively,
with zero failures/ignored tests. The P2-specific two-test run was also observed
failing before the fix. The analysis-only test links/runs without the entry
or I/O crates providing its native build setup. Metadata assertions and tracked
diff whitespace checks passed.
The desktop `d4_` suite and both exact formatting commands were rerun successfully
after resuming on 2026-10-21. The final workspace output confirms all six new
desktop regressions pass in both library and binary suites.

Native PowerShell verification:

```powershell
$env:TUNNY_NATIVE_CAPTURE_CHECK = "1"
cargo test -p tunny-desktop --locked --profile ci-test --test artifact_animation_native
Remove-Item Env:\TUNNY_NATIVE_CAPTURE_CHECK
```

The opt-in full-app wgpu preview/GIF check **passed**: three selected frames,
matching live/GIF order, 684x544 content-only output, scale 1, 330 ms timing,
68 responsive UI passes on the 2026-10-21 final rerun, full prefix observations, two independent instances,
seek/pause restoration and PNG/JPEG aspect ratios. Its temporary GIF was verified
absent after exit. The normal workspace command's native skip was **not** counted
as this pass.

An early workspace invocation timed out during compilation and was rerun with a
longer tool timeout. A development regression in the legacy bulk-Journal facade
returned scan-only metadata instead of populated analysis metadata; it was fixed
by returning the prepared contents' metadata, not weakening the ingestion tests.
The results above are from the completed final verification, not those attempts.

## Limits and documentation assessment

- No local macOS build/link/startup or Linux Clippy run; the exact Clippy command
  above ran on Windows. Cross-platform CI/native evidence remains required during
  review. There is no claim that CI was triggered or that every parent check is
  green.
- Live PostgreSQL/MySQL, user-supplied Artifact journals and ignored performance
  probes were not rerun. The fifteen ignored cases remain unverified here, not
  passes. Existing ordinary SQLite/snapshot and synthetic runner tests did run.
  No external Rhino.Compute instance was exercised; GH loop evidence uses the
  existing synthetic evaluators and Compute mocks.
- No new numerical accuracy, latency, memory, lock-free or database guarantees.
- `README.md` and `CONTRIBUTING.md` describe the current ownership/build setup.
  No new durable architecture decision was needed beyond the accepted ADRs.
- User-manual behavior remains unchanged. The docs repository was inspected at
  its actual `origin/dashboard-v0.2` base (`8c8a549`), which already documents D3
  and #272 behavior; no English/Japanese manual update is needed for extraction.
  Its unrelated deletion `.claude/settings.local.json` remains untouched.
  No forced CHANGELOG entry was added for the internal refactor.
