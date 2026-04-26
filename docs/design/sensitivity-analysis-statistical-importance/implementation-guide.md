# sensitivity-analysis-statistical-importance 実装ガイド

**作成日**: 2026-04-25
**関連設計**: [architecture.md](architecture.md), [dataflow.md](dataflow.md)

各ファイルへの変更差分を示す。

---

## 1. `rust_core/src/core/math/mod.rs` — モジュール追加

```rust
// 変更前
pub(crate) mod linear_algebra;

// 変更後
pub(crate) mod linear_algebra;
pub(crate) mod statistics;
```

---

## 2. `rust_core/src/core/math/statistics.rs` — 新規作成

```rust
//! 統計的仮説検定のためのユーティリティ関数
//!
//! pure Rust 実装（外部クレート不要）
//! t分布CDF: 不完全ベータ関数の継続分数展開（Lentz法）、誤差 < 10^{-6}

/// 不完全正規化ベータ関数 I_x(a, b) の継続分数展開（Lentz法）
///
/// t分布のCDF計算に使用: x = df/(df + t²), a = df/2, b = 0.5
fn regularized_incomplete_beta(x: f64, a: f64, b: f64) -> f64 {
    if x <= 0.0 { return 0.0; }
    if x >= 1.0 { return 1.0; }

    // 対称性: I_x(a,b) = 1 - I_{1-x}(b,a)
    let (x, a, b, flip) = if x > (a + 1.0) / (a + b + 2.0) {
        (1.0 - x, b, a, true)
    } else {
        (x, a, b, false)
    };

    // Lentz 継続分数展開
    let tiny = 1e-300;
    let mut h = tiny;
    let mut c = 1.0;
    let mut d = 1.0 - (a + b) * x / (a + 1.0);
    if d.abs() < tiny { d = tiny; }
    d = 1.0 / d;
    h = d;

    for m in 1..200 {
        let mf = m as f64;
        // 偶数ステップ
        let num_even = mf * (b - mf) * x / ((a + 2.0*mf - 1.0) * (a + 2.0*mf));
        d = 1.0 + num_even * d;
        if d.abs() < tiny { d = tiny; }
        c = 1.0 + num_even / c;
        if c.abs() < tiny { c = tiny; }
        d = 1.0 / d;
        h *= d * c;

        // 奇数ステップ
        let num_odd = -(a + mf) * (a + b + mf) * x / ((a + 2.0*mf) * (a + 2.0*mf + 1.0));
        d = 1.0 + num_odd * d;
        if d.abs() < tiny { d = tiny; }
        c = 1.0 + num_odd / c;
        if c.abs() < tiny { c = tiny; }
        d = 1.0 / d;
        let del = d * c;
        h *= del;

        if (del - 1.0).abs() < 3e-7 { break; }
    }

    // B(x; a, b) = x^a * (1-x)^b * h / (a * Beta(a,b))
    let ln_beta = lgamma(a) + lgamma(b) - lgamma(a + b);
    let result = (a * x.ln() + b * (1.0 - x).ln() - ln_beta - a.ln() + h.ln()).exp();

    if flip { 1.0 - result } else { result }
}

/// 対数ガンマ関数（Stirling近似）
fn lgamma(x: f64) -> f64 {
    // Lanczos近似 (g=7, n=9)
    const C: [f64; 9] = [
        0.99999999999980993,
        676.5203681218851,
        -1259.1392167224028,
        771.32342877765313,
        -176.61502916214059,
        12.507343278686905,
        -0.13857109526572012,
        9.9843695780195716e-6,
        1.5056327351493116e-7,
    ];
    if x < 0.5 {
        std::f64::consts::PI.ln() - (std::f64::consts::PI * x).sin().ln() - lgamma(1.0 - x)
    } else {
        let x = x - 1.0;
        let mut sum = C[0];
        for (i, &c) in C[1..].iter().enumerate() {
            sum += c / (x + i as f64 + 1.0);
        }
        let t = x + C.len() as f64 - 1.5;
        (2.0 * std::f64::consts::PI).sqrt().ln() + sum.ln() + (x + 0.5) * t.ln() - t
    }
}

/// t分布のCDF
///
/// df > 30 の場合は正規分布近似にフォールバック（精度は同等以上）
pub fn student_t_cdf(t: f64, df: f64) -> f64 {
    if df <= 0.0 { return 0.5; }
    if t.is_nan() || t.is_infinite() {
        return if t > 0.0 { 1.0 } else { 0.0 };
    }

    if df > 1000.0 {
        return normal_cdf(t);
    }

    let x = df / (df + t * t);
    let ib = regularized_incomplete_beta(x, df / 2.0, 0.5);
    if t >= 0.0 { 1.0 - 0.5 * ib } else { 0.5 * ib }
}

/// 両側p値（t分布）
pub fn t_two_sided_p(t_stat: f64, df: f64) -> f64 {
    let cdf = student_t_cdf(t_stat.abs(), df);
    (2.0 * (1.0 - cdf)).min(1.0)
}

/// 片側上側p値（t分布）- 重要度の有意性検定（H₀: μ=0, H₁: μ>0）
pub fn t_one_sided_upper_p(t_stat: f64, df: f64) -> f64 {
    (1.0 - student_t_cdf(t_stat, df)).min(1.0).max(0.0)
}

/// 標準正規分布のCDF（Abramowitz & Stegun 7.1.26）
pub fn normal_cdf(z: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.2316419 * z.abs());
    let poly = t * (0.319381530
        + t * (-0.356563782
        + t * (1.781477937
        + t * (-1.821255978
        + t * 1.330274429))));
    let pdf = (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt();
    let cdf = 1.0 - pdf * poly;
    if z >= 0.0 { cdf } else { 1.0 - cdf }
}

/// 両側p値（正規分布）
pub fn z_two_sided_p(z_stat: f64) -> f64 {
    (2.0 * (1.0 - normal_cdf(z_stat.abs()))).min(1.0)
}

/// t_{0.025, df} の近似（95%信頼区間の臨界値）
pub fn t_critical_95(df: f64) -> f64 {
    // df=1: 12.706, df=5: 2.571, df=10: 2.228, df=30: 2.042, df→∞: 1.960
    // 近似式（Gleason 1999の逆関数近似）
    if df >= 1000.0 { return 1.95996; }
    if df >= 100.0 { return 1.984; }
    if df >= 30.0 { return 2.042 + 0.051 * (30.0 - df).max(0.0) / 30.0; }
    // 低自由度: テーブル補間
    let table: &[(f64, f64)] = &[
        (1.0, 12.706), (2.0, 4.303), (3.0, 3.182), (4.0, 2.776),
        (5.0, 2.571), (6.0, 2.447), (7.0, 2.365), (8.0, 2.306),
        (9.0, 2.262), (10.0, 2.228), (15.0, 2.131), (20.0, 2.086),
        (25.0, 2.060), (30.0, 2.042),
    ];
    // 線形補間
    for i in 0..table.len()-1 {
        let (df0, t0) = table[i];
        let (df1, t1) = table[i+1];
        if df <= df1 {
            let alpha = (df - df0) / (df1 - df0);
            return t0 + alpha * (t1 - t0);
        }
    }
    2.042
}

/// Bonferroni補正
pub fn bonferroni_adjust(p_values: &[Option<f64>], n_params: usize) -> Vec<Option<f64>> {
    if n_params == 0 { return p_values.to_vec(); }
    p_values.iter().map(|&p| {
        p.map(|raw| (raw * n_params as f64).min(1.0))
    }).collect()
}

/// 有意性マーク
pub fn significance_mark(p_adjusted: Option<f64>) -> &'static str {
    match p_adjusted {
        Some(p) if p < 0.001 => "***",
        Some(p) if p < 0.01  => "**",
        Some(p) if p < 0.05  => "*",
        _ => "",
    }
}
```

---

## 3. `rust_core/src/sensitivity/types.rs` — 型拡張

```rust
// 以下の変更を適用:
// - SensitivityResult に spearman_p_values, spearman_ci_lower, spearman_ci_upper を追加
// - RidgeResult に std_errors, p_values, ci_lower, ci_upper, is_approximate を追加
// - RfAnovaResult に p_values, ci_lower, ci_upper を追加
// - MdiResult に p_values, ci_lower, ci_upper を追加
// - ShapResult に p_values, ci_lower, ci_upper を追加
// - SobolResult に first_order_ci_lower/upper/p_values, total_effect_ci_lower/upper/p_values,
//   surrogate_quality_warning を追加

// 詳細な型定義は interfaces.rs を参照
```

---

## 4. `rust_core/src/sensitivity/spearman.rs` — 統計計算追加

```rust
use crate::core::math::statistics::{t_two_sided_p, t_critical_95};

pub struct SpearmanStats {
    pub rho: f64,
    pub p_value_raw: Option<f64>,
    pub ci_lower: Option<f64>,
    pub ci_upper: Option<f64>,
}

pub fn compute_spearman_with_stats(x: &[f64], y: &[f64]) -> SpearmanStats {
    let n = x.len().min(y.len());
    let rho = compute_spearman(x, y);

    if n < 4 {
        return SpearmanStats { rho, p_value_raw: None, ci_lower: None, ci_upper: None };
    }

    let nf = n as f64;
    // t統計量（退化ケースの処理）
    let p_value_raw = if (rho.abs() - 1.0).abs() < f64::EPSILON {
        Some(0.0)  // ρ = ±1 → 完全相関 → p = 0
    } else {
        let t = rho * (nf - 2.0).sqrt() / (1.0 - rho * rho).sqrt();
        Some(t_two_sided_p(t, nf - 2.0))
    };

    // Fisher z変換で信頼区間
    let (ci_lower, ci_upper) = if n >= 4 {
        let z = rho.atanh();
        let se_z = 1.0 / (nf - 3.0).sqrt();
        let t_crit = t_critical_95(nf - 3.0);  // 近似
        let lo = (z - t_crit * se_z).tanh();
        let hi = (z + t_crit * se_z).tanh();
        (Some(lo.clamp(-1.0, 1.0)), Some(hi.clamp(-1.0, 1.0)))
    } else {
        (None, None)
    };

    SpearmanStats { rho, p_value_raw, ci_lower, ci_upper }
}
```

---

## 5. `rust_core/src/sensitivity/ridge.rs` — 対角逆行列と統計計算

```rust
use crate::core::math::statistics::{t_two_sided_p, t_critical_95};

/// A の対角逆行列要素を計算（各j: e_j を右辺としてガウス消去を実行）
fn compute_diagonal_inverse(a_flat: &[f64], p: usize) -> Vec<f64> {
    let mut diag_inv = vec![0.0f64; p];
    let a_2d: Vec<Vec<f64>> = (0..p).map(|i| a_flat[i*p..(i+1)*p].to_vec()).collect();

    for j in 0..p {
        let mut e_j = vec![0.0f64; p];
        e_j[j] = 1.0;
        if let Some(x) = gaussian_elimination(a_2d.clone(), e_j) {
            diag_inv[j] = x[j];
        }
    }
    diag_inv
}

/// Ridge回帰と統計指標を同時計算
pub fn compute_ridge_with_stats(x_matrix: &[Vec<f64>], y: &[f64], alpha: f64) -> RidgeResult {
    let n = y.len();
    let empty = RidgeResult { beta: vec![], r_squared: 0.0,
        std_errors: None, p_values: None, ci_lower: None, ci_upper: None, is_approximate: true };

    if n < 2 || x_matrix.len() != n { return empty; }
    let p = x_matrix[0].len();
    if p == 0 { return empty; }

    let x_cols = transpose_and_standardize(x_matrix, n, p);

    // XTX + αI を構築して保存（ガウス消去と対角逆行列の両方で使用）
    let mut a_flat = vec![0.0f64; p * p];
    for i in 0..p {
        for j in i..p {
            let col_i = &x_cols[i*n..(i+1)*n];
            let col_j = &x_cols[j*n..(j+1)*n];
            let val: f64 = col_i.iter().zip(col_j).map(|(a, b)| a * b).sum();
            a_flat[i*p+j] = val;
            a_flat[j*p+i] = val;
        }
        a_flat[i*p+i] += alpha;
    }

    // Ridge係数・R²（既存ロジックと同等）
    let result = compute_ridge_from_standardized_columns(&x_cols, n, y, alpha);
    let beta = result.beta.clone();
    let r_squared = result.r_squared;

    // 統計計算（n ≤ p+2 の場合はスキップ）
    if n <= p + 2 {
        return RidgeResult { beta, r_squared,
            std_errors: None, p_values: None, ci_lower: None, ci_upper: None,
            is_approximate: true };
    }

    // 残差分散
    let y_mean = y.iter().sum::<f64>() / n as f64;
    let y_c: Vec<f64> = y.iter().map(|&v| v - y_mean).collect();
    let y_hat: Vec<f64> = (0..n)
        .map(|i| (0..p).map(|j| x_cols[j*n+i] * beta[j]).sum())
        .collect();
    let rss: f64 = y_c.iter().zip(&y_hat).map(|(yi, yhi)| (yi - yhi).powi(2)).sum();
    let sigma2 = rss / (n - p - 1) as f64;

    // 対角逆行列
    let diag_inv = compute_diagonal_inverse(&a_flat, p);

    let df = (n - p - 1) as f64;
    let t_crit = t_critical_95(df);

    let (mut std_errors, mut p_values, mut ci_lower, mut ci_upper) =
        (vec![], vec![], vec![], vec![]);

    for j in 0..p {
        let se = (sigma2 * diag_inv[j]).max(0.0).sqrt();
        std_errors.push(se);
        if se < f64::EPSILON {
            p_values.push(1.0);  // SE=0 → 有意でない
        } else {
            let t = beta[j] / se;
            p_values.push(t_two_sided_p(t, df));
        }
        ci_lower.push(beta[j] - t_crit * se);
        ci_upper.push(beta[j] + t_crit * se);
    }

    RidgeResult {
        beta, r_squared,
        std_errors: Some(std_errors),
        p_values: Some(p_values),
        ci_lower: Some(ci_lower),
        ci_upper: Some(ci_upper),
        is_approximate: true,  // α > 0 による正則化バイアスがある
    }
}
```

---

## 6. `rust_core/src/sensitivity/rf_anova.rs` — 木ごと重要度記録

```rust
use crate::core::math::statistics::{t_one_sided_upper_p, t_critical_95};

// compute_rf_anova_importances() の内部ループを変更:
// 変更前: importances[j] += (permuted_mse - baseline_mse).max(0.0)
// 変更後: tree_importances[t][j] = (permuted_mse - baseline_mse).max(0.0)

pub fn compute_rf_anova_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64) {
    // ...（既存のロジック。ただし木ごとにimportanceを記録）...

    // 木ごとの記録: n_trees × p
    let mut tree_importances: Vec<Vec<f64>> = vec![vec![0.0; p]; n_trees];
    for t in 0..n_trees {
        // ...（既存の木構築・評価）...
        for j in 0..p {
            let perm_mse = /* permuted_mse for param j */;
            tree_importances[t][j] = (perm_mse - baseline_mse).max(0.0);
        }
    }

    // 平均（正規化前）
    let raw_means: Vec<f64> = (0..p).map(|j| {
        tree_importances.iter().map(|t| t[j]).sum::<f64>() / n_trees as f64
    }).collect();

    // 標準誤差
    let std_errors: Vec<f64> = (0..p).map(|j| {
        let mean = raw_means[j];
        let var = tree_importances.iter()
            .map(|t| (t[j] - mean).powi(2))
            .sum::<f64>() / (n_trees - 1) as f64;
        (var / n_trees as f64).sqrt()
    }).collect();

    // p値（t検定、df=99）
    let df = (n_trees - 1) as f64;
    let t_crit = t_critical_95(df);
    let p_values: Vec<f64> = (0..p).map(|j| {
        if std_errors[j] < f64::EPSILON { 1.0 }
        else { t_one_sided_upper_p(raw_means[j] / std_errors[j], df) }
    }).collect();

    // 正規化（既存互換）
    let sum_raw: f64 = raw_means.iter().sum();
    let norm: Vec<f64> = if sum_raw > f64::EPSILON {
        raw_means.iter().map(|&v| v / sum_raw).collect()
    } else {
        vec![0.0; p]
    };

    // CI（正規化後スケール）
    let ci_lower: Vec<f64> = (0..p).map(|j| {
        (norm[j] - t_crit * std_errors[j] / sum_raw).max(0.0)
    }).collect();
    let ci_upper: Vec<f64> = (0..p).map(|j| {
        norm[j] + t_crit * std_errors[j] / sum_raw
    }).collect();

    // TODO: これらの統計情報を RfAnovaResult に格納して返す
    // 現在のシグネチャ (Vec<f64>, f64) は後方互換のため維持
    // analysis/full.rs で統計情報を格納する別パスを追加する
    (norm, r_squared)
}
```

---

## 7. `rust_core/src/sensitivity/sobol.rs` — Jansen分散追加

```rust
use crate::core::math::statistics::{z_two_sided_p};

// compute_sobol() の内部ループ（パラメータpiのソーブルループ）に追加:

// 一次指標のJansen分散
let d_j: Vec<f64> = f_b[k].iter()
    .zip(f_ab_pi[k].iter())
    .zip(f_a[k].iter())
    .map(|((&fb, &fab), &fa)| fb * (fab - fa))
    .collect();

let mean_d = d_j.iter().sum::<f64>() / n_f;
let var_d = d_j.iter().map(|&v| (v - mean_d).powi(2)).sum::<f64>() / n_f;
let var_s_i = if var_y < f64::EPSILON { f64::INFINITY }
              else { var_d / (n_f * var_y * var_y) };
let se_s_i = var_s_i.sqrt();

let (ci_lo, ci_hi, p_val) = if se_s_i < f64::EPSILON || var_s_i.is_infinite() {
    (None, None, None)
} else {
    let lo = (s_i - 1.96 * se_s_i).max(0.0);
    let hi = s_i + 1.96 * se_s_i;
    let z = s_i / se_s_i;
    (Some(lo), Some(hi), Some(z_two_sided_p(z)))
};
// 同様に ST_i の分散も計算

// SobolResult に統計フィールドを追加
```

---

## 8. `egui-app/src/ui/widgets/importance_chart.rs` — UI変更

```rust
// 既存のスコアループを変更:
// 変更前:
// for (name, score) in &scores {
//     ui.horizontal(|ui| {
//         // ラベル + バー
//         ui.label(format!("{score:.3}"));
//     });
// }

// 変更後:
let n_params = scores.len();
for (idx, (name, score)) in scores.iter().enumerate() {
    // 統計情報を取得（Bonferroni補正済み）
    let stat = get_stat_info(sensitivity, sobol, &self.metric, idx, obj_idx, n_params);

    ui.horizontal(|ui| {
        // ラベル（変更なし）
        ui.add_sized([label_width, bar_height], Label::new(...).truncate());

        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(bar_max_width, bar_height - bar_gap),
            egui::Sense::hover(),
        );
        if ui.is_rect_visible(rect) {
            // バー描画（変更なし）
            let bar_width = (score / max_score * bar_max_width as f64) as f32;
            let bar_rect = Rect::from_min_size(rect.min, egui::vec2(bar_width, rect.height()));
            ui.painter().rect_filled(bar_rect, 2.0, bar_color);

            // エラーバー（CI）描画（追加）
            if let (Some(lo), Some(hi)) = (stat.ci_lower, stat.ci_upper) {
                const CI_COLOR: Color32 = Color32::from_rgb(100, 150, 220);
                let lo_x = rect.min.x + (lo / max_score * bar_max_width as f64).max(0.0) as f32;
                let hi_x = rect.min.x + (hi / max_score * bar_max_width as f64).min(bar_max_width as f64) as f32;
                let cy = rect.center().y;
                let stroke = egui::Stroke::new(1.5, CI_COLOR);
                let painter = ui.painter();
                painter.line_segment([egui::pos2(lo_x, cy), egui::pos2(hi_x, cy)], stroke);
                painter.line_segment([egui::pos2(lo_x, cy-4.0), egui::pos2(lo_x, cy+4.0)], stroke);
                painter.line_segment([egui::pos2(hi_x, cy-4.0), egui::pos2(hi_x, cy+4.0)], stroke);
            }
        }

        // p値テキスト（追加）
        let color = p_value_color(stat.p_adjusted);
        let prefix = if stat.is_approximate { "~" } else { "" };
        let text = if let Some(p) = stat.p_adjusted {
            format!("{prefix}{p:.3} {}", stat.mark)
        } else {
            format!("{score:.3}")
        };
        ui.label(egui::RichText::new(text).color(color));
    });
}
```

---

## 9. `rust_core/src/sensitivity/analysis/full.rs` および `selected.rs` — 統計情報の格納

```rust
// compute_sensitivity() の内部で統計情報を計算して SensitivityResult に格納:

// Spearmanの統計
let spearman_stats: Vec<Vec<SpearmanStats>> = objective_names.iter().enumerate()
    .map(|(k, _)| {
        param_names.iter().enumerate()
            .map(|(j, _)| {
                let x = /* param j のデータ */;
                let y = /* objective k のデータ */;
                compute_spearman_with_stats(&x, &y)
            })
            .collect()
    })
    .collect();

// Bonferroni補正を適用して SensitivityResult に格納
let n_params = param_names.len();
let spearman_p_values: Vec<Vec<f64>> = spearman_stats.iter().map(|obj_stats| {
    let raw: Vec<Option<f64>> = obj_stats.iter().map(|s| s.p_value_raw).collect();
    bonferroni_adjust(&raw, n_params)
        .into_iter().map(|p| p.unwrap_or(1.0)).collect()
}).collect();
```

---

## 実装チェックリスト

### Phase 1: 統計コア
- [ ] `statistics.rs` の `student_t_cdf` の実装と精度検証
- [ ] `normal_cdf` の実装と精度検証
- [ ] `bonferroni_adjust`, `significance_mark` の実装
- [ ] `statistics.rs` の単体テスト（scipy との比較）
- [ ] `mod.rs` に `pub(crate) mod statistics;` を追加

### Phase 2: 型拡張
- [ ] `types.rs` の全構造体に統計フィールドを追加
- [ ] `Default` / `empty` コンストラクタを更新

### Phase 3: メトリクス計算拡張
- [ ] `spearman.rs`: `compute_spearman_with_stats()` 追加
- [ ] `ridge.rs`: `compute_diagonal_inverse()` + `compute_ridge_with_stats()` 追加
- [ ] `rf_anova.rs`: 木ごと記録に変更
- [ ] `mdi.rs`: 木ごと記録に変更
- [ ] `shap.rs`: 木ごと記録に変更
- [ ] `sobol.rs`: Jansen分散計算 追加

### Phase 4: 分析結合
- [ ] `analysis/full.rs`: Bonferroni補正済みp値を `SensitivityResult` に格納
- [ ] `analysis/selected.rs`: 同上

### Phase 5: UI
- [ ] `importance_chart.rs`: `get_stat_info()` 実装
- [ ] `importance_chart.rs`: エラーバー描画ループ実装
- [ ] `importance_chart.rs`: p値テキスト + 色分け実装
- [ ] `importance_chart.rs`: Ridge の「～」近似フラグ表示

### テスト
- [ ] `statistics.rs`: `t_two_sided_p(0.0, 10) ≈ 1.0`, `t_two_sided_p(2.228, 10) ≈ 0.05`
- [ ] `spearman.rs`: ρ=1.0 → p≈0.0, ρ=0.0 → p≈1.0
- [ ] `ridge.rs`: 既知データで p値の数値確認
- [ ] Bonferroni補正: P=3, p_raw=[0.01,0.03,0.1] → p_adj=[0.03,0.09,0.3]
