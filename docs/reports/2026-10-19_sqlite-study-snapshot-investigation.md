# SQLite Study acquisition snapshot investigation and correction

Date: **2026-10-19**, matching the local environment date. Baseline: `a40db5d`.
Issue: [#260](https://github.com/Tunny-gh/Tunny-Dashboard/issues/260).
Branch: `fix/260-sqlite-study-snapshot`. The user reviewed the SQLite evidence,
accepted acquisition-limited DELETE writer contention and WAL checkpoint
retention, and approved the minimal consolidated production path (Q5 A).

The correction and regressions are implemented locally. No commit, push or PR
has been made; fresh independent review is still pending. The security audit
could not execute because `cargo-audit` is not installed (details below).
PostgreSQL, MySQL and MariaDB were explicitly excluded by the user and remain
unverified; their nonverification does not block the agreed SQLite scope. This
report does not close the original broader all-family Issue.

Support-boundary clarification (2026-10-20): the server-family references in this
report preserve the historical SQLite investigation scope, not the current
library support contract. Server database support is limited to PostgreSQL and
MySQL; MariaDB is outside the library's scope and unsupported, not a future
verification gate. See [ADR-0021](../adr/0021-study-analysis-and-io-boundary-contracts.md#database-acquisition-sessions).

Policies remain in [ADR-0020](../adr/0020-commit-study-updates-by-validated-batch.md)
and [ADR-0021](../adr/0021-study-analysis-and-io-boundary-contracts.md#database-acquisition-sessions).
ADR-0021 now clarifies the approved concrete SQLite API boundary; no redundant
ADR or domain terminology was introduced.

## Implemented boundary

Both existing public file-path APIs retain their signatures:

1. `sqlite::parse_single_study_rows` opens the existing read-only/no-mutex
   connection, then invokes concrete SQLite `with_study_read`.
2. That callback uses rusqlite `TransactionBehavior::Deferred` and a borrowed
   query facade. The original generic decoder reads schema presence, Study
   name/directions/metric names/counts, all-state extras/intermediate values,
   COMPLETE IDs/objectives/parameters/user attributes, and all-state constraints
   on this same connection/session. The first read establishes the snapshot.
3. The driver commits before returning owned rows. A callback/query/conversion
   error drops the RAII transaction and rolls back; a begin/finish failure returns
   an error rather than data. The file-path function then drops the connection.
4. `sqlite::parse_single_study` calls that rows API and only then invokes the
   shared `study_rows_to_dataframe` helper. That helper contains the unchanged
   assembly tail previously inside the generic DataFrame parser. It accepts only
   owned rows, not a connection or transaction: acquisition has ended before
   projection, handoff or analysis.

The ordinary SQLite backend and transaction facade share exactly one query,
conversion and table-existence implementation. The experimental SQL rollback
guard was removed. No connection/session is opened for a second SQLite load in
the DataFrame route. No timeout override, automatic retry, independent-read
fallback, fingerprint/list-scan/polling change, expanded trial semantics, new UI,
server transaction hook or crate extraction was introduced.

The public backend-agnostic `rdb::parse_single_study_rows` and
`rdb::parse_single_study` remain raw query decoders. Their documentation explicitly
assigns snapshot/session responsibility to the caller. Direct generic calls are
not covered by the file-path acquisition guarantee. Existing production SQLite
entry callers use the file-path APIs; they require no overhaul.

## Environment and fixture settings

Windows x64; Rust `1.99.1 (58cf8f9c3 2026-10-13)`; Cargo
`1.99.1 (5f94df478 2026-08-27)`. Both `libs/lib_lightgbm.dll` and
`libs/lib_lightgbm.lib` were present before compilation. Native linking and the
workspace tests succeeded. Locked dependency: rusqlite **0.40.2**, bundled SQLite:

```text
SQLite 3.53.2
sqlite_source_id():
2026-06-03 19:12:13 d6e03d8c777cfa2d35e3b60d8ec3e0187f3e9f99d8e2ee9cac695fd6fcdf1a24
```

Both DELETE and WAL fixtures reported `synchronous=2`, `page_size=4096`,
`wal_autocheckpoint=1000`, `read_uncommitted=0`, `locking_mode=normal`.
No shared-cache option is enabled. Readers retain the driver's **5000 ms** busy
timeout; writers use **0 ms** solely to bound the experiment. Production flags,
timeouts and retry policy are unchanged.

`rust_core/src/io/rdb/sqlite_snapshot_tests.rs` contains the exact nine-table
minimal reader-compatible schema and SQL. Each case owns an independent temporary
directory/database with one COMPLETE trial and one RUNNING trial, finite objective,
numeric parameter/distribution, nested JSON user attribute, constraint, timestamps
and intermediate value. This is not a full Optuna migration schema or an upgrade
test. All writes are confined to test-owned fixtures.

## Deterministic schedule and oracle

The main reader and a separate writer thread own independent connections. A
`cfg(test)` thread-local hook fires after a selected real query has finished and
its cursor/statement have dropped. The hook reaches the actual public file-path
APIs without a replacement parser or transaction guard; it is absent in release
builds and removed by a test guard even on unwind. One-slot channels use
`try_send` and five-second receive deadlines; there are no sleeps.

The writer performs `BEGIN IMMEDIATE`, applies one atomic mutation and attempts
COMMIT. It acknowledges either success or `SQLITE_BUSY`. The reader completes
and releases its session before the writer retries a pending DELETE commit.
That explicit harness retry is not product behavior. The deadline is not a
product latency guarantee. Threads finish before fixture removal.

Mutation/barrier pairs:

- **Attributes:** pause after COMPLETE user attributes, before constraints.
  Atomically replace `tag={"version":0}` with version 1 and `[-1]` constraints
  with `[2,3]`. Both trial counts and states stay unchanged.
- **Transition:** pause after extras/intermediate values, before COMPLETE IDs.
  Atomically change Study name/direction/metric name, trial 2 RUNNING to COMPLETE
  and completion time, trial 1 objective 10 to 20, x=1 to x=2, attributes,
  constraints and intermediate value, and add trial 2 objective 30.

Separate fixtures establish complete old/new expected views. The rows oracle
compares every exposed `RdbStudyRows` field, including metadata, columns/bounds,
distributions, IDs/numbers, all payload maps, constraints/width, timestamps and
all-state extras. The DataFrame route compares metadata/extras, all exposed table
column values and original user-attribute JSON. Full views are printed under
`--nocapture`. The fixture is finite; no all-category/nonfinite-value claim follows.

## Reproduction and corrected-route results

The same query implementation still reproduces the original defect through
independent generic decoding. Summary of all twelve final interleavings:

| Journal | Mutation | Independent decoder | Public rows path | Public DataFrame path | Writer commit while corrected reader retained |
| --- | --- | --- | --- | --- | --- |
| DELETE | Attributes, unchanged counts | Mixed | Whole old view | Whole old view | SQLITE_BUSY; succeeds after release |
| DELETE | RUNNING to COMPLETE | Mixed | Whole old view | Whole old view | SQLITE_BUSY; succeeds after release |
| WAL | Attributes, unchanged counts | Mixed | Whole old view | Whole old view | Succeeds before release |
| WAL | RUNNING to COMPLETE | Mixed | Whole old view | Whole old view | Succeeds before release |

Example final trace lines:

```text
DELETE Attributes route=Independent reader_busy_ms=5000 writer_commit_before_release=true result=mixed
DELETE Attributes route=Rows reader_busy_ms=5000 writer_commit_before_release=false result=whole-old
DELETE Attributes route=DataFrame reader_busy_ms=5000 writer_commit_before_release=false result=whole-old
WAL Transition route=Independent reader_busy_ms=5000 writer_commit_before_release=true result=mixed
WAL Transition route=Rows reader_busy_ms=5000 writer_commit_before_release=true result=whole-old
WAL Transition route=DataFrame reader_busy_ms=5000 writer_commit_before_release=true result=whole-old
```

The independent attribute view pairs old `tag={"version":0}` with new constraints
`[2.0,3.0]`/width 2, total=2 and COMPLETE=1. Neither committed whole view has that
pair. The independent transition view pairs old Study name/direction/metric and
RUNNING/null-completion extras with two COMPLETE rows, new objectives `[20.0]`
and `[30.0]`, x=2 and new attributes/constraints. Its intermediate value is still
10. These are demonstrated acquisition mixtures, not a demonstrated GUI symptom.
After writer completion, every fresh route matches its complete new expected view.

### Controls, errors and resource release

Both modes pass unchanged-database comparison. A commit after BEGIN DEFERRED but
before the first read appears as the whole new view: BEGIN itself does not pin
the snapshot. A first writer transaction commits attributes, the read establishes
that intermediate snapshot, then a second transaction changes state/payloads.
Repeated reads in the production session retain that legitimate intermediate
committed view; a subsequent session sees the final view. This does not make a
writer's multi-transaction business operation atomic. These control writers are
explicitly scheduled independent connections in one thread; the query-boundary
reproductions use the separate writer thread.

Fresh-fixture cases exercise the production RAII callback for missing-table query
failure, callback termination after one row, actual parser rejection of text in
`trial_params.param_value`, rejected write through the read-only reader, early
acquisition error, successful completion, and finish failure. The finish case
explicitly rolls back inside the test callback so the driver's subsequent COMMIT
fails; it verifies error propagation/no returned data, not storage-fault recovery.
Both public path APIs also reject the malformed fixture without fallback.

Expected errors are asserted by cause. All cases restore reader autocommit,
release pending DELETE writer commits, and allow reader reuse equal to a fresh
read. A nested BEGIN failure does not call acquisition or end the caller's existing
transaction. Connection drop releases a retained read lock; WAL truncation then
succeeds. There is no supported load-cancellation interface to test.

WAL `wal_checkpoint(TRUNCATE)` returns `(busy, log, checkpointed)`:

```text
query / callback / read-only-write / early-return / finish / success, pinned: (1, 2, 0)
parser-conversion, pinned:                                                  (1, 3, 1)
after each production session ended:                                        (0, 0, 0)
```

Thus the accepted cost is DELETE writer contention or WAL checkpoint retention
for acquisition lifetime only. No writer wait duration, WAL byte-growth, RSS,
large-database throughput, starvation or performance guarantee was measured.
I/O faults, failed rollback, abrupt process death, mid-query connection loss and
filesystem faults were not injected. Neither normal connection drop nor the
synthetic finish failure establishes recovery from those faults.

## Exact verification and current limits

Executed from the repository root in PowerShell:

```powershell
Get-Date -Format yyyy-MM-dd
Test-Path -LiteralPath "libs/lib_lightgbm.dll"
Test-Path -LiteralPath "libs/lib_lightgbm.lib"
cargo fmt --manifest-path rust_core/Cargo.toml --all
cargo test -p tunny-core --lib --locked --profile ci-test sqlite_snapshot -- --nocapture --test-threads=1
1..3 | ForEach-Object { cargo test -p tunny-core --lib --locked --profile ci-test sqlite_snapshot --quiet; if (-not $?) { throw 'Snapshot repeat failed' } }
cargo fmt --manifest-path rust_core/Cargo.toml --all -- --check
cargo fmt --manifest-path egui-app/Cargo.toml --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --profile ci-test
cargo audit --ignore RUSTSEC-2026-0194 --ignore RUSTSEC-2026-0195
git diff --check
```

- Focused regressions: **3 passed**, 0 failed; final trace runtime 1.00 s.
  Three default-parallel repeats passed in 0.62 / 0.62 / 0.59 s.
- Both exact formatting checks and all-target workspace Clippy passed.
- Full locked ci-test workspace command passed: Study unit/contract tests **2+8**;
  core **1022 passed, 3 ignored**, core integration **5 passed, 1 ignored** and
  **9 passed**; desktop library and binary **862 passed, 1 ignored each**;
  MCP **16+2 passed**. Three live PostgreSQL integration tests were ignored.
  Doc tests passed (zero cases). Native Artifact Animation capture explicitly
  skipped its opt-in interactive check. These skips are not backend/native GUI
  validation claims. All **2788 executed Rust tests passed**, 9 were ignored.
- Audit was attempted with the exact command but failed with
  `error: no such command: audit`. No cargo-audit installation or advisory scan
  occurred, so no clean security-audit result is claimed.
- `git diff --check` passed. No commit/push/PR or independent review is claimed.

The original investigation's first checkpoint assertion failed because a repeated
identical mutation generated no WAL frames; fresh fixtures corrected that setup
before the recorded evidence. The experimental guard is no longer present.
PostgreSQL/MySQL/MariaDB, fingerprints, scans, cross-Study snapshots and writer
business-transaction redesign remain outside this correction.
