"""Read-only #258 format/API probe; requires the investigated Optuna 4.9.0."""

import json
from pathlib import Path

import optuna
from optuna.exceptions import UpdateFinishedTrialError
from optuna.storages import JournalStorage
from optuna.storages.journal import BaseJournalBackend
from optuna.study import StudyDirection
from optuna.trial import TrialState


class MemoryJournal(BaseJournalBackend):
    def __init__(self, logs=()):
        self.logs = list(logs)

    def read_logs(self, log_number_from):
        return iter(self.logs[log_number_from:])

    def append_logs(self, logs):
        self.logs.extend(logs)


def upstream_probe(parent):
    """Optional official-project corroboration in an isolated environment."""
    from importlib.metadata import version
    import tempfile

    import rustuna
    from optuna_dashboard._inmemory_cache import InMemoryCache
    from optuna_dashboard._storage import get_trials
    from optuna_dashboard._storage_url import get_storage
    from optuna_dashboard.artifact._backend import get_trial_artifact_meta

    assert version("optuna-dashboard") == "0.21.0"
    assert version("rustuna") == "0.1.1"
    assert parent.is_dir()
    fixture = Path(__file__).resolve().parents[1] / "fixtures/issue258-terminal-attributes.jsonl"
    with tempfile.TemporaryDirectory(prefix="issue258-", dir=parent) as work:
        path = Path(work) / "exact.jsonl"
        path.write_bytes(fixture.read_bytes())
        dashboard = get_storage(str(path))
        assert isinstance(dashboard, JournalStorage)
        cache = InMemoryCache()
        for sid in [0, 1, 3, 4, 5, 8]:
            trials = get_trials(cache, dashboard, sid)
            print(json.dumps({"dashboard_python_study": sid, "storage": type(dashboard).__name__,
                              "trials": [{"id": t._trial_id, "user": t.user_attrs,
                                          "system": t.system_attrs} for t in trials]}, sort_keys=True))
        assert dashboard.get_trial(1).user_attrs == {"payload": {"old": True}}
        assert get_trial_artifact_meta(dashboard, 8, 12, "image") is None
        print("Dashboard Python exact fixture: old payload retained; late Artifact metadata absent")

        try:
            raw = rustuna.storages.JournalFileStorage(str(path))
            raw.get_trials(0)
        except RuntimeError as error:
            assert "StorageError: expected value" in str(error)
            print(f"Rustuna exact fixture: {type(error).__name__}: {error}")
        else:
            raise AssertionError("Rustuna 0.1.1 unexpectedly accepted the nonfinite fixture")

        finite_path = Path(work) / "without-nonfinite-record.jsonl"
        finite_path.write_bytes(b"".join(line for line in fixture.read_bytes().splitlines(keepends=True)
                                        if b"NaN,Infinity,-Infinity" not in line))
        finite = rustuna.storages.JournalFileStorage(str(finite_path))
        assert len(finite.get_studies()) == 9
        assert finite.get_trial(12).system_attrs == {}
        print("Rustuna variant removing only the nonfinite op9 record: all 9 Studies load; late Artifact absent")

        # Preserve exact bytes of the finite prefix; do not normalize nonfinite input.
        prefix = fixture.read_bytes().split(
            b'{"op_code":0,"worker_id":"fixture","study_name":"running-schema-control"', 1)[0]
        prefix_path = Path(work) / "finite-prefix.jsonl"
        prefix_path.write_bytes(prefix)
        raw = rustuna.storages.JournalFileStorage(str(prefix_path))
        for tid in [0, 1, 4, 6, 7]:
            trial = raw.get_trial(tid)
            assert trial.user_attrs == trial.system_attrs == {}
        inline = raw.get_trial(2)
        print("Rustuna finite prefix: trials 0/1/4/6/7 user/system attrs all {}; op8/9 Optuna fields not imported")
        print(json.dumps({"rustuna_inline_trial": 2, "user": dict(inline.user_attrs),
                          "system": dict(inline.system_attrs)}, sort_keys=True))
        assert inline.user_attrs["null"] == "null"
        assert inline.system_attrs["constraints"] == "[-1]"

        for state in (TrialState.COMPLETE, TrialState.PRUNED, TrialState.FAIL):
            native_path = Path(work) / f"native-{state.name}.jsonl"
            native = rustuna.storages.JournalFileStorage(str(native_path))
            adapted = get_storage(native)
            assert type(adapted).__name__ == "ToOptunaStorage"
            sid = adapted.create_new_study([StudyDirection.MINIMIZE], "native")
            tid = adapted.create_new_trial(sid)
            adapted.set_trial_user_attr(tid, "payload", {"old": True})
            adapted.set_trial_system_attr(tid, "constraints", [-1])
            adapted.set_trial_system_attr(tid, "artifacts:before", json.dumps({"artifact_id": "before"}))
            adapted.set_trial_state_values(tid, state, [1] if state == TrialState.COMPLETE else None)
            rejected = []
            for key, value, setter in (
                ("payload", {"new": True}, adapted.set_trial_user_attr),
                ("constraints", [1, 0], adapted.set_trial_system_attr),
                ("artifacts:late", json.dumps({"artifact_id": "late"}), adapted.set_trial_system_attr),
            ):
                before = len(native_path.read_text().splitlines())
                try:
                    setter(tid, key, value)
                except UpdateFinishedTrialError:
                    rejected.append(key)
                else:
                    raise AssertionError(f"Rustuna unexpectedly accepted {state.name}/{key}")
                assert len(native_path.read_text().splitlines()) == before + 1
            replay = get_storage(rustuna.storages.JournalFileStorage(str(native_path)))
            trial = get_trials(InMemoryCache(), replay, sid)[0]
            assert trial.user_attrs == {"payload": {"old": True}}
            assert trial.system_attrs["constraints:0"] == -1.0
            assert "artifacts:late" not in trial.system_attrs
            assert get_trial_artifact_meta(replay, sid, tid, "before") == {"artifact_id": "before"}
            assert get_trial_artifact_meta(replay, sid, tid, "late") is None
            print(json.dumps({"rustuna_state": state.name, "dashboard_adapter": type(replay).__name__,
                              "rejected": rejected, "user": trial.user_attrs,
                              "system": trial.system_attrs}, sort_keys=True))
        print("Dashboard 0.21.0 / Rustuna 0.1.1 corroboration assertions passed")


def main():
    assert optuna.__version__ == "4.9.0", optuna.__version__
    optuna.logging.set_verbosity(optuna.logging.WARNING)
    print(f"Optuna {optuna.__version__}")
    for state in (TrialState.COMPLETE, TrialState.PRUNED, TrialState.FAIL):
        backend = MemoryJournal()
        storage = JournalStorage(backend)
        sid = storage.create_new_study([StudyDirection.MINIMIZE], "probe")
        tid = storage.create_new_trial(sid)
        storage.set_trial_user_attr(tid, "payload", {"old": True})
        storage.set_trial_system_attr(tid, "constraints", [-1])
        storage.set_trial_state_values(tid, state, [1] if state == TrialState.COMPLETE else None)
        rejected = []
        for key, value, setter in (
            ("payload", {"new": [1, "x", False]}, storage.set_trial_user_attr),
            ("constraints", [-1, 0, 0], storage.set_trial_system_attr),
            ("artifacts:image", json.dumps({"artifact_id": "image"}), storage.set_trial_system_attr),
        ):
            before = len(backend.logs)
            try:
                setter(tid, key, value)
            except UpdateFinishedTrialError:
                rejected.append(key)
            else:
                raise AssertionError(f"Unexpected accepted post-terminal {state.name}/{key}")
            assert len(backend.logs) == before + 1
        trial = storage.get_trial(tid)
        replay = JournalStorage(backend).get_trial(tid)
        assert trial.user_attrs == replay.user_attrs == {"payload": {"old": True}}
        assert trial.system_attrs == replay.system_attrs == {"constraints": [-1]}
        print(json.dumps({"state": state.name, "rejected": rejected,
                          "ops": [log["op_code"] for log in backend.logs],
                          "replay_user": replay.user_attrs, "replay_system": replay.system_attrs},
                         sort_keys=True))

    # Exercise the sampler's real after_trial ordering, not its method name.
    backend = MemoryJournal()
    storage = JournalStorage(backend)
    study = optuna.create_study(storage=storage, study_name="sampler-order",
                               sampler=optuna.samplers.TPESampler(
                                   seed=0, constraints_func=lambda trial: [-1.0]))
    study.optimize(lambda trial: 1.0, n_trials=1)
    ops = [int(log["op_code"]) for log in backend.logs]
    assert ops == [0, 4, 9, 6], ops
    print(f"TPE constraints order: {ops}")

    fixture = Path(__file__).resolve().parents[1] / "fixtures/issue258-terminal-attributes.jsonl"
    logs = [json.loads(line) for line in fixture.read_text(encoding="utf-8").splitlines()]
    replay = JournalStorage(MemoryJournal(logs))
    for sid in range(9):
        trials = replay.get_all_trials(sid)
        print(json.dumps({"fixture_study": sid, "trials": [
            {"id": t._trial_id, "number": t.number, "state": t.state.name,
             "user": t.user_attrs, "system": t.system_attrs} for t in trials]}, sort_keys=True))
    assert replay.get_trial(1).user_attrs == {"payload": {"old": True}}
    assert replay.get_trial(1).system_attrs == {"constraints": [-1]}
    assert replay.get_trial(4).system_attrs == replay.get_trial(6).system_attrs == {"constraints": [-1]}
    assert replay.get_trial(7).user_attrs == replay.get_trial(7).system_attrs == {}
    assert replay.get_trial(12).system_attrs == {}
    assert len(replay.get_trial(9).system_attrs["constraints"]) == 5
    completed = replay.get_trial(2).datetime_complete
    assert completed is not None and completed.isoformat() == "2026-10-17T00:01:00"
    print("inline COMPLETE datetime_complete: 2026-10-17T00:01:00")
    for sid, tid, width in ((1, 1, 1), (3, 4, 3), (4, 6, 4), (5, 7, 0)):
        updates = [log for log in logs if log.get("trial_id") == tid and log["op_code"] in (8, 9)]
        reordered = []
        for log in logs:
            if log.get("trial_id") == tid and log["op_code"] in (8, 9):
                continue
            if log.get("trial_id") == tid and log["op_code"] == 6:
                reordered.extend(updates)
            reordered.append(log)
        trial = JournalStorage(MemoryJournal(reordered)).get_trial(tid)
        assert len(trial.system_attrs["constraints"]) == width
        assert trial.user_attrs
        print(f"before_finish study={sid} trial={tid}: attributes retained, width={width}")
    print("Optuna API and fixture replay assertions passed")


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--upstream-temp", type=Path,
                        help="Run optional Dashboard 0.21.0/Rustuna 0.1.1 checks under this existing temp directory")
    args = parser.parse_args()
    main()
    if args.upstream_temp is not None:
        upstream_probe(args.upstream_temp)
