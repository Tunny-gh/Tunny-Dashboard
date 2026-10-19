"""Disposable Optuna-created server fixtures; never use an existing database.

URLs and the fixture Python executable are supplied only to the opt-in Rust
tests. This script intentionally never prints URLs, credentials, or SQL params.
"""

import argparse
import importlib.metadata
import os

import optuna
import sqlalchemy as sa
from optuna.distributions import CategoricalDistribution, FloatDistribution
from optuna.trial import TrialState, create_trial


PREFIX = "tunny-server-snapshot-"
optuna.logging.set_verbosity(optuna.logging.ERROR)


def storage_url(backend):
    url = os.environ[f"TUNNY_SNAPSHOT_{backend.upper()}_URL"]
    parsed = sa.engine.make_url(url)
    if parsed.host != "127.0.0.1" or parsed.database != "snapshot":
        raise RuntimeError("Only the disposable loopback snapshot database is allowed")
    driver = "postgresql+psycopg2" if backend == "postgres" else "mysql+pymysql"
    return parsed.set(drivername=driver)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("backend", choices=["postgres", "mysql"])
    parser.add_argument("action", choices=["seed", "metadata", "transition"])
    parser.add_argument("study", nargs="?")
    args = parser.parse_args()
    url = storage_url(args.backend)
    # Bound fixture writers, not product acquisition latency or retry behavior.
    connect_args = (
        {"connect_timeout": 5, "options": "-c statement_timeout=15000 -c lock_timeout=5000"}
        if args.backend == "postgres"
        else {"connect_timeout": 5, "read_timeout": 15, "write_timeout": 15}
    )
    storage = optuna.storages.RDBStorage(
        url.render_as_string(hide_password=False),
        engine_kwargs={"connect_args": connect_args},
    )
    try:
        if args.action != "seed":
            if not args.study or not args.study.startswith(PREFIX):
                raise RuntimeError("Not a task-owned fixture Study")
            study = optuna.load_study(study_name=args.study, storage=storage)
            if args.action == "metadata":
                study.set_metric_names(["new-objective"])
            else:
                study.tell(1, 30.0)
            return

        for summary in optuna.get_all_study_summaries(storage):
            if summary.study_name.startswith(PREFIX):
                optuna.delete_study(study_name=summary.study_name, storage=storage)
        for index in range(32):
            study = optuna.create_study(storage=storage, study_name=f"{PREFIX}{index}")
            study.set_metric_names(["old-objective"])
            study.add_trial(create_trial(
                value=10.0,
                params={"x": 1.0, "kind": "a"},
                distributions={"x": FloatDistribution(0, 10),
                               "kind": CategoricalDistribution(["a", "b"])},
                user_attrs={"tag": {"version": 0}, "empty": [], "null": None},
                system_attrs={"constraints": [-1.0]},
                intermediate_values={0: 10.0},
            ))
            study.add_trial(create_trial(
                state=TrialState.RUNNING,
                params={"x": 2.0},
                distributions={"x": FloatDistribution(0, 10)},
                user_attrs={"tag": {"version": 1}},
                system_attrs={"constraints": [-1.0, -2.0]},
                intermediate_values={0: 20.0},
            ))
        print(f"{args.backend}: 32 genuine Optuna Studies seeded; schema={storage.get_current_version()}")
        with storage.engine.connect() as conn:
            if args.backend == "postgres":
                print("server:", conn.execute(sa.text("SELECT version()")).scalar())
                print("default isolation:", conn.execute(sa.text("SHOW default_transaction_isolation")).scalar())
            else:
                print("server/config:", tuple(conn.execute(sa.text(
                    "SELECT VERSION(), @@default_storage_engine, @@transaction_isolation, "
                    "@@autocommit, @@completion_type, @@innodb_lock_wait_timeout"
                )).one()))
                engines = list(conn.execute(sa.text(
                    "SELECT TABLE_NAME, ENGINE FROM information_schema.tables "
                    "WHERE TABLE_SCHEMA = DATABASE() ORDER BY TABLE_NAME"
                )))
                assert all(engine == "InnoDB" for _, engine in engines)
                print("Optuna-created engines:", [tuple(row) for row in engines])
        for package in ["optuna", "SQLAlchemy", "alembic", "psycopg2-binary", "PyMySQL"]:
            print(package, importlib.metadata.version(package))
    finally:
        storage.remove_session()
        storage.engine.dispose()


if __name__ == "__main__":
    # Python driver exception strings can contain connection arguments.
    try:
        main()
    except Exception as error:
        raise SystemExit(f"Fixture action failed ({type(error).__name__}); details suppressed") from None
