# rayon 導入による並列化高速化 アーキテクチャ設計

**作成日**: 2026-05-04  
**関連要件定義**: [requirements.md](../../spec/rayon-performance-optimization/requirements.md)  
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書概要・コードベース調査より*

`rust_core` の計算ヘビーパス 4 箇所に `rayon` の `par_iter` / `into_par_iter` を展開し、
マルチコア CPU を活用した並列計算でユーザー体感レスポンスを改善する。
`rayon = "1"` は既に `rust_core/Cargo.toml` に依存関係として存在し、
`pdp/kriging_core.rs` の PDP グリッド計算 4 箇所で実績がある。

---

## 変更対象モジュール一覧 🔵

**信頼性**: 🔵 *コードベース調査・ユーザヒアリングより*

| # | ファイル | 変更関数 | 並列化パターン | 要件 |
|---|---------|---------|--------------|------|
| 1 | `rust_core/src/sensitivity/analysis/common.rs` | `run_tree_metric_for_all_objectives` | `objective_columns.par_iter()` | REQ-001 |
| 2 | `rust_core/src/sensitivity/sobol.rs` | `build_sobol_surrogate` (per-obj Ridge) + `f_a`/`f_b`/`f_ab_pi`/Sobol指標 | `y_matrix.par_iter()` + `(0..n_params).into_par_iter()` | REQ-002 |
| 3 | `rust_core/src/core/random_forest/forest.rs` | `RandomForest::train` | `(0..n_trees).into_par_iter()` + 木ごとシード | REQ-003 |
| 4 | `rust_core/src/sensitivity/permutation.rs` | `compute_from_prepared` | 特徴量ごと x_eval コピー + `(0..p).into_par_iter()` | REQ-004 |

---

## 1. Sensitivity 目的変数ループ並列化 🔵

**信頼性**: 🔵 *コードベース調査・ユーザヒアリングより*

### 変更前

```rust
// sensitivity/analysis/common.rs
let results: Vec<(Vec<f64>, f64)> = objective_columns
    .iter()                                      // 直列
    .map(|y| run_tree_metric_for_objective(metric, x_matrix, y))
    .collect();
```

### 変更後

```rust
use rayon::prelude::*;

let results: Vec<(Vec<f64>, f64)> = objective_columns
    .par_iter()                                  // 並列
    .map(|y| run_tree_metric_for_objective(metric, x_matrix, y))
    .collect();
```

### スレッドセーフ性 🔵

| リソース | 共有方法 | 安全性 |
|---------|---------|--------|
| `metric` (`&M`) | 共有参照のみ | `M: TreeMetric` は `Sync` 要件なし（`&M` で渡す） |
| `x_matrix` | 共有参照のみ | `&[Vec<f64>]` は `Sync` |
| `y` (各 objective_column) | スライス参照 | 独立した要素ごとに取り出し |
| LightGBM C API 予測 | 各 objective で独立モデル | インスタンス独立なのでスレッドセーフ |

> **注意**: `M: Send + Sync` を `where` 節に追加する必要がある。

### 型シグネチャ変更

```rust
// 変更前
pub(super) fn run_tree_metric_for_all_objectives<M: TreeMetric>(...)

// 変更後
pub(super) fn run_tree_metric_for_all_objectives<M: TreeMetric + Sync>(...)
```

---

## 2. Sobol 指標計算まで並列化 🔵

**信頼性**: 🔵 *コードベース調査・ユーザヒアリングより*

### 2-A: `build_sobol_surrogate` の per-objective Ridge ループ 🔵

`SobolSurrogate` の betas / intercepts / r_squared を `par_iter` で収集する。

```rust
// 変更前
for y in y_matrix {
    ...
    betas.push(ridge_res.beta);
    intercepts.push(y_mean);
    r_squared.push(ridge_res.r_squared);
}

// 変更後
use rayon::prelude::*;

let triplets: Vec<(Vec<f64>, f64, f64)> = y_matrix
    .par_iter()
    .map(|y| {
        let y_mean = y.iter().sum::<f64>() / n as f64;
        let y_centered: Vec<f64> = y.iter().map(|&v| v - y_mean).collect();
        let ridge_res = compute_ridge(&x_quad_std, &y_centered, alpha);
        (ridge_res.beta, y_mean, ridge_res.r_squared)
    })
    .collect();

let (betas, intercepts, r_squared): (Vec<_>, Vec<_>, Vec<_>) =
    triplets.into_iter().map(|(b, i, r)| (b, i, r)).fold(
        (vec![], vec![], vec![]),
        |(mut b, mut i, mut r), (bv, iv, rv)| {
            b.push(bv); i.push(iv); r.push(rv); (b, i, r)
        },
    );
```

> `x_quad_std` は読み取り専用共有参照で Sync。`compute_ridge` は純粋関数。

### 2-B: `f_a` / `f_b` の per-objective 計算 🔵

```rust
// 変更前
let f_a: Vec<Vec<f64>> = (0..n_objectives)
    .map(|k| mat_a.iter().map(|row| surrogate_eval(&surrogate, row, k)).collect())
    .collect();

// 変更後
let f_a: Vec<Vec<f64>> = (0..n_objectives)
    .into_par_iter()
    .map(|k| mat_a.iter().map(|row| surrogate_eval(&surrogate, row, k)).collect())
    .collect();
```

> `SobolSurrogate` は `&Vec<f64>` のみアクセスで純粋関数呼び出し。`surrogate_eval` はスレッドセーフ。

### 2-C: `for pi in 0..n_params` の並列化 🔵

```rust
// 変更後
let sobol_pairs: Vec<(Vec<f64>, Vec<f64>)> = (0..n_params)
    .into_par_iter()
    .map(|pi| {
        let ab_pi: Vec<Vec<f64>> = mat_a.iter().zip(mat_b.iter())
            .map(|(a, b)| { let mut row = a.clone(); row[pi] = b[pi]; row })
            .collect();
        let f_ab_pi_k: Vec<Vec<f64>> = (0..n_objectives)
            .map(|k| ab_pi.iter().map(|row| surrogate_eval(&surrogate, row, k)).collect())
            .collect();

        let fo: Vec<f64> = compute_first_order(&f_a, &f_b, &f_ab_pi_k, n_samples, n_objectives);
        let te: Vec<f64> = compute_total_effect(&f_a, &f_ab_pi_k, n_samples, n_objectives);
        (fo, te)
    })
    .collect();

// collect into first_order[pi][k] / total_effect[pi][k]
```

> **リファクタリング必須**: 現行の Sobol 指標計算ロジックを `compute_first_order` / `compute_total_effect` ヘルパー関数に抽出してから並列化する。

---

## 3. RandomForest 木構築ループ並列化 🔵

**信頼性**: 🔵 *コードベース調査・ユーザヒアリングより*

### 変更前

```rust
let mut rng = Lcg::new(seed);
let mut trees = Vec::with_capacity(n_trees);

for _ in 0..n_trees {
    // rng.next_usize で bootstrap サンプリング（状態変更）
    ...
    trees.push(DecisionTree { root });
}
```

### 変更後

```rust
use rayon::prelude::*;

let trees: Vec<DecisionTree> = (0..n_trees)
    .into_par_iter()
    .map(|tree_idx| {
        // スレッドごとに独立した RNG（seed + tree_index）
        let mut local_rng = Lcg::new(seed.wrapping_add(tree_idx as u64));

        let mut x_boot = Vec::with_capacity(n);
        let mut y_boot = Vec::with_capacity(n);
        for _ in 0..n {
            let idx = local_rng.next_usize(n);
            x_boot.push(x[idx].clone());
            y_boot.push(y[idx]);
        }

        let root = build_tree(
            &x_boot, &y_boot, &feature_indices,
            0, max_depth, min_samples_leaf,
        );
        DecisionTree { root }
    })
    .collect();

RandomForest { trees }
```

### シード設計 🔵

| 変更前 | 変更後 |
|-------|-------|
| 1 つの `Lcg` を共有・順次更新 | 木ごとに `Lcg::new(seed.wrapping_add(tree_idx as u64))` |
| 同じ seed で再現可能 | 同じ (seed, n_trees) で各木が再現可能 |

> `Lcg::new` は `seed ^ 0xcafef00dd15ea5e5` を初期値とする。`tree_idx` を加算するため各木は独立した乱数列を持つ。

---

## 4. Permutation 特徴量ループ並列化 🔵

**信頼性**: 🔵 *コードベース調査・ユーザヒアリングより*

### 設計方針: 特徴量ごとに `x_eval` の独立コピー 🔵

```rust
// 変更前（in-place 共有 x_eval_work）
let mut x_eval_work = x_eval.to_vec();
for feature_idx in 0..p {
    ...
}

// 変更後（特徴量ごとに独立コピー）
use rayon::prelude::*;

let importances: Vec<f64> = (0..p)
    .into_par_iter()
    .map(|feature_idx| {
        // 各スレッドが独立した x_eval コピーを保持
        let mut x_work = x_eval.to_vec();
        let orig_col: Vec<f64> = x_work.iter().map(|r| r[feature_idx]).collect();

        let mut delta_sum = 0.0f64;
        for repeat_idx in 0..PFI_N_REPEATS {
            let seed = PFI_SEED_BASE
                + (feature_idx as u64) * (PFI_N_REPEATS as u64)
                + (repeat_idx as u64);
            for (i, row) in x_work.iter_mut().enumerate() {
                row[feature_idx] = orig_col[i];
            }
            permute_column_inplace(&mut x_work, feature_idx, seed);
            let permuted_mse = lgbm_mse(&booster, &x_work, y_eval)
                .unwrap_or(baseline_mse);
            delta_sum += (permuted_mse - baseline_mse).max(0.0);
        }
        delta_sum / PFI_N_REPEATS as f64
    })
    .collect();
```

### メモリコスト評価 🟡

**信頼性**: 🟡 *性能特性から妥当な推測*

| 項目 | 直列版 | 並列版 |
|-----|-------|-------|
| x_eval コピー数 | 1 | rayon スレッド数（= CPU コア数程度） |
| 1 コピーのサイズ | `n_eval × p × 8 bytes` | 同上 |
| 例 n=200, p=10 | 16 KB | 最大 ~16 KB × コア数 |

コア数 8 で `n=200, p=10` の場合: 約 128 KB。許容範囲内。

---

## ベンチマーク設計 🔵

**信頼性**: 🔵 *ユーザヒアリング（ベンチマーク：全 4 関数）より*

`rust_core/benches/` に以下を追加（既存の `sampling_bench.rs` と同ディレクトリ）:

| ファイル | 対象関数 | 入力パラメータ |
|---------|---------|--------------|
| `benches/sensitivity_bench.rs` | `run_tree_metric_for_all_objectives` (RfAnova) | `n=200, p=5, n_obj=[1,2,4,8]` |
| `benches/sobol_bench.rs` | `compute_sobol_from_df` | `n=200, p=5, n_obj=[1,2,4]` |
| `benches/rf_bench.rs` | `RandomForest::train` | `n=200, p=5, n_trees=[10,50,100]` |
| `benches/permutation_bench.rs` | `compute_from_prepared` (PFI) | `n=200, p=[3,10,20]` |

各ベンチマークは並列版を直接計測し、`criterion` の `BenchmarkGroup` で `n_obj` / `n_trees` / `p` をパラメタライズする。

```toml
# rust_core/Cargo.toml に追加
[[bench]]
name = "sensitivity_bench"
harness = false

[[bench]]
name = "sobol_bench"
harness = false

[[bench]]
name = "rf_bench"
harness = false

[[bench]]
name = "permutation_bench"
harness = false
```

---

## 変更ファイル一覧 🔵

**信頼性**: 🔵 *コードベース調査・設計より*

| ファイル | 変更種別 | 変更内容 |
|---------|---------|---------|
| `rust_core/src/sensitivity/analysis/common.rs` | 修正 | `par_iter()` 適用 + `M: Sync` 境界追加 |
| `rust_core/src/sensitivity/sobol.rs` | 修正 | `par_iter()` × 3 箇所 + ヘルパー関数抽出 |
| `rust_core/src/core/random_forest/forest.rs` | 修正 | `into_par_iter()` + 木ごとシード |
| `rust_core/src/sensitivity/permutation.rs` | 修正 | in-place → コピー設計 + `into_par_iter()` |
| `rust_core/benches/sensitivity_bench.rs` | 新規 | criterion ベンチマーク |
| `rust_core/benches/sobol_bench.rs` | 新規 | criterion ベンチマーク |
| `rust_core/benches/rf_bench.rs` | 新規 | criterion ベンチマーク |
| `rust_core/benches/permutation_bench.rs` | 新規 | criterion ベンチマーク |
| `rust_core/Cargo.toml` | 修正 | `[[bench]]` エントリ × 4 追加 |

---

## 既存コードとの整合性 🔵

**信頼性**: 🔵 *コードベース調査より*

- `rayon::prelude::*` の use 宣言は各変更ファイルの先頭に追加（既存 `kriging_core.rs` パターンに倣う）
- `egui-app` への変更は一切不要（計算は既に非同期スレッドで実行）
- `rust_core/Cargo.toml` の `rayon = "1"` バージョンは変更しない
- `egui-app/Cargo.toml` への `rayon` 追加は不要
