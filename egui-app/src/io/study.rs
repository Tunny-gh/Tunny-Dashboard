use crate::state::app_state::{Direction, StudyMeta};
use crate::state::messages::AppMessage;

/// 完了試行数が最多の Study を自動選択する（REQ-021 準拠）
pub fn auto_select_study(studies: &[StudyMeta]) -> Option<&StudyMeta> {
    studies.iter().max_by_key(|s| s.completed_trials)
}

/// バックグラウンドで select_study を実行し AppMessage を返す。
///
/// 行指向 `Vec<TrialRow>` は複製せず、`study_id` と Pareto ランクのみを送る（MEM-001）。
/// UI 側は `StudySelected` 受信時に共有ストアから `Arc<DataFrame>` を取得し
/// `StudyView` を構築する。
pub fn select_study_task(meta: StudyMeta) -> AppMessage {
    let is_minimize: Vec<bool> = meta
        .directions
        .iter()
        .map(|d| matches!(d, Direction::Minimize))
        .collect();

    let study_id = meta.study_id;
    match tunny_core::dataframe::select_study(study_id) {
        Ok(_result) => {
            let pareto = tunny_core::pareto::compute_pareto_ranks(&is_minimize);
            let pareto_indices = pareto.pareto_indices;
            AppMessage::StudySelected {
                meta,
                study_id,
                pareto_rank: pareto.ranks,
                pareto_indices,
            }
        }
        Err(e) => AppMessage::Error(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::Direction;

    fn make_study(id: u32, name: &str, completed: usize) -> StudyMeta {
        StudyMeta {
            study_id: id,
            name: name.to_string(),
            directions: vec![Direction::Minimize],
            completed_trials: completed,
            total_trials: completed,
            param_names: vec!["x".to_string()],
            objective_names: vec!["y".to_string()],
            user_attr_names: vec![],
            has_constraints: false,
            param_bounds: Default::default(),
        }
    }

    #[test]
    fn auto_select_study_empty() {
        let studies: Vec<StudyMeta> = vec![];
        assert!(auto_select_study(&studies).is_none());
    }

    #[test]
    fn auto_select_study_picks_most_completed() {
        let studies = vec![
            make_study(0, "a", 5),
            make_study(1, "b", 100),
            make_study(2, "c", 10),
        ];
        let selected = auto_select_study(&studies).unwrap();
        assert_eq!(selected.study_id, 1);
        assert_eq!(selected.name, "b");
    }

    #[test]
    fn auto_select_study_single() {
        let studies = vec![make_study(0, "only", 3)];
        let selected = auto_select_study(&studies).unwrap();
        assert_eq!(selected.study_id, 0);
    }

    #[test]
    fn select_study_task_invalid_id_returns_error() {
        let meta = make_study(999, "nonexistent", 0);
        let msg = select_study_task(meta);
        match msg {
            AppMessage::Error(_) => {}
            _ => panic!("Expected Error for nonexistent study"),
        }
    }
}
