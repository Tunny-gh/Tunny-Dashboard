# Recorded Journal attributes: correction and verification

Date: 2026-10-18. Scope: [#258](https://github.com/Tunny-gh/Tunny-Dashboard/issues/258),
before [D3/#251](https://github.com/Tunny-gh/Tunny-Dashboard/issues/251).
Branch: `investigate/258-journal-terminal-attributes`; starting HEAD `7249e6d`
(reference-guidelines commit above the production baseline `8ac8e7f`).

## Decision and historical evidence

The user approved Q3 option A: prioritize early postprocessing visualization and
retain recorded user/constraint attributes, regardless of producer acceptance.
[ADR-0022](../adr/0022-retain-recorded-journal-attributes-for-postprocessing.md)
records the interpretation and batching trade-off. The earlier proposal to align
legacy bulk with Optuna rejection is superseded, not implemented.

The [investigation report](2026-10-17_journal-terminal-attributes-investigation.md)
retains exact baseline fixture outputs and official-reference divergences. Those
outputs are historical; they are not claims about the corrected readers. The
Python/Node probes still describe upstream behavior, not the project's expected
postprocessing policy. No terminology or existing feasibility definition changes.

## Implementation and changed boundaries

- `rust_core/src/io/journal/parser/study.rs`: accept recorded op8 and op9
  `constraints` for existing terminal trials. Preserve later-wins key merging,
  full observation replacement, dirty-set coalescing, completed ordinals and
  fallible delivery. Existing completion-count batch boundaries and the final EOF
  batch are unchanged. Op5/op6/op7 remain blocked after terminal state in owned
  replay; unrelated system records and unknown trials do not introduce new data.
- `rust_core/src/io/journal/parser.rs`: replace the removed parallel legacy
  streaming accumulator with a local adapter over that owned replay/manager and
  TableProjection. Inspection found no production caller of the append-only
  callback after D2: its remaining callers are core/desktop test harnesses.
  `StudyStreamBatch` keeps newly COMPLETE rows and adds an optional corrected
  DataFrame. `apply_to(self, &mut DataFrame)` adopts a correction or appends the
  new rows, never both. All repository table-assembling callers now use it.
  This is an internal Rust API correction, not a backwards-compatibility promise.
  The existing legacy extras publication remains; #251 owns its removal.
- A private `read_batches_with_final` helper lets the adapter recognize EOF
  without holding an early batch until the next one arrives. The GUI/headless
  public owned sink remains fallible and unchanged. The adapter owns only its
  local manager, empty derived layout and emitted-ID set; it does not retain a
  second full row authority or install Study data into another ingestion store.
- `rust_core/src/io/journal/parser/study_projection.rs` and
  `rust_core/src/data/dataframe/model.rs`: rebuild corrected values in the
  discovered layout. Trim obsolete constraint columns when current all-state
  evidence shrinks, remapping numeric-role indices without reordering surviving
  columns. Retain discovered numeric/text attribute columns with missing cells
  when a late overwrite changes the last observation's JSON type. The new
  regression initially exposed a column-length assertion here; the correction
  fills these schema columns rather than retaining stale values.
- Legacy `parse_single_study`/`parse_journal` already retain recorded ordinary
  late attributes and still use their existing bulk finalization. MCP's existing
  bulk acquisition therefore retains that information without a new API migration.
  The compatibility tests now compare the independent bulk decoder with the
  owned decoder; legacy streaming intentionally shares owned replay, not an
  independent algorithm that is claimed to cross-validate it.
- `egui-app/src/app/tests.rs`: real open/select worker delivery, early prefix,
  correction before Complete, old-input retention and reload coverage. Production
  GUI orchestration already applies owned full upserts, so no new message or GUI
  ownership layer is needed.
- `rust_core/tests/{issue258_terminal_attributes,study_slice}.rs` and
  `rust_core/src/io/journal/parser/tests.rs`: corrected callback consumption and
  executable regression expectations. The earlier ignored discrepancy diagnostic
  is now a normal agreement test; baseline evidence stays in the historical report.
- `docs/adr/0022-*.md`, ADR index and `CHANGELOG.md`: accepted policy and actual
  user-visible correction. No dependency/workflow/AGENTS changes, crate extraction,
  analysis redesign, commit, push, PR or Issue edit is part of this work.

The independent source-wide Artifact scanner, association map, explicit session
root and GUI resources are unchanged. In particular, a late Artifact-only record
is still found by that scanner; owned trial associations are not substituted for
it. Neither arbitrary system attrs nor upstream rejection policies are imported.

## Direct regression evidence

The exact baseline bytes remain at
`rust_core/fixtures/issue258-terminal-attributes.jsonl`; additional controlled
replacement/limit fixtures are literal byte strings in the Rust regression file.

| Case | Verified result |
| --- | --- |
| COMPLETE then JSON op8 / constraint op9 | Bulk, batch sizes 1/1000 and owned full replay agree on later payload, explicit null/empty keys and infeasibility. All-Study bulk installation also preserves the final payload. |
| Late PRUNED/FAIL constraints | Available all-state evidence changes expected width and historical COMPLETE feasibility, without adding non-COMPLETE analysis rows. |
| Longer -> shorter -> empty replacement | Across successive batch boundaries width is `[1,3,1,0]`; historical rows reclassify from current observations. Invalid/nonfinite slots retain positions before replacement. Width 0 with attribute presence remains constrained/unverified. |
| JSON numeric -> string -> null | Original JSON type and explicit null retained, untouched keys merge, empty array/object survive; old numeric/text layout cells are missing, not stale. |
| Ordering and identity | No duplicate COMPLETE rows; streaming completion order and full-read ID order remain distinct. Parameter discovery order `[z,a]` survives corrections and constraint shrink. Existing completion-order/column-discovery suites pass. |
| Retained input | First published width/value/feasibility remain unchanged while later inputs change revision. |
| EOF correction | Final callback has zero newly COMPLETE rows and adopts corrected values/schema before finish. A rejected final sink delivery returns its error and leaves the accepted prefix intact. |
| Excluded mutations | Late op5/op6/op7 do not modify parameters, objectives, state/time or intermediate values in owned replay; unknown trials and unrelated system attrs do not create records. No new support for these sequences is claimed for bulk. |
| Live GUI | Real 1,000-row prefix is visible/Feasible; real final dirty batch has three upserts (one COMPLETE, one PRUNED, one FAIL), corrects values/width/feasibility while still Loading, then Complete/reload keeps corrected results. Old acquired input and DataFrame stay unchanged. |
| Artifact lifetime | Existing Artifact unit/GUI tests and native Animation capture pass. The baseline late-association scanner fixture remains covered without changing scanner/map ownership. |

The new integration tests serialize only access to the legacy global extras
inside their own test binary. Owned manager tests remain independent. The GUI
test uses the existing desktop store guard and handles actual worker messages
one at a time to observe the early prefix deterministically, not via sleeps.

## Controlled processing-cost observation

Command (run alone after compilation, Windows / Rust 1.99.1 / `ci-test`):

```text
cargo test -p tunny-core --locked --profile ci-test --test issue258_terminal_attributes journal_postprocessing_cost_probe -- --ignored --nocapture
```

Both cases use the same owned replay -> manager -> TableProjection path and
batch size 1,000, with 10,000 inline COMPLETE records, one numeric parameter and
one objective. The second appends 100,000 op8 overwrites across 1,000 existing
trials. Fixture generation, disk I/O and manager setup are outside the timer;
parsing, batch application and projection are inside. First visibility is timed
after the first projection; final time includes acquisition finish. Four paired
runs are printed, with run 0 treated as warm-up and runs 1-3 below reported
without cherry-picking. There is no product timing threshold.

| Case | Bytes | First projection ms (runs 1/2/3) | Final ms (runs 1/2/3) | Batches / upserts / projections |
| --- | --- | --- | --- | --- |
| Completion-heavy | 917,831 | 4.320 / 4.260 / 4.307 | 97.988 / 103.579 / 100.499 | 11 / 10,000 / 11 |
| Same completions + attribute-only tail | 6,495,721 | 5.668 / 5.854 / 5.773 | 171.969 / 175.197 / 176.299 | 11 / 11,000 / 11 |

The 100,000 tail operations produce 1,000 final dirty upserts, **not** 100,000
publications or projections. The EOF no-op delivery in the completion-only case
also counts as one of the 11 projections. Counts are asserted; timings are
observations. This measures current headless processing, not GUI scheduling,
full Pareto/analysis work, external storage, large JSON payloads or multi-Study
filtering. It is not an old-versus-new throughput comparison. The legacy adapter's
separate performance and RSS were not measured. Retaining acquisition builders
and existing full-prefix projections has costs; no constant-time, fastest,
no-regression or bounded-RSS claim is made.

## Verification

Environment: Windows, default Rust 1.99.1 (58cf8f9c3), existing native LightGBM
DLL/import library. Exact CONTRIBUTING commands, audit explicitly excluded:

```text
cargo fmt --manifest-path rust_core/Cargo.toml --all -- --check
cargo fmt --manifest-path egui-app/Cargo.toml --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --profile ci-test
git diff --check
```

| Final check | Outcome |
| --- | --- |
| Both exact manifest-based fmt checks | Passed. |
| Exact workspace all-target clippy | Passed with `-D warnings` on Windows. |
| Exact locked workspace `ci-test` run | Passed. Study 10 tests; core library 1,016 passed / 3 ignored; D2 headless 9 passed; desktop library and binary each 862 passed / 1 ignored; MCP 16 unit + 2 stdio integration passed. Existing RDB integration tests remain 3 ignored. |
| #258 regression binary | 5 passed / 1 explicit cost probe ignored in the normal run. No known failing regression is left behind. |
| Explicit processing-cost probe | Passed; four paired runs and asserted counts above. |
| Live GUI regression | Passed in focused execution and both desktop workspace suites. |
| Native opt-in Animation | Passed separately; the normal headless workspace run intentionally skips native capture. |
| Reference probes | Python Optuna 4.9.0 assertions and Node/browser-source diagnostic assertions passed, not counted as project policy tests. |
| Whitespace | Tracked diff and explicit no-index checks for new artifacts passed; only Git's existing LF/CRLF notices were emitted. |

Focused commands additionally exercise the new Rust integration suite, existing
Journal/Artifact suites, D2 `study_slice`, production-path GUI regression, and the
explicit processing-cost probe. Original Python 4.9.0 and browser-source probes
were rerun as upstream-reference diagnostics; they are not expected to implement
ADR-0022. Their successful assertions include expected upstream input failures.

Native Windows verification uses CONTRIBUTING's opt-in command:

```powershell
$env:TUNNY_NATIVE_CAPTURE_CHECK = "1"
cargo test -p tunny-desktop --locked --profile ci-test --test artifact_animation_native
Remove-Item Env:\TUNNY_NATIVE_CAPTURE_CHECK
```

The native wgpu full-app preview/GIF check passed: 3 frames, identical live/GIF
order, 684 x 544, full prefix observations, two instances, original seek/pause
restoration, PNG/JPEG aspect ratios, 330 ms and content-only crop. This is an
interactive rendering integration check, not a manual review of every workflow.

No Linux/macOS local execution, real RDB/Compute backend validation, security
audit, or memory profiling is claimed. Existing ignored/opt-in tests are not
counted as passing backend evidence. #259/#260 and D3 extraction remain separate.
The previously reported inline completion timestamp loss is unchanged/out of
scope. The approved ordinary-attribute policy needs no further design decision
for this correction; future extraction must preserve it rather than reinstate
upstream rejection as a compatibility requirement.
