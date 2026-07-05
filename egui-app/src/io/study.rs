use crate::state::app_state::{Direction, StudyMeta};
use crate::state::messages::AppMessage;

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
        Ok(()) => {
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
            param_names: vec!["x".to_string()],
            objective_names: vec!["y".to_string()],
            param_bounds: Default::default(),
        }
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
