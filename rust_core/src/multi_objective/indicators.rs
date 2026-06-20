//! 多目的最適化の全体評価指標（収束指標）。
//!
//! Hypervolume に加えて IGD+ / additive ε-indicator / R2 indicator を提供する。
//! これらの指標は多目的（目的数 >= 2）でのみ収束の尺度として意味を持ち、
//! 単目的では定義しない。
//!
//! ## 参照集合の共有
//!
//! IGD+ / ε / R2 は「真のパレート前面」を必要とするが、単一 Study の結果分析では
//! 真の前面は未知である。本実装では **全系列（基準 Study + 比較 Study）の観測点の和集合の
//! 非支配前面** を参照集合として固定し、各試行ステップでそこへの収束を測る
//! （自己参照型の収束分析）。参照集合と正規化スケールを全系列で共有することで、
//! 複数 Study を統一された指標で比較できる。
//!
//! ## 空間
//!
//! - すべて最小化方向に統一した正規化空間で計算する（最大化目的は符号反転）。
//! - IGD+ / ε / R2 は和集合の ideal/nadir で各目的を [0, 1] にスケールしてスケール不変にする。
//! - Hypervolume は既存実装との整合と参照点指定の単位を保つため、正規化（符号反転のみ）空間で
//!   計算し、参照点は全系列共有の nadir から算出する。

use super::pareto::{add_to_pareto_front, compute_ref_point, hypervolume_nd, normalize_objectives};

/// 全体評価指標の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoIndicator {
    /// Hypervolume（大きいほど良い）。
    Hypervolume,
    /// IGD+（小さいほど良い）。
    IgdPlus,
    /// additive ε-indicator（小さいほど良い）。
    Epsilon,
    /// R2 indicator（重み付き Tchebycheff、ideal 基準。小さいほど良い）。
    R2,
}

impl MoIndicator {
    /// すべての指標（UI のセレクタ列挙用）。
    pub fn all() -> [MoIndicator; 4] {
        [
            MoIndicator::Hypervolume,
            MoIndicator::IgdPlus,
            MoIndicator::Epsilon,
            MoIndicator::R2,
        ]
    }

    /// 表示名。
    pub fn label(self) -> &'static str {
        match self {
            MoIndicator::Hypervolume => "Hypervolume",
            MoIndicator::IgdPlus => "IGD+",
            MoIndicator::Epsilon => "ε-indicator",
            MoIndicator::R2 => "R2",
        }
    }

    /// 値が大きいほど良いか（Hypervolume のみ true）。
    pub fn higher_is_better(self) -> bool {
        matches!(self, MoIndicator::Hypervolume)
    }

    /// 参照集合（真のパレート前面の近似）を必要とするか。
    /// Hypervolume は参照「点」のみで計算でき、参照「集合」は不要。
    pub fn needs_reference_set(self) -> bool {
        !matches!(self, MoIndicator::Hypervolume)
    }
}

/// 1 系列分の入力（基準 Study または 1 つの比較 Study）。
/// `objectives` は試行順の目的値ベクトル（生の目的値。符号反転前）。
pub struct SeriesInput<'a> {
    /// 各点の trial_id（`objectives` と同じ順序・要素数）。
    pub trial_ids: &'a [u32],
    /// 試行順の目的値ベクトル。
    pub objectives: &'a [Vec<f64>],
}

/// 1 系列分の指標推移。
#[derive(Debug, Clone)]
pub struct IndicatorHistory {
    /// 各点の trial_id。
    pub trial_ids: Vec<u32>,
    /// 指標値の推移（`trial_ids` と同じ要素数）。
    pub values: Vec<f64>,
    /// Hypervolume 計算に使用した参照点（正規化最小化空間・全系列共有）。
    /// HV 以外、または計算不能時は空。
    pub ref_point: Vec<f64>,
}

/// 全系列に対して、共有参照集合・共有スケールで指標推移を計算する。
///
/// 戻り値は `series` と同じ順序・要素数。目的数が 2 未満の場合は各系列で
/// 空の推移（`values` 空）を返す（指標は単目的では未定義）。
///
/// `hv_ref_point_override` は HV 専用の参照点（正規化最小化空間。最大化目的は符号反転済み）。
/// 次元が一致し全要素有限のときのみ使用し、それ以外は共有 nadir から自動算出する。
pub fn compute_indicator_histories(
    series: &[SeriesInput],
    is_minimize: &[bool],
    indicator: MoIndicator,
    hv_ref_point_override: Option<&[f64]>,
) -> Vec<IndicatorHistory> {
    let m = is_minimize.len();

    let empty_result = || -> Vec<IndicatorHistory> {
        series
            .iter()
            .map(|s| IndicatorHistory {
                trial_ids: s.trial_ids.to_vec(),
                values: Vec::new(),
                ref_point: Vec::new(),
            })
            .collect()
    };

    if m < 2 {
        return empty_result();
    }

    // 各系列を最小化方向へ正規化（符号反転のみ）。
    let normalized: Vec<Vec<Vec<f64>>> = series
        .iter()
        .map(|s| normalize_objectives(s.objectives, is_minimize))
        .collect();

    // 有効点（NaN を含まない・次元一致）の和集合を集める。
    let mut union_valid: Vec<Vec<f64>> = Vec::new();
    for norm in &normalized {
        for obj in norm {
            if obj.len() == m && !obj.iter().any(|v| v.is_nan() || v.is_infinite()) {
                union_valid.push(obj.clone());
            }
        }
    }
    if union_valid.is_empty() {
        return empty_result();
    }

    // 全系列共有の参照前面（和集合の非支配集合）。
    let mut reference_front: Vec<Vec<f64>> = Vec::new();
    for p in &union_valid {
        add_to_pareto_front(&mut reference_front, p.clone());
    }

    // 全系列共有の ideal / nadir（[0,1] スケール用。和集合の全点から算出）。
    let mut ideal = vec![f64::INFINITY; m];
    let mut nadir = vec![f64::NEG_INFINITY; m];
    for p in &union_valid {
        for j in 0..m {
            if p[j] < ideal[j] {
                ideal[j] = p[j];
            }
            if p[j] > nadir[j] {
                nadir[j] = p[j];
            }
        }
    }
    let scale: Vec<f64> = (0..m)
        .map(|j| {
            let r = nadir[j] - ideal[j];
            if r > 0.0 {
                r
            } else {
                1.0
            }
        })
        .collect();
    let to_unit =
        |p: &[f64]| -> Vec<f64> { (0..m).map(|j| (p[j] - ideal[j]) / scale[j]).collect() };

    // HV 用参照点（共有 nadir + 10% マージン、または指定値）。
    // nadir は全有効点 `union_valid` の最悪点から算出する。参照前面（非支配集合）の
    // nadir を使うと参照点ボックスが良い解の境界に張り付き、序盤の劣った試行が
    // `p[j] < ref[j]` を満たせず HV 寄与 0 になる（推移が終端で突然立ち上がる）。
    let hv_ref_point: Vec<f64> = match hv_ref_point_override {
        Some(r) if r.len() == m && r.iter().all(|v| v.is_finite()) => r.to_vec(),
        _ => compute_ref_point(&union_valid, m),
    };

    // 参照集合を [0,1] へスケール（IGD+ / ε で使用）。
    let reference_unit: Vec<Vec<f64>> = reference_front.iter().map(|p| to_unit(p)).collect();

    // R2 用の重みベクトル（指標が R2 のときのみ生成）。
    let weights = if matches!(indicator, MoIndicator::R2) {
        simplex_lattice_weights(m)
    } else {
        Vec::new()
    };

    // 各系列で試行順に前面を蓄積し、ステップごとに指標を計算する。
    series
        .iter()
        .zip(normalized.iter())
        .map(|(s, norm)| {
            let n = norm.len();
            let mut current_front: Vec<Vec<f64>> = Vec::new();
            let mut values = Vec::with_capacity(n);

            for obj in norm.iter() {
                let invalid = obj.len() != m || obj.iter().any(|v| v.is_nan() || v.is_infinite());
                if invalid {
                    // 無効点は直前値を引き継ぐ（HV 履歴と同じ振る舞い）。
                    values.push(values.last().copied().unwrap_or(0.0));
                    continue;
                }
                add_to_pareto_front(&mut current_front, obj.clone());

                let v = match indicator {
                    MoIndicator::Hypervolume => hypervolume_nd(&current_front, &hv_ref_point),
                    _ => {
                        // [0,1] 空間の前面を同期して再構築し、参照集合と比較する。
                        let current_unit: Vec<Vec<f64>> =
                            current_front.iter().map(|p| to_unit(p)).collect();
                        match indicator {
                            MoIndicator::IgdPlus => igd_plus(&current_unit, &reference_unit),
                            MoIndicator::Epsilon => {
                                additive_epsilon(&current_unit, &reference_unit)
                            }
                            MoIndicator::R2 => r2_indicator(&current_unit, &weights),
                            MoIndicator::Hypervolume => unreachable!(),
                        }
                    }
                };
                values.push(v);
            }

            IndicatorHistory {
                trial_ids: s.trial_ids.to_vec(),
                values,
                ref_point: if matches!(indicator, MoIndicator::Hypervolume) {
                    hv_ref_point.clone()
                } else {
                    Vec::new()
                },
            }
        })
        .collect()
}

/// IGD+（inverted generational distance plus）。
///
/// 参照集合 `reference` の各点 z について、近似集合 `approx` 内の点 a への
/// 修正距離 d+(a, z) = sqrt(Σ max(a_j - z_j, 0)^2) の最小値を取り、その平均。
/// 最小化前提の [0,1] 空間で計算する。小さいほど良い。
pub fn igd_plus(approx: &[Vec<f64>], reference: &[Vec<f64>]) -> f64 {
    if reference.is_empty() {
        return 0.0;
    }
    if approx.is_empty() {
        return f64::INFINITY;
    }
    let sum: f64 = reference
        .iter()
        .map(|z| {
            approx
                .iter()
                .map(|a| dist_plus(a, z))
                .fold(f64::INFINITY, f64::min)
        })
        .sum();
    sum / reference.len() as f64
}

/// 修正距離 d+(a, z)（a が z より悪い目的のみ寄与する。最小化前提）。
fn dist_plus(a: &[f64], z: &[f64]) -> f64 {
    let s: f64 = a
        .iter()
        .zip(z.iter())
        .map(|(&ai, &zi)| {
            let d = ai - zi;
            if d > 0.0 {
                d * d
            } else {
                0.0
            }
        })
        .sum();
    s.sqrt()
}

/// 単項 additive ε-indicator I_ε+(A, Z)。
///
/// 参照集合 Z の各点 z を弱支配するために A の点を平行移動させる最小量 ε。
/// I_ε+ = max_{z in Z} min_{a in A} max_j (a_j - z_j)。最小化前提。小さいほど良い。
pub fn additive_epsilon(approx: &[Vec<f64>], reference: &[Vec<f64>]) -> f64 {
    if reference.is_empty() {
        return 0.0;
    }
    if approx.is_empty() {
        return f64::INFINITY;
    }
    reference
        .iter()
        .map(|z| {
            approx
                .iter()
                .map(|a| {
                    a.iter()
                        .zip(z.iter())
                        .map(|(&ai, &zi)| ai - zi)
                        .fold(f64::NEG_INFINITY, f64::max)
                })
                .fold(f64::INFINITY, f64::min)
        })
        .fold(f64::NEG_INFINITY, f64::max)
}

/// R2 indicator（重み付き Tchebycheff スカラー化、ideal 基準）。
///
/// 各重み w について min_{a in A} max_j w_j * a_j を取り、全重みで平均する。
/// ideal は [0,1] 空間の原点（= 0）を用いる。小さいほど良い。
pub fn r2_indicator(approx: &[Vec<f64>], weights: &[Vec<f64>]) -> f64 {
    if weights.is_empty() {
        return 0.0;
    }
    if approx.is_empty() {
        return f64::INFINITY;
    }
    let sum: f64 = weights
        .iter()
        .map(|w| {
            approx
                .iter()
                .map(|a| {
                    a.iter()
                        .zip(w.iter())
                        .map(|(&ai, &wi)| wi * ai)
                        .fold(f64::NEG_INFINITY, f64::max)
                })
                .fold(f64::INFINITY, f64::min)
        })
        .sum();
    sum / weights.len() as f64
}

/// m 次元の単体格子（Das-Dennis）重みベクトル集合を生成する。
///
/// 各成分は k/h（Σ = 1）。重み 0 が目的を無視しないよう微小値 ε を下限とする。
/// 個数 C(h+m-1, m-1) が約 100 以下になる最大の h を選ぶ（m=2 は h=99、m=3 は h≈13）。
fn simplex_lattice_weights(m: usize) -> Vec<Vec<f64>> {
    const TARGET: usize = 100;
    const EPS: f64 = 1e-6;
    if m == 0 {
        return Vec::new();
    }
    if m == 1 {
        return vec![vec![1.0]];
    }

    // 個数が TARGET 以下に収まる最大の h を選ぶ（最低 1）。
    let mut h = 1usize;
    loop {
        let next = h + 1;
        if lattice_count(next, m) > TARGET {
            break;
        }
        h = next;
        if h > 10_000 {
            break;
        }
    }

    let mut result = Vec::new();
    let mut current = vec![0usize; m];
    gen_lattice(&mut result, &mut current, 0, h, m);
    // k/h を [eps,1] の重みへ変換し正規化する。
    result
        .into_iter()
        .map(|counts| {
            let raw: Vec<f64> = counts
                .iter()
                .map(|&c| (c as f64 / h as f64).max(EPS))
                .collect();
            let s: f64 = raw.iter().sum();
            raw.into_iter().map(|v| v / s).collect()
        })
        .collect()
}

/// h 分割・m 次元の単体格子点の個数 = C(h+m-1, m-1)。
fn lattice_count(h: usize, m: usize) -> usize {
    // C(h+m-1, m-1) をオーバーフローを避けつつ計算する。
    let n = h + m - 1;
    let k = m - 1;
    let mut result: u128 = 1;
    for i in 0..k {
        result = result * (n - i) as u128 / (i as u128 + 1);
    }
    result.min(usize::MAX as u128) as usize
}

/// 単体格子点（成分の合計が `total` になる非負整数ベクトル）を再帰生成する。
fn gen_lattice(
    out: &mut Vec<Vec<usize>>,
    current: &mut Vec<usize>,
    dim: usize,
    remaining: usize,
    m: usize,
) {
    if dim == m - 1 {
        current[dim] = remaining;
        out.push(current.clone());
        return;
    }
    for k in 0..=remaining {
        current[dim] = k;
        gen_lattice(out, current, dim + 1, remaining - k, m);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "expected {b}, got {a}");
    }

    #[test]
    fn igd_plus_zero_when_approx_covers_reference() {
        // 近似集合が参照集合を含む（同一点）なら IGD+ = 0。
        let reference = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        let approx = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        approx_eq(igd_plus(&approx, &reference), 0.0);
    }

    #[test]
    fn igd_plus_only_counts_worse_objectives() {
        // a が z より全目的で良い（小さい）場合 d+ = 0。
        let reference = vec![vec![1.0, 1.0]];
        let approx = vec![vec![0.0, 0.0]];
        approx_eq(igd_plus(&approx, &reference), 0.0);
        // a が z より悪い場合のみ寄与。z=(0,0), a=(0,1) → d+ = 1。
        let reference = vec![vec![0.0, 0.0]];
        let approx = vec![vec![0.0, 1.0]];
        approx_eq(igd_plus(&approx, &reference), 1.0);
    }

    #[test]
    fn additive_epsilon_zero_when_identical() {
        let reference = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        let approx = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        approx_eq(additive_epsilon(&approx, &reference), 0.0);
    }

    #[test]
    fn additive_epsilon_translation_amount() {
        // z=(0,0) を a=(0.5,0.5) で弱支配するには ε=0.5 必要。
        let reference = vec![vec![0.0, 0.0]];
        let approx = vec![vec![0.5, 0.5]];
        approx_eq(additive_epsilon(&approx, &reference), 0.5);
    }

    #[test]
    fn additive_epsilon_can_be_negative_when_dominating() {
        // a=(−0.3,−0.3) は z=(0,0) を強く弱支配し ε=−0.3。
        let reference = vec![vec![0.0, 0.0]];
        let approx = vec![vec![-0.3, -0.3]];
        approx_eq(additive_epsilon(&approx, &reference), -0.3);
    }

    #[test]
    fn r2_zero_at_ideal() {
        // ideal(=原点) に解があれば全重みで Tchebycheff = 0。
        let weights = simplex_lattice_weights(2);
        let approx = vec![vec![0.0, 0.0]];
        approx_eq(r2_indicator(&approx, &weights), 0.0);
    }

    #[test]
    fn r2_decreases_as_set_approaches_ideal() {
        let weights = simplex_lattice_weights(2);
        let far = vec![vec![1.0, 1.0]];
        let near = vec![vec![0.2, 0.2]];
        assert!(r2_indicator(&near, &weights) < r2_indicator(&far, &weights));
    }

    #[test]
    fn simplex_lattice_sums_to_one() {
        for m in [2usize, 3, 4] {
            let ws = simplex_lattice_weights(m);
            assert!(!ws.is_empty());
            assert!(ws.len() <= 120, "m={m} produced {} weights", ws.len());
            for w in &ws {
                assert_eq!(w.len(), m);
                let s: f64 = w.iter().sum();
                approx_eq(s, 1.0);
            }
        }
    }

    #[test]
    fn histories_shared_reference_make_series_comparable() {
        // 2 系列・最小化 2 目的。両系列が同じ参照集合・スケールで評価される。
        let s0_objs = vec![vec![2.0, 2.0], vec![1.0, 1.0]];
        let s0_ids = vec![0u32, 1];
        let s1_objs = vec![vec![3.0, 3.0], vec![0.0, 0.0]];
        let s1_ids = vec![0u32, 1];
        let series = vec![
            SeriesInput {
                trial_ids: &s0_ids,
                objectives: &s0_objs,
            },
            SeriesInput {
                trial_ids: &s1_ids,
                objectives: &s1_objs,
            },
        ];
        let is_min = vec![true, true];
        let hist = compute_indicator_histories(&series, &is_min, MoIndicator::IgdPlus, None);
        assert_eq!(hist.len(), 2);
        assert_eq!(hist[0].values.len(), 2);
        // 系列1は最終的に和集合の ideal(0,0) に到達するので IGD+ は系列0より小さくなる。
        let last0 = *hist[0].values.last().unwrap();
        let last1 = *hist[1].values.last().unwrap();
        assert!(last1 <= last0);
    }

    #[test]
    fn single_objective_returns_empty_values() {
        let objs = vec![vec![1.0], vec![0.5]];
        let ids = vec![0u32, 1];
        let series = vec![SeriesInput {
            trial_ids: &ids,
            objectives: &objs,
        }];
        let hist = compute_indicator_histories(&series, &[true], MoIndicator::Hypervolume, None);
        assert_eq!(hist.len(), 1);
        assert!(hist[0].values.is_empty());
    }

    #[test]
    fn hypervolume_ref_point_bounds_all_observed_points() {
        // 回帰防止: 序盤の劣った試行も参照点ボックス内に収まり HV > 0 になること。
        // 参照点を非支配集合の nadir から算出すると、劣点（[10,10]）はボックス外になり
        // 序盤の HV が 0 に潰れて推移が終端で突然立ち上がる。参照点は全観測点の
        // 最悪点（[10,10]）+ マージンを基準にすべき。
        let objs = vec![vec![10.0, 10.0], vec![1.0, 1.0]];
        let ids = vec![0u32, 1];
        let series = vec![SeriesInput {
            trial_ids: &ids,
            objectives: &objs,
        }];
        let hist =
            compute_indicator_histories(&series, &[true, true], MoIndicator::Hypervolume, None);
        let v = &hist[0].values;
        assert_eq!(v.len(), 2);
        // 1 点目（劣点のみ）でも参照点に内包され HV > 0。
        assert!(
            v[0] > 0.0,
            "early dominated point should yield HV > 0, got {}",
            v[0]
        );
        assert!(v[1] > v[0]);
    }

    #[test]
    fn hypervolume_history_is_nondecreasing() {
        // HV は試行が進むほど単調非減少。
        let objs = vec![vec![2.0, 2.0], vec![1.0, 2.0], vec![1.0, 1.0]];
        let ids = vec![0u32, 1, 2];
        let series = vec![SeriesInput {
            trial_ids: &ids,
            objectives: &objs,
        }];
        let hist =
            compute_indicator_histories(&series, &[true, true], MoIndicator::Hypervolume, None);
        let v = &hist[0].values;
        assert_eq!(v.len(), 3);
        assert!(v[1] >= v[0]);
        assert!(v[2] >= v[1]);
    }
}
