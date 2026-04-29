# PROMETHEE Ranking 実装ガイド

**作成日**: 2026-04-29
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連型定義**: [interfaces.rs](interfaces.rs)
**関連要件定義**: [../../spec/promethee-ranking/requirements.md](../../spec/promethee-ranking/requirements.md)
**受け入れ基準**: [../../spec/promethee-ranking/acceptance-criteria.md](../../spec/promethee-ranking/acceptance-criteria.md)

---

## 実装順序

依存関係の順に実装する。各ステップは独立して `cargo test` / `cargo build` が通ることを確認してから次に進む。

```
Step 1: rust_core/src/mcdm/promethee.rs     ← アルゴリズム実装
Step 2: rust_core/src/mcdm/mod.rs           ← pub mod 追加
Step 3: rust_core/src/lib.rs                ← pub use 追加
Step 4: egui-app/src/state/results.rs       ← 型・enum 拡張
Step 5: egui-app/src/state/message_handler.rs ← Promethee 分岐追加
Step 6: egui-app/src/ui/chart_registry.rs   ← spawn_task 分岐追加
Step 7: egui-app/src/ui/widgets/mcdm_chart.rs ← UI 描画追加
```

---

## Step 1: rust_core/src/mcdm/promethee.rs を作成

**新規ファイル**: `rust_core/src/mcdm/promethee.rs`

### 1-1. ファイル先頭・use 宣言

```rust
//! PROMETHEE I / II (Preference Ranking Organisation Method for Enrichment Evaluations)
//! Linear preference function only; thresholds auto-set to q=0, p=0.2*range_j.
use std::time::Instant;

#[derive(Debug, Clone, serde::Serialize)]
pub struct PrometheeResult {
    pub phi_plus: Vec<f64>,
    pub phi_minus: Vec<f64>,
    pub phi_net: Vec<f64>,
    pub ranked_indices_i: Vec<u32>,
    pub ranked_indices_ii: Vec<u32>,
    pub duration_ms: f64,
}
```

### 1-2. compute_promethee 関数

```rust
pub fn compute_promethee(
    values: &[f64],
    n_trials: usize,
    n_objectives: usize,
    weights: &[f64],
    is_minimize: &[bool],
) -> Result<PrometheeResult, String> {
    let start = Instant::now();

    // バリデーション（既存関数を流用）
    super::validate_inputs(values, n_trials, n_objectives, weights, is_minimize)?;

    let valid_indices = super::filter_valid_indices(values, n_trials, n_objectives);

    // 全 NaN の場合: 0.0 フロー・デフォルトランキングで即時返却
    if valid_indices.is_empty() {
        return Ok(zero_result(n_trials, start));
    }

    let n_valid = valid_indices.len();

    // 各目的関数の範囲と p 閾値を計算
    let (ranges, p_thresholds) = compute_thresholds(values, n_objectives, &valid_indices, n_valid);

    // 有効試行の値を行優先フラット Vec に収める（キャッシュ効率向上）
    let valid_values = extract_valid_values(values, n_objectives, &valid_indices, n_valid);

    // 集約選好行列 π(a,b) の計算（O(n²)）
    let pi = compute_preference_matrix(
        &valid_values, n_valid, n_objectives, weights, is_minimize, &p_thresholds, &ranges,
    );

    // 正フロー / 負フロー / 純フローの計算
    let (valid_phi_plus, valid_phi_minus) = compute_flows(&pi, n_valid);

    // n_trials サイズの配列に展開（NaN 試行は 0.0 のまま）
    let mut phi_plus  = vec![0.0_f64; n_trials];
    let mut phi_minus = vec![0.0_f64; n_trials];
    let mut phi_net   = vec![0.0_f64; n_trials];
    for (vi, &ti) in valid_indices.iter().enumerate() {
        phi_plus[ti]  = valid_phi_plus[vi];
        phi_minus[ti] = valid_phi_minus[vi];
        phi_net[ti]   = valid_phi_plus[vi] - valid_phi_minus[vi];
    }

    // PROMETHEE I ランキング: Φ+ 降順、タイブレーク Φ- 昇順
    let ranked_indices_i  = rank_promethee_i(&phi_plus, &phi_minus, n_trials, &valid_indices);
    // PROMETHEE II ランキング: Φnet 降順
    let ranked_indices_ii = rank_promethee_ii(&phi_net, n_trials, &valid_indices);

    Ok(PrometheeResult {
        phi_plus,
        phi_minus,
        phi_net,
        ranked_indices_i,
        ranked_indices_ii,
        duration_ms: start.elapsed().as_secs_f64() * 1000.0,
    })
}
```

### 1-3. 補助関数

```rust
/// Linear 選好関数: P(d) = 0 if d≤0, d/p if 0<d≤p, 1 if d>p (q=0 固定)
fn linear_preference(d: f64, p: f64) -> f64 {
    if d <= 0.0 { return 0.0; }
    if p <= 0.0 { return if d > 0.0 { 1.0 } else { 0.0 }; }
    if d >= p   { return 1.0; }
    d / p
}

/// 各目的関数の range と p 閾値を計算
fn compute_thresholds(
    values: &[f64],
    n_objectives: usize,
    valid_indices: &[usize],
    n_valid: usize,
) -> (Vec<f64>, Vec<f64>) {
    let mut ranges = vec![0.0_f64; n_objectives];
    for j in 0..n_objectives {
        let mut min_j = f64::INFINITY;
        let mut max_j = f64::NEG_INFINITY;
        for &i in valid_indices {
            let v = values[i * n_objectives + j];
            if v < min_j { min_j = v; }
            if v > max_j { max_j = v; }
        }
        ranges[j] = if max_j > min_j { max_j - min_j } else { 0.0 };
    }
    let p_thresholds: Vec<f64> = ranges.iter().map(|&r| 0.2 * r).collect();
    (ranges, p_thresholds)
}

/// 有効試行の値を行優先フラット Vec に収める
fn extract_valid_values(
    values: &[f64],
    n_objectives: usize,
    valid_indices: &[usize],
    n_valid: usize,
) -> Vec<f64> {
    let mut out = Vec::with_capacity(n_valid * n_objectives);
    for &i in valid_indices {
        out.extend_from_slice(&values[i * n_objectives..(i + 1) * n_objectives]);
    }
    out
}

/// 集約選好行列 π(a,b) の計算
/// π(a,b) = Σ_j weight_j * P_j(d_j(a,b))
/// d_j(a,b) = value_j(b) - value_j(a) if minimize  (小さい方が良いため b が大きければ a が優位)
///          = value_j(a) - value_j(b) if maximize
fn compute_preference_matrix(
    valid_values: &[f64],
    n_valid: usize,
    n_objectives: usize,
    weights: &[f64],
    is_minimize: &[bool],
    p_thresholds: &[f64],
    _ranges: &[f64],
) -> Vec<f64> {
    let mut pi = vec![0.0_f64; n_valid * n_valid];
    for a in 0..n_valid {
        for b in 0..n_valid {
            if a == b { continue; }
            let mut agg = 0.0_f64;
            for j in 0..n_objectives {
                let va = valid_values[a * n_objectives + j];
                let vb = valid_values[b * n_objectives + j];
                // minimize: a が優位なら d>0
                let d = if is_minimize[j] { vb - va } else { va - vb };
                agg += weights[j] * linear_preference(d, p_thresholds[j]);
            }
            pi[a * n_valid + b] = agg;
        }
    }
    pi
}

/// Φ+(i), Φ-(i) の計算
fn compute_flows(pi: &[f64], n_valid: usize) -> (Vec<f64>, Vec<f64>) {
    let denom = if n_valid > 1 { (n_valid - 1) as f64 } else { 1.0 };
    let mut phi_plus  = vec![0.0_f64; n_valid];
    let mut phi_minus = vec![0.0_f64; n_valid];
    for i in 0..n_valid {
        let mut pos = 0.0_f64;
        let mut neg = 0.0_f64;
        for j in 0..n_valid {
            if i == j { continue; }
            pos += pi[i * n_valid + j];
            neg += pi[j * n_valid + i];
        }
        phi_plus[i]  = pos / denom;
        phi_minus[i] = neg / denom;
    }
    (phi_plus, phi_minus)
}

/// PROMETHEE I ランキング: valid 試行を Φ+ 降順 (tiebreak: Φ- 昇順)、NaN 末尾
fn rank_promethee_i(
    phi_plus: &[f64],
    phi_minus: &[f64],
    n_trials: usize,
    valid_indices: &[usize],
) -> Vec<u32> {
    let valid_set: std::collections::HashSet<usize> = valid_indices.iter().copied().collect();
    let mut valid: Vec<usize> = valid_indices.to_vec();
    valid.sort_by(|&a, &b| {
        phi_plus[b].partial_cmp(&phi_plus[a]).unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| phi_minus[a].partial_cmp(&phi_minus[b]).unwrap_or(std::cmp::Ordering::Equal))
    });
    let mut result: Vec<u32> = valid.iter().map(|&i| i as u32).collect();
    for i in 0..n_trials {
        if !valid_set.contains(&i) {
            result.push(i as u32);
        }
    }
    result
}

/// PROMETHEE II ランキング: valid 試行を Φnet 降順、NaN 末尾
fn rank_promethee_ii(
    phi_net: &[f64],
    n_trials: usize,
    valid_indices: &[usize],
) -> Vec<u32> {
    let valid_set: std::collections::HashSet<usize> = valid_indices.iter().copied().collect();
    let mut valid: Vec<usize> = valid_indices.to_vec();
    valid.sort_by(|&a, &b| {
        phi_net[b].partial_cmp(&phi_net[a]).unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut result: Vec<u32> = valid.iter().map(|&i| i as u32).collect();
    for i in 0..n_trials {
        if !valid_set.contains(&i) {
            result.push(i as u32);
        }
    }
    result
}

/// 全 NaN 時の 0.0 フロー結果
fn zero_result(n_trials: usize, start: Instant) -> PrometheeResult {
    PrometheeResult {
        phi_plus:          vec![0.0; n_trials],
        phi_minus:         vec![0.0; n_trials],
        phi_net:           vec![0.0; n_trials],
        ranked_indices_i:  (0..n_trials as u32).collect(),
        ranked_indices_ii: (0..n_trials as u32).collect(),
        duration_ms:       start.elapsed().as_secs_f64() * 1000.0,
    }
}
```

### 1-4. テストケース骨格

テスト命名規則: `tc_pr_XXX_NN`（受け入れ基準の TC-PR-XXX-NN に対応）

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // TC-PR-005-01: Linear 選好: d > p → P = 1.0
    #[test]
    fn tc_pr_005_01_linear_d_gt_p() {
        // values = [1.0, 5.0, 5.0, 1.0] (2試行×2目的)
        // 試行0 vs 試行1 の目的0: d=|1-5|=4, range=4, p=0.8, d>p → P=1.0
        let values = vec![1.0_f64, 5.0, 5.0, 1.0];
        let r = compute_promethee(&values, 2, 2, &[0.5, 0.5], &[true, true]).unwrap();
        // π(0,1) の目的0 寄与: P(4, 0.8) = 1.0
        assert!((r.phi_plus[0] - r.phi_minus[1]).abs() < 1e-9);
    }

    // TC-PR-005-02: 全同値 → 全フロー 0.0
    #[test]
    fn tc_pr_005_02_all_same() {
        let values = vec![1.0_f64; 6]; // 3試行×2目的、全同値
        let r = compute_promethee(&values, 3, 2, &[0.5, 0.5], &[true, true]).unwrap();
        assert!(r.phi_plus.iter().all(|&v| v.abs() < 1e-9));
        assert!(r.phi_minus.iter().all(|&v| v.abs() < 1e-9));
        assert!(r.phi_net.iter().all(|&v| v.abs() < 1e-9));
    }

    // TC-PR-006-02: Φnet = Φ+ - Φ-
    #[test]
    fn tc_pr_006_02_phi_net_identity() {
        let values = vec![1.0_f64, 4.0, 4.0, 1.0, 2.0, 2.0];
        let r = compute_promethee(&values, 3, 2, &[0.5, 0.5], &[true, true]).unwrap();
        for i in 0..3 {
            let diff = (r.phi_net[i] - (r.phi_plus[i] - r.phi_minus[i])).abs();
            assert!(diff < 1e-9, "trial {i}: phi_net mismatch");
        }
    }

    // TC-PR-008-01: PROMETHEE II ランキング: 最優秀試行が ranked_indices_ii[0]
    #[test]
    fn tc_pr_008_01_best_trial_first() {
        // 試行0 が全目的で最小（minimize=true）なので ranked_indices_ii[0] == 0
        let values = vec![1.0_f64, 1.0,  5.0, 5.0,  3.0, 3.0];
        let r = compute_promethee(&values, 3, 2, &[0.5, 0.5], &[true, true]).unwrap();
        assert_eq!(r.ranked_indices_ii[0], 0);
    }

    // TC-PR-003-E01: n_trials=0 → Err
    #[test]
    fn tc_pr_003_e01_zero_trials() {
        let result = compute_promethee(&[], 0, 2, &[0.5, 0.5], &[true, true]);
        assert!(result.is_err());
    }

    // TC-EDGE-PR-002: 全同値（range=0）→ 全フロー 0.0、クラッシュなし
    #[test]
    fn tc_edge_pr_002_range_zero() {
        let values = vec![3.0_f64; 4]; // 2試行×2目的
        let r = compute_promethee(&values, 2, 2, &[0.5, 0.5], &[true, true]).unwrap();
        assert!(r.phi_plus.iter().all(|&v| v.abs() < 1e-9));
    }

    // TC-PR-NFR-001-01: 50,000 試行 × 4 目的 — 200 ms 以内 (ignored: release only)
    #[test]
    #[ignore]
    fn tc_pr_nfr_001_01_performance_50k() {
        use std::time::Instant;
        let n_trials = 50_000;
        let n_obj = 4;
        let values: Vec<f64> = (0..n_trials * n_obj).map(|i| i as f64).collect();
        let weights = vec![0.25_f64; n_obj];
        let is_minimize = vec![true; n_obj];
        let start = Instant::now();
        let r = compute_promethee(&values, n_trials, n_obj, &weights, &is_minimize).unwrap();
        let elapsed = start.elapsed().as_millis();
        assert!(elapsed < 200, "took {elapsed} ms (target < 200 ms)");
        assert_eq!(r.ranked_indices_ii.len(), n_trials);
    }
}
```

---

## Step 2 & 3: rust_core モジュール登録

**変更ファイル**: `rust_core/src/mcdm/mod.rs`

```rust
// 既存の pub mod topsis; pub mod vikor; の後に追加:
pub mod promethee;
```

**変更ファイル**: `rust_core/src/lib.rs`

```rust
// 既存の pub use mcdm::topsis; pub use mcdm::vikor; の後に追加:
pub use mcdm::promethee;
```

確認コマンド:
```bash
rtk cargo check -p tunny-core
rtk cargo test -p tunny-core
```

---

## Step 4: egui-app/src/state/results.rs を変更

`PrometheeResult` 構造体を追加し、`McdmMethod` と `McdmResult` を拡張する。

**追加する型**（[interfaces.rs](interfaces.rs) 参照）:
- `PrometheeResult` 構造体
- `McdmMethod::PrometheeI` / `McdmMethod::PrometheeII` バリアント
- `McdmResult::PrometheeI(PrometheeResult)` / `McdmResult::PrometheeII(PrometheeResult)` バリアント
- `McdmResult::primary_scores()`, `ranked_indices()`, `duration_ms()`, `method()`, `method_label()` の match アーム追加

**注意**: `McdmMethod::all()` を `[Topsis, Vikor, PrometheeI, PrometheeII]` に更新することで、コンボボックスに自動追加される。

確認コマンド:
```bash
rtk cargo check -p egui-app
```

---

## Step 5: egui-app/src/state/message_handler.rs を変更

`AppMessage::McdmDone` のハンドラに Promethee 分岐を追加する。

**変更箇所**: 既存の `match &result { Topsis ... Vikor ... }` ブロックに追加:

```rust
McdmResult::PrometheeI(r) | McdmResult::PrometheeII(r) => {
    widget_states.mcdm_chart.cached_promethee = Some(r.clone());
}
```

確認コマンド:
```bash
rtk cargo check -p egui-app
```

---

## Step 6: egui-app/src/ui/chart_registry.rs を変更

`pending_compute` ハンドラの `match req.method { ... }` に Promethee 分岐を追加する。

**変更箇所**: 既存の `McdmMethod::Vikor => { ... }` の後に追加:

```rust
McdmMethod::PrometheeI | McdmMethod::PrometheeII => {
    let method = req.method;
    crate::app::spawn_task(tx, move || {
        match tunny_core::mcdm::promethee::compute_promethee(
            &objectives, n_trials, n_objectives, &weights, &is_minimize,
        ) {
            Ok(r) => {
                let result = crate::state::results::PrometheeResult {
                    phi_plus: r.phi_plus,
                    phi_minus: r.phi_minus,
                    phi_net: r.phi_net,
                    ranked_indices_i: r.ranked_indices_i,
                    ranked_indices_ii: r.ranked_indices_ii,
                    duration_ms: r.duration_ms,
                };
                let mcdm = if method == McdmMethod::PrometheeI {
                    McdmResult::PrometheeI(result)
                } else {
                    McdmResult::PrometheeII(result)
                };
                AppMessage::McdmDone(mcdm)
            }
            Err(e) => AppMessage::Error(format!("PROMETHEE computation failed: {e}")),
        }
    });
}
```

確認コマンド:
```bash
rtk cargo check -p egui-app
```

---

## Step 7: egui-app/src/ui/widgets/mcdm_chart.rs を変更

### 7-1. McdmRankChart フィールド追加

```rust
pub struct McdmRankChart {
    // ... 既存フィールド ...
    pub cached_topsis:    Option<TopsisResult>,
    pub cached_vikor:     Option<VikorResult>,
    pub cached_promethee: Option<PrometheeResult>,  // 追加
    // ...
}
```

### 7-2. コンボボックス切替（キャッシュ復元）

既存の `McdmMethod::Topsis` / `Vikor` の切替ロジックと同様に、`PrometheeI` / `PrometheeII` 切替時は `cached_promethee` から復元する（再計算不要）。

```rust
// メソッド切替時:
if self.selected_method == McdmMethod::PrometheeI
    || self.selected_method == McdmMethod::PrometheeII
{
    if self.cached_promethee.is_some() {
        // キャッシュから即時復元（pending_compute を設定しない）
    }
}
```

### 7-3. PROMETHEE I バー描画（2 本バー）

```rust
McdmResult::PrometheeI(r) => {
    for &idx in &r.ranked_indices_i {
        let i = idx as usize;
        // Φ+ バー（青）
        draw_bar(ui, r.phi_plus[i],  Color32::from_rgb(0x0c, 0x6a, 0xc0));
        // Φ- バー（赤）
        draw_bar(ui, r.phi_minus[i], Color32::from_rgb(0xc0, 0x20, 0x20));
    }
}
```

### 7-4. PROMETHEE II バー描画（Φnet、負値オレンジ）

```rust
McdmResult::PrometheeII(r) => {
    for &idx in &r.ranked_indices_ii {
        let i = idx as usize;
        let color = if r.phi_net[i] >= 0.0 {
            Color32::from_rgb(0x0c, 0x6a, 0xc0)  // 青
        } else {
            Color32::from_rgb(0xe0, 0x70, 0x00)  // オレンジ
        };
        draw_bar(ui, r.phi_net[i].abs(), color);
    }
}
```

確認コマンド:
```bash
rtk cargo build -p egui-app
```

---

## 最終確認

```bash
# 全テスト実行
rtk cargo test -p tunny-core
rtk cargo test -p egui-app

# パフォーマンステスト（Release ビルドのみ）
cargo test -p tunny-core --release -- --ignored tc_pr_nfr

# 全体ビルド確認
rtk cargo build
```

受け入れ基準チェックリストは [../../spec/promethee-ranking/acceptance-criteria.md](../../spec/promethee-ranking/acceptance-criteria.md) を参照。

---

## 信頼性レベルサマリー

- 🔵 青信号: 全実装ステップ（既存パターン踏襲・要件定義明確）
- 🟡 黄信号: キャッシュ共有の詳細実装（既存パターンからの推測）
- 🔴 赤信号: 0 件

**品質評価**: 高品質
