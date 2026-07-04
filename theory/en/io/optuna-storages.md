# Optuna Storage Formats

## Overview

Optuna persists an optimization study through one of two storage backends:

- **JournalStorage** — an append-only operation log (one JSON record per line).
- **RDBStorage** — a relational database (SQLite, PostgreSQL, MySQL, …) with a normalized table schema.

This app can read three input formats: an Optuna **journal file** (`.log`), an Optuna **SQLite database** (`.db` / `.sqlite` / `.sqlite3`), and a **DesignExplorer-style flat CSV** (one row per trial, produced by external optimization tools rather than Optuna itself). The first two are read directly from Optuna's own on-disk representations; the flat CSV is a separate convention documented for completeness, not an Optuna storage format.

---

## Journal Storage

A journal file is a plain-text log where each line is one JSON-encoded operation, tagged with an `op_code` that identifies what changed:

| op_code | Operation | Effect |
| ------- | --------- | ------ |
| 0 | `CREATE_STUDY` | Registers a new study (name, objective directions) |
| 3 | `SET_STUDY_SYSTEM_ATTR` | Sets a study-level system attribute (e.g. `study:metric_names`) |
| 4 | `CREATE_TRIAL` | Creates a trial, optionally with its initial params/values already attached |
| 5 | `SET_TRIAL_PARAM` | Records one parameter's internal value and distribution for a trial |
| 6 | `SET_TRIAL_STATE_VALUES` | Updates a trial's state and objective values (e.g. on completion) |
| 8 | `SET_TRIAL_USER_ATTR` | Sets a user attribute on a trial |
| 9 | `SET_TRIAL_SYSTEM_ATTR` | Sets a system attribute on a trial (e.g. `constraints`) |

Because every change is a new line appended to the file, a reader can resume from a previously-seen byte offset and parse only the newly appended lines. This app's **Live Update** feature relies on exactly this property: it periodically re-reads the journal file from the last consumed offset, decodes the new operations, and incrementally updates in-memory trial rows without re-parsing the whole file.

---

## RDB (SQLite) Storage

RDBStorage represents the same information as a set of normalized tables instead of an operation log:

| Table | Role |
| ----- | ---- |
| `studies` | One row per study (id, name) |
| `study_directions` | One row per objective per study: `MINIMIZE` / `MAXIMIZE` |
| `trials` | One row per trial: state (`COMPLETE` / `PRUNED` / `FAIL` / `RUNNING` / …), number, study id |
| `trial_values` | One row per (trial, objective): the numeric value, tagged with `value_type` (`FINITE` / `INF_POS` / `INF_NEG`) |
| `trial_params` | One row per (trial, parameter): internal representation (`param_value`) plus the serialized `distribution_json` |
| `trial_user_attributes` | User-defined attributes attached to a trial |
| `trial_system_attributes` | Optuna/plugin-defined attributes attached to a trial (e.g. `constraints`) |
| `study_system_attributes` | Optuna/plugin-defined attributes attached to a study (e.g. `study:metric_names`) |

Unlike the journal, there is no inherent "new lines since last read" boundary — the tables are mutated in place — so incremental diffing is not a natural fit for this backend (see [Characteristics & Limitations](#characteristics--limitations) below).

---

## Schema Mapping (this app's interpretation rules)

Both storage formats carry the same underlying concepts; this app applies the following rules uniformly when interpreting them:

- **Parameter internal representation**: `FloatDistribution` values are used as-is; `IntDistribution` values are integers. `CategoricalDistribution` values are stored internally as the **index into the `choices` array**, and the app resolves this back to the human-readable label only at display time.
- **Objective names**: read from the `study:metric_names` system attribute (set via Optuna's `study.set_metric_names()`). If absent, the app falls back to generic names `obj0`, `obj1`, ….
- **Constraints**: read from the `constraints` key under trial system attributes — the standard convention Optuna writes when a sampler is configured with a `constraints_func`. A non-positive value ($\le 0$) means the constraint is satisfied; a strictly positive value means it is violated.
- **Trial state filter**: only trials in the `COMPLETE` state are used for analysis (Pareto fronts, importance, surrogate models, etc.); `PRUNED`, `FAIL`, and `RUNNING` trials are excluded from those computations.
- **trial_id vs. trial number**: `trial_id` is a database-wide unique identifier (used e.g. to key artifacts), while the trial **number** is 0-indexed within a single study (used for display and export). The two must never be conflated when joining across tables.

---

## Characteristics & Limitations

- **Journal storage** is well suited to live monitoring: its append-only structure lets a reader track an in-progress optimization by tailing new lines, which is what powers this app's Live Update feature.
- **RDB (SQLite) storage** is better suited to concurrent/distributed optimization (multiple workers writing trials in parallel) and to interoperating with the broader Optuna ecosystem (e.g. `optuna-dashboard`, other tooling that expects a standard RDB schema).
- This app opens SQLite files **read-only**, so it can safely inspect a database belonging to an optimization that is still running elsewhere.
- However, **Live Update (incremental re-parsing) is only implemented for journal files**. SQLite databases are read once, in full, on load; there is no equivalent "tail the new rows" mode for the RDB backend in this app today.
