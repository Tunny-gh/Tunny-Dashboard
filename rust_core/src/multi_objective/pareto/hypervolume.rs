use super::helpers::{add_to_pareto_front, compute_ref_point, normalize_objectives};
use super::types::HvHistoryResult;

/// N次元ハイパーボリューム（再帰スライスアルゴリズム）
///
/// ref_point より小さい点のみ有効とし、最後の次元でスライスして再帰計算する。
pub fn hypervolume_nd(points: &[Vec<f64>], ref_point: &[f64]) -> f64 {
    let m = ref_point.len();
    if points.is_empty() || m == 0 {
        return 0.0;
    }

    let valid: Vec<Vec<f64>> = points
        .iter()
        .filter(|p| p.len() >= m && p.iter().zip(ref_point.iter()).all(|(pi, ri)| *pi < *ri))
        .cloned()
        .collect();

    if valid.is_empty() {
        return 0.0;
    }

    if m == 1 {
        let min_v = valid.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
        return ref_point[0] - min_v;
    }

    if m == 2 {
        let pts_2d: Vec<(f64, f64)> = valid.iter().map(|p| (p[0], p[1])).collect();
        return hypervolume_2d(&pts_2d, ref_point[0], ref_point[1]);
    }

    let last = m - 1;
    let mut sorted = valid;
    sorted.sort_by(|a, b| {
        a[last]
            .partial_cmp(&b[last])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut hv = 0.0f64;
    let mut prev = ref_point[last];

    for i in (0..sorted.len()).rev() {
        let thickness = prev - sorted[i][last];
        if thickness > 0.0 {
            let proj_nd = nd_front_projected(&sorted[..=i], last);
            let ref_proj = &ref_point[..last];
            hv += thickness * hypervolume_nd(&proj_nd, ref_proj);
        }
        prev = sorted[i][last];
    }

    hv
}

fn nd_front_projected(points: &[Vec<f64>], drop_dim: usize) -> Vec<Vec<f64>> {
    let mut front: Vec<Vec<f64>> = Vec::new();
    for p in points {
        add_to_pareto_front(&mut front, p[..drop_dim].to_vec());
    }
    front
}

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
    compute_hv_history_with_ref(trial_ids, objectives, is_minimize, None)
}

/// 参照点を任意指定できる HV 推移計算。
///
/// `ref_point_override` は正規化空間の参照点（最大化目的は符号反転済み）。
/// `None` の場合は観測点の nadir + 10% マージンから自動算出する。
/// 戻り値の `ref_point` には実際に使用した参照点（正規化空間）を入れる。
pub fn compute_hv_history_with_ref(
    trial_ids: &[u32],
    objectives: &[Vec<f64>],
    is_minimize: &[bool],
    ref_point_override: Option<&[f64]>,
) -> HvHistoryResult {
    let n = objectives.len();
    let m = if n > 0 { objectives[0].len() } else { 0 };

    // HV を計算しないケース（単目的・有効点なし）の空結果。
    let empty = || HvHistoryResult {
        trial_ids: trial_ids.to_vec(),
        hv_values: vec![0.0; n],
        ref_point: Vec::new(),
    };

    if m < 2 {
        return empty();
    }

    let norm_all = normalize_objectives(objectives, is_minimize);
    let valid_objs: Vec<Vec<f64>> = norm_all
        .iter()
        .filter(|obj| !obj.iter().any(|v| v.is_nan()))
        .cloned()
        .collect();
    if valid_objs.is_empty() {
        return empty();
    }
    // 指定があり次元が一致し全要素有限ならそれを使う。さもなくば自動算出。
    let ref_pt = match ref_point_override {
        Some(r) if r.len() == m && r.iter().all(|v| v.is_finite()) => r.to_vec(),
        _ => compute_ref_point(&valid_objs, m),
    };

    let mut current_pareto: Vec<Vec<f64>> = Vec::new();
    let mut hv_values = Vec::with_capacity(n);

    for obj in norm_all.iter().take(n) {
        if obj.iter().any(|v| v.is_nan()) {
            hv_values.push(hv_values.last().copied().unwrap_or(0.0));
            continue;
        }
        add_to_pareto_front(&mut current_pareto, obj.clone());
        hv_values.push(hypervolume_nd(&current_pareto, &ref_pt));
    }

    HvHistoryResult {
        trial_ids: trial_ids.to_vec(),
        hv_values,
        ref_point: ref_pt,
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
        ref_point: vec![],
    })
}
