# D3 I/O extraction: implementation and verification

Date: 2026-10-20. Branch: `refactor/251-extract-io`, based on `dd686af`.
Scope: [#251](https://github.com/Tunny-gh/Tunny-Dashboard/issues/251) under
[#248](https://github.com/Tunny-gh/Tunny-Dashboard/issues/248).
The separate server prerequisite [PR #268](https://github.com/Tunny-gh/Tunny-Dashboard/pull/268)
is retained unchanged in history. This records local implementation evidence, not
independent review, Issue closure, commit, push, PR creation or merge.

## Boundary and changed-file map

| Area | Working boundary |
| --- | --- |
| `crates/io/{Cargo.toml,src/lib.rs,src/io/}` | New `tdashboard-io`: Journal scanning/bulk decoding/owned replay, CSV, shared RDB queries and backend sessions, SQLite wrappers, fingerprint queries, Journal/CSV writers, atomic byte output and safe external-file resolution. Only production workspace dependency is `tdashboard-study`. |
| `crates/study/src/model.rs` | One authoritative attribute map accepts original JSON or numeric observations; structural containers retain nested nonfinite values. No parallel numeric attribute authority. Management/revision/retention rules remain unchanged. |
| `rust_core/src/io/` | Temporary delegates and analysis projection composition. No parser installs DataFrames/extras into globals. Statistical/report/export-selection code stays in legacy core. |
| `egui-app/src/{io/study_worker.rs,state/journal_studies.rs,app.rs}` | Scan dispatch has no global worker, loaded-ID cache or installation barrier. Every format uses the caller's source-scoped manager collection, acknowledged owned batches and ticket-checked completion/failure. Historical Journal message/type names are not format restrictions. |
| `egui-app/src/state/message_handler/study.rs` | Derives the current table/history/front from acquired manager inputs. Remaining global publications are read projections for unmigrated widgets, never ingestion targets. CSV import directions/ranges reach the authoritative batch before application. |
| `egui-app/src/{io/artifacts.rs,io/flat_csv.rs,state/app_state.rs,app.rs}` | Entry-owned storage map/session root and scan provenance; association extraction/resolution lives in I/O, URI/image/texture resources stay in GUI. |
| `mcp-server/src/storage.rs` | Request-local manager composes I/O acquisition and legacy projection. Summary/report/trial tools retain their existing protocol and supported source set; no CSV or Artifact tool expansion. |
| `egui-app/src/io/{file,export,report_export}.rs`, legacy runner facades | Existing exports/recording continue through neutral I/O encoding/atomic output and Journal writer APIs. Row selection, rendering and execution loops do not move into I/O. |

`Cargo.lock` changes only workspace package/dependency edges: external package
versions are unchanged. SQL driver production dependencies moved out of legacy
core; rusqlite remains a test dependency there for fixtures. CI manifest paths,
flags, package/executable names and native-library provider are unchanged.

### Acquisition and publication

- Journal primary loads retain incremental publication and fallible synchronous
  backpressure; comparisons/full reads retain their established full-read order.
  Existing terminal user/constraint corrections remain coalesced at batch/EOF
  boundaries. A sink error stops replay without rolling back accepted batches.
- RDB decoding retains available parameters, indexed objectives, attributes,
  constraints, times and intermediate values across all states. Visible columns
  remain COMPLETE-only; non-COMPLETE fields do not silently become features.
- SQL acquisition ends its scoped read session/connection before delivery to the
  caller or DataFrame projection. SQLite deferred transactions and server read-only
  REPEATABLE READ are retained; MySQL establishes its view at first data read.
- Scan results, batches, completion and errors are source/ticket checked. Reload
  keeps the old stamped display before first acceptance, then the new accepted
  prefix. Cached selection acquires from the manager, not a derived global mirror.
- CSV retains existing inference, sorted parameter layout, objective order,
  import options and disabled manual Reload policy. Image references remain
  associations even when files are missing; resolution rebases them at the current
  session root. Association grouping is linear in rows/references.

## Nonfinite correction and actual producer evidence

[ADR-0021](../adr/0021-study-analysis-and-io-boundary-contracts.md#logical-all-state-model)
records the approved narrow correction. `AttributeValue::Json` retains ordinary
JSON including explicit null; `Number` retains numeric CSV/Python observations,
and structural array/object variants retain nested nonfinite numbers. Missing
keys remain absent. NaN representation equality does not cause endless revisions;
retained inputs keep earlier values after updates.

The existing quote-aware regex dependency recognizes only Python's bare `NaN`,
`Infinity` and `-Infinity`. Acquisition-only marker keys are chosen absent from
decoded object keys, including escaped spellings; they never enter domain records.
Quoted strings are not numeric observations. Other invalid JSON remains invalid.
No product Python dependency, generic post-load framework or new storage encoding
is introduced. Standard JSON presentation still cannot encode nonfinite numbers;
analysis-specific finite-row filters and algorithm behavior are not broadened.

`crates/io/tests/create_nonfinite_fixture.py` was run with Optuna **5.0.0** using
`JournalFileBackend` with `JournalFileOpenLock` in a disposable temporary directory.
It confirmed COMPLETE `+inf`/`-inf`, infinity/NaN/null/string/nested user attributes,
and real Journal replay of infinity. Its exact output is retained at
`crates/io/fixtures/optuna5-nonfinite.jsonl`; Rust tests verify objective/attribute/
intermediate observations and null/missing distinctions through the owned handoff.
This is actual Journal evidence, not an assertion that every backend accepts
nonfinite user attributes. Python's nonstandard numeric literals are distinct from
strict JSON and from the documentation's JSON-serializable wording.

## Evidence against every #251 Done item

| Done item | Direct evidence |
| --- | --- |
| GUI/MCP use I/O handoff, no parser-global installs | `state/journal_studies.rs`, `io/study_worker.rs`, MCP request-local storage and production source searches. `d3_csv_and_sqlite_use_owned_managers_for_selection_and_reload` and live server GUI test poison derived globals, then successfully reselect from managers; reload advances generation. Old row/chunk/install delivery helpers survive only as test fixtures. |
| Bulk/incremental/reload equivalence | `rust_core/tests/d3_io.rs`: semantic Journal equality by TrialKey at batch sizes 1/2/1000, explicit reload, all five states, global IDs unlike local numbers, full versus streaming completion order, typed categories, absent numeric parameters, invalid constraint slots, empty/null attrs and metadata-only all-state schema growth. Existing `study_slice` and #258 suites cover retained prefixes, shrinking evidence, EOF correction, column discovery and fallible delivery. CSV compares the legacy projection with the owned path, including nonfinite attributes. SQLite tests retain all-state fields without adding visible columns; reload changes width with unchanged COMPLETE count. |
| Writer roundtrip/errors/masking | Relocated Journal writer/CSV encoding tests; existing generic/Grasshopper runner readback and injected I/O failures still exercise the extracted writer through the D6 facade. Existing URL/masking/TLS-precondition tests remain. Live MCP report output contains a masked source and no synthetic password; pagination remains COMPLETE-only. |
| Artifact lifetime/resolution matrix | Relocated I/O Artifact tests cover global CREATE_TRIAL ordinals over Studies/all states, late metadata, extensionless files/MIME, missing files and containment. CSV resolution tests rebase associations and reject missing/escaping paths. Ticket tests reject another source/root/generation/scan. Existing real GUI actions cover Study switching, Open/New/replacement, CLI/manual root precedence, adjacent discovery and reload/rescan/manual-choice races. Native full-app Animation verification passed separately. |
| No I/O analysis dependency, ownership of deferred work | Cargo metadata/tree confirms only `tdashboard-study` as an I/O production workspace edge. No I/O import of `tunny-core`, GUI or analysis. The removal inventory below assigns legacy projections, statistics/export selection, widget mirrors and writer/entry facades to D4-D7. |
| Live consumers remain usable, checks recorded | GUI Journal/CSV/SQLite tests drive production scan/request/message handling. Live PostgreSQL/MySQL GUI tests drive selection, manager-backed cached activation and Reload. Live MCP tests launch the real stdio binary for summary/report/trials/pagination. Both existing recording loops and export tests pass in the workspace run; native capture exercises atomic GIF output. |

Late Journal metric-name updates rename objective labels by their existing index
before rebuilding the retained empty schema; they do not append a second set of
objective columns or reorder discovered parameters. A dedicated metadata-only test
covers this adapter requirement. Indexed objective extension also keeps repeated
labels separate from each other and from same-named parameters; a focused handoff
test verifies their distinct values. These are projection field-routing changes,
not changes to numerical algorithms.

### Temporary adapters and removal owners

| Remaining adapter | Removal owner |
| --- | --- |
| Legacy `study_projection`, DataFrame/extras projections and raw decoder row-view consumers | D4/#252. I/O row views contain owned decoder records only, not an analysis table/cache; they are per-acquisition compatibility material, not a second writable loaded-Study authority. |
| Derived GUI globals, active-selection publications and old presentation-store test fixtures | D5/#253. Production ingestion uses managers; no legacy loader writes these mirrors. |
| `rust_core::io::journal::writer` re-export used by generic/GH execution | D6/#254. Actual writer is in I/O; loops/algorithms/progress are unchanged. Test-support feature keeps existing file-failure injection available without a production testing API. |
| Legacy storage/format facade imports, remaining entry/report/session/export composition | D7/#255, with statistical/derived export computation assigned to D4. Neutral encoding and external writes already use I/O; report rendering and row selection never become reverse I/O dependencies. |

## Commands and results

Run from the repository root in PowerShell, with the existing native LightGBM
DLL/import library. Windows Rust/Cargo **1.99.1**; fixture Python **3.11.0**.
Exact parent checks:

```text
cargo fmt --manifest-path rust_core/Cargo.toml --all -- --check
cargo fmt --manifest-path egui-app/Cargo.toml --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --profile ci-test
cargo audit --ignore RUSTSEC-2026-0194 --ignore RUSTSEC-2026-0195
```

Both exact formatting checks, all-target Clippy and the full workspace test
command passed after the final focused schema/decoder refinements. The audit
command was attempted and failed with `no such command: audit`:
no advisory scan or clean audit is claimed. Initial development regressions from
removed implicit test installations and old nonfinite-normalization expectations
were corrected by explicit fixture ownership and source-preserving assertions,
not by suppressing errors or restoring parser-global writes.

Final workspace result: **2801 passed**, 15 ignored, no failures. Breakdown:
I/O 96; Study 2+9; core 927 plus integrations 1+8+5+9; desktop 863 each for
library/binary; MCP 16+2. Doc tests passed (zero cases). Ignored cases include the
three I/O server tests, three pre-existing core probes, the #258 cost probe, two
desktop cases per suite (one live server case), three preseeded-server checks and
one MCP live case. The five distinct live Rust cases were run separately below;
the duplicate desktop binary live case remains ignored. Skips are not passes.

Additional direct commands:

```powershell
cargo test -p tunny-core --locked --profile ci-test --test d3_io -- --nocapture
cargo test -p tunny-core --locked --profile ci-test --test d3_csv_attribute_boundary
cargo test -p tdashboard-io --lib --locked --profile ci-test sqlite_snapshot -- --nocapture --test-threads=1
python "C:\Users\dev\AppData\Local\Temp\opencode\run_tunny_d3_server.py" seed
python "C:\Users\dev\AppData\Local\Temp\opencode\run_tunny_d3_server.py" test
$env:TUNNY_NATIVE_CAPTURE_CHECK = "1"
cargo test -p tunny-desktop --locked --profile ci-test --test artifact_animation_native
Remove-Item Env:\TUNNY_NATIVE_CAPTURE_CHECK
cargo tree -p tdashboard-io --locked --edges normal --depth 1
cargo metadata --locked --format-version 1 --no-deps
git diff --check
```

The temporary credential-free runner supplies explicit child environment variables
from only the two task-owned containers, without printing URLs/passwords. It runs:

```text
cargo test -p tdashboard-io --lib --locked --profile ci-test server_snapshot -- --ignored --nocapture --test-threads=1
cargo test -p tunny-desktop --lib --locked --profile ci-test d3_live_servers -- --ignored --nocapture --test-threads=1
cargo test -p tunny-mcp --locked --profile ci-test --test integration d3_live_servers -- --ignored --nocapture --test-threads=1
```

All three server regression tests passed on final rerun (14.89 s), as did the GUI
test (0.14 s) and real MCP stdio test (0.29 s), each covering both server families. They retain
deterministic writer barriers, same-count metric-name changes, RUNNING-to-COMPLETE,
snapshot timing, ordinary writer progress, failure/connection cleanup and error
propagation. Relocated I/O snapshot tests inspect owned decoded row views; actual
DataFrame/manager composition is additionally verified by the GUI/entry tests above.
They are not silently counted as passing in the normal ignored-test run.
The final focused SQLite DELETE/WAL snapshot rerun passed all three tests (1.00 s).

Native wgpu preview/GIF verification passed: three selected frames, matching
live/GIF order, 684x544 content-only capture, 330 ms delay, full prefix observations,
two independent instances, seek/pause restoration and PNG/JPEG aspect ratios.
The normal workspace command skips native capture; this separate opt-in result is
the native evidence, not the skip message.

### Disposable server environment and bounds

Only `tunny-d3-pg-20261019` and `tunny-d3-mysql-20261019`, with same-named new volumes,
were used. No existing database, volume or bind mount was touched. Loopback ports
were 55450/55451 initially and 50520/50521 after reprovisioning for the final
number-decoding rerun. Docker Desktop 4.72.0 / engine 29.4.2 was already running.

Provisioning used fresh synthetic credential environment variables (never printed)
and the following commands; the names were verified absent before creation:

```powershell
docker run -d --rm --name tunny-d3-pg-20261019 -p 127.0.0.1::5432 --mount type=volume,src=tunny-d3-pg-20261019,dst=/var/lib/postgresql -e POSTGRES_PASSWORD -e POSTGRES_USER -e POSTGRES_DB postgres@sha256:74935e72241653ca55e0414067e6d8763aceb8a810eb51b452253ec3dcfc4336
docker run -d --rm --name tunny-d3-mysql-20261019 -p 127.0.0.1::3306 --mount type=volume,src=tunny-d3-mysql-20261019,dst=/var/lib/mysql -e MYSQL_ROOT_PASSWORD -e MYSQL_PASSWORD -e MYSQL_USER -e MYSQL_DATABASE mysql@sha256:24989179f58856db763f418010c1169fecc944e3ec1ea7e921572d83eb070f70
```

For direct reproduction, supply `TUNNY_SNAPSHOT_POSTGRES_URL` and
`TUNNY_SNAPSHOT_MYSQL_URL` pointing only at the disposable loopback `snapshot`
database, `TUNNY_SNAPSHOT_MYSQL_OBSERVER_URL` for its task-only root observer, and
`TUNNY_SNAPSHOT_PYTHON` for the fixture venv executable. The runner derives these
without displaying credentials. Re-seed before each complete live regression run;
those tests intentionally commit fixture changes. Root is used only for observing/
terminating task connections, not application acquisition.

| Official image digest | Observed server |
| --- | --- |
| `postgres@sha256:74935e72241653ca55e0414067e6d8763aceb8a810eb51b452253ec3dcfc4336` | PostgreSQL 18.6, Debian `18.6-1.pgdg13+2`, default READ COMMITTED |
| `mysql@sha256:24989179f58856db763f418010c1169fecc944e3ec1ea7e921572d83eb070f70` | MySQL 8.4.11, InnoDB, default REPEATABLE-READ / NO_CHAIN |

`crates/io/tests/create_server_snapshot_fixture.py` creates genuine Optuna 5.0.0
schemas (Alembic `v3.2.0.a`) and synthetic data. All MySQL tables were observed as
InnoDB. Fixture dependencies: SQLAlchemy 2.0.55, alembic 1.20.0, psycopg2-binary
2.9.11 and PyMySQL 1.1.2, in the previously approved dependency-only temp venv.
This does not impose a product minimum server version or prove other engines.

PostgreSQL retained a snapshot horizon and MySQL retained table metadata locks
only while acquisition was held; observer counts returned to zero after success
and error. Writers committed while the controlled reader was retained. No
large-database latency/RSS, vacuum/undo growth, starvation or no-wait guarantee is
measured. Production timeouts/retries are unchanged; fixture writers use bounded
connect/read/write/statement timeouts and handshakes use 30-second deadlines.
Special/mixed engines and other server versions remain unverified.

### Resource cleanup

After the final live runs, the exact task-owned resources were removed:

```powershell
docker stop tunny-d3-pg-20261019 tunny-d3-mysql-20261019
docker volume rm tunny-d3-pg-20261019 tunny-d3-mysql-20261019
docker ps -a --filter name=tunny-d3-pg-20261019 --filter name=tunny-d3-mysql-20261019 --format '{{.Names}}'
docker volume ls --filter name=tunny-d3-pg-20261019 --filter name=tunny-d3-mysql-20261019 --format '{{.Name}}'
```

Both containers had `--rm`; the final filtered container/volume listings were empty.
Initial volume creation was `2026-10-19T13:54:08Z`; those resources were removed,
then recreated at `2026-10-19T22:34:12Z` for the final decoder/entry verification
and removed again. Both creations were task-owned and started with absent names.
No unrelated resource was stopped/removed. The native GIF's temporary output was
also confirmed absent after the harness exited. The prior dependency-only venv
and credential-free runner remain in the approved temp directory for reproduction;
cached official images and the already-running Docker Desktop were left alone.
No credential files were created in the workspace. No commit/push/PR/Issue change
was made, and HEAD remains `dd686af`.

The final decoder checks additionally verify that inline/event Journal numeric
parameters and numeric SQL driver values do not lose infinity in an intermediate
strict-JSON conversion. Finite inline categorical JSON retains its original
integer/string/null type rather than being coerced through an f64. This is parser
observation preservation, not a claim that an Optuna sampler permits nonfinite
parameters or a change to fitting/selection policy.

## Limits and concrete out-of-scope findings

- `rust_core/src/report/builder.rs` computes unconstrained/fallback nondominance
  before excluding nonfinite objective rows. A `-Infinity` row can dominate a
  finite row and then be filtered, leaving an empty finite front. This pre-existing
  raw-infinity analysis behavior is not fixed or hidden by normalizing source data;
  D3 preserves the observation and the existing analysis policy. Follow-up belongs
  with analysis, not this extraction.
- The existing naive datetime decoder rejects timezone suffixes, and existing
  inline completion-time interpretation is unchanged. The real Optuna 5 fixture
  is nonfinite-value evidence, not complete Optuna 5 timestamp/feature parity.
- JSON-only legacy presentation cannot faithfully serialize nonfinite numbers.
  Authoritative values remain typed; this work does not invent a new report/MCP
  wire encoding or change every widget/algorithm to support infinite inputs.
- No local Linux Clippy or macOS build/link/startup was performed. Existing
  preseeded-server tests, optional performance probes and user-data/native checks
  not explicitly executed remain ignored/unverified, not passing evidence.

Fresh review is required before any commit or stacked D3 PR. Task status remains
in Issues; this report does not merge or close #268/#251.
