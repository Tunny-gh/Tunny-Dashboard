/// 最小化前提のパレート支配判定。
///
/// `a` が `b` を支配する（全目的で a <= b かつ少なくとも 1 目的で a < b）
/// とき true。NaN を含む次元は両方向の比較が false になるため「等価」として
/// スキップされ、残りの次元だけで判定される（呼び出し側は原則として事前に
/// 有限値へフィルタしている前提）。
pub(crate) fn dominates_minimized(a: &[f64], b: &[f64]) -> bool {
    let mut strictly_better = false;
    for (&ai, &bi) in a.iter().zip(b.iter()) {
        if ai > bi {
            return false;
        }
        if ai < bi {
            strictly_better = true;
        }
    }
    strictly_better
}

/// 目的値を最小化方向へ統一する（最大化目的は符号反転）。
///
/// `is_minimize` の長さが目的数に満たない場合、不足分は最小化として扱う。
pub(crate) fn normalize_objectives(objectives: &[Vec<f64>], is_minimize: &[bool]) -> Vec<Vec<f64>> {
    objectives
        .iter()
        .map(|obj| {
            obj.iter()
                .enumerate()
                .map(|(j, &v)| {
                    if is_minimize.get(j).copied().unwrap_or(true) {
                        v
                    } else {
                        -v
                    }
                })
                .collect()
        })
        .collect()
}

/// 点をパレートフロントに追加する。支配されていれば何もしない。
pub(crate) fn add_to_pareto_front(front: &mut Vec<Vec<f64>>, point: Vec<f64>) {
    if front.iter().any(|f| dominates_minimized(f, &point)) {
        return;
    }
    front.retain(|f| !dominates_minimized(&point, f));
    front.push(point);
}

/// HV 用参照点の自動算出: nadir + 0.1·(nadir − ideal)。
///
/// マージンを観測範囲に比例させることで目的値のスケールに対して不変になる
/// （旧実装の定数 +1.0 はスケールの小さい study で HV を歪めていた）。
/// 範囲が退化している次元は、値自身の大きさに比例したマージン
/// （|nadir|·0.1、それも 0 なら 1.0）でフォールバックする。
pub(crate) fn compute_ref_point(pareto_objs: &[Vec<f64>], m: usize) -> Vec<f64> {
    let mut nadir = vec![f64::NEG_INFINITY; m];
    let mut ideal = vec![f64::INFINITY; m];
    for obj in pareto_objs {
        for (j, &v) in obj.iter().enumerate() {
            if v > nadir[j] {
                nadir[j] = v;
            }
            if v < ideal[j] {
                ideal[j] = v;
            }
        }
    }
    (0..m)
        .map(|j| {
            let range = nadir[j] - ideal[j];
            let offset = if range > 1e-12 {
                0.1 * range
            } else if nadir[j].abs() > 1e-12 {
                0.1 * nadir[j].abs()
            } else {
                1.0
            };
            nadir[j] + offset
        })
        .collect()
}
