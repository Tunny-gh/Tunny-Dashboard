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

/// スレッドローカルを使わずデータを直接受け取って HV 推移を計算する。
/// バックグラウンドスレッドから呼び出す場合はこちらを使用する。
pub fn compute_hv_history_from_data(
    trial_ids: &[u32],
    objectives: &[Vec<f64>],
    is_minimize: &[bool],
) -> HvHistoryResult {
    let n = objectives.len();
    let m = if n > 0 { objectives[0].len() } else { 0 };

    if m < 2 {
        return HvHistoryResult {
            trial_ids: trial_ids.to_vec(),
            hv_values: vec![0.0; n],
        };
    }

    let norm_all = normalize_objectives(objectives, is_minimize);
    let valid_objs: Vec<Vec<f64>> = norm_all
        .iter()
        .filter(|obj| !obj.iter().any(|v| v.is_nan()))
        .cloned()
        .collect();
    if valid_objs.is_empty() {
        return HvHistoryResult {
            trial_ids: trial_ids.to_vec(),
            hv_values: vec![0.0; n],
        };
    }
    let ref_pt = compute_ref_point(&valid_objs, m);

    let mut current_pareto: Vec<Vec<f64>> = Vec::new();
    let mut hv_values = Vec::with_capacity(n);

    for obj in norm_all.iter().take(n) {
        if obj.iter().any(|v| v.is_nan()) {
            hv_values.push(hv_values.last().copied().unwrap_or(0.0));
            continue;
        }
        let dominated = current_pareto.iter().any(|p| dominates_minimized(p, obj));
        if !dominated {
            current_pareto.retain(|p| !dominates_minimized(obj, p));
            current_pareto.push(obj.clone());
        }
        let pts_2d: Vec<(f64, f64)> = current_pareto.iter().map(|o| (o[0], o[1])).collect();
        hv_values.push(hypervolume_2d(&pts_2d, ref_pt[0], ref_pt[1]));
    }

    HvHistoryResult {
        trial_ids: trial_ids.to_vec(),
        hv_values,
    }
}

/// Documentation.
pub fn compute_hypervolume_history(is_minimize: &[bool]) -> HvHistoryResult {
    crate::dataframe::with_active_df(|df| {
        let n = df.row_count();
        let obj_names = df.objective_col_names();
        let trial_ids: Vec<u32> = (0..n).filter_map(|i| df.get_trial_id(i)).collect();
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
        compute_hv_history_from_data(&trial_ids, &all_objs, is_minimize)
    })
    .unwrap_or(HvHistoryResult {
        trial_ids: vec![],
        hv_values: vec![],
    })
}
