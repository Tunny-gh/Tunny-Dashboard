# RDB attribute-only fingerprint investigation (#259)

Date: 2026-10-18
Baseline: clean `main` at `b1ad17e` before adding regression fixtures.

## Contract and consumer evidence

The user explicitly approved detection of supported existing-trial attribute
insertion and in-place updates as a public API requirement, even without a GUI
consumer. This report establishes an API defect, **not a demonstrated GUI
refresh failure**. No GUI interaction was used as reproduction evidence.

Source anchors below refer to the baseline unless described as new:

- `rust_core/src/lib.rs:7,24`, `io/rdb/mod.rs:20-24,85-107` expose generic,
  reusable-session and one-shot URL fingerprint APIs; `io/sqlite/mod.rs:38-46`
  delegates SQLite to the same implementation.
- `rust_core/src/io/rdb/generic/fingerprint.rs:7-22,47-121` documents change
  detection but queries only trial/state/intermediate aggregates. Neither
  attribute table contributes to baseline equality.
- Workspace searches for `study_fingerprint`, `StudyFingerprint`,
  `RdbFingerprintSession`, `RdbLivePoller` and `restart_poller` located wrappers,
  re-exports, tests, the smoke example and historical references, not an active
  production fingerprint gate. External consumers are unknown.
- `rust_core/examples/rdb_smoke.rs:116-132` prints the fingerprint, without
  suppressing rereads. `egui-app/tests/rdb_integration.rs:91-118` explicitly
  documents that the GUI no longer polls and tests public API stability instead.
- GUI reads call `parse_single_study_rows[_url]` directly in
  `egui-app/src/io/{sqlite,rdb}.rs:46-64`; worker selection/caching is at
  `egui-app/src/io/study_worker.rs:172-222`. Reload starts scan/select again
  (`egui-app/src/app/reload.rs:112-179`), clears loaded IDs on successful scan
  (`study_worker.rs:150-169`) and forces comparison rereads (`234-266`).
- MCP delegates through `mcp-server/src/storage.rs:8` to
  `rust_core/src/io/storage.rs:42-57`, directly parsing storage.
- `README.md:21-28` documents JSON user attributes and manual Reload.
  Historical polling text in the roadmap is not current implementation evidence.
- Current parser coverage is COMPLETE user attributes
  (`rdb/generic/study_rows.rs:221-258`) and all-state constraints (`264-289`),
  including attribute presence and maximum observed width. The latter must retain
  ADR-0007 feasibility semantics and invalid positions. ADR-0019/0020/0021 separate
  this investigation from extraction, new polling and external snapshot work.

Read the full bodies and empty comment lists of #259 and #251 using:

```powershell
gh issue view 259 --repo Tunny-gh/Tunny-Dashboard --json body,comments
gh issue view 251 --repo Tunny-gh/Tunny-Dashboard --json number,title,body,comments,state,url
```

## Executable fixtures and environment

New tests in `rust_core/src/io/sqlite/tests.rs` reuse `create_schema`,
`seed_basic`, `tempfile::NamedTempFile`, `rusqlite::Connection` and the public
`SqliteBackend`. Study 1 contains COMPLETE trials 1/2/3 and PRUNED trial 4,
with an existing `score=12.5` user attribute on trial 1 and initially no constraints.

Each scenario creates its own database. A separate writer executes each SQL
statement below inside `BEGIN; ...; COMMIT;` and verifies autocommit afterward.
One retained read-only backend fetches both fingerprints and direct parsed Study
outputs before/after committed changes. Fresh path-wrapper reads check unchanged
stability after every mutation. The COMPLETE row count stays 3; all original
fingerprint fields are explicitly asserted unchanged. The helper collects all
missed changes before failing, so baseline execution reaches in-place updates too.
These are committed **database** mutations, not Git commits.

Executed on Windows with Rust `1.99.1 (58cf8f9c3 2026-10-13)` and Cargo
`1.99.1 (5f94df478 2026-08-27)`, using locked workspace dependencies.
`cargo tree -p rusqlite --locked --depth 1` confirmed `rusqlite 0.40.2` and
`libsqlite3-sys 0.38.2`; the project enables bundled SQLite.
`rusqlite::version()` returned SQLite **3.53.2**; `SELECT sqlite_source_id()`:

```text
2026-06-03 19:12:13 d6e03d8c777cfa2d35e3b60d8ec3e0187f3e9f99d8e2ee9cac695fd6fcdf1a24
```

No PostgreSQL or MySQL server was used or version queried. Their shared generic
queries are source evidence only, not runtime verification. The existing ignored
PostgreSQL integration suite was not enabled; no server mutation harness was added.

### Exact mutation SQL

COMPLETE trial user attributes:

```sql
INSERT INTO trial_user_attributes (trial_id,key,value_json) VALUES (2,'details','{"v":1}');
UPDATE trial_user_attributes SET value_json = '{"v":2}' WHERE trial_id = 2 AND key = 'details';
```

COMPLETE trial constraints:

```sql
INSERT INTO trial_system_attributes (trial_id,key,value_json) VALUES (1,'constraints','[-1]');
UPDATE trial_system_attributes SET value_json = '[1]' WHERE trial_id = 1 AND key = 'constraints';
```

PRUNED trial constraint presence/schema growth (all-state evidence):

```sql
INSERT INTO trial_system_attributes (trial_id,key,value_json) VALUES (4,'constraints','[]');
UPDATE trial_system_attributes SET value_json = '[-1]' WHERE trial_id = 4 AND key = 'constraints';
UPDATE trial_system_attributes SET value_json = '[-1,-2]' WHERE trial_id = 4 AND key = 'constraints';
```

### Before-fix execution

After adding tests, before changing production detection:

```powershell
cargo test -p tunny-core --locked --profile ci-test study_fingerprint_detects_ -- --nocapture
```

Result: **0 passed, 3 failed**, all failures at the final missed-mutation assertion:

```text
complete_constraint_mutations: attribute-only mutations missed: [0, 1]
complete_user_attribute_mutations: attribute-only mutations missed: [0, 1]
noncomplete_constraint_schema_mutations: attribute-only mutations missed: [0, 1, 2]
```

For every one of the seven mutations, before and after fingerprints were exactly:

```text
StudyFingerprint { total_trials: 4, completed_trials: 3, max_trial_id: 4,
intermediate_count: 0, state_digest: 14519413495919521364 }
```

Direct-load assertions passed before the detection assertions failed:

| Scenario | Direct output after successive mutations |
| --- | --- |
| COMPLETE user attribute | `details={"v":1}`, then `details={"v":2}` on trial 2 |
| COMPLETE constraint | width 1; trial 1 `Feasible`, then `Infeasible` |
| PRUNED constraint | constrained status true; widths 0, 1, 2; every COMPLETE trial `Unverified` |

This proves committed, supported data changes were readable but missed by the
public fingerprint, independently of any application refresh consumer.

### Correction and after-fix execution

Add `attribute_digest` to `StudyFingerprint`, leaving existing fields/queries
intact. Two ordered callback queries scan COMPLETE user attributes and all-state
constraints; the hash includes kind, trial ID and length-framed key/JSON text.
No new dependency, poller, GUI behavior, migration or transaction was added.
See [ADR-0023](../adr/0023-include-supported-attribute-changes-in-rdb-fingerprints.md).

```powershell
cargo fmt --manifest-path rust_core/Cargo.toml --all
cargo test -p tunny-core --locked --profile ci-test study_fingerprint -- --nocapture --test-threads=1
```

Result: **8 passed, 0 failed**. Direct outputs remained as above; existing
unchanged-data, trial insertion, state update, intermediate insertion and absent
intermediate-table tests passed. All original fields retained the values above
throughout the seven new mutations. Observed attribute digests:

| Scenario | Initial | After insertion | After update | After width growth |
| --- | --- | --- | --- | --- |
| COMPLETE user attribute | 13423530912629963592 | 14428014595441167251 | 12603345362095467054 | — |
| COMPLETE constraint | 13423530912629963592 | 420736720295189052 | 11810160314974684004 | — |
| PRUNED constraint | 13423530912629963592 | 6769244686832877827 | 6652745578231674791 | 11085641508577146171 |

## Cost, limits and migration relationship

Two extra queries read matching attribute payloads and order by trial ID/key.
Client work scales with those bytes, without JSON parsing or full-result
collection. Database join/sort work, driver buffering and network traffic remain;
no benchmark, latency target, constant-memory backend guarantee or speedup is
claimed. Raw JSON text intentionally permits formatting-only false positives.
The existing non-cryptographic 64-bit hash can collide; equality is not semantic
equivalence. Coverage does not expand to unrelated collision candidates.
The added public struct field requires callers constructing `StudyFingerprint`
literals to supply it; backward compatibility was not retained as a constraint.

No snapshot guarantee is introduced: concurrent writers across queries remain
#260. Runtime all-state schema coverage here uses PRUNED; other states share the
unfiltered query but were not individually mutated in this reproduction.

#251 should migrate this corrected public contract if/when moving fingerprint
APIs, not preserve this confirmed omission as compatibility behavior. There is no
active GUI detection path requiring a new poller, and no dependency making #259 a
prerequisite for every #251 path. Extraction and analysis APIs were not changed.

## Required verification

Executed exact CONTRIBUTING commands:

| Command | Result |
| --- | --- |
| `cargo test --workspace --locked --profile ci-test` | Passed on rerun with a longer command timeout; first invocation hit the tool's 120-second timeout during compilation, not a reported test failure. Core unit tests: 1019 passed, 3 ignored; Study: 2 unit + 8 integration passed; core integration: 5 passed/1 ignored and 9 passed; desktop library and binary: each 862 passed/1 ignored; MCP: 16 unit + 2 integration passed; doc tests passed. PostgreSQL suite: 3 ignored; opt-in native capture skipped. |
| `cargo fmt --manifest-path rust_core/Cargo.toml --all -- --check` | Passed, no output. |
| `cargo fmt --manifest-path egui-app/Cargo.toml --all -- --check` | Passed, no output. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed; no warnings, initially finished in 13.99 seconds; final rerun after rustdoc edits passed in 4.54 seconds. |

Both formatting checks also passed again after the final rustdoc edit.
`git diff --check` passed (Git printed only Windows LF/CRLF conversion notices).

The executable before/after SQLite evidence, public API correction, unchanged and
existing detection controls, ADR, changelog and requested checks are complete.
PostgreSQL/MySQL runtime behavior and external snapshot validation are not claimed.
No commit, push, PR or issue closure was performed; final independent review is
owned by the lead.
