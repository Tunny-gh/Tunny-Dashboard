use crate::state::app_state::{AppState, McdmResult};
use crate::state::layout_state::ChartId;
use crate::state::types::Direction;
use crate::ui::widget_states::WidgetStates;

pub fn build_chart_csv(
    chart_id: &ChartId,
    app_state: &AppState,
    widgets: &WidgetStates,
) -> Option<String> {
    match chart_id {
        ChartId::OptimizationHistory => build_optimization_history_csv(app_state, widgets),
        ChartId::HvHistory => build_hv_history_csv(app_state),
        ChartId::ImportanceChart => build_importance_csv(app_state, widgets),
        ChartId::PdpChart => build_pdp_csv(app_state, widgets),
        ChartId::PdpChart2D => build_pdp_2d_csv(app_state, widgets),
        ChartId::ParallelCoordinates => build_trial_based_csv(app_state),
        ChartId::ScatterMatrix => build_trial_based_csv(app_state),
        ChartId::ClusterScatter => build_cluster_csv(app_state),
        ChartId::SensitivityHeatmap => build_sensitivity_csv(app_state, widgets),
        ChartId::ParetoScatter2D => build_pareto_csv(app_state),
        ChartId::ParetoScatter3D => build_pareto_csv(app_state),
        ChartId::McdmRankChart => build_mcdm_rank_csv(app_state),
        ChartId::McdmScatterChart => build_mcdm_scatter_csv(app_state),
        ChartId::McdmTable => build_mcdm_table_csv(app_state),
        ChartId::AhpRankChart => build_ahp_rank_csv(app_state),
        ChartId::AhpTable => build_ahp_table_csv(app_state),
        ChartId::SliceChart => build_slice_csv(app_state, widgets),
        ChartId::SurfacePlot => None,
        ChartId::ClusterScatter3D => build_cluster_csv(app_state),
        ChartId::McdmScatterChart3D => build_mcdm_scatter_csv(app_state),
    }
}

pub fn has_csv_data(chart_id: &ChartId, app_state: &AppState, widgets: &WidgetStates) -> bool {
    match chart_id {
        ChartId::SurfacePlot => false,
        ChartId::OptimizationHistory | ChartId::ParallelCoordinates | ChartId::ScatterMatrix => {
            app_state
                .current_study
                .as_ref()
                .is_some_and(|s| s.trial_count() > 0)
        }
        ChartId::HvHistory => app_state.hv_history.is_some(),
        ChartId::ImportanceChart => {
            if widgets.importance.computing {
                return false;
            }
            let obj_idx = widgets.importance.objective_index;
            if widgets.importance.metric.is_sobol() {
                app_state.sobol_cache.contains_key(&obj_idx)
            } else {
                let key = (widgets.importance.metric.cache_id(), obj_idx);
                app_state.importance_cache.contains_key(&key)
            }
        }
        ChartId::PdpChart => {
            use crate::state::messages::PdpResult;
            widgets
                .pdp_chart
                .result
                .as_ref()
                .is_some_and(|r| matches!(r, PdpResult::OneDim(d) if !d.x_values.is_empty()))
        }
        ChartId::PdpChart2D => widgets
            .pdp_2d
            .result
            .as_ref()
            .is_some_and(|r| !r.x_values.is_empty() && !r.y_values.is_empty()),
        ChartId::ClusterScatter => app_state
            .current_study
            .as_ref()
            .zip(app_state.cluster_result.as_ref())
            .is_some_and(|(s, cr)| cr.labels.len() == s.trial_count()),
        ChartId::SensitivityHeatmap => {
            widgets
                .sensitivity_heatmap
                .result
                .as_ref()
                .is_some_and(|s| {
                    !s.param_names.is_empty()
                        && !s.objective_names.is_empty()
                        && s.spearman.len() == s.param_names.len()
                        && s.spearman
                            .iter()
                            .all(|row| row.len() == s.objective_names.len())
                })
        }
        ChartId::ParetoScatter2D | ChartId::ParetoScatter3D => app_state
            .current_study
            .as_ref()
            .is_some_and(|s| !s.pareto_indices.is_empty()),
        ChartId::McdmRankChart | ChartId::McdmScatterChart | ChartId::McdmTable => {
            app_state.mcdm_result.is_some() && app_state.current_study.is_some()
        }
        ChartId::AhpRankChart | ChartId::AhpTable => {
            app_state.ahp_result.is_some() && app_state.current_study.is_some()
        }
        ChartId::SliceChart => app_state.current_study.as_ref().is_some_and(|s| {
            s.trial_count() > 0
                && s.meta
                    .param_names
                    .get(widgets.slice_chart.selected_param_idx)
                    .is_some()
                && s.meta
                    .objective_names
                    .get(widgets.slice_chart.selected_obj_idx)
                    .is_some()
        }),
        ChartId::ClusterScatter3D => app_state
            .current_study
            .as_ref()
            .zip(app_state.cluster_result.as_ref())
            .is_some_and(|(s, cr)| cr.labels.len() == s.trial_count()),
        ChartId::McdmScatterChart3D => {
            app_state.mcdm_result.is_some() && app_state.current_study.is_some()
        }
    }
}

pub fn csv_export_filename(chart_id: &ChartId) -> String {
    let name = match chart_id {
        ChartId::OptimizationHistory => "optimization_history",
        ChartId::HvHistory => "hv_history",
        ChartId::ImportanceChart => "importance_chart",
        ChartId::PdpChart => "pdp_chart",
        ChartId::PdpChart2D => "pdp_chart_2d",
        ChartId::ParallelCoordinates => "parallel_coordinates",
        ChartId::ScatterMatrix => "scatter_matrix",
        ChartId::ClusterScatter => "cluster_scatter",
        ChartId::SensitivityHeatmap => "sensitivity_heatmap",
        ChartId::ParetoScatter2D => "pareto_scatter_2d",
        ChartId::ParetoScatter3D => "pareto_scatter_3d",
        ChartId::McdmRankChart => "mcdm_rank_chart",
        ChartId::McdmScatterChart => "mcdm_scatter_chart",
        ChartId::McdmTable => "mcdm_table",
        ChartId::AhpRankChart => "ahp_rank_chart",
        ChartId::AhpTable => "ahp_table",
        ChartId::SliceChart => "slice_chart",
        ChartId::SurfacePlot => "surface_plot",
        ChartId::ClusterScatter3D => "cluster_scatter_3d",
        ChartId::McdmScatterChart3D => "mcdm_scatter_chart_3d",
    };
    format!("{}.csv", name)
}

fn build_optimization_history_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let study = app_state.current_study.as_ref()?;
    let obj_idx = widgets.opt_history.obj_idx;
    let obj_name = study.meta.objective_names.get(obj_idx)?;
    let obj_col = study.view.numeric_column(obj_name)?;
    if obj_col.is_empty() {
        return None;
    }
    let is_minimize = !matches!(
        study.meta.directions.get(obj_idx),
        Some(Direction::Maximize)
    );
    let mut csv = String::from("trial_index,objective_value,best_value\n");
    let mut best = if is_minimize {
        f64::INFINITY
    } else {
        f64::NEG_INFINITY
    };
    for (i, &val) in obj_col.iter().enumerate() {
        if val.is_finite() {
            best = if is_minimize {
                best.min(val)
            } else {
                best.max(val)
            };
        }
        let best_str = if best.is_finite() {
            format!("{}", best)
        } else {
            String::new()
        };
        csv.push_str(&format!("{},{},{}\n", i, val, best_str));
    }
    Some(csv)
}

fn build_hv_history_csv(app_state: &AppState) -> Option<String> {
    let hv = app_state.hv_history.as_ref()?;
    let mut csv = String::from("trial_index,hypervolume\n");
    for (i, &hv_val) in hv.hv_values.iter().enumerate() {
        let trial_idx = i * hv.sample_step;
        csv.push_str(&format!("{},{}\n", trial_idx, hv_val));
    }
    Some(csv)
}
fn build_importance_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    if widgets.importance.computing {
        return None;
    }
    use crate::ui::widgets::importance_chart::{compute_sorted_importance, compute_sorted_sobol};
    let metric = &widgets.importance.metric;
    let obj_idx = widgets.importance.objective_index;
    let method_name = metric.label();
    let pairs: Vec<(String, f64)> = if metric.is_sobol() {
        let sobol = app_state.sobol_cache.get(&obj_idx)?;
        compute_sorted_sobol(sobol, obj_idx, metric)
    } else {
        let key = (metric.cache_id(), obj_idx);
        let sensitivity = app_state.importance_cache.get(&key)?;
        compute_sorted_importance(sensitivity, metric, obj_idx)
    };
    if pairs.is_empty() {
        return None;
    }
    let mut csv = String::from("variable,importance_score,method\n");
    for (name, score) in &pairs {
        csv.push_str(&format!("{},{},{}\n", name, score, method_name));
    }
    Some(csv)
}
fn build_pdp_csv(_app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    use crate::state::messages::PdpResult;
    let result = widgets.pdp_chart.result.as_ref()?;
    let r = match result {
        PdpResult::OneDim(r) => r,
        PdpResult::TwoDim(_) => return None,
    };
    if r.x_values.is_empty() {
        return None;
    }
    let mut csv = String::from("variable,variable_value,predicted_objective,lower_ci,upper_ci\n");
    for (i, (&x, &y)) in r.x_values.iter().zip(r.y_values.iter()).enumerate() {
        let lower = r.y_lower.as_ref().and_then(|v| v.get(i)).copied();
        let upper = r.y_upper.as_ref().and_then(|v| v.get(i)).copied();
        let lower_str = lower.map(|v| v.to_string()).unwrap_or_default();
        let upper_str = upper.map(|v| v.to_string()).unwrap_or_default();
        csv.push_str(&format!(
            "{},{},{},{},{}\n",
            r.param_name, x, y, lower_str, upper_str
        ));
    }
    Some(csv)
}

fn build_pdp_2d_csv(_app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let result = widgets.pdp_2d.result.as_ref()?;
    if result.x_values.is_empty() || result.y_values.is_empty() {
        return None;
    }
    let mut csv =
        String::from("param1_name,param1_value,param2_name,param2_value,predicted_objective\n");
    for (xi, &x) in result.x_values.iter().enumerate() {
        for (yi, &y) in result.y_values.iter().enumerate() {
            let z = result
                .z_values
                .get(xi)
                .and_then(|row| row.get(yi))
                .copied()
                .unwrap_or(f64::NAN);
            csv.push_str(&format!(
                "{},{},{},{},{}\n",
                result.param1_name, x, result.param2_name, y, z
            ));
        }
    }
    Some(csv)
}
fn build_trial_based_csv(app_state: &AppState) -> Option<String> {
    let study = app_state.current_study.as_ref()?;
    let n = study.trial_count();
    if n == 0 {
        return None;
    }
    let row_indices: Vec<usize> = (0..n).collect();
    Some(crate::io::export::build_csv_string_from_view(
        &study.view,
        &row_indices,
        &study.meta.param_names,
        &study.meta.objective_names,
    ))
}

fn build_cluster_csv(app_state: &AppState) -> Option<String> {
    let study = app_state.current_study.as_ref()?;
    let cr = app_state.cluster_result.as_ref()?;
    let n = study.trial_count();
    if cr.labels.len() != n {
        return None;
    }
    let param_names = &study.meta.param_names;
    let obj_names = &study.meta.objective_names;
    let param_cols = study.view.numeric_columns(param_names);
    let obj_cols = study.view.numeric_columns(obj_names);
    let mut csv = String::from("trial_id,trial_number");
    for name in param_names {
        csv.push_str(&format!(",{}", name));
    }
    for name in obj_names {
        csv.push_str(&format!(",{}", name));
    }
    csv.push_str(",cluster_id\n");
    for i in 0..n {
        let trial_id = study.view.trial_ids.get(i).copied().unwrap_or(i as u32);
        csv.push_str(&format!("{},{}", trial_id, i));
        for col in &param_cols {
            let v = col.and_then(|c| c.get(i)).copied().unwrap_or(f64::NAN);
            csv.push_str(&format!(",{}", v));
        }
        for col in &obj_cols {
            let v = col.and_then(|c| c.get(i)).copied().unwrap_or(f64::NAN);
            csv.push_str(&format!(",{}", v));
        }
        let label = cr.labels.get(i).copied().unwrap_or(-1);
        csv.push_str(&format!(",{}\n", label));
    }
    Some(csv)
}
fn build_sensitivity_csv(_app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let sens = widgets.sensitivity_heatmap.result.as_ref()?;
    if sens.param_names.is_empty() || sens.objective_names.is_empty() {
        return None;
    }
    let n_obj = sens.objective_names.len();
    if sens.spearman.len() != sens.param_names.len()
        || sens.spearman.iter().any(|row| row.len() != n_obj)
    {
        return None;
    }
    let mut csv = String::from("variable");
    for name in &sens.objective_names {
        csv.push_str(&format!(",{}", name));
    }
    csv.push('\n');
    for (i, param_name) in sens.param_names.iter().enumerate() {
        csv.push_str(param_name);
        for &val in &sens.spearman[i] {
            csv.push_str(&format!(",{}", val));
        }
        csv.push('\n');
    }
    Some(csv)
}
fn build_pareto_csv(app_state: &AppState) -> Option<String> {
    let study = app_state.current_study.as_ref()?;
    if study.pareto_indices.is_empty() {
        return None;
    }
    let pareto_set: std::collections::HashSet<u32> = study.pareto_indices.iter().copied().collect();
    let param_names = &study.meta.param_names;
    let obj_names = &study.meta.objective_names;
    let param_cols = study.view.numeric_columns(param_names);
    let obj_cols = study.view.numeric_columns(obj_names);
    let mut csv = String::from("trial_id,trial_number");
    for name in param_names {
        csv.push_str(&format!(",{}", name));
    }
    for name in obj_names {
        csv.push_str(&format!(",{}", name));
    }
    csv.push_str(",pareto_rank\n");
    for (i, &tid) in study.view.trial_ids.iter().enumerate() {
        if !pareto_set.contains(&tid) {
            continue;
        }
        let rank = study.view.pareto_rank.get(i).copied().unwrap_or(0);
        csv.push_str(&format!("{},{}", tid, i));
        for col in &param_cols {
            let v = col.and_then(|c| c.get(i)).copied().unwrap_or(f64::NAN);
            csv.push_str(&format!(",{}", v));
        }
        for col in &obj_cols {
            let v = col.and_then(|c| c.get(i)).copied().unwrap_or(f64::NAN);
            csv.push_str(&format!(",{}", v));
        }
        csv.push_str(&format!(",{}\n", rank));
    }
    Some(csv)
}
fn build_mcdm_rank_csv(app_state: &AppState) -> Option<String> {
    let result = app_state.mcdm_result.as_ref()?;
    let trial_ids = &app_state.current_study.as_ref()?.view.trial_ids;
    let method_name = result.method_label();
    let scores = result.primary_scores();
    let ranked = result.ranked_indices();
    let mut csv = String::from("trial_id,rank,score,method\n");
    for (rank, &idx) in ranked.iter().enumerate() {
        let i = idx as usize;
        let trial_id = trial_ids.get(i).copied().unwrap_or(i as u32);
        let score = scores.get(i).copied().unwrap_or(f64::NAN);
        csv.push_str(&format!(
            "{},{},{},{}\n",
            trial_id,
            rank + 1,
            score,
            method_name
        ));
    }
    Some(csv)
}

fn build_mcdm_scatter_csv(app_state: &AppState) -> Option<String> {
    let result = app_state.mcdm_result.as_ref()?;
    let trial_ids = &app_state.current_study.as_ref()?.view.trial_ids;
    let scores = result.primary_scores();
    let ranked = result.ranked_indices();
    let mut csv = String::from("trial_id,rank,primary_score\n");
    for (rank, &idx) in ranked.iter().enumerate() {
        let i = idx as usize;
        let trial_id = trial_ids.get(i).copied().unwrap_or(i as u32);
        let score = scores.get(i).copied().unwrap_or(f64::NAN);
        csv.push_str(&format!("{},{},{}\n", trial_id, rank + 1, score));
    }
    Some(csv)
}

fn build_mcdm_table_csv(app_state: &AppState) -> Option<String> {
    let result = app_state.mcdm_result.as_ref()?;
    let trial_ids = &app_state.current_study.as_ref()?.view.trial_ids;
    let tid = |idx: u32| trial_ids.get(idx as usize).copied().unwrap_or(idx);
    match result {
        McdmResult::Topsis(r) => {
            let mut csv = String::from("trial_id,rank,topsis_score\n");
            for (rank, &idx) in r.ranked_indices.iter().enumerate() {
                let score = r.scores.get(idx as usize).copied().unwrap_or(f64::NAN);
                csv.push_str(&format!("{},{},{}\n", tid(idx), rank + 1, score));
            }
            Some(csv)
        }
        McdmResult::Vikor(r) => {
            let mut csv = String::from("trial_id,rank,s_value,r_value,q_value\n");
            for (rank, &idx) in r.ranked_indices.iter().enumerate() {
                let i = idx as usize;
                let s = r.s_values.get(i).copied().unwrap_or(f64::NAN);
                let rv = r.r_values.get(i).copied().unwrap_or(f64::NAN);
                let q = r.q_values.get(i).copied().unwrap_or(f64::NAN);
                csv.push_str(&format!("{},{},{},{},{}\n", tid(idx), rank + 1, s, rv, q));
            }
            Some(csv)
        }
        McdmResult::PrometheeI(r) => {
            let mut csv = String::from("trial_id,rank,phi_plus,phi_minus\n");
            for (rank, &idx) in r.ranked_indices_i.iter().enumerate() {
                let i = idx as usize;
                let phi_plus = r.phi_plus.get(i).copied().unwrap_or(f64::NAN);
                let phi_minus = r.phi_minus.get(i).copied().unwrap_or(f64::NAN);
                csv.push_str(&format!(
                    "{},{},{},{}\n",
                    tid(idx),
                    rank + 1,
                    phi_plus,
                    phi_minus
                ));
            }
            Some(csv)
        }
        McdmResult::PrometheeII(r) => {
            let mut csv = String::from("trial_id,rank,phi_net\n");
            for (rank, &idx) in r.ranked_indices_ii.iter().enumerate() {
                let phi_net = r.phi_net.get(idx as usize).copied().unwrap_or(f64::NAN);
                csv.push_str(&format!("{},{},{}\n", tid(idx), rank + 1, phi_net));
            }
            Some(csv)
        }
    }
}

fn build_ahp_rank_csv(app_state: &AppState) -> Option<String> {
    let result = app_state.ahp_result.as_ref()?;
    let trial_ids = &app_state.current_study.as_ref()?.view.trial_ids;
    let mut csv = String::from("trial_id,rank,ahp_score\n");
    for (rank, &idx) in result.ranked_indices.iter().enumerate() {
        let i = idx as usize;
        let trial_id = trial_ids.get(i).copied().unwrap_or(i as u32);
        let score = result.scores.get(i).copied().unwrap_or(f64::NAN);
        csv.push_str(&format!("{},{},{}\n", trial_id, rank + 1, score));
    }
    Some(csv)
}

fn build_ahp_table_csv(app_state: &AppState) -> Option<String> {
    let result = app_state.ahp_result.as_ref()?;
    let study = app_state.current_study.as_ref()?;
    let obj_names = &study.meta.objective_names;
    let obj_cols = study.view.numeric_columns(obj_names);
    let mut csv = String::from("trial_id,rank,ahp_score");
    for name in obj_names {
        csv.push_str(&format!(",{}", name));
    }
    csv.push('\n');
    for (rank, &idx) in result.ranked_indices.iter().enumerate() {
        let i = idx as usize;
        let trial_id = study.view.trial_ids.get(i).copied().unwrap_or(i as u32);
        let score = result.scores.get(i).copied().unwrap_or(f64::NAN);
        csv.push_str(&format!("{},{},{}", trial_id, rank + 1, score));
        for col in &obj_cols {
            let v = col.and_then(|c| c.get(i)).copied().unwrap_or(f64::NAN);
            csv.push_str(&format!(",{}", v));
        }
        csv.push('\n');
    }
    Some(csv)
}
fn build_slice_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let study = app_state.current_study.as_ref()?;
    let n = study.trial_count();
    if n == 0 {
        return None;
    }
    let param_idx = widgets.slice_chart.selected_param_idx;
    let obj_idx = widgets.slice_chart.selected_obj_idx;
    let param_name = study.meta.param_names.get(param_idx)?;
    let obj_name = study.meta.objective_names.get(obj_idx)?;
    let param_col = study.view.numeric_column(param_name);
    let obj_col = study.view.numeric_column(obj_name);
    let pareto_set: std::collections::HashSet<u32> = study.pareto_indices.iter().copied().collect();
    let mut csv = format!("trial_id,{},{},is_pareto\n", param_name, obj_name);
    for (i, &tid) in study.view.trial_ids.iter().enumerate() {
        let param_val = param_col
            .and_then(|c| c.get(i))
            .copied()
            .unwrap_or(f64::NAN);
        let obj_val = obj_col.and_then(|c| c.get(i)).copied().unwrap_or(f64::NAN);
        let is_pareto = pareto_set.contains(&tid);
        csv.push_str(&format!(
            "{},{},{},{}\n",
            tid, param_val, obj_val, is_pareto
        ));
    }
    Some(csv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::AppState;
    use crate::state::results::HvHistory;
    use crate::state::types::{Direction, StudyContext, StudyMeta, TrialRow};
    use crate::ui::widget_states::WidgetStates;
    use std::collections::HashMap;

    fn make_study(
        param_names: Vec<String>,
        obj_names: Vec<String>,
        directions: Vec<Direction>,
    ) -> StudyContext {
        let meta = StudyMeta {
            study_id: 0,
            name: "test".to_string(),
            directions,
            completed_trials: 0,
            total_trials: 0,
            param_names,
            objective_names: obj_names,
            user_attr_names: vec![],
            has_constraints: false,
        };
        StudyContext::from_rows_for_test(meta, vec![])
    }

    fn make_trial(id: u32, params: HashMap<String, f64>, objectives: Vec<f64>) -> TrialRow {
        TrialRow {
            trial_id: id,
            trial_number: id,
            params,
            objectives,
            ..Default::default()
        }
    }

    #[test]
    fn csv_export_filename_optimization_history() {
        assert_eq!(
            csv_export_filename(&ChartId::OptimizationHistory),
            "optimization_history.csv"
        );
    }

    #[test]
    fn csv_export_filename_all_end_with_csv() {
        let ids = vec![
            ChartId::OptimizationHistory,
            ChartId::HvHistory,
            ChartId::ImportanceChart,
            ChartId::PdpChart,
            ChartId::PdpChart2D,
            ChartId::ParallelCoordinates,
            ChartId::ScatterMatrix,
            ChartId::ClusterScatter,
            ChartId::SensitivityHeatmap,
            ChartId::ParetoScatter2D,
            ChartId::ParetoScatter3D,
            ChartId::McdmRankChart,
            ChartId::McdmScatterChart,
            ChartId::McdmTable,
            ChartId::AhpRankChart,
            ChartId::AhpTable,
            ChartId::SliceChart,
            ChartId::SurfacePlot,
        ];
        for id in &ids {
            assert!(
                csv_export_filename(id).ends_with(".csv"),
                "{:?} filename does not end with .csv",
                id
            );
        }
    }

    #[test]
    fn opt_history_csv_minimize_tracks_cumulative_min() {
        let mut state = AppState::default();
        let mut study = make_study(
            vec!["x".into()],
            vec!["f".into()],
            vec![Direction::Minimize],
        );
        study.set_rows_for_test(vec![
            make_trial(0, HashMap::new(), vec![3.0]),
            make_trial(1, HashMap::new(), vec![1.0]),
            make_trial(2, HashMap::new(), vec![2.0]),
        ]);
        state.current_study = Some(study);
        let widgets = WidgetStates::default();

        let csv = build_optimization_history_csv(&state, &widgets).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "trial_index,objective_value,best_value");
        assert_eq!(lines[1], "0,3,3");
        assert_eq!(lines[2], "1,1,1");
        assert_eq!(lines[3], "2,2,1");
    }

    #[test]
    fn opt_history_csv_returns_none_when_no_study() {
        let state = AppState::default();
        let widgets = WidgetStates::default();
        assert!(build_optimization_history_csv(&state, &widgets).is_none());
    }

    #[test]
    fn hv_history_csv_uses_index_times_step() {
        let state = AppState {
            hv_history: Some(HvHistory {
                trial_ids: vec![10, 20, 30],
                hv_values: vec![0.1, 0.5, 0.8],
                sample_step: 5,
            }),
            ..AppState::default()
        };
        let csv = build_hv_history_csv(&state).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "trial_index,hypervolume");
        assert_eq!(lines[1], "0,0.1");
        assert_eq!(lines[2], "5,0.5");
        assert_eq!(lines[3], "10,0.8");
    }

    #[test]
    fn hv_history_csv_returns_none_when_missing() {
        let state = AppState::default();
        assert!(build_hv_history_csv(&state).is_none());
    }

    #[test]
    fn importance_csv_returns_none_when_computing() {
        let state = AppState::default();
        let mut widgets = WidgetStates::default();
        widgets.importance.computing = true;
        assert!(build_importance_csv(&state, &widgets).is_none());
    }

    #[test]
    fn importance_csv_returns_none_when_no_cache() {
        let state = AppState::default();
        let widgets = WidgetStates::default();
        // importance_cache is empty, should return None
        assert!(build_importance_csv(&state, &widgets).is_none());
    }

    #[test]
    fn importance_csv_has_expected_columns() {
        use crate::state::app_state::SensitivityResult;
        use crate::state::results::RidgeResult;
        let mut state = AppState::default();
        let result = SensitivityResult {
            param_names: vec!["x".into(), "y".into()],
            objective_names: vec!["f".into()],
            spearman: vec![vec![0.9, 0.3]],
            ridge: vec![RidgeResult {
                beta: vec![0.8, 0.2],
                r_squared: 0.95,
            }],
            rf_anova: None,
            mdi: None,
            shap: None,
            permutation: None,
        };
        // Spearman is cache_id=0
        state.importance_cache.insert((0u8, 0), result);
        let widgets = WidgetStates::default(); // metric=Spearman, obj_idx=0
        let csv = build_importance_csv(&state, &widgets).unwrap();
        let header = csv.lines().next().unwrap();
        assert_eq!(header, "variable,importance_score,method");
        // 2 params → 2 data rows + header
        assert_eq!(csv.lines().count(), 3);
    }

    #[test]
    fn sensitivity_csv_has_objective_columns_in_header() {
        use crate::state::app_state::SensitivityResult;
        let mut widgets = WidgetStates::default();
        widgets.sensitivity_heatmap.result = Some(SensitivityResult {
            param_names: vec!["x".into(), "y".into()],
            objective_names: vec!["f1".into(), "f2".into()],
            spearman: vec![vec![0.9, 0.3], vec![0.5, 0.7]],
            ridge: vec![],
            rf_anova: None,
            mdi: None,
            shap: None,
            permutation: None,
        });
        let state = AppState::default();
        let csv = build_sensitivity_csv(&state, &widgets).unwrap();
        let header = csv.lines().next().unwrap();
        assert_eq!(header, "variable,f1,f2");
        assert_eq!(csv.lines().count(), 3); // header + 2 params
    }

    #[test]
    fn sensitivity_csv_returns_none_when_no_result() {
        let state = AppState::default();
        let widgets = WidgetStates::default(); // sensitivity_heatmap.result = None
        assert!(build_sensitivity_csv(&state, &widgets).is_none());
    }

    #[test]
    fn trial_based_csv_has_trial_id_header() {
        let mut state = AppState::default();
        let mut study = make_study(
            vec!["x".into()],
            vec!["f".into()],
            vec![Direction::Minimize],
        );
        let mut p = HashMap::new();
        p.insert("x".to_string(), 1.0_f64);
        study.set_rows_for_test(vec![make_trial(0, p, vec![0.5])]);
        state.current_study = Some(study);
        let csv = build_trial_based_csv(&state).unwrap();
        assert!(csv.lines().next().unwrap().contains("trial_id"));
    }

    #[test]
    fn trial_based_csv_returns_none_when_no_study() {
        let state = AppState::default();
        assert!(build_trial_based_csv(&state).is_none());
    }

    #[test]
    fn cluster_csv_returns_none_when_no_cluster_result() {
        let mut state = AppState::default();
        let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
        study.set_rows_for_test(vec![make_trial(0, HashMap::new(), vec![1.0])]);
        state.current_study = Some(study);
        // no cluster_result set
        assert!(build_cluster_csv(&state).is_none());
    }

    #[test]
    fn cluster_csv_includes_cluster_id_column() {
        use crate::state::results::ClusterResult;
        let mut state = AppState::default();
        let mut study = make_study(
            vec!["x".into()],
            vec!["f".into()],
            vec![Direction::Minimize],
        );
        let mut p = HashMap::new();
        p.insert("x".to_string(), 1.0_f64);
        study.set_rows_for_test(vec![
            make_trial(0, p.clone(), vec![0.5]),
            make_trial(1, p.clone(), vec![1.0]),
        ]);
        state.current_study = Some(study);
        state.cluster_result = Some(ClusterResult {
            labels: vec![0, 1],
            n_clusters: 2,
        });
        let csv = build_cluster_csv(&state).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert!(lines[0].ends_with(",cluster_id"), "header: {}", lines[0]);
        assert!(lines[1].ends_with(",0"), "row0: {}", lines[1]);
        assert!(lines[2].ends_with(",1"), "row1: {}", lines[2]);
    }

    #[test]
    fn pareto_csv_only_includes_pareto_trials() {
        let mut state = AppState::default();
        let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
        study.set_rows_for_test(vec![
            make_trial(0, HashMap::new(), vec![1.0]),
            make_trial(1, HashMap::new(), vec![2.0]),
            make_trial(2, HashMap::new(), vec![3.0]),
        ]);
        study.pareto_indices = vec![0];
        state.current_study = Some(study);
        let csv = build_pareto_csv(&state).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 2, "header + 1 pareto row: {:?}", lines);
        assert!(lines[0].contains("pareto_rank"));
    }

    #[test]
    fn pareto_csv_returns_none_when_no_pareto() {
        let mut state = AppState::default();
        let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
        study.set_rows_for_test(vec![make_trial(0, HashMap::new(), vec![1.0])]);
        // pareto_indices is empty
        state.current_study = Some(study);
        assert!(build_pareto_csv(&state).is_none());
    }

    // ── TASK-2325: PDP tests ──────────────────────────────────────

    #[test]
    fn pdp_csv_returns_none_when_no_result() {
        let state = AppState::default();
        let widgets = WidgetStates::default();
        assert!(build_pdp_csv(&state, &widgets).is_none());
    }

    #[test]
    fn pdp_csv_has_correct_header() {
        use crate::state::messages::{PdpResult, PdpResult1d};
        let mut widgets = WidgetStates::default();
        widgets.pdp_chart.result = Some(PdpResult::OneDim(PdpResult1d {
            x_values: vec![0.0, 1.0],
            y_values: vec![0.5, 0.8],
            y_upper: Some(vec![0.6, 0.9]),
            y_lower: Some(vec![0.4, 0.7]),
            ice_lines: vec![],
            r2: None,
            param_name: "x".to_string(),
            objective_name: "f".to_string(),
        }));
        let state = AppState::default();
        let csv = build_pdp_csv(&state, &widgets).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(
            lines[0],
            "variable,variable_value,predicted_objective,lower_ci,upper_ci"
        );
        assert_eq!(lines.len(), 3); // header + 2 points
        assert_eq!(lines[1], "x,0,0.5,0.4,0.6");
    }

    #[test]
    fn pdp_csv_handles_missing_ci() {
        use crate::state::messages::{PdpResult, PdpResult1d};
        let mut widgets = WidgetStates::default();
        widgets.pdp_chart.result = Some(PdpResult::OneDim(PdpResult1d {
            x_values: vec![0.0],
            y_values: vec![0.5],
            y_upper: None,
            y_lower: None,
            ice_lines: vec![],
            r2: None,
            param_name: "x".to_string(),
            objective_name: "f".to_string(),
        }));
        let state = AppState::default();
        let csv = build_pdp_csv(&state, &widgets).unwrap();
        // lower_ci and upper_ci should be empty strings
        assert_eq!(csv.lines().nth(1).unwrap(), "x,0,0.5,,");
    }

    #[test]
    fn pdp_2d_csv_returns_none_when_no_result() {
        let state = AppState::default();
        let widgets = WidgetStates::default();
        assert!(build_pdp_2d_csv(&state, &widgets).is_none());
    }

    #[test]
    fn pdp_2d_csv_has_correct_header_and_grid() {
        use crate::state::messages::PdpResult2d;
        let mut widgets = WidgetStates::default();
        widgets.pdp_2d.result = Some(PdpResult2d {
            x_values: vec![0.0, 1.0],
            y_values: vec![2.0, 3.0],
            z_values: vec![vec![0.1, 0.2], vec![0.3, 0.4]],
            param1_name: "x".to_string(),
            param2_name: "y".to_string(),
            objective_name: "f".to_string(),
            uncertainties: None,
        });
        let state = AppState::default();
        let csv = build_pdp_2d_csv(&state, &widgets).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(
            lines[0],
            "param1_name,param1_value,param2_name,param2_value,predicted_objective"
        );
        // 2x2 grid → 4 data rows
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[1], "x,0,y,2,0.1");
        assert_eq!(lines[2], "x,0,y,3,0.2");
    }

    // ── TASK-2324: MCDM/AHP tests ─────────────────────────────────

    fn make_topsis_mcdm(trial_rows_len: usize) -> crate::state::app_state::McdmResult {
        use crate::state::results::TopsisResult;
        McdmResult::Topsis(TopsisResult {
            scores: (0..trial_rows_len).map(|i| i as f64 * 0.1 + 0.5).collect(),
            ranked_indices: (0..trial_rows_len as u32).rev().collect(),
            positive_ideal: vec![],
            negative_ideal: vec![],
            duration_ms: 1.0,
        })
    }

    #[test]
    fn mcdm_rank_csv_has_correct_header_and_method_topsis() {
        let mut state = AppState::default();
        let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
        study.set_rows_for_test(vec![
            make_trial(10, HashMap::new(), vec![1.0]),
            make_trial(11, HashMap::new(), vec![2.0]),
        ]);
        state.current_study = Some(study);
        state.mcdm_result = Some(make_topsis_mcdm(2));
        let csv = build_mcdm_rank_csv(&state).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "trial_id,rank,score,method");
        assert!(lines[1].ends_with(",TOPSIS"), "method column: {}", lines[1]);
        assert_eq!(lines.len(), 3); // header + 2 rows
    }

    #[test]
    fn mcdm_rank_csv_returns_none_when_no_result() {
        let state = AppState::default();
        assert!(build_mcdm_rank_csv(&state).is_none());
    }

    #[test]
    fn mcdm_scatter_csv_has_correct_header() {
        let mut state = AppState::default();
        let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
        study.set_rows_for_test(vec![make_trial(0, HashMap::new(), vec![1.0])]);
        state.current_study = Some(study);
        state.mcdm_result = Some(make_topsis_mcdm(1));
        let csv = build_mcdm_scatter_csv(&state).unwrap();
        assert_eq!(csv.lines().next().unwrap(), "trial_id,rank,primary_score");
    }

    #[test]
    fn mcdm_table_csv_topsis_header() {
        let mut state = AppState::default();
        let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
        study.set_rows_for_test(vec![make_trial(0, HashMap::new(), vec![1.0])]);
        state.current_study = Some(study);
        state.mcdm_result = Some(make_topsis_mcdm(1));
        let csv = build_mcdm_table_csv(&state).unwrap();
        assert_eq!(csv.lines().next().unwrap(), "trial_id,rank,topsis_score");
    }

    #[test]
    fn mcdm_table_csv_vikor_header() {
        use crate::state::results::VikorResult;
        let mut state = AppState::default();
        let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
        study.set_rows_for_test(vec![make_trial(0, HashMap::new(), vec![1.0])]);
        state.current_study = Some(study);
        state.mcdm_result = Some(McdmResult::Vikor(VikorResult {
            s_values: vec![0.3],
            r_values: vec![0.2],
            q_values: vec![0.1],
            display_scores: vec![0.4],
            ranked_indices: vec![0],
            best_values: vec![],
            worst_values: vec![],
            duration_ms: 1.0,
        }));
        let csv = build_mcdm_table_csv(&state).unwrap();
        assert_eq!(
            csv.lines().next().unwrap(),
            "trial_id,rank,s_value,r_value,q_value"
        );
    }

    #[test]
    fn mcdm_table_csv_returns_none_when_no_result() {
        let state = AppState::default();
        assert!(build_mcdm_table_csv(&state).is_none());
    }

    #[test]
    fn ahp_rank_csv_has_correct_header() {
        use crate::state::results::AhpResult;
        let mut state = AppState::default();
        let mut study = make_study(vec![], vec!["f".into()], vec![Direction::Minimize]);
        study.set_rows_for_test(vec![
            make_trial(5, HashMap::new(), vec![1.0]),
            make_trial(6, HashMap::new(), vec![2.0]),
        ]);
        state.current_study = Some(study);
        state.ahp_result = Some(AhpResult {
            priority_vector: vec![1.0],
            scores: vec![0.7, 0.3],
            ranked_indices: vec![0, 1],
            lambda_max: 1.0,
            ci: 0.0,
            ri: 0.0,
            cr: 0.0,
            is_consistent: true,
            duration_ms: 1.0,
        });
        let csv = build_ahp_rank_csv(&state).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "trial_id,rank,ahp_score");
        assert_eq!(lines.len(), 3); // header + 2 rows
                                    // first ranked row should have trial_id=5, rank=1
        assert!(lines[1].starts_with("5,1,"), "row: {}", lines[1]);
    }

    #[test]
    fn ahp_rank_csv_returns_none_when_no_result() {
        let state = AppState::default();
        assert!(build_ahp_rank_csv(&state).is_none());
    }

    #[test]
    fn ahp_table_csv_includes_objective_columns() {
        use crate::state::results::AhpResult;
        let mut state = AppState::default();
        let mut study = make_study(
            vec![],
            vec!["f1".into(), "f2".into()],
            vec![Direction::Minimize, Direction::Minimize],
        );
        study.set_rows_for_test(vec![make_trial(0, HashMap::new(), vec![1.0, 2.0])]);
        state.current_study = Some(study);
        state.ahp_result = Some(AhpResult {
            priority_vector: vec![0.5, 0.5],
            scores: vec![0.9],
            ranked_indices: vec![0],
            lambda_max: 2.0,
            ci: 0.0,
            ri: 0.0,
            cr: 0.0,
            is_consistent: true,
            duration_ms: 1.0,
        });
        let csv = build_ahp_table_csv(&state).unwrap();
        let header = csv.lines().next().unwrap();
        assert_eq!(header, "trial_id,rank,ahp_score,f1,f2");
        let data = csv.lines().nth(1).unwrap();
        assert_eq!(data, "0,1,0.9,1,2");
    }

    #[test]
    fn ahp_table_csv_returns_none_when_no_study() {
        use crate::state::results::AhpResult;
        // ahp_result is Some but current_study is None
        let state = AppState {
            ahp_result: Some(AhpResult {
                priority_vector: vec![],
                scores: vec![],
                ranked_indices: vec![],
                lambda_max: 0.0,
                ci: 0.0,
                ri: 0.0,
                cr: 0.0,
                is_consistent: true,
                duration_ms: 0.0,
            }),
            ..AppState::default()
        };
        assert!(build_ahp_table_csv(&state).is_none());
    }

    #[test]
    fn slice_csv_includes_param_obj_and_pareto() {
        let mut state = AppState::default();
        let mut study = make_study(
            vec!["x".into()],
            vec!["f".into()],
            vec![Direction::Minimize],
        );
        let mut p = HashMap::new();
        p.insert("x".to_string(), 1.5_f64);
        study.set_rows_for_test(vec![make_trial(0, p, vec![0.5])]);
        study.pareto_indices = vec![0];
        state.current_study = Some(study);
        let widgets = WidgetStates::default();

        let csv = build_slice_csv(&state, &widgets).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "trial_id,x,f,is_pareto");
        assert_eq!(lines[1], "0,1.5,0.5,true");
    }
}
