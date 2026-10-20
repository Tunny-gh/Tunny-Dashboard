# PostgreSQL/MySQL Study acquisition snapshot prerequisite

Date: 2026-10-19 (environment date). Baseline: `main` at `30c45cf`.
Branch: `fix/server-study-snapshot-prerequisite`.
Related: [#248](https://github.com/Tunny-gh/Tunny-Dashboard/issues/248),
[#251](https://github.com/Tunny-gh/Tunny-Dashboard/issues/251) and the separate
SQLite correction in [#260](https://github.com/Tunny-gh/Tunny-Dashboard/issues/260).

This is the separately approved coherent server acquisition prerequisite, not D3
extraction. #251's scope and Done requirements are unchanged. Implementation and
local verification are ready for fresh review; no commit, push, Issue change or PR
is claimed. Security audit remains unavailable as described below.

## Implemented boundary

- Both public URL acquisition APIs now use one path:
  `rdb::parse_single_study_rows_url` opens a concrete backend, runs the existing
  decoder inside its private read-only REPEATABLE READ session, and returns owned
  rows only after commit and connection drop. `rdb::parse_single_study_url` calls
  that API once and projects the owned rows after acquisition has ended.
- PostgreSQL uses `postgres::Client::build_transaction`; MySQL uses ordinary
  `mysql::Conn::start_transaction` with read-only/repeatable-read options. Private
  borrowed facades reuse the existing streaming query, bind and conversion logic.
  The object-safe `OptunaBackend` trait gains no transaction method.
- MySQL's InnoDB snapshot begins at the first ordinary data read. The explicit
  `WITH CONSISTENT SNAPSHOT` driver option is not used: locked mysql 28.0.3 unwraps
  a query error in that startup path. No dependency upgrade or panic catcher was
  introduced. Begin/query/callback/conversion/finish failures return errors, with
  driver RAII rollback/drop and no successful partial result or retry.
- Generic backend parsers remain caller-session query decoders. Scans,
  fingerprints, TLS handling, data/order/error fallbacks and the existing
  COMPLETE-only payload coverage are unchanged. There is no new all-state
  expansion, engine probe, rejection policy, warning or special-engine support.

Policy and trade-offs are in
[ADR-0021](../adr/0021-study-analysis-and-io-boundary-contracts.md#database-acquisition-sessions).

## Actual test environment

Windows x64; Rust 1.99.1 (`58cf8f9c3`), Cargo 1.99.1 (`5f94df478`), Python 3.11.0.
Both native LightGBM DLL and import library were present. Docker Desktop 4.72.0
was already reachable at implementation start, using the local `desktop-linux`
engine 29.4.2 (linux/amd64); no Desktop/service startup was necessary.

Only these task-created containers and newly created named volumes were used:

| Resource | Configuration observed |
| --- | --- |
| `tunny-snapshot-pg-20261019` | Official `postgres:18`, PostgreSQL **18.6**, Debian `18.6-1.pgdg13+2`, loopback host port 63043; same-named new volume mounted at `/var/lib/postgresql`. |
| `tunny-snapshot-mysql-20261019` | Official `mysql:8.4`, MySQL **8.4.11**, loopback host port 63044; same-named new volume mounted at `/var/lib/mysql`. |

Pulled image RepoDigests (not portable server-version guarantees):

```text
postgres@sha256:74935e72241653ca55e0414067e6d8763aceb8a810eb51b452253ec3dcfc4336
mysql@sha256:24989179f58856db763f418010c1169fecc944e3ec1ea7e921572d83eb070f70
```

The database and fixture user are named `snapshot`. Credentials were generated
only for these disposable containers; no URL/password or container environment
dump is retained in this report. MySQL root was used only by the test observer
for connection termination and metadata-lock inspection, not product acquisition.
No existing database, volume or bind mount was used.

PostgreSQL's default isolation was READ COMMITTED. MySQL reported default engine
InnoDB, isolation REPEATABLE-READ, completion type NO_CHAIN and
`innodb_lock_wait_timeout=50`. The Python SQLAlchemy connection reported
`autocommit=0`; this is a DBAPI connection observation, not a claim that the
server's global autocommit default is zero. Product connection/timeouts are not
overridden. Fixture-only Python connections use a five-second connect timeout;
PostgreSQL writers use 15-second statement/five-second lock timeouts and MySQL
writers use 15-second read/write timeouts. The Rust writer handoff has a 30-second
receive deadline. These bounds are test safeguards, not product latency promises.

### Genuine Optuna schema and inputs

`rust_core/tests/create_server_snapshot_fixture.py` creates tables through
Optuna **5.0.0** `RDBStorage` with normal table creation and compatibility checking,
not a hand-authored reduced SQL schema. It records Alembic head `v3.2.0.a`.
It seeds 32 explicitly named disposable Studies, each with a COMPLETE and a
RUNNING trial, numeric/category parameters, JSON attributes (including null and
empty array), existing-format constraints and intermediate values. Available
fields are retained under the existing parser's coverage; this is not full
Optuna 5 feature parity or a constraint/timestamp semantics migration.

Every generated MySQL table, including Optuna's version/heartbeat tables, was
observed as InnoDB. Optuna does not explicitly force that engine in its models;
this is ordinary MySQL 8.4 default creation, not proof for altered defaults or
mixed engines.

The isolated fixture venv was
`C:\Users\dev\AppData\Local\Temp\opencode\tunny-server-snapshot-20261019`.
Its complete installed dependency versions were:

```text
alembic==1.20.0           cffi==2.1.1              colorama==0.4.6
colorlog==6.12.0         cryptography==50.0.2      greenlet==3.5.6
Mako==1.4.3              MarkupSafe==3.0.4         numpy==2.4.6
optuna==5.0.0            packaging==26.3           psycopg2-binary==2.9.11
pycparser==3.11         PyMySQL==1.1.2            PyYAML==6.0.3
SQLAlchemy==2.0.55       tqdm==4.70.1             typing_extensions==4.16.0
```

Product dependencies and Cargo.lock did not change: postgres **0.19.14**,
tokio-postgres **0.7.18**, mysql **28.0.3**. Python is a fixture prerequisite only,
not a new application runtime dependency.

## Deterministic schedule and oracle

`rust_core/src/io/rdb/server_snapshot_tests.rs` has three ignored, explicitly
opt-in live tests. A test-only thread-local hook runs after the real query
iterator/cursor drops, on ordinary and scoped query facades. It is absent from
production builds and cleared on unwind. An isolated writer thread waits for a
one-slot-channel handoff, invokes the actual Optuna Python API, and acknowledges
commit before the reader resumes. There are no schedule-establishing sleeps.
The live tests serialize their shared fixture access with a test-only mutex.

Two writer operations use the public producer API, not post-terminal SQL edits:

- **Transition:** `study.tell(1, 30.0)` atomically completes the RUNNING trial with
  objective/time changes. Pause after all-state trial extras, before COMPLETE IDs.
- **Same-count metadata:** `study.set_metric_names(["new-objective"])`. Pause
  after Study-name reading, before directions/metric names. Total and COMPLETE
  trial counts remain unchanged.

The rows oracle compares every exposed acquisition field: Study metadata,
directions/names/counts/bounds, column names, ordered rows and IDs/numbers,
parameters/category labels/distribution metadata, objectives, numeric/text/original
JSON attributes, constraints/width, all-state timestamps/states/intermediates.
The DataFrame oracle compares metadata/extras, ordered physical columns/values
and original JSON attributes. Values are finite in this snapshot fixture; existing
nonfinite/invalid-constraint parser semantics remain covered by workspace tests.

| Server / change | Independent decoder | Corrected rows URL | Corrected DataFrame URL |
| --- | --- | --- | --- |
| PostgreSQL / transition | Mixed old extras and new COMPLETE rows | Whole old view | Whole old view |
| MySQL / transition | Mixed old extras and new COMPLETE rows | Whole old view | Whole old view |
| PostgreSQL / metadata | Whole new view | Whole old view | Whole old view |
| MySQL / metadata | Whole new view | Whole old view | Whole old view |

All twelve interleavings let the writer commit while the acquisition was retained.
The metadata control is not itself a reproduced mixed read: it tests retention of
the earlier snapshot across a same-count metadata commit. Both transition
independent-query controls demonstrably differ from the complete old and new
oracles. After release, every fresh acquisition matches the whole new view.

### Timing, failures and resources

- Unchanged database reads match. A writer commit after transaction begin but
  before first data read appears in the whole new view for both backends.
  A commit after the first data read is excluded until the next acquisition.
- A metadata commit followed by a separate completion commit demonstrates a valid
  intermediate view (new metric names, old completion state). It stays stable in
  one session, while a later session sees both commits. There is no producer-wide
  multi-transaction atomicity promise.
- Direct scoped-session cases cover query failure, callback error after a row,
  rejected write through the read-only transaction, explicit conversion error,
  and success. A no-op write outside each ended session and a subsequent read
  succeed, proving the healthy connection is not left read-only or aborted.
  Callback/conversion errors retain their supplied diagnostic text. Conversion
  is a controlled callback conversion error, not a malformed Optuna database.
- Terminating the owned connection before begin, during acquisition and immediately
  before finish returns errors without panic or successful data. Begin failure
  never enters the callback. Fresh connections can acquire afterward; terminated
  connections are not reused. This tests real connection loss, not a mocked result.
- Both public URL routes additionally fail after the Study-name query and after
  the final constraints query. Final-query termination must return the finish
  error rather than acquired data. Missing Study errors remain errors. Diagnostics
  do not contain the test password/full URL.
- PostgreSQL's observer sees one retained `backend_xmin` during a direct acquired
  read and zero afterward, on success and callback error. This observes an
  acquisition-limited MVCC horizon, not vacuum/bloat throughput.
- MySQL's observer sees one granted application-table metadata lock after a data
  read and zero after session end, on success and callback error. Old data remains
  readable across the writer commit, demonstrating the retained InnoDB read view.
  Undo/purge byte growth or cleanup speed was not measured.

These observations accept acquisition-limited retention of old versions and
metadata locks, not an analysis-long transaction. Ordinary fixture writers made
progress; concurrent DDL, schema migration, starvation and no-wait guarantees
were not tested or added. Large payload/RSS/latency and rollback/network-fault
recovery beyond deliberate connection termination remain unmeasured.

## Commands and results

Commands ran from the repository root in PowerShell. Parent directories were
verified before venv creation. Provisioning used the following commands, with
fresh synthetic credential variables populated without printing their values:

```powershell
docker pull postgres:18
docker pull mysql:8.4
docker run -d --rm --name tunny-snapshot-pg-20261019 -p 127.0.0.1::5432 --mount type=volume,src=tunny-snapshot-pg-20261019,dst=/var/lib/postgresql -e POSTGRES_PASSWORD -e POSTGRES_USER -e POSTGRES_DB postgres:18
docker run -d --rm --name tunny-snapshot-mysql-20261019 -p 127.0.0.1::3306 --mount type=volume,src=tunny-snapshot-mysql-20261019,dst=/var/lib/mysql -e MYSQL_ROOT_PASSWORD -e MYSQL_PASSWORD -e MYSQL_USER -e MYSQL_DATABASE mysql:8.4
docker port tunny-snapshot-pg-20261019
docker port tunny-snapshot-mysql-20261019
python -m venv "C:\Users\dev\AppData\Local\Temp\opencode\tunny-server-snapshot-20261019"
```

The venv pip install command pinned `optuna==5.0.0`, `SQLAlchemy==2.0.55`,
`psycopg2-binary==2.9.11`, `PyMySQL==1.1.2`, and installed `cryptography`.
The complete resolved versions above are from `python -m pip freeze`.

A temporary stdlib-only runner at
`C:\Users\dev\AppData\Local\Temp\opencode\run_tunny_server_snapshot.py` obtained
only these named task containers' credentials/loopback ports without displaying
them, and supplied these child environment variables:

- `TUNNY_SNAPSHOT_POSTGRES_URL`, `TUNNY_SNAPSHOT_MYSQL_URL`: fixture-owner URLs to
  the disposable loopback `snapshot` database, without query options.
- `TUNNY_SNAPSHOT_MYSQL_OBSERVER_URL`: same disposable MySQL endpoint, root observer.
- `TUNNY_SNAPSHOT_PYTHON`: the fixture-only venv Python executable.

Its `all` mode reseeds both families then runs the exact opt-in test command:

```powershell
python "C:\Users\dev\AppData\Local\Temp\opencode\run_tunny_server_snapshot.py" all
# Equivalent child commands once the explicit environment is supplied:
& $env:TUNNY_SNAPSHOT_PYTHON rust_core/tests/create_server_snapshot_fixture.py postgres seed
& $env:TUNNY_SNAPSHOT_PYTHON rust_core/tests/create_server_snapshot_fixture.py mysql seed
cargo test -p tunny-core --lib --locked --profile ci-test server_snapshot -- --ignored --nocapture --test-threads=1
cargo test -p tunny-core --lib --locked --profile ci-test sqlite_snapshot -- --nocapture --test-threads=1
cargo fmt --manifest-path rust_core/Cargo.toml --all -- --check
cargo fmt --manifest-path egui-app/Cargo.toml --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --profile ci-test
cargo audit --ignore RUSTSEC-2026-0194 --ignore RUSTSEC-2026-0195
git diff --check
```

Run seeding before each full opt-in rerun: those tests intentionally commit
fixture changes. No server test falls back to another DB if variables are missing.

| Check | Result |
| --- | --- |
| Live server regressions | Three tests passed, covering both backends and both public routes; final verification repeated after the final fixture timeout edit (14.84 s). |
| SQLite snapshot regressions | Three passed; final focused rerun 1.00 s, deterministic DELETE/WAL traces retained. |
| Both exact formatting checks | Passed. |
| Exact all-target workspace Clippy | Passed. Initial run found a large test enum; boxing its PostgreSQL variant fixed it without suppressing the lint. |
| Exact locked workspace ci-test | Passed: Study 2+8, core 1022, core integrations 5+9, desktop 862 each for lib/bin, MCP 16+2. Doc tests passed. |
| Security audit | Attempted; unavailable (`error: no such command: audit`). Not a passing audit. No cargo-audit installation was performed. |
| Whitespace | Tracked and new-file diff checks passed; Git's LF/CRLF notices are not whitespace errors. |

The workspace command normally ignores the three new live tests (executed
separately above), three pre-existing core tests, one cost probe, two desktop
tests and three preseeded PostgreSQL integration tests. Native Artifact Animation
capture is opt-in and was not run: its resource path was not changed. These skips
are not passing validation claims. No local macOS build/test, Linux Clippy, GUI
interaction or live MCP protocol-to-server run was performed; existing entry
callers reach the corrected public API without changing their composition.

## Resource cleanup

After verification, the exact task-owned resources were removed:

```powershell
docker stop tunny-snapshot-pg-20261019 tunny-snapshot-mysql-20261019
docker volume rm tunny-snapshot-pg-20261019 tunny-snapshot-mysql-20261019
docker ps -a --filter name=tunny-snapshot-pg-20261019 --filter name=tunny-snapshot-mysql-20261019 --format '{{.Names}}'
docker volume ls --filter name=tunny-snapshot-pg-20261019 --filter name=tunny-snapshot-mysql-20261019 --format '{{.Name}}'
```

Both containers used `--rm`; the final two filtered listings were empty. Volume
creation timestamps were `2026-10-19T01:03:30Z`, matching this run's container
creation, not pre-existing data. No other container/volume was stopped or removed.
The dependency-only venv and credential-free temporary runner remain in the
approved temp directory for review/reproduction. Pulled official images remain
cached. Docker Desktop was already running and was left running. Disposable
database contents and their container credential environments were removed with
the task resources; no credential files were created in the workspace.

## Sources and limits

- [Optuna 5.0.0 storage implementation](https://github.com/optuna/optuna/blob/v5.0.0/optuna/storages/_rdb/storage.py),
  [models](https://github.com/optuna/optuna/blob/v5.0.0/optuna/storages/_rdb/models.py)
  and [server CI](https://github.com/optuna/optuna/blob/v5.0.0/.github/workflows/tests-storage.yml).
- [PostgreSQL repeatable read](https://www.postgresql.org/docs/18/transaction-iso.html),
  [MySQL 8.4 engine defaults](https://docs.oracle.com/cd/E17952_01/mysql-8.4-en/storage-engine-setting.html)
  and [first-read isolation](https://docs.oracle.com/cd/E17952_01/mysql-8.4-en/innodb-transaction-isolation-levels.html).
- [mysql 28.0.3 startup source](https://github.com/blackbeam/rust-mysql-simple/blob/v28.0.3/src/conn/mod.rs).

Evidence is bounded to the exact tested versions/configurations, ordinary
Optuna-created tables and current decoder coverage. Altered/mixed engines,
older server compatibility, new Optuna features, concurrent schema changes and
cross-Study simultaneous snapshots are not validated. The generic query APIs do
not gain an automatic snapshot guarantee. D3 extraction remains a separate task.
