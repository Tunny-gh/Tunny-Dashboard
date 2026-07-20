//! Axis option generation, axis value extraction, rank-map construction, and
//! scatter point computation for the MCDM scatter chart.

use crate::state::results::McdmResult;
use crate::state::types::StudyView;
use crate::theme::chart_colors::COLOR_MCDM_NONE;
use crate::theme::colormap::ColorMap;
use egui::Color32;

/// Axis identifier constants (shared by `get_axis_options` and `extract_axis_values`)
const AXIS_VIKOR_Q: &str = "VIKOR_Q";
const AXIS_VIKOR_S: &str = "VIKOR_S";
const AXIS_VIKOR_R: &str = "VIKOR_R";
const AXIS_TOPSIS_SCORE: &str = "TOPSIS_Score";
const AXIS_PHI_PLUS: &str = "Phi+";
const AXIS_PHI_MINUS: &str = "Phi-";
const AXIS_PHI_NET: &str = "Phi_Net";

/// Axis selection option
#[derive(Clone, Debug)]
pub(crate) struct AxisOption {
    pub id: String,
    pub label: String,
}

/// Scatter plot computation metadata
#[derive(Clone, Debug)]
pub(crate) struct ScatterMetadata {
    pub total_trials: usize,
    pub compute_time_ms: f64,
}

// ──────────────────────────────────────────────────────────────
// Axis option generation
// ──────────────────────────────────────────────────────────────

/// Generates the available axis options from the MCDM result
pub(crate) fn get_axis_options(mcdm_result: &McdmResult, obj_names: &[String]) -> Vec<AxisOption> {
    let mut options = Vec::with_capacity(obj_names.len() + 5);

    // Objective function options
    for (i, name) in obj_names.iter().enumerate() {
        options.push(AxisOption {
            id: format!("Objective{}", i),
            label: format!("Objective {} ({})", i, name),
        });
    }

    // Score options per MCDM method
    match mcdm_result {
        McdmResult::Vikor(_) => {
            for (id, label) in [
                (AXIS_VIKOR_Q, "VIKOR Q Score"),
                (AXIS_VIKOR_S, "VIKOR S Value"),
                (AXIS_VIKOR_R, "VIKOR R Value"),
            ] {
                options.push(AxisOption {
                    id: id.to_string(),
                    label: label.to_string(),
                });
            }
        }
        McdmResult::Topsis(_) => {
            options.push(AxisOption {
                id: AXIS_TOPSIS_SCORE.to_string(),
                label: "TOPSIS Score".to_string(),
            });
        }
        McdmResult::PrometheeI(_) | McdmResult::PrometheeII(_) => {
            for (id, label) in [
                (AXIS_PHI_PLUS, "Phi+ (Positive Flow)"),
                (AXIS_PHI_MINUS, "Phi- (Negative Flow)"),
                (AXIS_PHI_NET, "Phi Net"),
            ] {
                options.push(AxisOption {
                    id: id.to_string(),
                    label: label.to_string(),
                });
            }
        }
    }

    options
}

// ──────────────────────────────────────────────────────────────
// Axis value extraction
// ──────────────────────────────────────────────────────────────

/// Extracts each trial's value for the given axis identifier
pub(crate) fn extract_axis_values(
    axis_id: &str,
    mcdm_result: &McdmResult,
    view: &StudyView,
    obj_names: &[String],
) -> Result<Vec<f64>, String> {
    // For the objective function "Objective{N}"
    if let Some(idx_str) = axis_id.strip_prefix("Objective") {
        let idx: usize = idx_str
            .parse()
            .map_err(|_| format!("Invalid objective index in axis: '{}'", axis_id))?;
        let obj_name = obj_names
            .get(idx)
            .ok_or_else(|| format!("Objective index {} out of range", idx))?;
        let values = view
            .numeric_column(obj_name)
            .map(|col| col.to_vec())
            .unwrap_or_else(|| vec![f64::NAN; view.row_count()]);
        return Ok(values);
    }

    // Score per MCDM method (independent of view)
    match mcdm_result {
        McdmResult::Vikor(r) => {
            if axis_id == AXIS_VIKOR_Q {
                Ok(r.q_values.clone())
            } else if axis_id == AXIS_VIKOR_S {
                Ok(r.s_values.clone())
            } else if axis_id == AXIS_VIKOR_R {
                Ok(r.r_values.clone())
            } else {
                Err(format!("Unknown axis '{}' for VIKOR result", axis_id))
            }
        }
        McdmResult::Topsis(r) => {
            if axis_id == AXIS_TOPSIS_SCORE {
                Ok(r.scores.clone())
            } else {
                Err(format!("Unknown axis '{}' for TOPSIS result", axis_id))
            }
        }
        McdmResult::PrometheeI(r) | McdmResult::PrometheeII(r) => {
            if axis_id == AXIS_PHI_PLUS {
                Ok(r.phi_plus.clone())
            } else if axis_id == AXIS_PHI_MINUS {
                Ok(r.phi_minus.clone())
            } else if axis_id == AXIS_PHI_NET {
                Ok(r.phi_net.clone())
            } else {
                Err(format!("Unknown axis '{}' for PROMETHEE result", axis_id))
            }
        }
    }
}

/// Builds a `trial_idx -> rank` reverse lookup map (shared by the 2D/3D scatter plots ·
/// D-6). Since `ranked_indices[rank] = trial_idx`, a reverse lookup is needed.
pub(crate) fn build_rank_map(ranked_indices: &[u32], n_trials: usize) -> Vec<usize> {
    let mut rank_map = vec![usize::MAX; n_trials];
    for (rank, &trial_idx) in ranked_indices.iter().enumerate() {
        let idx = trial_idx as usize;
        if idx < n_trials {
            rank_map[idx] = rank;
        }
    }
    rank_map
}

/// MCDM rank -> scatter point color (shared by the 2D/3D scatter plots · D-6).
/// If `rank` (the value from `build_rank_map`; `usize::MAX` when outside the ranking) is
/// less than `colored_range`, returns a continuous colormap color (rank 0 = best -> t=1.0);
/// otherwise returns gray (`COLOR_MCDM_NONE`).
pub(crate) fn mcdm_rank_color(rank: usize, colored_range: usize, colormap: &ColorMap) -> Color32 {
    if rank == usize::MAX || rank >= colored_range {
        COLOR_MCDM_NONE()
    } else {
        let t = if colored_range > 1 {
            1.0 - rank as f32 / (colored_range - 1) as f32
        } else {
            1.0
        };
        colormap.interpolate(t)
    }
}

/// Returns the default axis ID to use when axis selection becomes invalid (shared by the
/// 2D/3D scatter plots · D-6). The `nth` option, or the first one if unavailable, or an
/// empty string if there are none at all.
pub(crate) fn fallback_axis_id(options: &[AxisOption], nth: usize) -> String {
    options
        .get(nth)
        .or_else(|| options.first())
        .map(|o| o.id.clone())
        .unwrap_or_default()
}

// ──────────────────────────────────────────────────────────────
// Scatter point computation
// ──────────────────────────────────────────────────────────────

/// A single scatter plot point: (x coordinate, y coordinate, color, trial_id).
/// `trial_id` is used to determine graying-out for the selection filter (PCP brush, etc.).
pub(super) type ScatterPoint = (f64, f64, Color32, u32);
/// Return type alias for `compute_scatter_points`.
type ScatterPointsResult = (Vec<ScatterPoint>, Vec<(f64, f64)>, ScatterMetadata);

/// Computes the MCDM scatter plot points
/// - Extract axis values -> continuous coloring via colormap
/// - Return value: (feasible points, infeasible points, metadata)
pub(crate) fn compute_scatter_points(
    mcdm_result: &McdmResult,
    view: &StudyView,
    obj_names: &[String],
    x_axis: &str,
    y_axis: &str,
    colormap: &ColorMap,
    top_n: usize,
) -> Result<ScatterPointsResult, String> {
    let n_trials = view.row_count();
    if n_trials == 0 {
        return Ok((
            vec![],
            vec![],
            ScatterMetadata {
                total_trials: 0,
                compute_time_ms: 0.0,
            },
        ));
    }

    let x_vals = extract_axis_values(x_axis, mcdm_result, view, obj_names)?;
    let y_vals = extract_axis_values(y_axis, mcdm_result, view, obj_names)?;
    let feas = view.feasibility();

    let ranked = mcdm_result.ranked_indices();
    let rank_map = build_rank_map(ranked, n_trials);
    // Assign color contours within the top_n range, ensuring at least 1
    let colored_range = top_n.max(1);

    let mut feasible_pts: Vec<ScatterPoint> = Vec::with_capacity(n_trials);
    let mut infeasible_pts: Vec<(f64, f64)> = Vec::new();

    for (i, &rank) in rank_map.iter().enumerate() {
        let x = match x_vals.get(i).copied() {
            Some(v) if v.is_finite() => v,
            _ => continue,
        };
        let y = match y_vals.get(i).copied() {
            Some(v) if v.is_finite() => v,
            _ => continue,
        };

        if !feas.is_feasible(i) {
            infeasible_pts.push((x, y));
            continue;
        }
        // Rank -> color (colormap within top_n, gray outside; shared with 3D · D-6)
        let color = mcdm_rank_color(rank, colored_range, colormap);
        let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
        feasible_pts.push((x, y, color, trial_id));
    }

    let total = feasible_pts.len() + infeasible_pts.len();
    Ok((
        feasible_pts,
        infeasible_pts,
        ScatterMetadata {
            total_trials: total,
            compute_time_ms: 0.0,
        },
    ))
}
