use super::helpers::{add_to_pareto_front, compute_ref_point, normalize_objectives};
use super::types::HvHistoryResult;

/// N次元ハイパーボリューム（最小化前提・厳密値）。
///
/// ref_point より全次元で厳密に小さい点のみ有効とする。m=1/2 は専用の高速パス、
/// m>=3 は WFG アルゴリズム (While, Bradstreet, Barone 2012) で計算する。
/// 入力に支配される点や重複点が含まれていてもよい（内部で非支配集合に縮約する）。
/// 手法の詳細は theory/ja/optimization/hypervolume.md を参照。
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

    // WFG の再帰コストは点数に対して増えるため、先に非支配集合へ縮約する。
    // 最後の目的の昇順ソートは limitset 内の支配点を増やし枝刈りを効かせるための
    // ヒューリスティック（正しさはソート順に依存しない）。
    let mut front: Vec<Vec<f64>> = Vec::new();
    for p in valid {
        add_to_pareto_front(&mut front, p);
    }
    front.sort_by(|a, b| {
        a[m - 1]
            .partial_cmp(&b[m - 1])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    wfg(&front, ref_point)
}

/// WFG 本体: 非支配集合の HV を点ごとの排他的寄与 exclhv の和として計算する。
/// 点数 0/1/2 は閉形式で打ち切り、再帰を浅くする。
fn wfg(front: &[Vec<f64>], ref_point: &[f64]) -> f64 {
    match front.len() {
        0 => 0.0,
        1 => inclhv(&front[0], ref_point),
        2 => {
            // 包除原理: |A∪B| = |A| + |B| − |A∩B|。共通部分は成分ごとの max。
            let joint: Vec<f64> = front[0]
                .iter()
                .zip(front[1].iter())
                .map(|(a, b)| a.max(*b))
                .collect();
            inclhv(&front[0], ref_point) + inclhv(&front[1], ref_point) - inclhv(&joint, ref_point)
        }
        _ => (0..front.len()).map(|i| exclhv(front, i, ref_point)).sum(),
    }
}

/// 点 p 単独の包含 HV: Π_k (ref_k − p_k)。
fn inclhv(p: &[f64], ref_point: &[f64]) -> f64 {
    p.iter()
        .zip(ref_point.iter())
        .map(|(pi, ri)| ri - pi)
        .product()
}

/// front[i] の排他的寄与: inclhv(front[i]) から後続点 front[i+1..] が front[i] の
/// box 内に落とす「影」(limitset = 成分ごとの max) の HV を引く。
/// 影は非支配集合へ縮約してから再帰する（WFG の中核となる枝刈り）。
fn exclhv(front: &[Vec<f64>], i: usize, ref_point: &[f64]) -> f64 {
    let p = &front[i];
    let mut limit: Vec<Vec<f64>> = Vec::new();
    for q in &front[i + 1..] {
        let shadow: Vec<f64> = p.iter().zip(q.iter()).map(|(pi, qi)| pi.max(*qi)).collect();
        add_to_pareto_front(&mut limit, shadow);
    }
    inclhv(p, ref_point) - wfg(&limit, ref_point)
}

/// 2次元ハイパーボリューム（最小化前提・厳密値）。
///
/// ref 点より両次元で厳密に小さい点のみ有効とする。入力に支配される点や
/// 重複点が含まれていてもよい（内部で非支配フロントへ縮約してから
/// x 昇順の区間和で計算する）。
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
    pts.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    });

    // 区間和は「x 昇順で y が厳密に減少する非支配フロント」を前提とするため、
    // 支配される点・重複点をここで除去する。縮約しないと支配点の帯が
    // 二重にカウントされ HV が過大になる。
    let mut front: Vec<(f64, f64)> = Vec::with_capacity(pts.len());
    for &(x, y) in &pts {
        if front.last().is_none_or(|&(_, last_y)| y < last_y) {
            front.push((x, y));
        }
    }

    let mut hv = 0.0f64;
    for i in 0..front.len() {
        let next_x = if i + 1 < front.len() {
            front[i + 1].0
        } else {
            ref_x
        };
        let width = next_x - front[i].0;
        let height = ref_y - front[i].1;
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

#[cfg(test)]
mod wfg_tests {
    use super::*;

    /// 旧・再帰スライス実装（WFG 導入前の本番コード）。WFG の検証用リファレンス
    /// としてテスト内にのみ残す。概算 O(n^m) のため小規模入力専用。
    fn hypervolume_nd_slicing(points: &[Vec<f64>], ref_point: &[f64]) -> f64 {
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
                let mut proj_front: Vec<Vec<f64>> = Vec::new();
                for p in &sorted[..=i] {
                    add_to_pareto_front(&mut proj_front, p[..last].to_vec());
                }
                hv += thickness * hypervolume_nd_slicing(&proj_front, &ref_point[..last]);
            }
            prev = sorted[i][last];
        }
        hv
    }

    /// 決定的な擬似乱数 (LCG)。テスト再現性のためシード固定。
    fn lcg_next(state: &mut u64) -> f64 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (*state >> 11) as f64 / (1u64 << 53) as f64
    }

    /// WFG がリファレンス実装（旧スライス法）と一致することをランダム前面で検証する。
    #[test]
    fn wfg_matches_slicing_reference_on_random_fronts() {
        for m in [3usize, 4, 5] {
            for n in [1usize, 2, 3, 5, 8, 12] {
                let mut seed = (m * 1000 + n) as u64;
                let points: Vec<Vec<f64>> = (0..n)
                    .map(|_| (0..m).map(|_| lcg_next(&mut seed)).collect())
                    .collect();
                let ref_pt = vec![1.1; m];
                let a = hypervolume_nd(&points, &ref_pt);
                let b = hypervolume_nd_slicing(&points, &ref_pt);
                assert!((a - b).abs() < 1e-9, "m={m} n={n}: wfg={a} slicing={b}");
            }
        }
    }

    /// 手計算検証: 点 (0,1,1), (1,0,0)、参照点 (1.1, 1.1, 1.1)。
    /// 包除原理: 1.1·0.1·0.1 + 0.1·1.1·1.1 − 0.1³ = 0.011 + 0.121 − 0.001 = 0.131
    #[test]
    fn wfg_3d_two_points_hand_computed() {
        let pts = vec![vec![0.0, 1.0, 1.0], vec![1.0, 0.0, 0.0]];
        let ref_pt = vec![1.1, 1.1, 1.1];
        let hv = hypervolume_nd(&pts, &ref_pt);
        assert!((hv - 0.131).abs() < 1e-12, "HV = {hv}, expected 0.131");
    }

    /// 支配される点を混ぜても HV は変わらない（内部で非支配集合へ縮約するため）。
    #[test]
    fn wfg_unaffected_by_dominated_points() {
        let front = vec![vec![0.0, 1.0, 1.0], vec![1.0, 0.0, 0.0]];
        let mut with_dominated = front.clone();
        with_dominated.push(vec![1.05, 1.05, 1.05]); // 両点に支配される
        let ref_pt = vec![1.1, 1.1, 1.1];
        let a = hypervolume_nd(&front, &ref_pt);
        let b = hypervolume_nd(&with_dominated, &ref_pt);
        assert!((a - b).abs() < 1e-12, "a={a} b={b}");
    }

    /// 重複点は 1 回だけ数えられる。
    #[test]
    fn wfg_duplicate_points_counted_once() {
        let front = vec![vec![0.2, 0.8, 0.5], vec![0.8, 0.2, 0.5]];
        let mut with_dup = front.clone();
        with_dup.push(vec![0.2, 0.8, 0.5]);
        let ref_pt = vec![1.0, 1.0, 1.0];
        let a = hypervolume_nd(&front, &ref_pt);
        let b = hypervolume_nd(&with_dup, &ref_pt);
        assert!((a - b).abs() < 1e-12, "a={a} b={b}");
    }

    /// 単一点の HV は包含 HV（box 体積）に一致する。
    #[test]
    fn wfg_single_point_is_box_volume() {
        let pts = vec![vec![0.25, 0.5, 0.75, 0.5]];
        let ref_pt = vec![1.0, 1.0, 1.0, 1.0];
        let hv = hypervolume_nd(&pts, &ref_pt);
        let expected = 0.75 * 0.5 * 0.25 * 0.5;
        assert!((hv - expected).abs() < 1e-12, "HV = {hv}");
    }
}
