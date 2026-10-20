//! SQLite acquisition regressions and controlled independent-query comparison for #260.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::time::Duration;

use rusqlite::ErrorCode;
use serde_json::{json, Value};

use super::*;
use crate::io::rdb::{parse_single_study_rows, RdbStudyRows};

const DEADLINE: Duration = Duration::from_secs(5);

struct Fixture {
    directory: tempfile::TempDir,
}

impl Fixture {
    fn new(mode: &str) -> Self {
        let fixture = Self {
            directory: tempfile::tempdir().unwrap(),
        };
        let conn = fixture.writer();
        let actual: String = conn
            .query_row(&format!("PRAGMA journal_mode={mode}"), [], |row| row.get(0))
            .unwrap();
        assert_eq!(actual.to_uppercase(), mode);
        conn.execute_batch(
            r#"CREATE TABLE studies(study_id INTEGER PRIMARY KEY, study_name TEXT);
             CREATE TABLE study_directions(study_id INTEGER, objective INTEGER, direction TEXT);
             CREATE TABLE study_system_attributes(study_id INTEGER, key TEXT, value_json TEXT);
             CREATE TABLE trials(trial_id INTEGER PRIMARY KEY, number INTEGER, study_id INTEGER,
                                 state TEXT, datetime_start TEXT, datetime_complete TEXT);
             CREATE TABLE trial_values(trial_id INTEGER, objective INTEGER, value REAL, value_type TEXT);
             CREATE TABLE trial_params(trial_id INTEGER, param_name TEXT, param_value REAL, distribution_json TEXT);
             CREATE TABLE trial_user_attributes(trial_id INTEGER, key TEXT, value_json TEXT);
             CREATE TABLE trial_system_attributes(trial_id INTEGER, key TEXT, value_json TEXT);
             CREATE TABLE trial_intermediate_values(trial_id INTEGER, step INTEGER,
                                 intermediate_value REAL, intermediate_value_type TEXT);
             INSERT INTO studies VALUES(1,'old');
             INSERT INTO study_directions VALUES(1,0,'MINIMIZE');
             INSERT INTO study_system_attributes VALUES(1,'study:metric_names','["old-objective"]');
             INSERT INTO trials VALUES(1,0,1,'COMPLETE','2024-01-01 00:00:00','2024-01-01 00:00:10');
             INSERT INTO trials VALUES(2,1,1,'RUNNING','2024-01-01 00:00:11',NULL);
             INSERT INTO trial_values VALUES(1,0,10,'FINITE');
             INSERT INTO trial_params VALUES(1,'x',1,'{"name":"FloatDistribution","attributes":{"low":0,"high":10,"log":false,"step":null}}');
             INSERT INTO trial_user_attributes VALUES(1,'tag','{"version":0}');
             INSERT INTO trial_system_attributes VALUES(1,'constraints','[-1]');
             INSERT INTO trial_intermediate_values VALUES(1,0,10,'FINITE');"#,
        )
        .unwrap();
        fixture
    }

    fn path(&self) -> std::path::PathBuf {
        self.directory.path().join("study.db")
    }

    fn writer(&self) -> Connection {
        let conn = Connection::open(self.path()).unwrap();
        conn.busy_timeout(Duration::ZERO).unwrap();
        conn
    }

    fn reader(&self) -> SqliteBackend {
        SqliteBackend::open_readonly(&self.path()).unwrap()
    }

    fn view(&self) -> Value {
        view(parse_single_study_rows(&mut self.reader(), 1).unwrap())
    }
}

// Compare every exposed acquisition field, not only row counts or version tags.
fn view(load: RdbStudyRows) -> Value {
    json!({
        "meta": {
            "id": load.meta.study_id, "name": load.meta.name,
            "directions": format!("{:?}", load.meta.directions),
            "total": load.meta.total_trials, "complete": load.meta.completed_trials,
            "params": load.meta.param_names, "objectives": load.meta.objective_names,
            "attrs": load.meta.user_attr_names, "constraints": load.meta.has_constraints,
            "bounds": load.meta.param_bounds,
        },
        "columns": [load.param_names, load.objective_names,
                    load.user_attr_numeric_names, load.user_attr_string_names],
        "width": load.max_constraints,
        "rows": load.rows.into_iter().map(|r| json!({
            "id": r.trial_id, "number": r.trial_number,
            "params": r.param_display, "labels": r.param_category_label,
            "distribution": {
                "categorical": r.distribution_metadata.categorical,
                "discrete": r.distribution_metadata.numeric_discrete,
                "categories": r.distribution_metadata.categories,
            },
            "values": r.objective_values, "numeric": r.user_attrs_numeric,
            "text": r.user_attrs_string, "json": r.user_attrs_json,
            "constraints": r.constraint_values,
        })).collect::<Vec<_>>(),
        "extras": load.extras.trials.into_iter().map(|t| json!({
            "id": t.trial_id, "number": t.trial_number, "state": t.state.label(),
            "start": t.datetime_start, "end": t.datetime_complete,
            "intermediate": t.intermediate_values,
        })).collect::<Vec<_>>(),
    })
}

#[derive(Clone, Copy, Debug)]
enum Change {
    Attributes,
    Transition,
}

fn change(conn: &Connection, change: Change) {
    conn.execute_batch("BEGIN IMMEDIATE").unwrap();
    conn.execute_batch(match change {
        Change::Attributes =>
            r#"UPDATE trial_user_attributes SET value_json='{"version":1}';
             UPDATE trial_system_attributes SET value_json='[2,3]';"#,
        Change::Transition =>
            r#"UPDATE studies SET study_name='new';
             UPDATE study_directions SET direction='MAXIMIZE';
             UPDATE study_system_attributes SET value_json='["new-objective"]';
             UPDATE trials SET state='COMPLETE',datetime_complete='2024-01-01 00:00:30' WHERE trial_id=2;
             UPDATE trial_values SET value=20;
             INSERT INTO trial_values VALUES(2,0,30,'FINITE');
             UPDATE trial_params SET param_value=2;
             UPDATE trial_user_attributes SET value_json='{"version":1}';
             UPDATE trial_system_attributes SET value_json='[2,3]';
             UPDATE trial_intermediate_values SET intermediate_value=20;"#,
    }).unwrap();
}

fn commit(conn: &Connection) -> bool {
    match conn.execute_batch("COMMIT") {
        Ok(()) => true,
        Err(rusqlite::Error::SqliteFailure(error, _)) if error.code == ErrorCode::DatabaseBusy => {
            false
        }
        other => panic!("unexpected commit result: {other:?}"),
    }
}

type QueryAction = (&'static str, Option<Box<dyn FnOnce()>>);

thread_local! {
    static AFTER_QUERY: RefCell<Option<QueryAction>> = const { RefCell::new(None) };
}

struct QueryHook;

impl QueryHook {
    fn install(prefix: &'static str, after: impl FnOnce() + 'static) -> Self {
        AFTER_QUERY.with(|hook| {
            assert!(hook.borrow().is_none());
            *hook.borrow_mut() = Some((prefix, Some(Box::new(after))));
        });
        Self
    }

    fn fired(&self) -> bool {
        AFTER_QUERY.with(|hook| hook.borrow().as_ref().unwrap().1.is_none())
    }
}

impl Drop for QueryHook {
    fn drop(&mut self) {
        AFTER_QUERY.with(|hook| *hook.borrow_mut() = None);
    }
}

pub(super) fn after_query(sql: &str) {
    let after = AFTER_QUERY.with(|hook| {
        let mut hook = hook.borrow_mut();
        hook.as_mut().and_then(|(prefix, after)| {
            if sql.starts_with(*prefix) {
                after.take()
            } else {
                None
            }
        })
    });
    if let Some(after) = after {
        after();
    }
}

#[derive(Clone, Copy, Debug)]
enum Route {
    Independent,
    Rows,
    DataFrame,
}

impl Route {
    fn read(self, fixture: &Fixture) -> Value {
        match self {
            Self::Independent => fixture.view(),
            Self::Rows => {
                view(crate::io::sqlite::parse_single_study_rows(&fixture.path(), 1).unwrap())
            }
            Self::DataFrame => {
                let (meta, frame, extras) =
                    crate::io::sqlite::parse_single_study(&fixture.path(), 1).unwrap();
                json!({
                    "meta": format!("{meta:?}"), "extras": format!("{extras:?}"),
                    "contents": frame,
                })
            }
        }
    }
}

fn receive<T>(rx: &Receiver<T>) -> T {
    rx.recv_timeout(DEADLINE)
        .expect("bounded handshake timed out")
}

fn send<T>(tx: &SyncSender<T>, message: T) {
    tx.try_send(message)
        .unwrap_or_else(|_| panic!("handshake queue unavailable"));
}

#[test]
fn sqlite_snapshot_interleaved_acquisition() {
    println!(
        "SQLite {} source_id={}",
        rusqlite::version(),
        Connection::open_in_memory()
            .unwrap()
            .query_row("SELECT sqlite_source_id()", [], |r| r.get::<_, String>(0))
            .unwrap()
    );
    for mode in ["DELETE", "WAL"] {
        for mutation in [Change::Attributes, Change::Transition] {
            for route in [Route::Independent, Route::Rows, Route::DataFrame] {
                let transactional = !matches!(route, Route::Independent);
                let fixture = Fixture::new(mode);
                let old = route.read(&fixture);
                let expected = Fixture::new(mode);
                let expected_writer = expected.writer();
                change(&expected_writer, mutation);
                assert!(commit(&expected_writer));
                let new = route.read(&expected);
                assert_ne!(old, new);
                let reader = fixture.reader();
                let timeout: i64 = reader
                    .conn
                    .query_row("PRAGMA busy_timeout", [], |r| r.get(0))
                    .unwrap();
                if !transactional && matches!(mutation, Change::Attributes) {
                    let settings = fixture.writer();
                    for pragma in [
                        "synchronous",
                        "page_size",
                        "wal_autocheckpoint",
                        "read_uncommitted",
                    ] {
                        let value: i64 = settings
                            .query_row(&format!("PRAGMA {pragma}"), [], |r| r.get(0))
                            .unwrap();
                        println!("{mode} {pragma}={value}");
                    }
                    let locking: String = settings
                        .query_row("PRAGMA locking_mode", [], |r| r.get(0))
                        .unwrap();
                    println!(
                        "{mode} locking_mode={locking} writer_busy_ms=0 shared_cache=not-enabled"
                    );
                }
                let (commands, incoming) = mpsc::sync_channel(1);
                let (events, outgoing) = mpsc::sync_channel(1);
                let path = fixture.path();
                let writer = std::thread::spawn(move || {
                    let conn = Connection::open(path).unwrap();
                    conn.busy_timeout(Duration::ZERO).unwrap();
                    receive(&incoming);
                    change(&conn, mutation);
                    let committed = commit(&conn);
                    send(&events, committed);
                    receive(&incoming);
                    if !committed {
                        assert!(commit(&conn), "writer must commit after reader release");
                    }
                    assert!(conn.is_autocommit());
                });
                let writer_committed = Rc::new(Cell::new(None));
                let observed = Rc::clone(&writer_committed);
                let request = commands.clone();
                let hook = QueryHook::install(
                    match mutation {
                        Change::Attributes => "SELECT tua.trial_id",
                        Change::Transition => "SELECT tiv.trial_id",
                    },
                    move || {
                        send(&request, ());
                        observed.set(Some(receive(&outgoing)));
                    },
                );
                let actual = route.read(&fixture);
                assert!(hook.fired(), "query boundary must be reached");
                drop(hook);
                let committed = writer_committed.get().unwrap();
                assert_eq!(committed, !transactional || mode == "WAL");
                send(&commands, ());
                writer.join().unwrap();
                if transactional {
                    assert_eq!(actual, old);
                } else {
                    assert_ne!(actual, old);
                    assert_ne!(actual, new);
                    match mutation {
                        Change::Attributes => {
                            assert_eq!(actual["rows"][0]["json"], old["rows"][0]["json"]);
                            assert_eq!(
                                actual["rows"][0]["constraints"],
                                new["rows"][0]["constraints"]
                            );
                            assert_eq!(actual["meta"]["total"], old["meta"]["total"]);
                            assert_eq!(actual["meta"]["complete"], old["meta"]["complete"]);
                        }
                        Change::Transition => {
                            assert_eq!(actual["meta"]["name"], "old");
                            assert_eq!(actual["extras"][1]["state"], "RUNNING");
                            assert_eq!(actual["rows"].as_array().unwrap().len(), 2);
                            assert_eq!(actual["rows"][0]["values"], json!([20.0]));
                        }
                    }
                }
                assert_eq!(route.read(&fixture), new);
                println!("{mode} {mutation:?} route={route:?} reader_busy_ms={timeout} writer_commit_before_release={committed} result={}",
                    if transactional { "whole-old" } else { "mixed" });
                println!("  acquired={actual}");
            }
        }
    }
}

#[test]
fn sqlite_snapshot_establishment_and_multiple_writer_transactions() {
    for mode in ["DELETE", "WAL"] {
        let fixture = Fixture::new(mode);
        let writer = fixture.writer();
        let mut reader = fixture.reader();
        let unchanged = fixture.view();
        assert_eq!(
            view(
                reader
                    .with_study_read(|b| parse_single_study_rows(b, 1))
                    .unwrap()
            ),
            unchanged
        );
        let observed = reader
            .with_study_read(|backend| {
                // BEGIN DEFERRED alone does not pin a snapshot.
                change(&writer, Change::Attributes);
                assert!(commit(&writer));
                parse_single_study_rows(backend, 1)
            })
            .unwrap();
        let intermediate = fixture.view();
        assert_eq!(view(observed), intermediate);
        let observed = reader
            .with_study_read(|backend| {
                let first = view(parse_single_study_rows(backend, 1)?);
                assert_eq!(first, intermediate);
                change(&writer, Change::Transition);
                assert_eq!(commit(&writer), mode == "WAL");
                let second = parse_single_study_rows(backend, 1)?;
                assert_eq!(view(parse_single_study_rows(backend, 1)?), first);
                Ok(second)
            })
            .unwrap();
        assert_eq!(view(observed), intermediate);
        if !writer.is_autocommit() {
            assert!(commit(&writer));
        }
        let final_view = fixture.view();
        assert_ne!(final_view, intermediate);
        assert_eq!(
            view(
                reader
                    .with_study_read(|b| parse_single_study_rows(b, 1))
                    .unwrap()
            ),
            final_view
        );
        println!("{mode} controls: unchanged=whole-old, commit-after-BEGIN-before-read=whole-new, two-writer-transactions=legitimate-intermediate, next-session=whole-new");
    }
}

#[test]
fn sqlite_snapshot_errors_release_reader_and_wal_checkpoint() {
    for mode in ["DELETE", "WAL"] {
        for failure in [
            "query",
            "callback",
            "parser-conversion",
            "read-only-write",
            "early-return",
            "finish",
            "success",
        ] {
            let fixture = Fixture::new(mode);
            let writer = fixture.writer();
            if failure == "parser-conversion" {
                writer
                    .execute_batch("UPDATE trial_params SET param_value='invalid'")
                    .unwrap();
                assert!(
                    crate::io::sqlite::parse_single_study_rows(&fixture.path(), 1)
                        .err()
                        .unwrap()
                        .contains("param_value is not numeric")
                );
                assert!(crate::io::sqlite::parse_single_study(&fixture.path(), 1)
                    .err()
                    .unwrap()
                    .contains("param_value is not numeric"));
            }
            let mut reader = fixture.reader();
            let result: Result<(), String> = reader.with_study_read(|backend| {
                backend.query("SELECT study_name FROM studies", &[])?;
                change(&writer, Change::Attributes);
                assert_eq!(commit(&writer), mode == "WAL");
                if mode == "WAL" {
                    let checkpoint: (i64, i64, i64) = writer
                        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| {
                            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
                        })
                        .unwrap();
                    assert_eq!(checkpoint.0, 1);
                    assert!(checkpoint.1 > checkpoint.2);
                    println!("WAL {failure} pinned checkpoint={checkpoint:?}");
                }
                match failure {
                    "query" => backend
                        .query("SELECT * FROM missing_table", &[])
                        .map(|_| ()),
                    "callback" => {
                        let mut calls = 0;
                        let result =
                            backend.query_for_each("SELECT trial_id FROM trials", &[], &mut |_| {
                                calls += 1;
                                Err("injected callback failure".into())
                            });
                        assert_eq!(calls, 1);
                        result
                    }
                    "parser-conversion" => parse_single_study_rows(backend, 1).map(|_| ()),
                    "read-only-write" => backend
                        .query("UPDATE studies SET study_name='forbidden'", &[])
                        .map(|_| ()),
                    "early-return" => Err("injected acquisition early return".into()),
                    // Force the driver's COMMIT to fail without filesystem fault injection.
                    "finish" => backend.query("ROLLBACK", &[]).map(|_| ()),
                    _ => Ok(()),
                }
            });
            assert_eq!(result.is_err(), failure != "success");
            if let Err(error) = &result {
                let expected = match failure {
                    "query" => "no such table: missing_table",
                    "callback" => "injected callback failure",
                    "parser-conversion" => "param_value is not numeric",
                    "read-only-write" => "readonly database",
                    "finish" => "Failed to finish SQLite study read",
                    _ => "injected acquisition early return",
                };
                assert!(
                    error.contains(expected),
                    "unexpected {failure} error: {error}"
                );
            }
            assert!(reader.conn.is_autocommit());
            if !writer.is_autocommit() {
                assert!(commit(&writer));
            }
            if failure == "parser-conversion" {
                writer
                    .execute_batch("UPDATE trial_params SET param_value=1")
                    .unwrap();
            }
            if mode == "WAL" {
                let checkpoint: (i64, i64, i64) = writer
                    .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| {
                        Ok((r.get(0)?, r.get(1)?, r.get(2)?))
                    })
                    .unwrap();
                assert_eq!(checkpoint, (0, 0, 0));
            }
            assert_eq!(
                view(parse_single_study_rows(&mut reader, 1).unwrap()),
                fixture.view()
            );
            println!("{mode} {failure}: expected result returned, autocommit restored, writer released, reader reusable");
        }
        let fixture = Fixture::new(mode);
        let writer = fixture.writer();
        let mut reader = fixture.reader();
        reader.conn.execute_batch("BEGIN DEFERRED").unwrap();
        assert!(reader
            .with_study_read(|_| -> Result<(), String> {
                panic!("failed BEGIN must not invoke acquisition")
            })
            .is_err());
        assert!(
            !reader.conn.is_autocommit(),
            "failed BEGIN must not end an existing transaction"
        );
        parse_single_study_rows(&mut reader, 1).unwrap();
        change(&writer, Change::Transition);
        assert_eq!(commit(&writer), mode == "WAL");
        drop(reader);
        if !writer.is_autocommit() {
            assert!(commit(&writer));
        }
        if mode == "WAL" {
            let busy: i64 = writer
                .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| r.get(0))
                .unwrap();
            assert_eq!(busy, 0);
        }
        println!("{mode} connection drop: writer released");
    }
}
