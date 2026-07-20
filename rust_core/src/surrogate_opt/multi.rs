//! Multi-objective surrogate optimization: fitting one surrogate per
//! objective (with Pareto-front inducing-point concentration) and estimating
//! the Pareto front via NSGA-II.

use super::models;
use super::optimizers;
use super::progress::FitProgress;
use super::slice::{best_observed_index, build_slice};
use super::types::{
    ParetoFrontPoint, SurrogateFitRequest, SurrogateMultiOptRequest, SurrogateMultiOptResult,
    SurrogateMultiOptimizeSpec, TrainedSurrogate,
};
use super::{
    fit_validated_inner, subsample_indices, take_rows, SurrogateModelKind, MAX_TRAIN_FOR_FIT,
    MIN_TRIALS_FOR_SURROGATE_OPT,
};

/// Common input to multi-objective optimization (for a single objective).
pub(crate) struct MultiObjectiveEntry<'a> {
    surrogate: &'a models::FittedSurrogate,
    /// Training data used to find the observed-best point (initial seed).
    x_matrix: &'a [Vec<f64>],
    y: &'a [f64],
}

/// Common logic that runs NSGA-II against a set of fitted surrogates and
/// post-processes the resulting front.
///
/// Assumes every entry's surrogate shares the same normalization transform
/// (col_stats), i.e. all were fit from an x_matrix over the same parameter
/// space.
pub(crate) fn run_multi_optimize(
    entries: &[MultiObjectiveEntry<'_>],
    minimize: &[bool],
    slice_params: Option<(usize, usize)>,
    n_grid: usize,
) -> SurrogateMultiOptResult {
    let n_obj = entries.len();
    let surrogates: Vec<&models::FittedSurrogate> = entries.iter().map(|e| e.surrogate).collect();
    let ref_surrogate = surrogates[0];
    let n_dims = ref_surrogate.col_stats.len();

    let r_squared: Vec<f64> = surrogates.iter().map(|s| s.r_squared).collect();

    // ── Initial seeds: normalize the observed-best point per objective ─────
    // col_stats is shared across all surrogates, so use the first surrogate's
    // to_norm_x.
    let seeds: Vec<Vec<f64>> = entries
        .iter()
        .zip(minimize.iter())
        .map(|(e, &min_k)| {
            let best_idx = best_observed_index(e.y, min_k);
            ref_surrogate.to_norm_x(&e.x_matrix[best_idx])
        })
        .collect();

    // ── Run NSGA-II ──────────────────────────────────────────────────
    let signs: Vec<f64> = minimize
        .iter()
        .map(|&m| if m { 1.0 } else { -1.0 })
        .collect();
    let raw_front = optimizers::multi_objective_nsga2(&surrogates, &signs, &seeds);

    // ── Post-process front points ───────────────────────────────────
    // Remove duplicate genomes (within 1e-9 across every dimension).
    let mut deduped: Vec<(Vec<f64>, Vec<f64>)> = Vec::new();
    'outer: for (genome, fitness) in raw_front {
        for (existing, _) in &deduped {
            if genome
                .iter()
                .zip(existing.iter())
                .all(|(a, b)| (a - b).abs() < 1e-9)
            {
                continue 'outer;
            }
        }
        deduped.push((genome, fitness));
    }

    // Clamp each point's genome to [0,1] and compute the surrogate-predicted
    // value (original units) for every objective.
    let mut front_points: Vec<ParetoFrontPoint> = deduped
        .into_iter()
        .map(|(genome, _)| {
            let clamped: Vec<f64> = genome.iter().map(|v| v.clamp(0.0, 1.0)).collect();
            let params = ref_surrogate.to_original_x(&clamped);
            let values: Vec<f64> = surrogates
                .iter()
                .map(|s| s.to_original_y(s.predict_norm(&clamped)))
                .collect();
            ParetoFrontPoint { params, values }
        })
        .collect();

    // Sort ascending by the first objective's value.
    front_points.sort_by(|a, b| {
        a.values[0]
            .partial_cmp(&b.values[0])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // ── Slices ───────────────────────────────────────────────────────
    let slices = if let Some((px, py)) = slice_params {
        // Balance point: the point closest to the ideal point in normalized
        // objective space. Ideal point = the sign-adjusted minimum of each
        // objective (NSGA-II's minimization direction).
        if front_points.is_empty() || px >= n_dims || py >= n_dims || px == py {
            Vec::new()
        } else {
            // Ideal and nadir points in the sign-adjusted (minimization) frame.
            let ideal: Vec<f64> = (0..n_obj)
                .map(|k| {
                    front_points
                        .iter()
                        .map(|p| signs[k] * p.values[k])
                        .fold(f64::INFINITY, f64::min)
                })
                .collect();
            let nadir: Vec<f64> = (0..n_obj)
                .map(|k| {
                    front_points
                        .iter()
                        .map(|p| signs[k] * p.values[k])
                        .fold(f64::NEG_INFINITY, f64::max)
                })
                .collect();
            // Per-objective range, used to normalize each objective to [0, 1]
            // before measuring distance. Without this the Euclidean distance is
            // dominated by whichever objective has the larger numeric magnitude
            // (e.g. one in [0, 1000] vs one in [0, 1]), so the chosen "balance"
            // point would not actually be balanced. Guard the degenerate
            // zero-range case.
            let ranges: Vec<f64> = (0..n_obj)
                .map(|k| (nadir[k] - ideal[k]).max(1e-12))
                .collect();

            // Find the normalized parameters of the balance point (the point
            // closest to the ideal point in normalized objective space).
            let ideal_dist = |p: &ParetoFrontPoint| -> f64 {
                (0..n_obj)
                    .map(|k| ((signs[k] * p.values[k] - ideal[k]) / ranges[k]).powi(2))
                    .sum()
            };
            let balance_norm = front_points
                .iter()
                .min_by(|a, b| {
                    ideal_dist(a)
                        .partial_cmp(&ideal_dist(b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|p| ref_surrogate.to_norm_x(&p.params))
                .unwrap_or_else(|| vec![0.5; n_dims]);

            // Build a slice for each objective.
            surrogates
                .iter()
                .filter_map(|s| build_slice(s, &balance_norm, px, py, n_grid.max(2), n_dims))
                .collect()
        }
    } else {
        Vec::new()
    };

    SurrogateMultiOptResult {
        front: front_points,
        r_squared,
        slices,
    }
}

/// Estimates the Pareto front via NSGA-II against a set of validated fit results.
/// `trained[k]` is the surrogate for objective k. Every element must share the
/// same param_names and training-data dimensionality.
pub fn optimize_multi_on_trained(
    trained: &[&TrainedSurrogate],
    spec: &SurrogateMultiOptimizeSpec,
) -> Result<SurrogateMultiOptResult, String> {
    let n_obj = trained.len();
    if n_obj < 2 {
        return Err(format!(
            "At least 2 objectives required (current: {})",
            n_obj
        ));
    }
    if spec.minimize.len() != n_obj {
        return Err("trained and minimize length mismatch".to_string());
    }
    let first = trained[0];
    if trained.iter().any(|t| t.param_names != first.param_names) {
        return Err("trained surrogates have inconsistent param_names".to_string());
    }
    let n_dims = first.surrogate.col_stats.len();
    if trained
        .iter()
        .any(|t| t.surrogate.col_stats.len() != n_dims)
    {
        return Err("trained surrogates have inconsistent dimensions".to_string());
    }

    let entries: Vec<MultiObjectiveEntry<'_>> = trained
        .iter()
        .map(|t| MultiObjectiveEntry {
            surrogate: &t.surrogate,
            x_matrix: &t.x_matrix,
            y: &t.y,
        })
        .collect();

    Ok(run_multi_optimize(
        &entries,
        &spec.minimize,
        spec.slice_params,
        spec.n_grid,
    ))
}

/// Fits multi-objective surrogate models and estimates the Pareto front via
/// NSGA-II.
///
/// Does not depend on a thread-local DataFrame, so it can be called from a
/// background thread.
pub fn run_surrogate_multi_optimization(
    req: &SurrogateMultiOptRequest,
) -> Result<SurrogateMultiOptResult, String> {
    // ── Validation ───────────────────────────────────────────────────
    let n_obj = req.ys.len();
    if n_obj < 2 {
        return Err(format!(
            "At least 2 objectives required (current: {})",
            n_obj
        ));
    }
    if req.objective_names.len() != n_obj {
        return Err("ys and objective_names length mismatch".to_string());
    }
    if req.minimize.len() != n_obj {
        return Err("ys and minimize length mismatch".to_string());
    }

    let n = req.ys[0].len();
    if n < MIN_TRIALS_FOR_SURROGATE_OPT {
        return Err(format!(
            "At least {} trials required (current: {})",
            MIN_TRIALS_FOR_SURROGATE_OPT, n
        ));
    }
    for (k, yk) in req.ys.iter().enumerate() {
        if yk.len() != n {
            return Err(format!(
                "ys[{}] length {} does not match ys[0] length {}",
                k,
                yk.len(),
                n
            ));
        }
    }
    if req.x_matrix.len() != n {
        return Err("x_matrix and y length mismatch".to_string());
    }
    let n_dims = req.x_matrix.first().map(|r| r.len()).unwrap_or(0);
    if n_dims == 0 {
        return Err("No numeric parameters available".to_string());
    }
    if req.x_matrix.iter().any(|row| row.len() != n_dims) {
        return Err("x_matrix rows have inconsistent dimensions".to_string());
    }
    if req.x_matrix.iter().flatten().any(|v| !v.is_finite())
        || req.ys.iter().flatten().any(|v| !v.is_finite())
    {
        return Err("Input contains non-finite values".to_string());
    }

    // ── Fit a surrogate for each objective ──────────────────────────
    let surrogates: Vec<models::FittedSurrogate> = req
        .ys
        .iter()
        .map(|yk| models::fit_surrogate(req.model, &req.x_matrix, yk))
        .collect::<Result<Vec<_>, _>>()?;

    let entries: Vec<MultiObjectiveEntry<'_>> = surrogates
        .iter()
        .zip(req.ys.iter())
        .map(|(surrogate, yk)| MultiObjectiveEntry {
            surrogate,
            x_matrix: &req.x_matrix,
            y: yk,
        })
        .collect();

    Ok(run_multi_optimize(
        &entries,
        &req.minimize,
        req.slice_params,
        req.n_grid,
    ))
}

/// Fits a multi-objective surrogate for each objective (with Pareto-front
/// concentration).
///
/// `objective_values[k]` is the column for objective k (length N); `minimize[k]`
/// is its optimization direction. Reassembles all objectives into row vectors,
/// finds the non-dominated (rank == 0) trials via `nd_sort`, and prioritizes
/// them as inducing points for every GP (`SurrogateFitRequest.priority_rows`).
///
/// Front concentration only changes the model when N exceeds the GP's
/// inducing-point cap (100). For N <= 100 each GP uses Z = X (all points), so
/// the priority setting has no effect on the result.
pub fn fit_multi_surrogates(
    x_matrix: &[Vec<f64>],
    objective_values: &[Vec<f64>],
    param_names: &[String],
    objective_names: &[String],
    model: SurrogateModelKind,
    minimize: &[bool],
) -> Result<Vec<TrainedSurrogate>, String> {
    fit_multi_surrogates_tracked(
        x_matrix,
        objective_values,
        param_names,
        objective_names,
        model,
        minimize,
        None,
        &FitProgress::default(),
    )
}

/// Same as [`fit_multi_surrogates`], but supports progress reporting and
/// cancellation via `progress` (used by background training from the UI).
/// Progress is expressed as the total fit count across every objective, and
/// the label shows the objective name while fitting objective k.
#[allow(clippy::too_many_arguments)]
pub fn fit_multi_surrogates_tracked(
    x_matrix: &[Vec<f64>],
    objective_values: &[Vec<f64>],
    param_names: &[String],
    objective_names: &[String],
    model: SurrogateModelKind,
    minimize: &[bool],
    param_bounds: Option<&[Option<(f64, f64)>]>,
    progress: &FitProgress,
) -> Result<Vec<TrainedSurrogate>, String> {
    let n_obj = objective_values.len();
    if n_obj != objective_names.len() || n_obj != minimize.len() {
        return Err(
            "objective_values, objective_names and minimize must have equal length".to_string(),
        );
    }
    if n_obj == 0 {
        return Err("At least 1 objective required".to_string());
    }
    let n = x_matrix.len();
    for (k, col) in objective_values.iter().enumerate() {
        if col.len() != n {
            return Err(format!(
                "objective_values[{}] length {} does not match x_matrix rows {}",
                k,
                col.len(),
                n
            ));
        }
    }

    // Subsample large data into a single subset shared across all objectives
    // (using a different subset per objective would make the Pareto front
    // inconsistent). After subsampling, each objective's fit already has
    // N <= cap, so it isn't subsampled a second time. Priority rows (rank 0)
    // are also recomputed on the subsampled set.
    let obj_cols: Vec<&[f64]> = objective_values.iter().map(Vec::as_slice).collect();
    let subset = subsample_indices(&obj_cols, minimize, MAX_TRAIN_FOR_FIT, 42);
    let x_subset: Vec<Vec<f64>>;
    let obj_subset: Vec<Vec<f64>>;
    let (x_matrix, objective_values): (&[Vec<f64>], &[Vec<f64>]) = match &subset {
        Some(idx) => {
            x_subset = take_rows(x_matrix, idx);
            obj_subset = objective_values.iter().map(|c| take_rows(c, idx)).collect();
            (&x_subset, &obj_subset)
        }
        None => (x_matrix, objective_values),
    };
    let n = x_matrix.len();

    // Build the per-row objective vector rows[i][k] and make non-dominated
    // trials (rank == 0) the priority rows.
    let rows: Vec<Vec<f64>> = (0..n)
        .map(|i| objective_values.iter().map(|col| col[i]).collect())
        .collect();
    let ranks = crate::multi_objective::pareto::nd_sort(&rows, minimize);
    let priority: Vec<usize> = ranks
        .iter()
        .enumerate()
        .filter(|(_, &r)| r == 0)
        .map(|(i, _)| i)
        .collect();

    // Total progress: each objective fits (1 holdout + k CV) + 1 final model.
    // Each objective's req has auto_select=false and no constraints, so this
    // matches estimate_fit_count.
    let per_obj = (1 + n.min(5)) + 1;
    progress.set_total(n_obj * per_obj);

    let mut trained = Vec::with_capacity(n_obj);
    for k in 0..n_obj {
        let req = SurrogateFitRequest {
            x_matrix: x_matrix.to_vec(),
            y: objective_values[k].clone(),
            param_names: param_names.to_vec(),
            objective_name: objective_names[k].clone(),
            model,
            auto_select: false,
            constraints: vec![],
            priority_rows: priority.clone(),
            param_bounds: param_bounds.map(|b| b.to_vec()),
        };
        // The training data is already subsampled (N <= cap), so call the core
        // directly, skipping subsampling and set_total (each objective's
        // inc_done accumulates on the shared handle).
        let prefix = format!("Objective {}/{} ({}): ", k + 1, n_obj, objective_names[k]);
        let t = fit_validated_inner(&req, progress, &prefix).map_err(|e| {
            format!(
                "Fitting failed for objective '{}': {}",
                objective_names[k], e
            )
        })?;
        trained.push(t);
    }
    Ok(trained)
}
