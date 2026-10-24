"""Create a disposable MCP protocol fixture using only Python's standard library.

The reduced Optuna-shaped schema tests entry composition, not Optuna producer
compatibility. The caller supplies a new path inside its temporary directory.
"""

import json
import sqlite3
import sys
from pathlib import Path

path = Path(sys.argv[1])
if path.exists() or not path.parent.is_dir():
    raise RuntimeError("Expected a new fixture path in an existing temporary directory")

with sqlite3.connect(path) as connection:
    connection.executescript("""
        CREATE TABLE studies(study_id INTEGER PRIMARY KEY, study_name TEXT);
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
        INSERT INTO studies VALUES(0,'other'),(1,'constrained');
        INSERT INTO study_directions VALUES(0,0,'MINIMIZE'),(1,0,'MINIMIZE'),(1,1,'MINIMIZE');
    """)
    constraints = [[-1], [1], None, [float("nan"), -1], [-1, -1], [-1, -1, -1]]
    for number, values in enumerate(constraints):
        trial_id = 10 * (number + 1)
        connection.execute("INSERT INTO trials VALUES(?,?,1,?,NULL,NULL)",
                           (trial_id, number, "RUNNING" if number == 5 else "COMPLETE"))
        if values is not None:
            connection.execute("INSERT INTO trial_system_attributes VALUES(?,'constraints',?)",
                               (trial_id, json.dumps(values)))
        if number == 5:
            continue
        connection.execute("INSERT INTO trial_params VALUES(?,'x',?,?)", (
            trial_id, float("inf") if number == 3 else number,
            json.dumps({"name": "FloatDistribution", "attributes": {
                "low": 0, "high": 10, "log": False, "step": None}})))
        connection.executemany("INSERT INTO trial_values VALUES(?,?,?,?)", [
            (trial_id, 0, None if number == 3 else number + 1,
             "INF_POS" if number == 3 else "FINITE"),
            (trial_id, 1, 2 if number == 4 else 5 - number, "FINITE"),
        ])
