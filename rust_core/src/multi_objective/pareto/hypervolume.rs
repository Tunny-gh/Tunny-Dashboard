use super::helpers::{compute_ref_point, dominates_minimized, normalize_objectives};
use super::types::HvHistoryResult;

/// Documentation.
///
/// Documentation.
/// Documentation.
pub fn hypervolume_2d(pareto_points: &[(f64, f64)], ref_x: f64, ref_y: f64) -> f64 {
    if pareto_points.is_empty() {
        return 0.0;
    }
    let mut pts: Vec<(f64, f64)> = pareto_points
        .iter()
        .filter(|&&(x, y)| x < ref_x && y < ref_y)
        .cloned()
        .collect();
    if pts.is_empty() {
        return 0.0;
    }
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut hv = 0.0f64;
    for i in 0..pts.len() {
        let next_x = if i + 1 < pts.len() {
            pts[i + 1].0
        } else {
            ref_x
        };
        let width = next_x - pts[i].0;
        let height = ref_y - pts[i].1;
        if width > 0.0 && height > 0.0 {
            hv += width * height;
        }
    }
    hv
}

/// Documentation.
pub fn compute_hypervolume_history(is_minimize: &[bool]) -> HvHistoryResult {
    crate::dataframe::with_active_df(|df| {
        let n = df.row_count();
        let obj_names = df.objective_col_names();
        let m = obj_names.len();

        let trial_ids: Vec<u32> = (0..n).filter_map(|i| df.get_trial_id(i)).collect();

        if m < 2 {
            return HvHistoryResult {
                trial_ids,
                hv_values: vec![0.0; n],
            };
        }

        let all_objs: Vec<Vec<f64>> = (0..n)
            .map(|row| {
                obj_names
                    .iter()
                    .map(|name| {
                        df.get_numeric_column(name)
                            .and_then(|col| col.get(row))
                            .copied()
                            .unwrap_or(f64::NAN)
                    })
                    .collect()
            })
            .collect();
        let norm_all = normalize_objectives(&all_objs, is_minimize);
        let valid_objs: Vec<Vec<f64>> = norm_all
            .iter()
            .filter(|obj| !obj.iter().any(|v| v.is_nan()))
            .cloned()
            .collect();
        if valid_objs.is_empty() {
            return HvHistoryResult {
                trial_ids,
                hv_values: vec![0.0; n],
            };
        }
        let ref_pt = compute_ref_point(&valid_objs, m);

        let mut current_pareto: Vec<Vec<f64>> = Vec::new();
        let mut hv_values = Vec::with_capacity(n);

        for row in 0..n {
            let obj = norm_all[row].clone();
            if obj.iter().any(|v| v.is_nan()) {
                hv_values.push(hv_values.last().copied().unwrap_or(0.0));
                continue;
            }
            let dominated = current_pareto.iter().any(|p| dominates_minimized(p, &obj));
            if !dominated {
                current_pareto.retain(|p| !dominates_minimized(&obj, p));
                current_pareto.push(obj);
            }
            let pts_2d: Vec<(f64, f64)> = current_pareto.iter().map(|o| (o[0], o[1])).collect();
            hv_values.push(hypervolume_2d(&pts_2d, ref_pt[0], ref_pt[1]));
        }

        HvHistoryResult {
            trial_ids,
            hv_values,
        }
    })
    .unwrap_or(HvHistoryResult {
        trial_ids: vec![],
        hv_values: vec![],
    })
}
