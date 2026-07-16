//! Optuna 互換の journal ログ（JSON Lines）を書き出す writer。
//!
//! 既存パーサ（`super::parser` / `super::live_update`）が読める形式で、かつ
//! 実際の Optuna の `JournalStorage` が書く形式（distribution の二重シリアライズ等）に
//! 合わせて 1 レコード 1 行を追記する。ライブ更新ポーラーがバイトオフセット差分で
//! 読むことを前提に、1 行書くごとに単一の `write` + `flush` を行う。
//!
//! Reference: 仕様書「Optuna 互換 journal writer」（rust_core/src/io/journal/writer.rs）

use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::data::extras::TrialState;
use crate::io::datetime::format_naive_datetime;
use crate::io::journal::live_update::count_created_trials;
use crate::io::journal::parser::OptimizationDirection;

/// worker_id の既定値（Optuna 本家リーダとの互換のために全レコードへ含める）。
const DEFAULT_WORKER_ID: &str = "tunny-dashboard";

/// param の探索範囲指定（journal の distribution として書かれる）。
#[derive(Debug, Clone)]
pub enum ParamDistribution {
    Float { low: f64, high: f64 },
    Int { low: i64, high: i64 },
}

/// Optuna 互換 journal ログの writer。
///
/// 1 レコード 1 行の JSON Lines を追記し、行ごとに flush する。
/// スレッド安全ではない（呼び出し側が Mutex 等で直列化する）。
pub struct JournalWriter {
    file: File,
    path: PathBuf,
    /// 次に採番する study_id（ファイル内の既存 op0 数 + 自分が書いた op0 数）。
    next_study_id: u32,
    /// 次に採番するグローバル trial_id（ファイル内の既存 op4 数 + 自分が書いた op4 数）。
    next_trial_id: u32,
    worker_id: String,
}

impl JournalWriter {
    /// ファイルを追記モードで開く（無ければ作成）。既存内容をスキャンして
    /// 次の study_id / trial_id を採番する。worker_id は既定値
    /// (`"tunny-dashboard"`) を使う。
    pub fn open(path: &Path) -> Result<Self, String> {
        Self::open_with_worker_id(path, DEFAULT_WORKER_ID)
    }

    /// worker_id を指定して開く。それ以外の挙動は [`JournalWriter::open`] と同じ。
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

    /// op0（と objective_names が非空なら op3）を書き、study_id を返す。
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

        let record = serde_json::json!({
            "op_code": 0,
            "worker_id": self.worker_id,
            "study_name": study_name,
            "directions": directions_json,
        });
        self.write_line(&record)?;

        if !objective_names.is_empty() {
            let attr_record = serde_json::json!({
                "op_code": 3,
                "worker_id": self.worker_id,
                "study_id": study_id,
                "system_attr": { "study:metric_names": objective_names },
            });
            self.write_line(&attr_record)?;
        }

        Ok(study_id)
    }

    /// op4 を書き、グローバル trial_id を返す。datetime_start は現在時刻。
    ///
    /// `distributions` フィールドは含めない（ファイルストレージ形式 = RUNNING 状態として
    /// 登録させ、以降の op5/op6 を待たせるため）。
    pub fn create_trial(&mut self, study_id: u32) -> Result<u32, String> {
        let trial_id = self.next_trial_id;
        self.next_trial_id += 1;

        let record = serde_json::json!({
            "op_code": 4,
            "worker_id": self.worker_id,
            "study_id": study_id,
            "datetime_start": format_naive_datetime(now_unix_secs()),
        });
        self.write_line(&record)?;

        Ok(trial_id)
    }

    /// op5 を書く。value は実値（Float はそのまま、Int も実値を f64 で）。
    ///
    /// `distribution` は実際の Optuna が書く形式に合わせ、JSON 文字列として
    /// 二重シリアライズする（`serde_json::Value::String` 経由の機械的な変換）。
    pub fn set_trial_param(
        &mut self,
        trial_id: u32,
        param_name: &str,
        value: f64,
        distribution: &ParamDistribution,
    ) -> Result<(), String> {
        let inner = match distribution {
            ParamDistribution::Float { low, high } => serde_json::json!({
                "name": "FloatDistribution",
                "attributes": {
                    "log": false,
                    "low": low,
                    "high": high,
                    "step": Value::Null,
                }
            }),
            ParamDistribution::Int { low, high } => serde_json::json!({
                "name": "IntDistribution",
                "attributes": {
                    "log": false,
                    "low": low,
                    "high": high,
                    "step": 1,
                }
            }),
        };
        // 二重シリアライズ: distribution オブジェクトを一度文字列化してから
        // JSON 文字列値として埋め込む（手組みのエスケープはしない）。
        let distribution_str = Value::String(inner.to_string());

        let record = serde_json::json!({
            "op_code": 5,
            "worker_id": self.worker_id,
            "trial_id": trial_id,
            "param_name": param_name,
            "param_value_internal": value,
            "distribution": distribution_str,
        });
        self.write_line(&record)
    }

    /// op6 を書く。state が Complete のとき values を配列で、そうでなければ null を書く。
    /// datetime_complete は現在時刻。
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

        let record = serde_json::json!({
            "op_code": 6,
            "worker_id": self.worker_id,
            "trial_id": trial_id,
            "state": state_code,
            "values": values_json,
            "datetime_complete": format_naive_datetime(now_unix_secs()),
        });
        self.write_line(&record)
    }

    /// journal ファイルのパス。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 1 レコードを 1 行の JSON として書き、単一の `write` + `flush` を行う。
    fn write_line(&mut self, value: &Value) -> Result<(), String> {
        let mut line = serde_json::to_string(value)
            .map_err(|err| format!("failed to serialize journal record: {err}"))?;
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

/// 現在時刻を unix 秒（f64）で返す。取得に失敗した場合は 0.0（panic はしない）。
fn now_unix_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or(0.0)
}

/// 既存ファイル内容から op_code=0（CREATE_STUDY）の行数を数える。
/// パースできない行・末尾の不完全行は無視する。
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

    /// ラウンドトリップ（最重要）: writer で書いた journal を既存パーサで読み、
    /// study 情報・DataFrame・StudyExtras が期待どおりになることを検証する。
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

        // trial1: 同 param 構成 -> Fail（values なし）
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

        // study 一覧・directions・objective_names。
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

        // 1 study 分の詳細解析。
        let (meta, df, extras) = parse_single_study(&data, 0).unwrap();
        assert_eq!(meta.completed_trials, 2);
        assert_eq!(meta.total_trials, 3);
        assert_eq!(meta.param_bounds.get("span"), Some(&(3.0, 12.0)));
        assert_eq!(meta.param_bounds.get("count"), Some(&(1.0, 10.0)));

        // DataFrame: COMPLETE のみ 2 行（trial0, trial2）。
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

        // StudyExtras: trial 3 件、state が Complete/Fail/Complete、日時が Some。
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

    /// 追記の採番: 一度閉じて再 open すると、study_id / trial_id が
    /// 既存件数から連番で続くこと。
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

    /// 二重シリアライズ: 書いた op5 行を素で serde_json パースし、`distribution` が
    /// 文字列型で、その中身を再パースすると name/attributes が正しいこと。
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
            .filter(|line| line.contains(r#""op_code":5"#))
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

    /// 全レコードに worker_id が含まれ、1 行ごとに `\n` 終端されていること。
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
        }
    }
}
