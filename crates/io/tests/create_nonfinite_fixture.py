"""Print a real Optuna 5 Journal fixture; use only disposable temporary files."""
import math
import tempfile
from pathlib import Path
import optuna
from optuna.storages import JournalStorage
from optuna.storages.journal import JournalFileBackend, JournalFileOpenLock

assert optuna.__version__ == "5.0.0"
optuna.logging.set_verbosity(optuna.logging.ERROR)
with tempfile.TemporaryDirectory(dir=r"C:\Users\dev\AppData\Local\Temp\opencode") as directory:
    path = str(Path(directory) / "nonfinite.log")
    storage = JournalStorage(JournalFileBackend(path, lock_obj=JournalFileOpenLock(path)))
    study = optuna.create_study(storage=storage, study_name="optuna5-nonfinite")
    for value in [float("inf"),float("-inf"),2.0]:
        trial = study.ask()
        trial.set_user_attr("positive",float("inf"))
        trial.set_user_attr("negative",float("-inf"))
        trial.set_user_attr("nan",float("nan"))
        trial.set_user_attr("null",None)
        trial.set_user_attr("text","Infinity")
        trial.set_user_attr("nested",[float("inf"),None,{"x":float("nan")}])
        trial.report(float("inf"),0)
        trial.report(float("nan"),1)
        study.tell(trial,value)
    trials = study.trials
    assert all(t.state.name == "COMPLETE" for t in trials)
    assert trials[0].value == float("inf") and trials[1].value == float("-inf")
    assert all(t.user_attrs["positive"] == float("inf") and math.isnan(t.user_attrs["nan"]) for t in trials)
    replay = optuna.load_study(study_name=study.study_name,
        storage=JournalStorage(JournalFileBackend(path, lock_obj=JournalFileOpenLock(path))))
    assert replay.trials[0].value == float("inf")
    assert replay.trials[0].user_attrs["positive"] == float("inf")
    print(Path(path).read_text(),end="")
