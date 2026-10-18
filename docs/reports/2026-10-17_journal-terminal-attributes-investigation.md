# Journal terminal attributes: reproduction and D3 handoff

**2026-10-18 decision update:** the user approved information-preserving
postprocessing (Q3 option A), now recorded in
[ADR-0022](../adr/0022-retain-recorded-journal-attributes-for-postprocessing.md).
The earlier core-conformance recommendation and pending Q2 alternatives below
are **superseded**, not implementation requirements. The following investigation
outputs and limitations describe the historical `8ac8e7f` baseline, not current
post-fix reader behavior. Current correction/verification evidence is in the
[implementation report](2026-10-18_journal-terminal-attributes-validation.md).

Date: 2026-10-17. Baseline: `8ac8e7f` (D2 PR #262 merged).
Branch: `investigate/258-journal-terminal-attributes`.
Original scope: investigation/reproduction for [#258](https://github.com/Tunny-gh/Tunny-Dashboard/issues/258),
not a production correction, Issue closure, or authorization to implement D3.
At that investigation stage no production code, requirements, ADR, terminology,
CHANGELOG, dependencies, or GitHub metadata was changed. The original governing
decisions were ADR-0007/0019/0020/0021; ADR-0022 now resolves project interpretation.

## Conclusion

**The reader discrepancy is reproduced, but loss of supported post-terminal
ordinary user/constraint updates under the Optuna core contract is not established.** Optuna 4.9.0 rejects both
setters on finished trials; replay ignores those rejected updates. Legacy bulk
accepts them, while legacy streaming and both D2 readers ignore them. Making
streaming accept every such record would therefore not be a core-compatibility fix.

**Official-project corroboration revises the breadth of the earlier recommendation:**
Optuna Dashboard's Python server and Rustuna corroborate finished-state rejection,
but Dashboard's independent browser Journal importer applies late updates, with
significant JSON/Artifact limitations. There is no single behavior shared by all
official readers. The proposed legacy-bulk correction remains a core-aligned
option, not an upstream-consensus requirement or an authorized fix. See the
cross-project matrix below before answering the pending Q2 policy question.

There is a separate, already observable Tunny Dashboard Artifact contract: the raw
metadata scanner accepts post-terminal `artifacts:*` associations, and GUI reload
tests depend on that. D2's trial associations do not capture them. Ordinary trial
attribute mutability must not be inferred from this resource-specific behavior.

## Prior requirements checked

- [#200](https://github.com/Tunny-gh/Tunny-Dashboard/issues/200) is closed and
  requires original JSON user values, null versus absence, inline/op8 support and
  append equivalence. It does not explicitly authorize finished-trial mutation;
  Trial System Attributes are an explicit non-goal.
- [#199](https://github.com/Tunny-gh/Tunny-Dashboard/issues/199) is closed and
  concerns presentation of existing attributes, not mutability or log ordering.
- [#194](https://github.com/Tunny-gh/Tunny-Dashboard/issues/194) and ADR-0007 require
  all-state constraint evidence, positional invalidity, empty-attribute evidence
  and historical reclassification. They do not establish that a rejected Optuna
  operation becomes an observation.
- [#245](https://github.com/Tunny-gh/Tunny-Dashboard/issues/245), ADR-0011/0012 and
  current Artifact tests establish a distinct resource/association workflow.
- [#251](https://github.com/Tunny-gh/Tunny-Dashboard/issues/251),
  [#248](https://github.com/Tunny-gh/Tunny-Dashboard/issues/248), the design handoff
  M14/bridge table, and the D2 report preserve staged extraction and separate
  investigation/correction. D2 explicitly limits ordinary updates to before
  terminal state. A parser discrepancy alone is not a new supported format.

## Valid-order evidence

Executed environment: Windows, Python 3.11.0, installed **Optuna 4.9.0**, Rust
stable **1.99.1 (58cf8f9c3)**, existing local LightGBM prerequisites.

Version-pinned upstream source (also inspected in the installed package):

1. [BaseStorage setters](https://github.com/optuna/optuna/blob/v4.9.0/optuna/storages/_base.py#L398-L438)
   document `UpdateFinishedTrialError` for both user and system attribute setters.
2. [JournalStorage](https://github.com/optuna/optuna/blob/v4.9.0/optuna/storages/journal/_storage.py):
   operation enum is 0/4/6/8/9 for Study/trial/state/user/system; setters at
   363-381 write a single-key `user_attr`/`system_attr` record **before** replay;
   replay at 646-679 checks finished state for both. The issuing worker gets the
   exception; a fresh worker ignores the record. Thus a syntactically valid
   record can be present in the journal without being an accepted update.
3. [JournalFileBackend.append_logs](https://github.com/optuna/optuna/blob/v4.9.0/optuna/storages/journal/_file.py#L103-L111)
   appends, flushes and fsyncs logs. The executable probe uses the real
   JournalStorage with a tiny in-memory BaseJournalBackend, not a modified replay
   implementation; physical file locking/durability is source evidence, not a
   file-backend runtime test.
4. [Template creation](https://github.com/optuna/optuna/blob/v4.9.0/optuna/storages/journal/_storage.py#L235-L270)
   embeds state, attributes, distributions, times and scalar `value` (single
   objective) in op4. Inline COMPLETE with attributes is supported; it is not a
   post-terminal update. The exact fixture replays successfully in Optuna.
5. [Tell ordering](https://github.com/optuna/optuna/blob/v4.9.0/optuna/study/_tell.py#L167-L173)
   calls sampler `after_trial` **before** writing the final state.
   [Constraint handling](https://github.com/optuna/optuna/blob/v4.9.0/optuna/samplers/_base.py#L239-L268)
   sets constraints for COMPLETE/PRUNED (not FAIL) and rejects NaN from the
   sampler callback. A real one-trial TPE run confirms `[0,4,9,6]`.
6. [Artifact upload](https://github.com/optuna/optuna/blob/v4.9.0/optuna/artifacts/_upload.py#L109-L115)
   uses the same system-attribute setter; accepting FrozenTrial as an argument
   does not bypass finished-state rejection. The probe verifies rejection of an
   `artifacts:image` setter too; no completed-trial upload support is inferred.

As a historical source cross-check, [Optuna 3.6.1 Journal replay](https://github.com/optuna/optuna/blob/v3.6.1/optuna/storages/_journal/storage.py)
has the same finished-state guard, raising RuntimeError for its issuing worker.
That version was inspected, not installed/executed. No universal claim is made
about every Optuna version or external Tunny producer.

Repository producer evidence:

- `rust_core/src/io/journal/writer.rs::{set_trial_user_attr,set_trial_constraints}`
  directly append records without keeping trial state. Their ability to emit a
  late record is not proof that Optuna accepts it. The writer rejects nonfinite
  constraints; generic JSON parsing has a different ingestion boundary.
- `rust_core/src/runner/mod.rs::finish_complete` writes constraints and user
  attributes before `finish_trial`; `rust_core/src/gh/runner/recorder.rs::finish_complete`
  does the same under one writer lock. Both FAIL paths only finish the trial.
- `rust_core/src/io/artifacts.rs::parse_artifact_metadata` reads associations
  without a terminal-state filter. Its global-ID test includes an inline
  terminal trial followed by an association record. In
  `egui-app/src/app/tests.rs`, the session-root/reload test appends
  `artifacts:new` after completion and requires the rescanned map to contain it.
  This is direct local compatibility evidence for Artifacts only.

## Exact fixtures and executable probes

The byte fixture is
`rust_core/fixtures/issue258-terminal-attributes.jsonl` (UTF-8, newline-delimited
records, one key per op8/op9). It is included verbatim by the Rust harness and
read by the Python probe. Worker IDs and timestamps are fixed; no guessed op2
directions record, duplicate completion, state reversal, or fabricated trial ID
is used. IDs count all CREATE_TRIAL records globally; trial numbers are per Study.
Study 6 intentionally contains Python JSON bare nonfinite literals, supported by
ADR-0007; it is not strict RFC JSON. Invalid constraint values are generic stored
evidence, not a claim that the Optuna sampler accepts an invalid numeric vector.

| Study | Global trial IDs | Case |
| --- | --- | --- |
| 0 | 0 | Before-terminal JSON object/nested array, empty containers, explicit null and JSON-looking plain string; constraint `[-1]`. |
| 1 | 1 | COMPLETE after old payload/`[-1]`, then replacement payload, new null/empty keys and constraint `[1]`. |
| 2 | 2 | Inline COMPLETE template with the same attributes as Study 0 and scalar `value=2`. |
| 3 | 3,4 | COMPLETE `[-1]`, then PRUNED with `[-1]`, then late payload/wider `[-1,0,0]`. |
| 4 | 5,6 | COMPLETE `[-1]`, then FAIL with `[-1]`, then late payload/wider `[-1,0,0,0]`. |
| 5 | 7 | Unconstrained COMPLETE, then explicit-null user attr and `constraints:[]`. |
| 6 | 8,9 | COMPLETE `[-1]`, then still-RUNNING payload `[]` and `[-1,"invalid",NaN,Infinity,-Infinity]`. Valid timing control for all-state schema growth. |
| 7 | 10,11 | Both receive payload before completion; ID 11 completes before ID 10. |
| 8 | 12 | COMPLETE followed only by `artifacts:image` metadata. |

`rust_core/tests/issue258_terminal_attributes.rs::before_finish` supplies paired
controls for trials 1/4/6/7: move all their op8/op9 lines, in original relative
order and without changing their bytes, immediately before their op6. All other
fixture lines are unchanged. The Python probe independently replays the same
permutations in Optuna. This isolates timing from value, state and schema.

Rust harness entry points:

- `supported_before_terminal_and_inline_controls_agree` is a normal passing
  test. It checks valid timing controls, known JSON values, historical schema
  growth, immutable retained input and established row ordering.
- `print_terminal_attribute_reader_observations` is an **explicitly ignored
  diagnostic**, invoked below. It prints each reader's actual metadata, rows,
  raw JSON attrs, padded constraint positions, feasibility, extras, all owned
  records, batch sequence/revision and retained first input. Its unequal-result
  assertions document observations, not a permanent supported-bug contract.
  No deliberately failing expected-behavior test is left in the normal suite.

Legacy streaming is assembled with `DataFrame::append_trials`, cumulative width
and `mark_constrained`, including final zero-row delivery; extras are read after
the parser stores them. D2 uses real StudyManager apply receipts/finish and a
caller-owned TableProjection after every accepted batch. No production reader
or classifier is patched, mocked or bypassed.

## Observed outputs

`python rust_core/tests/issue258_optuna_probe.py` passed. Representative exact
output (the same ops/rejection/retained attributes repeat for PRUNED and FAIL):

```text
Optuna 4.9.0
{"ops": [0, 4, 8, 9, 6, 8, 9, 9], "rejected": ["payload", "constraints", "artifacts:image"], "replay_system": {"constraints": [-1]}, "replay_user": {"payload": {"old": true}}, "state": "COMPLETE"}
TPE constraints order: [0, 4, 9, 6]
before_finish study=1 trial=1: attributes retained, width=1
before_finish study=3 trial=4: attributes retained, width=3
before_finish study=4 trial=6: attributes retained, width=4
before_finish study=5 trial=7: attributes retained, width=0
Optuna API and fixture replay assertions passed
```

All three attempted late setters append one log before raising; fresh replay
retains only the original payload/constraints. The expected experimental warning
for TPE `constraints_func` is emitted; it is not a failed assertion.

Rust final-input observations (`S` = legacy streaming at **both 1 and 1000**;
`D2` = read_batches at **both 1 and 1000**, plus read_full):

| Study | Legacy bulk | S / D2 | Interpretation |
| --- | --- | --- | --- |
| 0 | Original nested JSON/null/string retained, width 1, Feasible | Same | Supported control agrees. |
| 1 | New payload, null and empty keys retained, `[1]`, Infeasible | Old payload only, `[-1]`, Feasible | Confirmed interpretation difference on rejected late updates; no supported streaming loss established. |
| 2 | Original JSON retained, width 1, Feasible | Same | Supported inline attrs agree. See separate timestamp finding below. |
| 3 | Width 3; historical COMPLETE becomes Feasibility unverified | Width 1; historical COMPLETE stays Feasible | Bulk uses late PRUNED evidence that Optuna replay rejects. |
| 4 | Width 4; historical COMPLETE becomes Feasibility unverified | Width 1; historical COMPLETE stays Feasible | Same for FAIL. |
| 5 | Null key present; constrained, width 0, Feasibility unverified | Key absent; unconstrained, width 0, Feasible | Presence differs, not merely numeric width. |
| 6 | Width 5, historical COMPLETE unverified, two extras | Same final projection; D2 additionally retains RUNNING raw attr/constraint slots | Supported all-state evidence works, including invalid/nonfinite positions. |
| 7 | Rows `[10,11]`, numbers `[0,1]` | S/read_batches `[11,10]`, numbers `[1,0]`; read_full `[10,11]` | Established per-path order, not identity/equivalence defect. Values remain attached to the correct IDs. |
| 8 | No ordinary attr/constraint effect | Same projection; D2 association list empty | Independent scanner returns `trial=12 ids=[image]`; preserve its existing behavior. |

All paired before_finish permutations agree across all readers, including width
3/4 reclassification from PRUNED/FAIL records and width-0 constrained emptiness.
Thus neither terminal trial *state* nor JSON/container handling alone causes the
late discrepancy. Ordinary large-batch streaming still detaches a completed row
at op6: delaying callback delivery does not let later attrs update that row.

For Study 6 at batch size 1, legacy streaming emits one row at width 1/Feasible,
then a **final zero-row batch** at width 5/unverified. D2 goes from sequence 1,
revision 1, width 1 to sequence 2, revision 2, width 5 with unchanged COMPLETE row
count. Its retained first input stays width 1/Feasible. At size 1000/read_full,
one publication contains width 5 at revision 1, as expected for a different batch
boundary. No row-count-based revision assumption is made.

Extras agree across Rust readers: all states/IDs/numbers and non-inline times
survive, including PRUNED/FAIL and final RUNNING state. Legacy extras do not store
attrs/constraints; their agreement cannot establish payload retention. D2's
all-state records show PRUNED/FAIL keep their pre-terminal `[-1]`, with late attrs
absent. No extra COMPLETE analysis rows are introduced.

### Source-level explanation

- `parser.rs::parse_single_study` retains builders through ordered replay and
  finalizes afterward. `parser/state.rs` applies op8/op9 to any existing builder
  without checking terminal state.
- `parser.rs::StreamAccum::resolve_trial_state` removes COMPLETE/PRUNED/FAIL
  builders; the 5..=9 dispatch ignores absent builders. Final metadata cannot
  recover the discarded updates, even at a large batch size.
- `parser/study.rs::read_batches` retains builders for acquisition but explicitly
  skips op5..9 for IDs in its terminal set. `read_full` uses that same replay and
  adjusts COMPLETE ordering, not attribute mutability.
- The independent Artifact scanner has neither the ordinary-builder removal nor
  that terminal guard. Using D2 trial associations as a drop-in replacement for
  its source-wide metadata map would lose an existing supported GUI behavior.

## D3 prerequisite and review disposition

The core evidence does **not** justify calling acceptance of ordinary
post-terminal updates a core-compatibility correction. On the tested Optuna contract, the narrow semantic
correction candidate is instead to prevent legacy bulk from applying rejected
ordinary user/constraint updates. That would be observable behavior on logs that
can actually contain rejected operations, so it needs a separately authorized
correction/review, not a hidden extraction change. No correction is implemented.

Before migrating the legacy bulk consumers to D2-owned replay in #251
(`mcp-server/src/storage.rs` delegating to `rust_core/src/io/storage.rs`, and
legacy full-read paths/comparisons), review this exact
compatibility delta: bulk currently applies the records; D2 does not. Do not call
the delta either an approved fix or guaranteed bulk-equivalent migration. D2's
existing before-terminal path does not need new domain API design on this evidence.

Before accepting the #251 Artifact handoff, retain the scanner's source-wide
association coverage, including late metadata already covered by GUI reload
tests. This is within the accepted Artifact contract, not permission to make
ordinary trial attributes mutable or tie maps to a StudyManager. No new general
post-load layer, polling, state transition support or all-at-once loading follows.
CSV/writer/resolver work independent of the affected replay can proceed under its
existing contract; #259/#260 remain separate and are not validated here.

**Genuine next user decision (revised after corroboration):** select the ordinary
attribute compatibility target before authorizing any correction: core/normal
server replay semantics, or intentional raw-record retention closer to the
browser importer/current legacy bulk. Prefer core semantics if Optuna-accepted
updates are the intended authority, while preserving the already accepted local
Artifact resource behavior. Browser parity is a real alternative now evidenced,
but does not itself authorize new mutability or copying its JSON limitations.
Q2 remains unanswered; this investigation does not select that policy, authorize
a fix, change an ADR, finalize a plan, or close #258. Independent D3 work remains
independent; the affected bulk/Artifact handoff cannot hide this difference.

## Official-project corroboration (same-day extension)

This extension is additional reference research, not an answer to Q2 or permission
to correct production behavior. It supersedes any implication that the initial
core-only recommendation represents every official implementation.

### Ownership, pins and kinds of evidence

The [official Optuna website](https://optuna.org/#rustuna) links to
**optuna/rustuna** and [its Dashboard section](https://optuna.org/#dashboard) links
to **optuna/optuna-dashboard**. Their repository URLs, project metadata and README
cross-links agree. The unrelated `neka-nat/rustuna` search result is not this
reference. Rustuna's README credits “Optuna Developers”; being official does not
make all of its implementation details normative for other products.

| Reference | Examined source pin | Runtime pin |
| --- | --- | --- |
| Optuna core | `v4.9.0` (links above) | Installed Optuna 4.9.0, Python 3.11.0. |
| Optuna Dashboard | `v0.21.0`, `fa6ecb50b43d1a9fc8443d69692c8304bf8536a7` | PyPI 0.21.0 wheel in an isolated venv; browser loader transpiled from this exact clone. |
| Rustuna | `python-v0.1.1`, `57302d2c1d6b707be6834dec5742e2280591197f` | PyPI 0.1.1 Windows cp311-abi3 wheel in that venv. No local Rustuna native build claimed. |

As an additional source-only cross-check, [Optuna v5.0.0 Journal replay](https://github.com/optuna/optuna/blob/v5.0.0/optuna/storages/journal/_storage.py)
still routes both attribute operations through its finished-trial guard. Optuna
5.0.0 was not installed or executed; the runtime matrix remains pinned to 4.9.0.

Also fetched main at Dashboard `da449093992a1b889feaaf765acc5645569da0b0`
(2026-10-15) and Rustuna `1105a71cbc629df2d6aa42b299e1139e48cb7ae2`
(2026-10-15). `git diff HEAD FETCH_HEAD` was empty for Dashboard's
`optuna_dashboard/_storage_url.py`, `tslib/storage/src/journal.ts`,
`tslib/storage/package.json`, and Rustuna's `rustuna_storage/src/journal/storage.rs`
and `rustuna_core/src/storage.rs`. This is a targeted source cross-check, not a
main-branch runtime test or a claim that all repository files are identical.

Normative documentation is kept distinct from implementation evidence:

- Optuna BaseStorage explicitly documents finished-trial exceptions.
- [Rustuna's migration guide](https://github.com/optuna/rustuna/blob/python-v0.1.1/docs/docs/migration.md#L58-L67)
  documents string-only attribute values; its README says compatibility is not
  complete and the product is experimental. These are reasons to inspect its
  adapter/schema rather than assume raw Journal interchangeability.
- Dashboard documents an experimental browser-only subset. No examined docs or
  tests promise arbitrary post-terminal JSON mutation. Its permissive browser
  implementation is real runtime evidence, but not a new universal format rule.

### Entry paths and observed behavior matrix

| Entry / actual integration | User attrs and constraints | Artifacts | Rejected-op replay / worker semantics | Tests, runtime and limits |
| --- | --- | --- | --- | --- |
| **Optuna core 4.9.0 JournalStorage** | Both setters reject COMPLETE/PRUNED/FAIL mutations; valid before-terminal values retained. | `artifacts:*` uses the same system setter and guard. | Write precedes replay: issuing worker raises, a new worker ignores late records. | Original probe passed; exact fixture replays with old values. This is the core contract, not every importer. |
| **Dashboard Python server 0.21.0, path/string input** | `get_storage(path)` constructs Optuna JournalStorage; `get_trials` calls its `get_all_trials`. No second raw Journal parser here. | Metadata comes from storage system attrs; late trial association in the exact fixture is absent. Upload HTTP route explicitly rejects finished trials. | Inherits Optuna replay; `/api/trials/.../user-attrs` delegates to storage, not a bypass. | Exact fixture executed through real Dashboard `get_storage`/`get_trials`; old payload/`[-1]` retained. Two upstream Artifact tests passed. No browser/HTTP end-to-end run claimed. |
| **Dashboard Python server, explicit Rustuna storage object** | `get_storage(native)` wraps `rustuna.converter.ToOptunaStorage`. JSON conversion calls native string-map APIs. Native Journal rejects late attrs; constraints convert to indexed `constraints:0` etc. | Existing before-terminal metadata can be retrieved through the adapter; late setter rejected and late metadata absent on fresh replay. | Native Journal appends, then issuing worker raises TrialAlreadyFinished mapped to Optuna UpdateFinishedTrialError; fresh native replay ignores those records. | Probe passed for COMPLETE/PRUNED/FAIL with real files; each rejected attempt appended one record. This does not test every Rustuna backend. Dashboard rejects `may_omit_trials()` storage objects, so discard-enabled storage is not silently accepted. |
| **Dashboard browser / VS Code Journal importer 0.21.0** | Independent TypeScript `JournalFileStorage`, not Optuna core or Rustuna Journal replay. It applies late scalar user attrs and late constraints without a terminal guard. Op8 values are `.toString()`-coerced; null throws. | Trial importer does not populate an Artifact association field; system handler only takes constraints. | Read-only import does not implement an issuing-worker identity or rejection branch; different/same record worker IDs both import late scalar attrs. | Exact fixture throws TypeError. A clearly labeled variant deleting only its three null-valued op8 records executes and applies late COMPLETE/PRUNED/FAIL constraints. Source transpilation probe passed, not a full browser UI/typecheck or upstream JS suite run. |

Pinned integration/source anchors:

- Dashboard [`_storage_url.py`](https://github.com/optuna/optuna-dashboard/blob/v0.21.0/optuna_dashboard/_storage_url.py#L46-L121):
  path input -> Optuna; explicit native object -> ToOptunaStorage. Merely installing
  Rustuna does not switch file-path Journal reading to Rustuna.
- Dashboard [`_storage.py`](https://github.com/optuna/optuna-dashboard/blob/v0.21.0/optuna_dashboard/_storage.py#L15-L41)
  and [`_app.py`](https://github.com/optuna/optuna-dashboard/blob/v0.21.0/optuna_dashboard/_app.py):
  cached acquisition uses storage.get_all_trials; API entry calls get_storage;
  user-attr write endpoint delegates at 504-515. Cache TTL is not a raw-log replay rule.
- Dashboard [`artifact/_backend.py`](https://github.com/optuna/optuna-dashboard/blob/v0.21.0/optuna_dashboard/artifact/_backend.py#L123-L132)
  rejects upload to a finished trial; metadata lookup at 318-336 reads storage
  attrs (also handling a legacy study-level Dashboard prefix), not raw log lines.
- Browser [`browser_app_entry.tsx`](https://github.com/optuna/optuna-dashboard/blob/v0.21.0/standalone_app/src/browser_app_entry.tsx)
  starts [`storage_worker.ts`](https://github.com/optuna/optuna-dashboard/blob/v0.21.0/tslib/storage/src/storage_worker.ts#L160-L178),
  which selects the independent [`journal.ts`](https://github.com/optuna/optuna-dashboard/blob/v0.21.0/tslib/storage/src/journal.ts#L377-L403)
  loader for a Journal buffer. Browser and VS Code share this storage worker;
  neither that Journal module nor its package manifest depends on Rustuna.
  Rustuna's general README mentions Dashboard development, but does not override
  this traced version-specific Journal entry path.
- Rustuna [`journal/storage.rs`](https://github.com/optuna/rustuna/blob/python-v0.1.1/rustuna_storage/src/journal/storage.rs):
  `set_trial_attrs` at 652-721 writes dummy Optuna fields plus `*_attr_str` maps;
  `trial_exists_and_updatable` at 906-945 implements issuing/other-worker handling;
  user replay at 1371-1413 and system replay at 1416-1468 reject finished records.
  System handling checks each key but has **no Artifact exception**.
- Rustuna [`converter/_storage.py`](https://github.com/optuna/rustuna/blob/python-v0.1.1/rustuna_pyo3/rustuna/converter/_storage.py#L401-L449)
  maps exceptions and returns converted trials; [`converter/_attrs.py`](https://github.com/optuna/rustuna/blob/python-v0.1.1/rustuna_pyo3/rustuna/converter/_attrs.py)
  defines JSON markers and indexed constraints. Native `_str` storage, converted
  Optuna views, and raw Optuna-format journals are not interchangeable representations.

### Exact-fixture results and meaningful limitations

The Python server reads the **unchanged** fixture. Its Study 1 retains
`{"payload":{"old":true}}` and `constraints:[-1]`; Study 5 retains no null or
constraint evidence; Study 8's `get_trial_artifact_meta(...,"image")` returns None.

Rustuna rejects the unchanged full fixture while creating Journal storage:

```text
RuntimeError: Failed to create journal storage: StorageError: expected value at line 1 column 92 (at rustuna_storage\src\journal\file.rs:431)
```

Removing only the single op9 line containing `NaN,Infinity,-Infinity` makes all
nine Studies load, with trial 12's late Artifact still absent. That isolates the
generic-JSON/nonfinite input boundary in Study 6, not a late-attribute rejection;
it is not normalization or a claim that the original file loaded. The probe also
writes an **exact finite byte prefix**,
stopping immediately before Study 6, without rewriting earlier records. Native
Rustuna reads the trials but ignores ordinary Optuna op8 `user_attr` / op9
`system_attr` fields, even before completion. In contrast, op4's plural
`user_attrs`/`system_attrs` are imported as strings: inline null becomes `"null"`
and inline constraints become `"[-1]"`. Source and runtime agree on that
distinction. Therefore this raw-file comparison cannot establish that Rustuna
lost valid late JSON attrs; native/adapter writes must be tested separately.

Those real adapter/native-file tests passed. Representative output (same result
for PRUNED and FAIL):

```text
{"dashboard_adapter": "ToOptunaStorage", "rejected": ["payload", "constraints", "artifacts:late"], "rustuna_state": "COMPLETE", "system": {"artifacts:before": "{\"artifact_id\": \"before\"}", "constraints:0": -1.0}, "user": {"payload": {"old": true}}}
Dashboard 0.21.0 / Rustuna 0.1.1 corroboration assertions passed
```

Browser exact-fixture output:

```text
Dashboard browser exact fixture: TypeError: Cannot read properties of null (reading 'toString')
```

The derived browser variant removes **only** the three lines containing
`"user_attr":{"null":null}`. It keeps inline null and every other record. It
produces Study 1 `constraints:[1]`, PRUNED width 3, FAIL width 4; payload objects
become `"[object Object]"` and empty array attrs become `""`. A separate four-operation
scalar fixture in the probe (Study/create/complete/op8)
imports `text:"late"` for both record worker IDs. It also confirms inline COMPLETE
constraints are not populated by this importer. Nonfinite constraint positions
remain JS NaN/Infinity/-Infinity, checked directly; JSON console serialization
shows nulls for these numbers and is **not** evidence of zero/null imputation.
These limitations prevent treating this importer as a full JSON-semantic oracle
or expanding #200/#258 merely to copy it.

### Upstream tests versus probes

- Executed all five tests in Rustuna's
  [`test_journal_storage.py`](https://github.com/optuna/rustuna/blob/python-v0.1.1/rustuna_pyo3/tests/storage_tests/test_journal_storage.py):
  **5 passed** against the published 0.1.1 wheel. They include explicit late
  user-attr rejection/future readability and dummy-field Optuna interoperability.
- Inspected Rustuna's Rust unit `late_multi_attr_write_does_not_break_future_replay`
  and ignored integration test
  [`get_trials_from_optuna_journal`](https://github.com/optuna/rustuna/blob/python-v0.1.1/rustuna_storage/tests/journal.rs#L151-L225).
  The latter explicitly expects ordinary Optuna user-attr events to be ignored.
  These Rust tests were **not** executed; the runtime native checks above use the wheel.
- Executed Dashboard's `test_get_artifact_meta` and
  [`test_upload_artifact_invalid_complete_trial`](https://github.com/optuna/optuna-dashboard/blob/v0.21.0/python_tests/artifact/test_backend.py#L230-L248):
  **2 passed / 15 deselected**. The upload test uses InMemoryStorage, not Journal;
  Journal/server behavior is separately tested by the fixture probe.
- Inspected browser [`journal.test.mjs`](https://github.com/optuna/optuna-dashboard/blob/v0.21.0/tslib/storage/test/journal.test.mjs):
  ordinary loads, constraints, nonfinite values and parse errors are covered;
  no examined case establishes post-terminal mutation as a promised contract.
  The upstream JS suite was **not run** (local Node 18 lacks its `openAsBlob`
  entry API). The diagnostic transpiles the unmodified Journal module with
  TypeScript 5.9.3 and executes its public loader under Node 18.14.0 instead.

### Reproduction commands and outcomes for this extension

All clones, Python installations and JS compiler dependencies are outside the
workspace under the approved temp root. The two clones remain at their release
pins and clean. No upstream source is patched or vendored into this repository.
Equivalent PowerShell commands using the exact locations/versions executed:

```powershell
$tmp = "C:\Users\dev\AppData\Local\Temp\opencode"
Test-Path -LiteralPath $tmp
git clone --depth 1 --branch v0.21.0 https://github.com/optuna/optuna-dashboard.git "$tmp\issue258-optuna-dashboard"
git clone --depth 1 --branch python-v0.1.1 https://github.com/optuna/rustuna.git "$tmp\issue258-rustuna"
python -m venv --system-site-packages "$tmp\issue258-upstream-env"
$py = "$tmp\issue258-upstream-env\Scripts\python.exe"
& $py -m pip install optuna-dashboard==0.21.0 rustuna==0.1.1 pytest==8.4.2
npm install --prefix "$tmp\issue258-js-tools" --no-package-lock --no-save typescript@5.9.3
& $py rust_core/tests/issue258_optuna_probe.py --upstream-temp $tmp
node rust_core/tests/issue258_dashboard_probe.cjs "$tmp\issue258-optuna-dashboard" "$tmp\issue258-js-tools\node_modules\typescript"
$env:PYTHONDONTWRITEBYTECODE = "1"
$env:TMP = $tmp
$env:TEMP = $tmp
& $py -m pytest -q -p no:cacheprovider --import-mode=importlib "$tmp\issue258-rustuna\rustuna_pyo3\tests\storage_tests\test_journal_storage.py"
& $py -m pytest -q -p no:cacheprovider --import-mode=importlib "$tmp\issue258-optuna-dashboard\python_tests\artifact\test_backend.py" -k "upload_artifact_invalid_complete_trial or get_artifact_meta"
```

Both final probes passed their diagnostic assertions (including expected input
failures), and upstream pytest outcomes are as above. Initial pytest collection
without `--import-mode=importlib` shadowed the wheel with the unbuilt clone and
failed to find `rustuna._rustuna`; the final importlib run avoids that environment
problem. No unrun test is counted as passing. This research does not validate
full HTTP/UI behavior, alternate backends, all versions, Rustuna JS bindings, or
the #259/#260 gates. Security audit remains excluded.

Local extension checks also passed: `node --check
rust_core/tests/issue258_dashboard_probe.cjs`, the normal Rust investigation suite
(1 passed / 1 diagnostic ignored), both exact manifest-based fmt commands, exact
workspace all-target clippy, `git diff --check`, and no-index whitespace checks
of the updated report/Python probe/new JS probe. The Rust fixture and Rust test
were not changed by this extension. No full workspace runtime rerun is claimed.

## Separate finding (outside the terminal-attribute investigation)

Study 2 contains inline `datetime_complete="2026-10-17T00:01:00"`. Optuna retains
it, but all three Rust reader implementations yield `complete:null` in extras.
`parser/state.rs::process_create_trial` initializes inline `datetime_complete` to
None. This is a shared pre-existing inline timestamp loss, not a late-attribute
or bulk/streaming difference. It is recorded here and not corrected. Inline
intermediate-value retention and categorical encoding were not investigated.

## Verification commands and limits

All commands are from the repository root. Final command outcomes are recorded
after the investigation harness compiles; initial authoring compile/format errors
were corrected in the new test file only, not treated as reader failures.

```text
python rust_core/tests/issue258_optuna_probe.py
cargo test -p tunny-core --locked --profile ci-test --test issue258_terminal_attributes
cargo test -p tunny-core --locked --profile ci-test --test issue258_terminal_attributes -- --ignored --nocapture
cargo test -p tunny-core --locked --profile ci-test --lib io::journal
cargo test -p tunny-core --locked --profile ci-test --lib io::artifacts
cargo test -p tunny-core --locked --profile ci-test --test study_slice
cargo test -p tdashboard-study --locked --profile ci-test
cargo fmt --manifest-path rust_core/Cargo.toml --all -- --check
cargo fmt --manifest-path egui-app/Cargo.toml --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
git diff --check
```

The first completed combined harness invocation (`--include-ignored --nocapture`)
reported 2 passed. Normal invocation reports 1 passed / 1 diagnostic ignored.
The final explicit diagnostic invocation reports 1 passed / 1 filtered out.

| Final check | Outcome |
| --- | --- |
| Optuna probe | Passed, including COMPLETE/PRUNED/FAIL rejection, rejected-log replay, actual sampler order, inline completion timestamp and four reordered controls. |
| Normal investigation Rust suite | 1 passed / 1 diagnostic ignored. |
| Explicit ignored diagnostic | 1 passed / 1 filtered out; all nine Studies and six reader/batch modes printed. |
| Journal library suite | 82 passed, including writer round-trip and parser schema/JSON/nonfinite cases. |
| Artifact library suite | 16 passed. |
| D2 headless `study_slice` | 9 passed. |
| `tdashboard-study` | 10 passed (2 unit + 8 management); 0 doctests. |
| Both exact manifest-based fmt checks | Passed. |
| Exact all-target workspace clippy | Passed on Windows / Rust 1.99.1. Linux CI was not run locally. |
| Whitespace | `git diff --check` passed; new, untracked files are additionally checked using `git diff --no-index --check -- /dev/null <path>`. |

The version-pinned Python probe is manual, not an added CI dependency. Full
workspace runtime tests, GUI/native capture, live server DBs, and other Optuna
versions were not executed for this investigation. Security audit is explicitly
excluded. These limits are not passing claims. No performance/locking guarantee
or new product support is inferred from this fixture.
