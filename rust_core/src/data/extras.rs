//! COMPLETE 限定の `DataFrame` と並走する、全 trial（全 state）の付帯情報。
//!
//! `DataFrame` は COMPLETE trial のみを列指向で保持し全解析の基盤となるが、
//! 中間値（intermediate values）・trial state・開始/完了日時は全 trial 分が必要になる
//! （学習曲線・タイムライン・進捗表示など）。これらを `DataFrame` とは別の
//! per-study 構造 `StudyExtras` として保持し、共有ストアに study_id ごとに格納する。

/// Optuna trial state（journal の数値表現 / SQLite の文字列表現の両方から変換）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrialState {
    Running,
    Complete,
    Pruned,
    Fail,
    Waiting,
}

impl TrialState {
    /// journal ストレージの数値 state から変換する。
    /// 0=Running, 1=Complete, 2=Pruned, 3=Fail, 4=Waiting。未知値は Running とみなす。
    pub fn from_journal(value: u8) -> Self {
        match value {
            1 => TrialState::Complete,
            2 => TrialState::Pruned,
            3 => TrialState::Fail,
            4 => TrialState::Waiting,
            _ => TrialState::Running,
        }
    }

    /// SQLite (RDBStorage) の文字列 state から変換する。
    /// 未知値は Running とみなす。
    pub fn from_rdb_str(value: &str) -> Self {
        match value {
            "COMPLETE" => TrialState::Complete,
            "PRUNED" => TrialState::Pruned,
            "FAIL" => TrialState::Fail,
            "WAITING" => TrialState::Waiting,
            _ => TrialState::Running,
        }
    }

    /// Optuna の大文字表記の state 名を返す。
    pub fn label(&self) -> &'static str {
        match self {
            TrialState::Running => "RUNNING",
            TrialState::Complete => "COMPLETE",
            TrialState::Pruned => "PRUNED",
            TrialState::Fail => "FAIL",
            TrialState::Waiting => "WAITING",
        }
    }
}

/// 単一 trial の付帯情報（state / 日時 / 中間値）。
#[derive(Debug, Clone)]
pub struct TrialExtra {
    /// ストレージ横断のグローバル trial_id（op_code=4 出現順）。
    pub trial_id: u32,
    /// Study 内 0 始まりの trial.number（作成順）。
    pub trial_number: u32,
    pub state: TrialState,
    /// 開始日時。unix 秒（naive、タイムゾーン変換なし）。
    pub datetime_start: Option<f64>,
    /// 完了日時。unix 秒（naive、タイムゾーン変換なし）。
    pub datetime_complete: Option<f64>,
    /// 中間値 `(step, value)`。step 昇順。
    pub intermediate_values: Vec<(u64, f64)>,
}

/// Study 内全 trial（全 state）の付帯情報。trial_id 昇順。
#[derive(Debug, Clone, Default)]
pub struct StudyExtras {
    pub trials: Vec<TrialExtra>,
}

impl StudyExtras {
    /// いずれかの trial が中間値を持つか。
    pub fn has_intermediate(&self) -> bool {
        self.trials
            .iter()
            .any(|t| !t.intermediate_values.is_empty())
    }

    /// いずれかの trial が開始/完了日時を持つか。
    pub fn has_datetimes(&self) -> bool {
        self.trials
            .iter()
            .any(|t| t.datetime_start.is_some() || t.datetime_complete.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trial_state_from_journal_maps_known_and_unknown() {
        assert_eq!(TrialState::from_journal(0), TrialState::Running);
        assert_eq!(TrialState::from_journal(1), TrialState::Complete);
        assert_eq!(TrialState::from_journal(2), TrialState::Pruned);
        assert_eq!(TrialState::from_journal(3), TrialState::Fail);
        assert_eq!(TrialState::from_journal(4), TrialState::Waiting);
        assert_eq!(TrialState::from_journal(99), TrialState::Running);
    }

    #[test]
    fn trial_state_from_rdb_str_maps_known_and_unknown() {
        assert_eq!(TrialState::from_rdb_str("RUNNING"), TrialState::Running);
        assert_eq!(TrialState::from_rdb_str("COMPLETE"), TrialState::Complete);
        assert_eq!(TrialState::from_rdb_str("PRUNED"), TrialState::Pruned);
        assert_eq!(TrialState::from_rdb_str("FAIL"), TrialState::Fail);
        assert_eq!(TrialState::from_rdb_str("WAITING"), TrialState::Waiting);
        assert_eq!(TrialState::from_rdb_str("???"), TrialState::Running);
    }

    #[test]
    fn trial_state_label_is_uppercase_optuna_name() {
        assert_eq!(TrialState::Running.label(), "RUNNING");
        assert_eq!(TrialState::Complete.label(), "COMPLETE");
        assert_eq!(TrialState::Pruned.label(), "PRUNED");
        assert_eq!(TrialState::Fail.label(), "FAIL");
        assert_eq!(TrialState::Waiting.label(), "WAITING");
    }

    #[test]
    fn study_extras_helpers_detect_presence() {
        let mut extras = StudyExtras::default();
        assert!(!extras.has_intermediate());
        assert!(!extras.has_datetimes());

        extras.trials.push(TrialExtra {
            trial_id: 0,
            trial_number: 0,
            state: TrialState::Complete,
            datetime_start: Some(1.0),
            datetime_complete: None,
            intermediate_values: vec![],
        });
        assert!(!extras.has_intermediate());
        assert!(extras.has_datetimes());

        extras.trials.push(TrialExtra {
            trial_id: 1,
            trial_number: 1,
            state: TrialState::Running,
            datetime_start: None,
            datetime_complete: None,
            intermediate_values: vec![(0, 0.5)],
        });
        assert!(extras.has_intermediate());
    }
}
