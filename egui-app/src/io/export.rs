use crate::state::app_state::TrialRow;

/// CSVエクスポートの対象
#[derive(Debug, Clone, PartialEq)]
pub enum ExportTarget {
    AllData,
    SelectedOnly,
    ParetoOnly,
}

/// エクスポート対象の TrialRow をフィルタリングして返す
pub fn select_rows_for_export<'a>(
    trial_rows: &'a [TrialRow],
    selected_indices: &[u32],
    pareto_indices: &[u32],
    target: &ExportTarget,
) -> Vec<&'a TrialRow> {
    match target {
        ExportTarget::AllData => trial_rows.iter().collect(),
        ExportTarget::SelectedOnly => trial_rows
            .iter()
            .filter(|r| selected_indices.contains(&r.trial_id))
            .collect(),
        ExportTarget::ParetoOnly => trial_rows
            .iter()
            .filter(|r| pareto_indices.contains(&r.trial_id))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::{TrialState};
    use std::collections::HashMap;

    fn make_trial(id: u32) -> TrialRow {
        TrialRow {
            trial_id: id,
            params: HashMap::new(),
            objectives: vec![],
            pareto_rank: 0,
            cluster_id: None,
            state: TrialState::Complete,
            user_attrs: HashMap::new(),
        }
    }

    #[test]
    fn all_data_returns_all_rows() {
        let rows = vec![make_trial(0), make_trial(1), make_trial(2)];
        let result = select_rows_for_export(&rows, &[], &[], &ExportTarget::AllData);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn selected_only_filters_by_indices() {
        let rows = vec![make_trial(0), make_trial(1), make_trial(2)];
        let result = select_rows_for_export(&rows, &[0, 2], &[], &ExportTarget::SelectedOnly);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].trial_id, 0);
        assert_eq!(result[1].trial_id, 2);
    }

    #[test]
    fn pareto_only_filters_by_pareto_indices() {
        let rows = vec![make_trial(0), make_trial(1), make_trial(2)];
        let result = select_rows_for_export(&rows, &[], &[1], &ExportTarget::ParetoOnly);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].trial_id, 1);
    }

    #[test]
    fn selected_only_empty_selection_returns_none() {
        let rows = vec![make_trial(0), make_trial(1)];
        let result = select_rows_for_export(&rows, &[], &[], &ExportTarget::SelectedOnly);
        assert_eq!(result.len(), 0);
    }
}
