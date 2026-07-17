//! Pure data-shaping helpers.
//!
//! Groups together egui-independent pure functions for trial_id filtering,
//! pagination, and matching cluster/MCDM results against artifacts (kept
//! separate from widget state to make them easy to test).

use std::collections::{BTreeMap, HashMap};

use crate::io::artifacts::ArtifactEntry;
use crate::state::app_state::AppState;
use crate::state::results::{ClusterResult, McdmResult};
use crate::theme::colormap::ColorMap;

/// Returns a map of each trial's objective values, formatted as `name: value` pairs joined by newlines.
pub(super) fn build_objective_labels(app_state: &AppState) -> HashMap<u32, String> {
    let mut out: HashMap<u32, String> = HashMap::new();
    let Some(ctx) = app_state.current_study.as_ref() else {
        return out;
    };
    let obj_names = &ctx.meta.objective_names;
    if obj_names.is_empty() {
        return out;
    }
    let view = &ctx.view;
    let cols = view.numeric_columns(obj_names);
    // Only trials with an artifact can be displayed. Limit the string formatting to those.
    let artifact_map = &app_state.artifact_map;
    for (idx, &trial_id) in view.trial_ids.iter().enumerate() {
        if !artifact_map.contains_key(&trial_id) {
            continue;
        }
        let text = obj_names
            .iter()
            .zip(cols.iter())
            .map(|(name, col)| {
                let v = col.and_then(|c| c.get(idx)).copied().unwrap_or(f64::NAN);
                format!("{name}: {v:.4}")
            })
            .collect::<Vec<_>>()
            .join("\n");
        out.insert(trial_id, text);
    }
    out
}

/// Derives a color from a cluster label (same rule as ClusterTable).
pub(super) fn cluster_color(label: i32, n_clusters: usize, colormap: &ColorMap) -> egui::Color32 {
    if label < 0 {
        return crate::theme::TEXT_SECONDARY();
    }
    let t = if n_clusters <= 1 {
        0.5
    } else {
        label as f32 / (n_clusters - 1) as f32
    };
    colormap.interpolate(t)
}

/// Returns, in ascending order, the trial_ids that have an artifact at the given index.
pub(super) fn artifact_trials_with_index(
    artifact_map: &std::collections::HashMap<u32, Vec<ArtifactEntry>>,
    index: usize,
) -> Vec<u32> {
    let mut ids: Vec<u32> = artifact_map
        .iter()
        .filter(|(_, entries)| entries.len() > index)
        .map(|(&id, _)| id)
        .collect();
    ids.sort_unstable();
    ids
}

/// Restricts `ids` to only the trial_ids that belong to the current Study.
/// `artifact_map` contains trials from the entire Journal (all Studies), so
/// only ones present in the target Study's `view.trial_ids` are kept.
/// Returns empty when no Study is selected.
pub(super) fn restrict_to_current_study(ids: Vec<u32>, app_state: &AppState) -> Vec<u32> {
    let Some(ctx) = app_state.current_study.as_ref() else {
        return Vec::new();
    };
    let set: std::collections::HashSet<u32> = ctx.view.trial_ids.iter().copied().collect();
    ids.into_iter().filter(|id| set.contains(id)).collect()
}

/// Filters the trial_id list based on a selection filter (PCP brush, etc.).
/// Returns all items when `selected_indices` is empty (same "empty = all" convention as tables, etc.).
pub(super) fn filter_ids_by_selection(ids: Vec<u32>, selected_indices: &[u32]) -> Vec<u32> {
    if selected_indices.is_empty() {
        return ids;
    }
    let set: std::collections::HashSet<u32> = selected_indices.iter().copied().collect();
    ids.into_iter().filter(|id| set.contains(id)).collect()
}

/// Returns the slice of `items` for `page` (0-indexed, `per_page` items per page).
pub(super) fn paginate<T>(items: &[T], page: usize, per_page: usize) -> &[T] {
    if per_page == 0 || items.is_empty() {
        return &[];
    }
    let start = page.saturating_mul(per_page).min(items.len());
    let end = (start + per_page).min(items.len());
    &items[start..end]
}

/// Groups artifacts by cluster.
/// Returns (label, [(trial_id, &paths)]) pairs ordered by ascending label
/// (unclustered -1 goes last).
/// Trials without an artifact are excluded.
/// Trials without an artifact at `artifact_index` are excluded.
#[allow(clippy::type_complexity)]
pub(super) fn cluster_sections<'a>(
    cluster_result: &ClusterResult,
    trial_ids: &[u32],
    artifact_map: &'a std::collections::HashMap<u32, Vec<ArtifactEntry>>,
    artifact_index: usize,
) -> Vec<(i32, Vec<(u32, &'a ArtifactEntry)>)> {
    let mut by_label: BTreeMap<i32, Vec<(u32, &ArtifactEntry)>> = BTreeMap::new();
    for (idx, &label) in cluster_result.labels.iter().enumerate() {
        let Some(&trial_id) = trial_ids.get(idx) else {
            continue;
        };
        let Some(entry) = artifact_map
            .get(&trial_id)
            .and_then(|entries| entries.get(artifact_index))
        else {
            continue;
        };
        by_label.entry(label).or_default().push((trial_id, entry));
    }
    // BTreeMap iterates in ascending order. Move unclustered (-1) to the end.
    let mut sections: Vec<(i32, Vec<(u32, &ArtifactEntry)>)> = Vec::new();
    let mut unclustered: Option<(i32, Vec<(u32, &ArtifactEntry)>)> = None;
    for (label, members) in by_label {
        if label < 0 {
            unclustered = Some((label, members));
        } else {
            sections.push((label, members));
        }
    }
    if let Some(u) = unclustered {
        sections.push(u);
    }
    sections
}

/// An entry in MCDM ranking order.
pub(super) struct McdmArtifactEntry<'a> {
    pub rank: usize,
    pub score: f64,
    pub trial_id: u32,
    pub entry: &'a ArtifactEntry,
}

/// Orders the MCDM result by rank and returns up to `top_n` trials that have
/// an artifact at `artifact_index`.
pub(super) fn mcdm_ordered<'a>(
    result: &McdmResult,
    trial_ids: &[u32],
    artifact_map: &'a std::collections::HashMap<u32, Vec<ArtifactEntry>>,
    artifact_index: usize,
    top_n: usize,
) -> Vec<McdmArtifactEntry<'a>> {
    let scores = result.primary_scores();
    let ranked = result.ranked_indices();
    let mut out: Vec<McdmArtifactEntry<'a>> = Vec::new();
    for (rank0, &row_idx) in ranked.iter().enumerate() {
        let idx = row_idx as usize;
        let Some(&trial_id) = trial_ids.get(idx) else {
            continue;
        };
        let Some(entry) = artifact_map
            .get(&trial_id)
            .and_then(|entries| entries.get(artifact_index))
        else {
            continue;
        };
        out.push(McdmArtifactEntry {
            rank: rank0 + 1,
            score: scores.get(idx).copied().unwrap_or(0.0),
            trial_id,
            entry,
        });
        if out.len() >= top_n {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::results::TopsisResult;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn entry(name: &str) -> ArtifactEntry {
        ArtifactEntry {
            path: PathBuf::from(name),
            filename: format!("{name}.png"),
            mimetype: "image/png".into(),
        }
    }

    fn map_with(ids: &[u32]) -> HashMap<u32, Vec<ArtifactEntry>> {
        ids.iter()
            .map(|&id| (id, vec![entry(&format!("{id}"))]))
            .collect()
    }

    #[test]
    fn artifact_trials_with_index_filters_and_sorts() {
        let mut m = map_with(&[5, 2, 9]);
        m.insert(3, vec![]); // empty is excluded
        assert_eq!(artifact_trials_with_index(&m, 0), vec![2, 5, 9]);
        // Only trials with index 1.
        m.insert(7, vec![entry("a"), entry("b")]);
        assert_eq!(artifact_trials_with_index(&m, 1), vec![7]);
    }

    fn study_ctx_with_trial_ids(ids: &[u32]) -> crate::state::types::StudyContext {
        use crate::state::types::{StudyContext, StudyMeta, TrialRow as UiRow};
        let rows: Vec<UiRow> = ids
            .iter()
            .enumerate()
            .map(|(i, &id)| UiRow {
                trial_id: id,
                trial_number: i as u32,
                params: HashMap::new(),
                objectives: vec![],
                pareto_rank: 0,
                cluster_id: None,
                user_attrs: HashMap::new(),
            })
            .collect();
        let meta = StudyMeta {
            study_id: 0,
            name: "test".to_string(),
            directions: vec![],
            completed_trials: ids.len(),
            param_names: vec![],
            objective_names: vec![],
            param_bounds: Default::default(),
        };
        StudyContext::from_rows_for_test(meta, rows)
    }

    #[test]
    fn restrict_to_current_study_keeps_only_study_trials() {
        // artifact_map contains the entire Journal (study A: 0,1 / study B: 100,101).
        let mut state = AppState::new();
        state.artifact_map = map_with(&[0, 1, 100, 101]);
        // The current Study has only trials 0,1.
        state.current_study = Some(study_ctx_with_trial_ids(&[0, 1]));

        let ids = artifact_trials_with_index(&state.artifact_map, 0);
        assert_eq!(restrict_to_current_study(ids, &state), vec![0, 1]);
    }

    #[test]
    fn restrict_to_current_study_empty_without_study() {
        let mut state = AppState::new();
        state.artifact_map = map_with(&[0, 1]);
        let ids = artifact_trials_with_index(&state.artifact_map, 0);
        assert!(restrict_to_current_study(ids, &state).is_empty());
    }

    #[test]
    fn filter_ids_by_selection_empty_returns_all() {
        let ids = vec![2u32, 5, 9];
        assert_eq!(filter_ids_by_selection(ids.clone(), &[]), ids);
    }

    #[test]
    fn filter_ids_by_selection_keeps_only_selected() {
        let ids = vec![2u32, 5, 9, 11];
        assert_eq!(filter_ids_by_selection(ids, &[5, 11, 99]), vec![5, 11]);
    }

    #[test]
    fn paginate_clamps_range() {
        let v = vec![0, 1, 2, 3, 4];
        assert_eq!(paginate(&v, 0, 2), &[0, 1]);
        assert_eq!(paginate(&v, 2, 2), &[4]);
        assert_eq!(paginate(&v, 9, 2), &[] as &[i32]);
        assert_eq!(paginate(&v, 0, 0), &[] as &[i32]);
    }

    #[test]
    fn cluster_sections_groups_and_puts_unclustered_last() {
        // Make row index and trial_id different values to verify the conversion.
        let trial_ids = vec![10, 11, 12, 13];
        let cr = ClusterResult {
            labels: vec![1, 0, -1, 0],
            n_clusters: 2,
        };
        let m = map_with(&[10, 11, 12, 13]);
        let sections = cluster_sections(&cr, &trial_ids, &m, 0);
        let labels: Vec<i32> = sections.iter().map(|(l, _)| *l).collect();
        assert_eq!(labels, vec![0, 1, -1]); // unclustered last
                                            // cluster 0 is trials 11, 13
        let c0: Vec<u32> = sections[0].1.iter().map(|(t, _)| *t).collect();
        assert_eq!(c0, vec![11, 13]);
    }

    #[test]
    fn cluster_sections_excludes_trials_without_artifacts() {
        let trial_ids = vec![10, 11, 12];
        let cr = ClusterResult {
            labels: vec![0, 0, 1],
            n_clusters: 2,
        };
        let m = map_with(&[10]); // 11, 12 have no artifact
        let sections = cluster_sections(&cr, &trial_ids, &m, 0);
        let total: usize = sections.iter().map(|(_, v)| v.len()).sum();
        assert_eq!(total, 1);
    }

    #[test]
    fn cluster_sections_selects_requested_artifact_index() {
        let trial_ids = vec![10, 11];
        let cr = ClusterResult {
            labels: vec![0, 0],
            n_clusters: 1,
        };
        let mut m = HashMap::new();
        m.insert(10, vec![entry("a"), entry("b")]); // has index 1
        m.insert(11, vec![entry("c")]); // no index 1 -> excluded
        let sections = cluster_sections(&cr, &trial_ids, &m, 1);
        let members = &sections[0].1;
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].0, 10);
        assert_eq!(members[0].1.filename, "b.png"); // the second one selected
    }

    #[test]
    fn mcdm_ordered_respects_rank_and_topn() {
        let trial_ids = vec![10, 11, 12];
        // ranked_indices are row indices. Scores are keyed by row index.
        let result = McdmResult::Topsis(TopsisResult {
            scores: vec![0.1, 0.9, 0.5],
            ranked_indices: vec![1, 2, 0],
            duration_ms: 0.0,
        });
        let m = map_with(&[10, 11, 12]);
        let ordered = mcdm_ordered(&result, &trial_ids, &m, 0, 2);
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].rank, 1);
        assert_eq!(ordered[0].trial_id, 11); // row index 1 -> trial 11
        assert!((ordered[0].score - 0.9).abs() < 1e-9);
        assert_eq!(ordered[1].trial_id, 12);
    }

    #[test]
    fn mcdm_ordered_skips_trials_without_artifacts() {
        let trial_ids = vec![10, 11, 12];
        let result = McdmResult::Topsis(TopsisResult {
            scores: vec![0.1, 0.9, 0.5],
            ranked_indices: vec![1, 2, 0],
            duration_ms: 0.0,
        });
        let m = map_with(&[12]); // row index 2 -> only trial 12
        let ordered = mcdm_ordered(&result, &trial_ids, &m, 0, 10);
        assert_eq!(ordered.len(), 1);
        assert_eq!(ordered[0].trial_id, 12);
        assert_eq!(ordered[0].rank, 2); // overall rank is 2nd
    }
}
