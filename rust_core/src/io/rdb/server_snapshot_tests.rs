//! Opt-in live regressions against disposable, genuine Optuna-created schemas.

use std::cell::RefCell;
use std::process::Command;
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::Duration;

use serde_json::{json, Value};

use super::*;

static LIVE_TEST: Mutex<()> = Mutex::new(());

type QueryAction = (&'static str, Option<Box<dyn FnOnce()>>);
thread_local! {
    static AFTER_QUERY: RefCell<Option<QueryAction>> = const { RefCell::new(None) };
}

struct Hook;
impl Hook {
    fn install(prefix: &'static str, action: impl FnOnce() + 'static) -> Self {
        AFTER_QUERY.with(|hook| {
            assert!(hook.borrow().is_none());
            *hook.borrow_mut() = Some((prefix, Some(Box::new(action))));
        });
        Self
    }
    fn fired(&self) -> bool {
        AFTER_QUERY.with(|h| h.borrow().as_ref().unwrap().1.is_none())
    }
}
impl Drop for Hook {
    fn drop(&mut self) {
        AFTER_QUERY.with(|hook| *hook.borrow_mut() = None);
    }
}
pub(super) fn after_query(sql: &str) {
    let action = AFTER_QUERY.with(|hook| {
        hook.borrow_mut()
            .as_mut()
            .and_then(|(prefix, action)| sql.starts_with(*prefix).then(|| action.take()).flatten())
    });
    if let Some(action) = action {
        action();
    }
}

enum Backend {
    Postgres(Box<PostgresBackend>),
    Mysql(MysqlBackend),
}
impl Backend {
    fn connect(url: &RdbUrl) -> Self {
        match url.kind {
            RdbKind::Postgres => {
                Self::Postgres(Box::new(PostgresBackend::connect(&url.url).unwrap()))
            }
            RdbKind::Mysql => Self::Mysql(MysqlBackend::connect(&url.url).unwrap()),
        }
    }
    fn query(&mut self) -> &mut dyn OptunaBackend {
        match self {
            Self::Postgres(b) => b.as_mut(),
            Self::Mysql(b) => b,
        }
    }
    fn read<T>(
        &mut self,
        read: impl FnOnce(&mut dyn OptunaBackend) -> Result<T, String>,
    ) -> Result<T, String> {
        match self {
            Self::Postgres(b) => b.with_study_read(read),
            Self::Mysql(b) => b.with_study_read(read),
        }
    }
}

#[derive(Clone)]
struct Fixture {
    url: RdbUrl,
    python: String,
    name: String,
    id: u32,
}
impl Fixture {
    fn new(family: &str, index: usize) -> Self {
        let url = std::env::var(format!("TUNNY_SNAPSHOT_{}_URL", family.to_uppercase()))
            .expect("explicit disposable database URL required");
        let url = RdbUrl::parse(&url).unwrap();
        assert!(url.url.contains("@127.0.0.1:"));
        assert!(url.url.ends_with("/snapshot"));
        let name = format!("tunny-server-snapshot-{index}");
        let id = scan_study_list_url(&url)
            .unwrap()
            .into_iter()
            .find(|s| s.name == name)
            .expect("run the genuine Optuna fixture generator first")
            .study_id;
        Self {
            url,
            python: std::env::var("TUNNY_SNAPSHOT_PYTHON").expect("fixture Python required"),
            name,
            id,
        }
    }
    fn change(&self, action: &str) {
        let family = match self.url.kind {
            RdbKind::Postgres => "postgres",
            RdbKind::Mysql => "mysql",
        };
        let status = Command::new(&self.python)
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/create_server_snapshot_fixture.py"
            ))
            .args([family, action, &self.name])
            .output()
            .expect("start the fixture-only Optuna writer");
        assert!(
            status.status.success(),
            "Optuna writer failed; diagnostics suppressed"
        );
    }
}

fn rows_view(load: RdbStudyRows) -> Value {
    json!({
        "meta": {"id": load.meta.study_id, "name": load.meta.name,
            "directions": format!("{:?}", load.meta.directions),
            "total": load.meta.total_trials, "complete": load.meta.completed_trials,
            "params": load.meta.param_names, "objectives": load.meta.objective_names,
            "attrs": load.meta.user_attr_names, "constraints": load.meta.has_constraints,
            "bounds": load.meta.param_bounds},
        "columns": [load.param_names, load.objective_names,
            load.user_attr_numeric_names, load.user_attr_string_names],
        "width": load.max_constraints,
        "rows": load.rows.into_iter().map(|r| json!({
            "id": r.trial_id, "number": r.trial_number, "params": r.param_display,
            "labels": r.param_category_label, "values": r.objective_values,
            "distribution": {"categorical": r.distribution_metadata.categorical,
                "discrete": r.distribution_metadata.numeric_discrete,
                "categories": r.distribution_metadata.categories},
            "numeric": r.user_attrs_numeric, "text": r.user_attrs_string,
            "json": r.user_attrs_json, "constraints": r.constraint_values,
        })).collect::<Vec<_>>(),
        "extras": load.extras.trials.into_iter().map(|t| json!({
            "id": t.trial_id, "number": t.trial_number, "state": t.state.label(),
            "start": t.datetime_start, "end": t.datetime_complete,
            "intermediate": t.intermediate_values,
        })).collect::<Vec<_>>(),
    })
}

#[derive(Clone, Copy, Debug)]
enum Route {
    Independent,
    Rows,
    DataFrame,
}
impl Route {
    fn read(self, f: &Fixture) -> Value {
        match self {
            Self::Independent => {
                rows_view(parse_single_study_rows(Backend::connect(&f.url).query(), f.id).unwrap())
            }
            Self::Rows => rows_view(parse_single_study_rows_url(&f.url, f.id).unwrap()),
            Self::DataFrame => {
                let (meta, frame, extras) = parse_single_study_url(&f.url, f.id).unwrap();
                json!({"meta": format!("{meta:?}"), "extras": format!("{extras:?}"),
                    "rows": frame.row_count(),
                    "columns": frame.column_names().into_iter().map(|name| json!({
                        "numeric": frame.get_numeric_column(&name),
                        "text": frame.get_string_column(&name), "name": name,
                    })).collect::<Vec<_>>(),
                    "attributes": (0..frame.row_count()).map(|row|
                        frame.user_attr_names().map(|name| (name.to_string(),
                            json!(frame.user_attr_value(name, row))))
                            .collect::<serde_json::Map<_, _>>()
                    ).collect::<Vec<_>>(),
                })
            }
        }
    }
}

#[test]
#[ignore = "requires disposable PostgreSQL/MySQL and genuine Optuna fixtures; see validation report"]
fn server_snapshot_interleaved_acquisition() {
    let _guard = LIVE_TEST.lock().unwrap();
    for family in ["postgres", "mysql"] {
        let mut index = 0;
        for action in ["transition", "metadata"] {
            for route in [Route::Independent, Route::Rows, Route::DataFrame] {
                let f = Fixture::new(family, index);
                index += 1;
                let old = route.read(&f);
                assert_eq!(route.read(&f), old, "unchanged database control");
                let writer = f.clone();
                let (start, go) = mpsc::sync_channel(1);
                let (done, committed) = mpsc::sync_channel(1);
                let thread = std::thread::spawn(move || {
                    go.recv_timeout(Duration::from_secs(30)).unwrap();
                    writer.change(action);
                    done.try_send(()).unwrap();
                });
                let prefix = if action == "transition" {
                    "SELECT trial_id, number, state"
                } else {
                    "SELECT study_name"
                };
                let hook = Hook::install(prefix, move || {
                    start.try_send(()).unwrap();
                    committed
                        .recv_timeout(Duration::from_secs(30))
                        .expect("writer must commit while acquisition is retained");
                });
                let acquired = route.read(&f);
                assert!(hook.fired());
                drop(hook);
                thread.join().unwrap();
                let new = route.read(&f);
                assert_ne!(old, new);
                match route {
                    Route::Independent if action == "transition" => {
                        assert_ne!(acquired, old);
                        assert_ne!(acquired, new);
                    }
                    Route::Independent => assert_eq!(acquired, new),
                    _ => assert_eq!(acquired, old),
                }
                if action == "metadata" && !matches!(route, Route::DataFrame) {
                    assert_eq!(old["meta"]["total"], new["meta"]["total"]);
                    assert_eq!(old["meta"]["complete"], new["meta"]["complete"]);
                }
                println!("{family} {action} {route:?}: writer committed; view verified");
            }
        }
    }
}

fn name(session: &mut dyn OptunaBackend, id: u32) -> Result<String, String> {
    let rows = session.query(
        "SELECT value_json FROM study_system_attributes WHERE study_id = ?",
        &[SqlParam::I64(i64::from(id))],
    )?;
    Ok(rows[0][0].as_text().unwrap().to_string())
}

#[test]
#[ignore = "requires disposable PostgreSQL/MySQL and genuine Optuna fixtures; see validation report"]
fn server_snapshot_timing_and_cleanup() {
    let _guard = LIVE_TEST.lock().unwrap();
    for family in ["postgres", "mysql"] {
        let f = Fixture::new(family, 10);
        let mut backend = Backend::connect(&f.url);
        backend
            .read(|session| {
                f.change("metadata");
                assert_eq!(name(session, f.id)?, "[\"new-objective\"]");
                Ok(())
            })
            .unwrap();
        let f = Fixture::new(family, 11);
        backend
            .read(|session| {
                assert_eq!(name(session, f.id)?, "[\"old-objective\"]");
                f.change("metadata");
                assert_eq!(name(session, f.id)?, "[\"old-objective\"]");
                Ok(())
            })
            .unwrap();
        assert_eq!(name(backend.query(), f.id).unwrap(), "[\"new-objective\"]");

        // Producer API calls commit separately; a view between calls is valid.
        let middle = Fixture::new(family, 12);
        middle.change("metadata");
        backend
            .read(|session| {
                let acquired = rows_view(parse_single_study_rows(session, middle.id)?);
                assert_eq!(acquired["meta"]["objectives"], json!(["new-objective"]));
                assert_eq!(acquired["meta"]["complete"], 1);
                middle.change("transition");
                assert_eq!(
                    rows_view(parse_single_study_rows(session, middle.id)?),
                    acquired
                );
                Ok(())
            })
            .unwrap();
        assert_eq!(
            parse_single_study_rows_url(&middle.url, middle.id)
                .unwrap()
                .rows
                .len(),
            2
        );

        for failure in ["query", "callback", "readonly", "conversion", "success"] {
            let result = backend.read(|session| {
                let _ = name(session, f.id)?;
                match failure {
                    "query" => {
                        session.query("SELECT * FROM tunny_missing_snapshot_table", &[])?;
                    }
                    "callback" => {
                        session.query_for_each("SELECT study_id FROM studies", &[], &mut |_| {
                            Err("fixture callback stopped".into())
                        })?;
                    }
                    "readonly" => {
                        session.query("UPDATE studies SET study_name = study_name", &[])?;
                    }
                    "conversion" => {
                        let rows = session.query("SELECT 'not-an-integer'", &[])?;
                        rows[0][0].as_i64().ok_or("fixture conversion failed")?;
                    }
                    _ => (),
                }
                Ok(())
            });
            if failure == "success" {
                assert!(result.is_ok());
            } else {
                assert!(result.is_err(), "{failure}");
            }
            if failure == "callback" {
                assert_eq!(result.unwrap_err(), "fixture callback stopped");
            } else if failure == "conversion" {
                assert_eq!(result.unwrap_err(), "fixture conversion failed");
            }
            // A leaked read-only/aborted transaction would reject this write.
            backend
                .query()
                .query(
                    "UPDATE studies SET study_name = study_name WHERE study_id = ?",
                    &[SqlParam::I64(i64::from(f.id))],
                )
                .unwrap();
            assert_eq!(name(backend.query(), f.id).unwrap(), "[\"new-objective\"]");
            println!("{family} {failure}: session released; connection reusable");
        }

        for phase in ["begin", "finish", "query"] {
            let mut reader = Backend::connect(&f.url);
            let id_sql = if family == "postgres" {
                "SELECT pg_backend_pid()"
            } else {
                "SELECT CONNECTION_ID()"
            };
            let id = reader.query().query(id_sql, &[]).unwrap()[0][0]
                .as_i64()
                .unwrap();
            let mut killer = Backend::connect(&f.url);
            let mut kill = || {
                let sql = if family == "postgres" {
                    format!("SELECT pg_terminate_backend({id})")
                } else {
                    format!("KILL CONNECTION {id}")
                };
                killer.query().query(&sql, &[]).unwrap();
            };
            if phase == "begin" {
                kill();
            }
            let mut entered = false;
            let result = reader.read(|session| {
                entered = true;
                let _ = name(session, f.id)?;
                kill();
                if phase == "query" {
                    let _ = name(session, f.id)?;
                }
                Ok(())
            });
            assert!(
                result.is_err(),
                "{family} {phase} must not return successful data"
            );
            assert_eq!(entered, phase != "begin");
            assert!(parse_single_study_rows_url(&f.url, f.id).is_ok());
            println!("{family} connection loss at {phase}: error, fresh acquisition succeeds");
        }
    }
}

fn scalar(backend: &mut dyn OptunaBackend, sql: &str) -> i64 {
    backend.query(sql, &[]).unwrap()[0][0].as_i64().unwrap()
}

fn observer_url(f: &Fixture) -> RdbUrl {
    match f.url.kind {
        RdbKind::Postgres => f.url.clone(),
        RdbKind::Mysql => RdbUrl::parse(
            &std::env::var("TUNNY_SNAPSHOT_MYSQL_OBSERVER_URL")
                .expect("disposable MySQL observer required for resource inspection"),
        )
        .unwrap(),
    }
}

fn retained_resources(observer: &mut dyn OptunaBackend, family: &str, id: i64) -> i64 {
    if family == "postgres" {
        scalar(
            observer,
            &format!(
                "SELECT COUNT(*) FROM pg_stat_activity WHERE pid={id} AND backend_xmin IS NOT NULL"
            ),
        )
    } else {
        scalar(
            observer,
            &format!(
                "SELECT COUNT(*) FROM performance_schema.metadata_locks ml \
             JOIN performance_schema.threads th ON ml.OWNER_THREAD_ID=th.THREAD_ID \
             WHERE th.PROCESSLIST_ID={id} AND ml.OBJECT_SCHEMA='snapshot' \
             AND ml.OBJECT_TYPE='TABLE' AND ml.LOCK_STATUS='GRANTED'"
            ),
        )
    }
}

#[test]
#[ignore = "requires disposable PostgreSQL/MySQL and genuine Optuna fixtures; see validation report"]
fn server_snapshot_resource_lifetime_and_public_errors() {
    let _guard = LIVE_TEST.lock().unwrap();
    for family in ["postgres", "mysql"] {
        let f = Fixture::new(family, 20);
        let mut observer = Backend::connect(&observer_url(&f));
        let mut reader = Backend::connect(&f.url);
        let id_sql = if family == "postgres" {
            "SELECT pg_backend_pid()"
        } else {
            "SELECT CONNECTION_ID()"
        };
        let id = scalar(reader.query(), id_sql);
        for fail in [false, true] {
            let result = reader.read(|session| {
                let _ = name(session, f.id)?;
                let held = retained_resources(observer.query(), family, id);
                assert!(held > 0, "observe the retained snapshot/metadata locks");
                println!("{family} fail={fail}: retained resource count={held}");
                if fail {
                    Err("fixture stopped".into())
                } else {
                    Ok(())
                }
            });
            assert_eq!(result.is_err(), fail);
            assert_eq!(retained_resources(observer.query(), family, id), 0);
        }
        drop(reader);
        // Every public path opens its own connection; only the observer is
        // otherwise live. Identify that connection without logging credentials.
        for dataframe in [false, true] {
            for prefix in ["SELECT study_name", "SELECT tsa.trial_id"] {
                let observer_url = observer_url(&f);
                let family_owned = family.to_string();
                let hook = Hook::install(prefix, move || {
                    let mut killer = Backend::connect(&observer_url);
                    let sql = if family_owned == "postgres" {
                        "SELECT pid FROM pg_stat_activity WHERE datname='snapshot' \
                         AND usename='snapshot' AND pid<>pg_backend_pid() AND state='idle in transaction'"
                    } else {
                        "SELECT ID FROM information_schema.PROCESSLIST WHERE USER='snapshot' AND DB='snapshot'"
                    };
                    let ids = killer.query().query(sql, &[]).unwrap();
                    assert_eq!(ids.len(), 1);
                    let id = ids[0][0].as_i64().unwrap();
                    let kill = if family_owned == "postgres" {
                        format!("SELECT pg_terminate_backend({id})")
                    } else {
                        format!("KILL CONNECTION {id}")
                    };
                    killer.query().query(&kill, &[]).unwrap();
                });
                let result = if dataframe {
                    parse_single_study_url(&f.url, f.id).map(|_| ())
                } else {
                    parse_single_study_rows_url(&f.url, f.id).map(|_| ())
                };
                assert!(hook.fired());
                drop(hook);
                let error = result.expect_err("a failed public acquisition must not return data");
                assert!(!error.contains(&f.url.url));
                let password = f
                    .url
                    .url
                    .split("://")
                    .nth(1)
                    .unwrap()
                    .split('@')
                    .next()
                    .unwrap()
                    .split(':')
                    .nth(1)
                    .unwrap();
                assert!(
                    !error.contains(password),
                    "credentials must not appear in diagnostics"
                );
                if prefix == "SELECT tsa.trial_id" {
                    assert!(error.contains("Failed to finish"), "{error}");
                }
                assert!(parse_single_study_rows_url(&f.url, f.id).is_ok());
                println!(
                    "{family} dataframe={dataframe} failure after {prefix}: no successful data"
                );
            }
            let error = if dataframe {
                parse_single_study_url(&f.url, u32::MAX).map(|_| ())
            } else {
                parse_single_study_rows_url(&f.url, u32::MAX).map(|_| ())
            }
            .unwrap_err();
            assert!(error.contains("not found"));
        }
    }
}
