//! Writer that produces an Optuna-compatible journal log (JSON Lines).
//!
//! Appends one record per line in a format that the existing parsers
//! (`super::parser` / `super::live_update`) can read, matching the actual format that
//! Optuna's `JournalStorage` writes (e.g. double-serialization of distributions). Since
//! the live-update poller reads based on byte-offset diffs, a single `write` + `flush`
//! is performed after each line written.
//!
//! Reference: spec doc "Optuna-compatible journal writer" (rust_core/src/io/journal/writer.rs)

use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::data::extras::TrialState;
use crate::io::datetime::format_naive_datetime;
use crate::io::journal::live_update::count_created_trials;
use crate::io::journal::parser::OptimizationDirection;

/// Default value for worker_id (included in every record for compatibility with the original Optuna reader).
const DEFAULT_WORKER_ID: &str = "tunny-dashboard";

/// The search-range specification for a param (written as the journal's distribution).
#[derive(Debug, Clone)]
pub enum ParamDistribution {
    Float { low: f64, high: f64 },
    Int { low: i64, high: i64 },
}

/// Writer for an Optuna-compatible journal log.
///
/// Appends one record per line of JSON Lines, flushing after each line.
/// Not thread-safe (the caller must serialize access, e.g. with a Mutex).
pub struct JournalWriter {
    file: File,
    path: PathBuf,
    /// The next study_id to assign (existing op0 count in the file + op0 count written by this writer).
    next_study_id: u32,
    /// The next global trial_id to assign (existing op4 count in the file + op4 count written by this writer).
    next_trial_id: u32,
    worker_id: String,
}

impl JournalWriter {
    /// Opens the file in append mode (creating it if absent). Scans existing content
    /// to assign the next study_id / trial_id. Uses the default worker_id
    /// (`"tunny-dashboard"`).
    pub fn open(path: &Path) -> Result<Self, String> {
        Self::open_with_worker_id(path, DEFAULT_WORKER_ID)
    }

    /// Opens with a specified worker_id. Otherwise behaves the same as [`JournalWriter::open`].
    pub fn open_with_worker_id(path: &Path, worker_id: &str) -> Result<Self, String> {
        let existing = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(err) => {
                return Err(format!(
                    "failed to read existing journal file {}: {err}",
                    path.display()
                ))
            }
        };

        let next_study_id = count_created_studies(&existing);
        let next_trial_id = count_created_trials(&existing);

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|err| format!("failed to open journal file {}: {err}", path.display()))?;

        Ok(JournalWriter {
            file,
            path: path.to_path_buf(),
            next_study_id,
            next_trial_id,
            worker_id: worker_id.to_string(),
        })
    }

    /// Writes op0 (and op3 if objective_names is non-empty), and returns study_id.
    pub fn create_study(
        &mut self,
        study_name: &str,
        directions: &[OptimizationDirection],
        objective_names: &[String],
    ) -> Result<u32, String> {
        let study_id = self.next_study_id;
        self.next_study_id += 1;

        let directions_json: Vec<u8> = directions
            .iter()
            .map(|direction| match direction {
                OptimizationDirection::Minimize => 1,
                OptimizationDirection::Maximize => 2,
            })
            .collect();

        let record = ordered_object(&[
            ("op_code", j(0)),
            ("worker_id", j(&self.worker_id)),
            ("study_name", j(study_name)),
            ("directions", j(&directions_json)),
        ]);
        self.write_line(&record)?;

        if !objective_names.is_empty() {
            let attr_record = ordered_object(&[
                ("op_code", j(3)),
                ("worker_id", j(&self.worker_id)),
                ("study_id", j(study_id)),
                (
                    "system_attr",
                    j(serde_json::json!({ "study:metric_names": objective_names })),
                ),
            ]);
            self.write_line(&attr_record)?;
        }

        Ok(study_id)
    }

    /// Writes op4 and returns the global trial_id. datetime_start is the current time.
    ///
    /// Does not include the `distributions` field (this registers it as RUNNING in the
    /// file-storage format, so that subsequent op5/op6 are awaited).
    pub fn create_trial(&mut self, study_id: u32) -> Result<u32, String> {
        let trial_id = self.next_trial_id;
        self.next_trial_id += 1;

        let record = ordered_object(&[
            ("op_code", j(4)),
            ("worker_id", j(&self.worker_id)),
            ("study_id", j(study_id)),
            ("datetime_start", j(format_naive_datetime(now_unix_secs()))),
        ]);
        self.write_line(&record)?;

        Ok(trial_id)
    }

    /// Writes op5. value is the actual value (Float as-is; Int also as its actual value in f64).
    ///
    /// `distribution` is double-serialized as a JSON string to match the format that
    /// Optuna actually writes (a mechanical conversion via `serde_json::Value::String`).
    pub fn set_trial_param(
        &mut self,
        trial_id: u32,
        param_name: &str,
        value: f64,
        distribution: &ParamDistribution,
    ) -> Result<(), String> {
        let inner = match distribution {
            ParamDistribution::Float { low, high } => ordered_object(&[
                ("name", j("FloatDistribution")),
                (
                    "attributes",
                    ordered_object(&[
                        ("step", j(Value::Null)),
                        ("low", j(low)),
                        ("high", j(high)),
                        ("log", j(false)),
                    ]),
                ),
            ]),
            ParamDistribution::Int { low, high } => ordered_object(&[
                ("name", j("IntDistribution")),
                (
                    "attributes",
                    ordered_object(&[
                        ("log", j(false)),
                        ("step", j(1)),
                        ("low", j(low)),
                        ("high", j(high)),
                    ]),
                ),
            ]),
        };

        let record = ordered_object(&[
            ("op_code", j(5)),
            ("worker_id", j(&self.worker_id)),
            ("trial_id", j(trial_id)),
            ("param_name", j(param_name)),
            ("param_value_internal", j(value)),
            // Double serialization: the distribution object is embedded as a JSON
            // string value, matching what Optuna itself writes.
            ("distribution", j(&inner)),
        ]);
        self.write_line(&record)
    }

    /// Writes op8 (SET_TRIAL_USER_ATTR) with a single user attribute. The value is
    /// any JSON value; the existing parsers read numbers into numeric attribute
    /// columns and strings into string attribute columns.
    pub fn set_trial_user_attr(
        &mut self,
        trial_id: u32,
        key: &str,
        value: &Value,
    ) -> Result<(), String> {
        let record = ordered_object(&[
            ("op_code", j(8)),
            ("worker_id", j(&self.worker_id)),
            ("trial_id", j(trial_id)),
            ("user_attr", ordered_object(&[(key, j(value))])),
        ]);
        self.write_line(&record)
    }

    /// Writes op9 (SET_TRIAL_SYSTEM_ATTR) with the trial's constraint values under
    /// the `"constraints"` key — the same layout Optuna's samplers write, which the
    /// existing parsers read for feasibility (feasible when every value <= 0).
    pub fn set_trial_constraints(
        &mut self,
        trial_id: u32,
        constraints: &[f64],
    ) -> Result<(), String> {
        let record = ordered_object(&[
            ("op_code", j(9)),
            ("worker_id", j(&self.worker_id)),
            ("trial_id", j(trial_id)),
            (
                "system_attr",
                ordered_object(&[("constraints", j(constraints))]),
            ),
        ]);
        self.write_line(&record)
    }

    /// Writes op6. Writes values as an array when state is Complete, otherwise null.
    /// datetime_complete is the current time.
    pub fn finish_trial(
        &mut self,
        trial_id: u32,
        state: TrialState,
        values: &[f64],
    ) -> Result<(), String> {
        let state_code: u8 = match state {
            TrialState::Running => 0,
            TrialState::Complete => 1,
            TrialState::Pruned => 2,
            TrialState::Fail => 3,
            TrialState::Waiting => 4,
        };
        let values_json = if state == TrialState::Complete {
            Value::Array(
                values
                    .iter()
                    .map(|value| serde_json::json!(value))
                    .collect(),
            )
        } else {
            Value::Null
        };

        let record = ordered_object(&[
            ("op_code", j(6)),
            ("worker_id", j(&self.worker_id)),
            ("trial_id", j(trial_id)),
            ("state", j(state_code)),
            ("values", j(&values_json)),
            (
                "datetime_complete",
                j(format_naive_datetime(now_unix_secs())),
            ),
        ]);
        self.write_line(&record)
    }

    /// The path to the journal file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Writes one record as a single line of JSON, performing a single `write` + `flush`.
    fn write_line(&mut self, object: &str) -> Result<(), String> {
        let mut line = object.to_string();
        line.push('\n');
        self.file.write_all(line.as_bytes()).map_err(|err| {
            format!(
                "failed to write journal record to {}: {err}",
                self.path.display()
            )
        })?;
        self.file.flush().map_err(|err| {
            format!(
                "failed to flush journal file {}: {err}",
                self.path.display()
            )
        })
    }
}

/// Serializes an object with the given key order and Python-style separators
/// (`", "` / `": "`), matching what Optuna's JournalFileBackend writes via
/// `json.dumps`. serde_json's own object type sorts keys alphabetically, which
/// would bury `op_code` in the middle of the line; field order carries no JSON
/// semantics, but keeping Optuna's layout makes the files diffable against
/// journals produced by Optuna itself.
///
/// Values are pre-serialized JSON fragments (use [`j`] for leaves, or another
/// `ordered_object` result for nested objects).
fn ordered_object(fields: &[(&str, String)]) -> String {
    let mut out = String::from("{");
    for (i, (key, value)) in fields.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&Value::String((*key).to_string()).to_string());
        out.push_str(": ");
        out.push_str(value);
    }
    out.push('}');
    out
}

/// Serializes a leaf value as a JSON fragment for [`ordered_object`].
fn j<T: serde::Serialize>(value: T) -> String {
    serde_json::to_string(&value).expect("primitive JSON serialization cannot fail")
}

/// Returns the current time as unix seconds (f64). Returns 0.0 on failure (never panics).
fn now_unix_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or(0.0)
}

/// Counts the number of op_code=0 (CREATE_STUDY) lines in the existing file content.
/// Unparseable lines and a trailing incomplete line are ignored.
fn count_created_studies(data: &[u8]) -> u32 {
    let mut count = 0u32;
    for line in data.split(|&byte| byte == b'\n') {
        if line.is_empty() {
            continue;
        }
        let Ok(json) = serde_json::from_slice::<Value>(line) else {
            continue;
        };
        if json.get("op_code").and_then(Value::as_u64) == Some(0) {
            count += 1;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::journal::parser::{parse_single_study, scan_study_list};

    /// Round-trip (most important): reads a journal written by the writer using the
    /// existing parser and verifies the study info / DataFrame / StudyExtras match expectations.
    #[test]
    fn writer_roundtrip_via_existing_parser() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("study.journal");
        let mut writer = JournalWriter::open(&path).unwrap();

        let study_id = writer
            .create_study(
                "my-study",
                &[
                    OptimizationDirection::Minimize,
                    OptimizationDirection::Maximize,
                ],
                &["weight".to_string(), "disp".to_string()],
            )
            .unwrap();
        assert_eq!(study_id, 0);

        // trial0: Float "span"(3..12)=5.5, Int "count"(1..10)=3 -> Complete [12.3, 4.5]
        let trial0 = writer.create_trial(study_id).unwrap();
        assert_eq!(trial0, 0);
        writer
            .set_trial_param(
                trial0,
                "span",
                5.5,
                &ParamDistribution::Float {
                    low: 3.0,
                    high: 12.0,
                },
            )
            .unwrap();
        writer
            .set_trial_param(
                trial0,
                "count",
                3.0,
                &ParamDistribution::Int { low: 1, high: 10 },
            )
            .unwrap();
        writer
            .finish_trial(trial0, TrialState::Complete, &[12.3, 4.5])
            .unwrap();

        // trial1: same param config -> Fail (no values)
        let trial1 = writer.create_trial(study_id).unwrap();
        assert_eq!(trial1, 1);
        writer
            .set_trial_param(
                trial1,
                "span",
                6.0,
                &ParamDistribution::Float {
                    low: 3.0,
                    high: 12.0,
                },
            )
            .unwrap();
        writer
            .set_trial_param(
                trial1,
                "count",
                4.0,
                &ParamDistribution::Int { low: 1, high: 10 },
            )
            .unwrap();
        writer.finish_trial(trial1, TrialState::Fail, &[]).unwrap();

        // trial2: -> Complete [10.0, 6.0]
        let trial2 = writer.create_trial(study_id).unwrap();
        assert_eq!(trial2, 2);
        writer
            .finish_trial(trial2, TrialState::Complete, &[10.0, 6.0])
            .unwrap();

        let data = std::fs::read(&path).unwrap();

        // Study list, directions, objective_names.
        let studies = scan_study_list(&data).unwrap();
        assert_eq!(studies.len(), 1);
        assert_eq!(studies[0].name, "my-study");
        assert_eq!(
            studies[0].directions,
            vec![
                OptimizationDirection::Minimize,
                OptimizationDirection::Maximize
            ]
        );
        assert_eq!(
            studies[0].objective_names,
            vec!["weight".to_string(), "disp".to_string()]
        );

        // Detailed parsing for one study.
        let (meta, df, extras) = parse_single_study(&data, 0).unwrap();
        assert_eq!(meta.completed_trials, 2);
        assert_eq!(meta.total_trials, 3);
        assert_eq!(meta.param_bounds.get("span"), Some(&(3.0, 12.0)));
        assert_eq!(meta.param_bounds.get("count"), Some(&(1.0, 10.0)));

        // DataFrame: only COMPLETE, 2 rows (trial0, trial2).
        assert_eq!(df.row_count(), 2);
        assert_eq!(df.get_trial_id(0), Some(0));
        assert_eq!(df.get_trial_id(1), Some(2));
        assert_eq!(
            df.get_numeric_column("weight"),
            Some([12.3, 10.0].as_slice())
        );
        assert_eq!(df.get_numeric_column("disp"), Some([4.5, 6.0].as_slice()));
        assert_eq!(df.get_numeric_column("span"), Some([5.5, 0.0].as_slice()));
        assert_eq!(df.get_numeric_column("count"), Some([3.0, 0.0].as_slice()));

        // StudyExtras: 3 trials, states Complete/Fail/Complete, datetimes are Some.
        assert_eq!(extras.trials.len(), 3);
        assert_eq!(extras.trials[0].trial_id, 0);
        assert_eq!(extras.trials[0].state, TrialState::Complete);
        assert!(extras.trials[0].datetime_start.is_some());
        assert!(extras.trials[0].datetime_complete.is_some());
        assert_eq!(extras.trials[1].trial_id, 1);
        assert_eq!(extras.trials[1].state, TrialState::Fail);
        assert!(extras.trials[1].datetime_start.is_some());
        assert!(extras.trials[1].datetime_complete.is_some());
        assert_eq!(extras.trials[2].trial_id, 2);
        assert_eq!(extras.trials[2].state, TrialState::Complete);
        assert!(extras.trials[2].datetime_start.is_some());
        assert!(extras.trials[2].datetime_complete.is_some());
    }

    /// Constraint round-trip: op9 written by the writer is read back by the
    /// existing parser as constraint columns and feasibility.
    #[test]
    fn constraints_roundtrip_via_existing_parser() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("study.journal");
        let mut writer = JournalWriter::open(&path).unwrap();

        let study_id = writer
            .create_study("con-study", &[OptimizationDirection::Minimize], &[])
            .unwrap();

        // trial0: feasible (all <= 0), trial1: infeasible
        let trial0 = writer.create_trial(study_id).unwrap();
        writer.set_trial_constraints(trial0, &[-0.5, 0.0]).unwrap();
        writer
            .finish_trial(trial0, TrialState::Complete, &[1.0])
            .unwrap();
        let trial1 = writer.create_trial(study_id).unwrap();
        writer.set_trial_constraints(trial1, &[0.25, -1.0]).unwrap();
        writer
            .finish_trial(trial1, TrialState::Complete, &[2.0])
            .unwrap();

        let data = std::fs::read(&path).unwrap();
        // The lightweight study-list scan does not track constraints; the full
        // parse does.
        let (meta, df, _) = parse_single_study(&data, 0).unwrap();
        assert!(meta.has_constraints);
        assert_eq!(df.get_numeric_column("c1"), Some([-0.5, 0.25].as_slice()));
        assert_eq!(df.get_numeric_column("c2"), Some([0.0, -1.0].as_slice()));
        assert_eq!(
            df.get_numeric_column("is_feasible"),
            Some([1.0, 0.0].as_slice())
        );
    }

    /// User-attribute round-trip: op8 written by the writer is read back by the
    /// existing parser into numeric / string attribute columns.
    #[test]
    fn user_attrs_roundtrip_via_existing_parser() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("study.journal");
        let mut writer = JournalWriter::open(&path).unwrap();

        let study_id = writer
            .create_study("attr-study", &[OptimizationDirection::Minimize], &[])
            .unwrap();
        let trial0 = writer.create_trial(study_id).unwrap();
        writer
            .set_trial_user_attr(trial0, "area", &serde_json::json!(12.5))
            .unwrap();
        writer
            .set_trial_user_attr(trial0, "material", &serde_json::json!("steel"))
            .unwrap();
        writer
            .finish_trial(trial0, TrialState::Complete, &[1.0])
            .unwrap();

        let data = std::fs::read(&path).unwrap();
        let (_, df, _) = parse_single_study(&data, 0).unwrap();
        assert_eq!(df.get_numeric_column("area"), Some([12.5].as_slice()));
        assert_eq!(
            df.get_string_column("material").map(<[String]>::to_vec),
            Some(vec!["steel".to_string()])
        );
    }

    /// Append numbering: after closing once and reopening, study_id / trial_id must
    /// continue sequentially from the existing counts.
    #[test]
    fn writer_reopen_continues_numbering() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("study.journal");

        {
            let mut writer = JournalWriter::open(&path).unwrap();
            let study_id = writer
                .create_study("study-a", &[OptimizationDirection::Minimize], &[])
                .unwrap();
            assert_eq!(study_id, 0);
            for _ in 0..3 {
                writer.create_trial(study_id).unwrap();
            }
        }

        {
            let mut writer = JournalWriter::open(&path).unwrap();
            let study_id = writer
                .create_study("study-b", &[OptimizationDirection::Maximize], &[])
                .unwrap();
            assert_eq!(study_id, 1);
            let trial_id = writer.create_trial(study_id).unwrap();
            assert_eq!(trial_id, 3);
        }

        let data = std::fs::read(&path).unwrap();
        let studies = scan_study_list(&data).unwrap();
        assert_eq!(studies.len(), 2);
        assert_eq!(studies[0].name, "study-a");
        assert_eq!(studies[1].name, "study-b");
    }

    /// Double serialization: parse the written op5 line raw with serde_json, verify
    /// `distribution` is a string type, and verify that re-parsing its contents gives
    /// the correct name/attributes.
    #[test]
    fn set_trial_param_double_serializes_distribution() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("study.journal");
        let mut writer = JournalWriter::open(&path).unwrap();

        let study_id = writer
            .create_study("my-study", &[OptimizationDirection::Minimize], &[])
            .unwrap();
        let trial_id = writer.create_trial(study_id).unwrap();
        writer
            .set_trial_param(
                trial_id,
                "span",
                5.5,
                &ParamDistribution::Float {
                    low: 3.0,
                    high: 12.0,
                },
            )
            .unwrap();
        writer
            .set_trial_param(
                trial_id,
                "count",
                3.0,
                &ParamDistribution::Int { low: 1, high: 10 },
            )
            .unwrap();

        let data = std::fs::read_to_string(&path).unwrap();
        let op5_lines: Vec<&str> = data
            .lines()
            .filter(|line| line.starts_with(r#"{"op_code": 5"#))
            .collect();
        assert_eq!(op5_lines.len(), 2);

        let float_line: Value = serde_json::from_str(op5_lines[0]).unwrap();
        let distribution_value = float_line.get("distribution").unwrap();
        assert!(distribution_value.is_string());
        let inner: Value = serde_json::from_str(distribution_value.as_str().unwrap()).unwrap();
        assert_eq!(inner["name"], "FloatDistribution");
        assert_eq!(inner["attributes"]["low"], 3.0);
        assert_eq!(inner["attributes"]["high"], 12.0);
        assert_eq!(inner["attributes"]["log"], false);
        assert!(inner["attributes"]["step"].is_null());

        let int_line: Value = serde_json::from_str(op5_lines[1]).unwrap();
        let distribution_value = int_line.get("distribution").unwrap();
        assert!(distribution_value.is_string());
        let inner: Value = serde_json::from_str(distribution_value.as_str().unwrap()).unwrap();
        assert_eq!(inner["name"], "IntDistribution");
        assert_eq!(inner["attributes"]["low"], 1);
        assert_eq!(inner["attributes"]["high"], 10);
        assert_eq!(inner["attributes"]["step"], 1);
    }

    /// Every record includes worker_id, and each line is terminated by `\n`.
    #[test]
    fn writer_includes_worker_id_and_newline_terminated_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("study.journal");
        let mut writer = JournalWriter::open_with_worker_id(&path, "custom-worker").unwrap();
        assert_eq!(writer.path(), path.as_path());

        let study_id = writer
            .create_study("my-study", &[OptimizationDirection::Minimize], &[])
            .unwrap();
        writer.create_trial(study_id).unwrap();

        let data = std::fs::read_to_string(&path).unwrap();
        assert!(data.ends_with('\n'));
        for line in data.lines() {
            let json: Value = serde_json::from_str(line).unwrap();
            assert_eq!(
                json.get("worker_id").and_then(Value::as_str),
                Some("custom-worker")
            );
            // Optuna's journal layout: op_code is the first key on every line
            // (`{"op_code": N, ...}` with Python json.dumps separators).
            assert!(
                line.starts_with(r#"{"op_code": "#),
                "op_code must lead the line: {line}"
            );
        }
    }
}
