use std::sync::mpsc;

use crate::state::app_state::{Direction, StudyContext};
use crate::state::messages::{AppMessage, McdmChartSource};
use crate::state::results::{EntropyResult, McdmMethod, McdmResult};
use crate::ui::widgets::mcdm_chart::{McdmCacheKey, McdmComputeRequest, McdmControls};

/// Launches the Entropy weight computation if needed (from each chart's controls).
pub(super) fn dispatch_mcdm_entropy(
    app_state: &crate::state::app_state::AppState,
    controls: &mut McdmControls,
    _ctx: &StudyContext,
    obj_names: &[String],
    source: McdmChartSource,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    if !controls.pending_entropy || controls.computing {
        return;
    }
    controls.computing = true;
    let tx = tx.clone();
    crate::state::analysis_job::spawn(
        app_state,
        tx,
        crate::state::analysis_job::DeliverySlot::Entropy(source),
        obj_names.to_vec(),
        tdashboard_analysis::preparation::Preparation::Entropy,
        move |prepared| match tdashboard_analysis::entropy::compute_entropy_weights(
            &prepared.data().objectives().values,
            prepared.data().objectives().rows.len(),
            prepared.data().objectives().names.len(),
        ) {
            Ok(r) => AppMessage::EntropyDone {
                source,
                result: EntropyResult {
                    weights: r.weights,
                    entropies: r.entropies,
                    diversities: r.diversities,
                    duration_ms: r.duration_ms,
                },
            },
            Err(e) => AppMessage::McdmFailed {
                source,
                message: format!("Entropy computation failed: {e}"),
            },
        },
    );
}

/// Launches the MCDM ranking computation if needed (from each chart's controls).
/// The result is returned with a config key and stored in `app_state.mcdm_cache`.
pub(super) fn dispatch_mcdm_compute(
    app_state: &crate::state::app_state::AppState,
    controls: &mut McdmControls,
    _ctx: &StudyContext,
    _obj_names: &[String],
    _directions: &[Direction],
    source: McdmChartSource,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    if app_state.study_streaming {
        return;
    }
    let Some(req) = controls.pending_compute.take() else {
        return;
    };
    controls.computing = true;

    let key = match McdmCacheKey::from_request(&req, controls.weight_mode) {
        Ok(key) => key,
        Err(message) => {
            crate::state::analysis_job::reject_settings(
                app_state,
                tx.clone(),
                crate::state::analysis_job::DeliverySlot::Mcdm(source),
                message,
            );
            return;
        }
    };
    let McdmComputeRequest { method, weights, v } = req;

    let tx = tx.clone();
    crate::state::analysis_job::spawn(
        app_state,
        tx,
        crate::state::analysis_job::DeliverySlot::Mcdm(source),
        tdashboard_analysis::session::Condition::from((
            format!("{:?}", key.method),
            format!("{:?}", key.weight_mode),
            key.weights_q.clone(),
            key.v_q,
        )),
        tdashboard_analysis::preparation::Preparation::Mcdm,
        move |prepared| {
            let data = prepared.data().objectives();
            let computed = compute_mcdm_result(
                method,
                v,
                &weights,
                &data.values,
                data.total_rows,
                data.rows.len(),
                data.names.len(),
                &data.minimize,
                &data.rows,
            );
            match computed {
                Ok(result) => AppMessage::McdmDone {
                    source,
                    key,
                    result,
                },
                Err(message) => AppMessage::McdmFailed { source, message },
            }
        },
    );
}

/// Computes MCDM over the Pareto-front subset and returns the result expanded to full trial length.
#[allow(clippy::too_many_arguments)]
fn compute_mcdm_result(
    method: McdmMethod,
    v: f64,
    weights: &[f64],
    objectives: &[f64],
    n_total: usize,
    n_pareto: usize,
    n_objectives: usize,
    is_minimize: &[bool],
    pareto_row_indices: &[usize],
) -> Result<McdmResult, String> {
    let start = std::time::Instant::now();

    if n_pareto == 0 {
        return Err("MCDM: Pareto front is empty. Run the optimizer first.".to_string());
    }

    // Helper that converts an index within the subset to a full-trial index
    let remap = |subset_idx: u32| -> u32 {
        pareto_row_indices
            .get(subset_idx as usize)
            .copied()
            .unwrap_or(0) as u32
    };
    let expand_scores = |subset_scores: Vec<f64>| -> Vec<f64> {
        let mut full = vec![0.0f64; n_total];
        for (j, &row) in pareto_row_indices.iter().enumerate() {
            full[row] = subset_scores[j];
        }
        full
    };
    let expand_counts = |subset_counts: Vec<u32>| -> Vec<u32> {
        let mut full = vec![0u32; n_total];
        for (j, &row) in pareto_row_indices.iter().enumerate() {
            full[row] = subset_counts[j];
        }
        full
    };

    match method {
        McdmMethod::Topsis => tdashboard_analysis::topsis::compute_topsis(
            objectives,
            n_pareto,
            n_objectives,
            weights,
            is_minimize,
        )
        .map(|r| {
            McdmResult::Topsis(crate::state::results::TopsisResult {
                scores: expand_scores(r.scores),
                ranked_indices: r.ranked_indices.into_iter().map(remap).collect(),
                duration_ms: start.elapsed().as_secs_f64() * 1000.0,
            })
        })
        .map_err(|e| format!("TOPSIS computation failed: {e}")),
        McdmMethod::Vikor => tdashboard_analysis::vikor::compute_vikor(
            objectives,
            n_pareto,
            n_objectives,
            weights,
            is_minimize,
            v,
        )
        .map(|r| {
            McdmResult::Vikor(crate::state::results::VikorResult {
                s_values: expand_scores(r.s_values),
                r_values: expand_scores(r.r_values),
                q_values: expand_scores(r.q_values),
                display_scores: expand_scores(r.display_scores),
                ranked_indices: r.ranked_indices.into_iter().map(remap).collect(),
                compromise_indices: r
                    .compromise_indices
                    .into_iter()
                    .map(|i| remap(i as u32) as usize)
                    .collect(),
                duration_ms: start.elapsed().as_secs_f64() * 1000.0,
            })
        })
        .map_err(|e| format!("VIKOR computation failed: {e}")),
        McdmMethod::PrometheeI | McdmMethod::PrometheeII => {
            tdashboard_analysis::promethee::compute_promethee(
                objectives,
                n_pareto,
                n_objectives,
                weights,
                is_minimize,
            )
            .map(|r| {
                let result = crate::state::results::PrometheeResult {
                    phi_plus: expand_scores(r.phi_plus),
                    phi_minus: expand_scores(r.phi_minus),
                    phi_net: expand_scores(r.phi_net),
                    ranked_indices_i: r.ranked_indices_i.into_iter().map(&remap).collect(),
                    ranked_indices_ii: r.ranked_indices_ii.into_iter().map(remap).collect(),
                    incomparable_counts: expand_counts(r.incomparable_counts),
                    duration_ms: r.duration_ms,
                };
                if method == McdmMethod::PrometheeI {
                    McdmResult::PrometheeI(result)
                } else {
                    McdmResult::PrometheeII(result)
                }
            })
            .map_err(|e| format!("PROMETHEE computation failed: {e}"))
        }
    }
}
