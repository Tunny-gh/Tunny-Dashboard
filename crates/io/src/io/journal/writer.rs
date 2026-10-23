//! Writer that produces an Optuna-compatible journal log (JSON Lines).
//!
//! Appends one record per line in a format that the existing parsers
//! (`super::parser` / `super::live_update`) can read, matching the actual format that
//! Optuna's `JournalStorage` writes (e.g. double-serialization of distributions). Since
//! the live-update poller reads based on byte-offset diffs, a single `write` + `flush`
//! is performed after each line written.
//!
//! Recording is separate from optimization orchestration and analysis.

use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::extras::TrialState;
use crate::io::datetime::format_naive_datetime;
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
    studies: Vec<(String, Vec<OptimizationDirection>)>,
    /// Next global trial ID after validating the existing prefix's references.
    next_trial_id: u32,
    worker_id: String,
}

impl JournalWriter {
    /// Injects a file handle for runner I/O failure tests.
    #[cfg(any(test, feature = "test-support"))]
    pub fn replace_file_for_test(&mut self, file: File) {
        self.file = file;
    }

    /// Opens in append mode after validating the existing prefix's identities.
    /// Rejects malformed records, invalid references and unsupported operations;
    /// it cannot detect valid-looking references to an unintended Study.
    /// Callers must serialize all writers to this path, including open/preparation.
    /// Uses the default worker_id
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

        let identity = read_append_identity(&existing)?;

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|err| format!("failed to open journal file {}: {err}", path.display()))?;

        Ok(JournalWriter {
            file,
            path: path.to_path_buf(),
            studies: identity.studies,
            next_trial_id: identity.next_trial_id,
            worker_id: worker_id.to_string(),
        })
    }

    /// Creates a new Study, rejecting an existing name without writing records.
    /// Writes op0 (and op3 if objective_names is non-empty), and returns study_id.
    pub fn create_study(
        &mut self,
        study_name: &str,
        directions: &[OptimizationDirection],
        objective_names: &[String],
    ) -> Result<u32, String> {
        self.create_study_with_options(study_name, directions, objective_names, false)
    }

    /// Resolves a recording destination, optionally reusing a compatible Study.
    /// With `load_if_exists`, an existing name returns its ID only if the objective
    /// count and ordered directions match. Requested names do not relabel it.
    /// Reuse writes no metadata and does not restore optimizer state.
    pub fn create_study_with_options(
        &mut self,
        study_name: &str,
        directions: &[OptimizationDirection],
        objective_names: &[String],
        load_if_exists: bool,
    ) -> Result<u32, String> {
        if let Some((id, (_, stored))) = self
            .studies
            .iter()
            .enumerate()
            .find(|(_, (name, _))| name == study_name)
        {
            if !load_if_exists {
                return Err(format!(
                    "Study '{study_name}' already exists; enable load_if_exists to add new results"
                ));
            }
            if stored != directions {
                return Err(format!("Study '{study_name}' objective count or ordered directions do not match the requested objectives"));
            }
            return Ok(id as u32);
        }
        if directions.is_empty() {
            return Err("A Study must have at least one objective".into());
        }
        let study_id = u32::try_from(self.studies.len())
            .map_err(|_| "Journal Study identity exhausted".to_string())?;

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
        self.studies
            .push((study_name.to_string(), directions.to_vec()));

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
        if study_id as usize >= self.studies.len() {
            return Err(format!("Unknown Journal Study ID {study_id}"));
        }
        let trial_id = self.next_trial_id;
        let next = trial_id
            .checked_add(1)
            .ok_or("Journal trial identity exhausted")?;

        let record = ordered_object(&[
            ("op_code", j(4)),
            ("worker_id", j(&self.worker_id)),
            ("study_id", j(study_id)),
            ("datetime_start", j(format_naive_datetime(now_unix_secs()))),
        ]);
        self.write_line(&record)?;
        self.next_trial_id = next;

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
        // serde_json serializes non-finite f64 as null, which the parsers then
        // drop — silently shifting later constraints into earlier columns.
        // Reject instead of corrupting the journal.
        if constraints.iter().any(|c| !c.is_finite()) {
            return Err(format!(
                "constraint values must be finite (trial {trial_id}: {constraints:?})"
            ));
        }
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

/// Validate only the replay evidence needed to append without guessing identities.
/// Rejected duplicate creation attempts are normal Optuna Journal records.
fn read_append_identity(data: &[u8]) -> Result<AppendIdentity, String> {
    if !data.is_empty() && !data.ends_with(b"\n") {
        return Err(
            "Cannot append to a Journal without a complete newline-terminated prefix".into(),
        );
    }
    let text = std::str::from_utf8(data).map_err(|e| format!("Invalid Journal text: {e}"))?;
    let mut studies: Vec<(String, Vec<OptimizationDirection>)> = Vec::new();
    let mut names = std::collections::HashSet::new();
    let mut trials = 0_u32;
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let invalid = || {
            format!(
                "Cannot safely append to Journal: invalid identity record at line {}",
                index + 1
            )
        };
        let record = crate::optuna_json::record(line).map_err(|_| invalid())?;
        let json = &record.json;
        let id = |key: &str| json.get(key).and_then(Value::as_u64).ok_or_else(invalid);
        match id("op_code")? {
            0 => {
                let name = json
                    .get("study_name")
                    .and_then(Value::as_str)
                    .ok_or_else(invalid)?;
                let directions = json
                    .get("directions")
                    .and_then(Value::as_array)
                    .ok_or_else(invalid)?;
                if directions.is_empty() {
                    return Err(invalid());
                }
                let directions = directions
                    .iter()
                    .map(|d| match d.as_u64() {
                        Some(1) => Ok(OptimizationDirection::Minimize),
                        Some(2) => Ok(OptimizationDirection::Maximize),
                        _ => Err(invalid()),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if names.insert(name.to_string()) {
                    u32::try_from(studies.len()).map_err(|_| invalid())?;
                    studies.push((name.to_string(), directions));
                }
            }
            1 => {
                return Err(
                    "Cannot append to a Journal containing unsupported Study deletion/recreation"
                        .into(),
                )
            }
            2..=4 => {
                if id("study_id")? >= studies.len() as u64 {
                    return Err(invalid());
                }
                if id("op_code")? == 4 {
                    trials = trials.checked_add(1).ok_or_else(invalid)?;
                }
            }
            5..=9 => {
                if id("trial_id")? >= u64::from(trials) {
                    return Err(invalid());
                }
            }
            _ => {
                return Err(format!(
                    "Cannot append to a Journal containing an unsupported operation at line {}",
                    index + 1
                ))
            }
        }
    }
    Ok(AppendIdentity {
        studies,
        next_trial_id: trials,
    })
}

struct AppendIdentity {
    studies: Vec<(String, Vec<OptimizationDirection>)>,
    next_trial_id: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::journal::parser::scan_study_list;

    fn read(
        bytes: &[u8],
        id: u32,
    ) -> (
        tdashboard_study::StudyMetadata,
        Vec<std::sync::Arc<tdashboard_study::TrialRecord>>,
    ) {
        use tdashboard_study::*;
        let key = StudyKey {
            source: IdentityAllocator::new().unwrap().source().unwrap(),
            stored_id: id,
        };
        let StudyBatch::Replace { metadata, trials } =
            crate::journal::parser::study::read_full(bytes, key).unwrap()
        else {
            panic!("Expected full acquisition")
        };
        (metadata, trials)
    }

    #[test]
    fn duplicate_preparation_requires_explicit_compatible_reuse_without_relabelling() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("run.log");
        let directions = [
            OptimizationDirection::Minimize,
            OptimizationDirection::Maximize,
        ];
        let names = ["first".into(), "second".into()];
        let mut writer = JournalWriter::open(&path).unwrap();
        assert_eq!(writer.create_study("A", &directions, &names).unwrap(), 0);
        let before = std::fs::read(&path).unwrap();
        assert!(writer
            .create_study("A", &directions, &names)
            .unwrap_err()
            .contains("already exists"));
        for requested in [
            vec![OptimizationDirection::Minimize],
            vec![
                OptimizationDirection::Maximize,
                OptimizationDirection::Minimize,
            ],
        ] {
            assert!(writer
                .create_study_with_options("A", &requested, &[], true)
                .unwrap_err()
                .contains("directions"));
        }
        assert_eq!(
            writer
                .create_study_with_options(
                    "A",
                    &directions,
                    &["changed".into(), "labels".into()],
                    true
                )
                .unwrap(),
            0
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        drop(writer);
        let mut writer = JournalWriter::open(&path).unwrap();
        assert!(writer.create_study("A", &directions, &names).is_err());
        assert_eq!(
            writer
                .create_study_with_options("B", &directions, &names, true)
                .unwrap(),
            1
        );
        let studies = scan_study_list(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(studies[0].objective_names, names);
        assert_eq!(studies[1].name, "B");
    }

    #[test]
    fn rejected_optuna_attempts_preserve_study_and_trial_identity_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("run.log");
        let directions = [OptimizationDirection::Minimize];
        {
            let mut writer = JournalWriter::open(&path).unwrap();
            writer
                .create_study("A", &directions, &["original".into()])
                .unwrap();
            for state in [
                TrialState::Running,
                TrialState::Complete,
                TrialState::Pruned,
                TrialState::Fail,
                TrialState::Waiting,
            ] {
                let id = writer.create_trial(0).unwrap();
                writer.finish_trial(id, state, &[10.0]).unwrap();
            }
        }
        let mut prefix = std::fs::read(&path).unwrap();
        // Optuna writes rejected creation attempts, including load_if_exists=True.
        prefix.extend_from_slice(b"{\"op_code\":0,\"worker_id\":\"optuna\",\"study_name\":\"A\",\"directions\":[2]}\n{\"op_code\":2,\"study_id\":0,\"user_attr\":{\"note\":null}}\n{\"op_code\":8,\"trial_id\":0,\"user_attr\":{\"numeric\":NaN}}\n");
        std::fs::write(&path, &prefix).unwrap();
        {
            let mut writer = JournalWriter::open(&path).unwrap();
            assert_eq!(writer.create_study("B", &directions, &[]).unwrap(), 1);
            assert_eq!(writer.create_trial(1).unwrap(), 5);
        }
        let before_reuse = std::fs::read(&path).unwrap();
        let mut writer = JournalWriter::open(&path).unwrap();
        assert_eq!(
            writer
                .create_study_with_options("A", &directions, &["replacement".into()], true)
                .unwrap(),
            0
        );
        assert_eq!(writer.create_trial(0).unwrap(), 6);
        writer
            .finish_trial(6, TrialState::Complete, &[20.0])
            .unwrap();
        assert_eq!(writer.create_study("C", &directions, &[]).unwrap(), 2);
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(&before_reuse));
        let (meta, trials) = read(&bytes, 0);
        assert_eq!(meta.objectives[0].name.as_deref(), Some("original"));
        assert_eq!(
            trials
                .iter()
                .map(|t| (t.key.stored_id, t.number.0))
                .collect::<Vec<_>>(),
            vec![(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (6, 5)]
        );
        assert_eq!(
            trials[1].objectives[&tdashboard_study::ObjectiveIndex(0)].0,
            10.0
        );
        assert_eq!(
            trials[5].objectives[&tdashboard_study::ObjectiveIndex(0)].0,
            20.0
        );
        let (_, other) = read(&bytes, 1);
        assert_eq!((other[0].key.stored_id, other[0].number.0), (5, 0));
    }

    #[test]
    fn unsafe_append_prefixes_are_rejected_without_repair() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("run.log");
        let prefix = "{\"op_code\":0,\"study_name\":\"A\",\"directions\":[1]}\n";
        for suffix in [
            "{\"op_code\":0,\"study_name\":\"A\",\"directions\":[1]}\n{\"op_code\":3,\"study_id\":1}\n",
            "{\"op_code\":4,\"study_id\":1}\n",
            "{\"op_code\":5,\"trial_id\":0}\n",
            "{\"op_code\":6,\"trial_id\":4294967296}\n",
            "{\"op_code\":1,\"study_id\":0}\n{\"op_code\":0,\"study_name\":\"A\",\"directions\":[1]}\n",
            "{\"op_code\":10}\n",
            "{\"op_code\":0,\"study_name\":\"B\",\"directions\":[3]}\n",
            "{\"op_code\":4",
            "not JSON\n",
        ] {
            let bytes = format!("{prefix}{suffix}").into_bytes();
            std::fs::write(&path, &bytes).unwrap();
            assert!(JournalWriter::open(&path).is_err(), "{suffix}");
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
        }
        let mut writer = JournalWriter::open(&dir.path().join("new.log")).unwrap();
        assert!(writer.create_trial(0).is_err());
        assert!(std::fs::read(writer.path()).unwrap().is_empty());
    }

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
        let (meta, trials) = read(&data, 0);
        use tdashboard_study::{Number, ParameterValue, TrialState as DomainState};
        assert_eq!(
            meta.declared_bounds.get("span"),
            Some(&(Number(3.0), Number(12.0)))
        );
        assert_eq!(
            meta.declared_bounds.get("count"),
            Some(&(Number(1.0), Number(10.0)))
        );
        let complete: Vec<_> = trials
            .iter()
            .filter(|t| t.state == DomainState::Complete)
            .collect();
        assert_eq!(complete.len(), 2);
        assert_eq!(complete[0].key.stored_id, 0);
        assert_eq!(complete[1].key.stored_id, 2);
        assert_eq!(
            complete[0]
                .objectives
                .values()
                .map(|v| v.0)
                .collect::<Vec<_>>(),
            vec![12.3, 4.5]
        );
        assert_eq!(
            complete[1]
                .objectives
                .values()
                .map(|v| v.0)
                .collect::<Vec<_>>(),
            vec![10.0, 6.0]
        );
        for (name, observed) in [("span", 5.5), ("count", 3.0)] {
            assert_eq!(
                complete[0].parameters[name].value,
                ParameterValue::Numeric(Number(observed))
            );
            assert!(!complete[1].parameters.contains_key(name));
        }

        assert_eq!(trials.len(), 3);
        for (id, state) in [
            (0, DomainState::Complete),
            (1, DomainState::Fail),
            (2, DomainState::Complete),
        ] {
            assert_eq!(trials[id].key.stored_id, id as u32);
            assert_eq!(trials[id].state, state);
            assert!(trials[id].datetime_start.is_some());
            assert!(trials[id].datetime_complete.is_some());
        }
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
        let (_, trials) = read(&data, 0);
        assert!(trials.iter().all(|t| t.constraints.present));
        assert_eq!(
            trials[0]
                .constraints
                .values
                .iter()
                .map(|n| n.0)
                .collect::<Vec<_>>(),
            vec![-0.5, 0.0]
        );
        assert_eq!(
            trials[1]
                .constraints
                .values
                .iter()
                .map(|n| n.0)
                .collect::<Vec<_>>(),
            vec![0.25, -1.0]
        );
    }

    /// Non-finite constraint values are rejected instead of being serialized as
    /// null (which the parsers would silently drop, shifting columns).
    #[test]
    fn non_finite_constraints_are_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("study.journal");
        let mut writer = JournalWriter::open(&path).unwrap();
        let study_id = writer
            .create_study("s", &[OptimizationDirection::Minimize], &[])
            .unwrap();
        let trial = writer.create_trial(study_id).unwrap();
        assert!(writer
            .set_trial_constraints(trial, &[f64::NAN, 5.0])
            .is_err());
        assert!(writer
            .set_trial_constraints(trial, &[f64::INFINITY])
            .is_err());
        assert!(writer.set_trial_constraints(trial, &[-0.5]).is_ok());
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
        let (_, trials) = read(&data, 0);
        assert_eq!(
            trials[0].user_attributes["area"].json_view(),
            serde_json::json!(12.5)
        );
        assert_eq!(
            trials[0].user_attributes["material"].json_view(),
            serde_json::json!("steel")
        );
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
