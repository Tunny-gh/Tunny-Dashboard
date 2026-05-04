# pdp-maintainability アーキテクチャ設計

**作成日**: 2026-05-04
**関連要件定義**: [requirements.md](../../spec/pdp-maintainability/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書REF-1〜5・PERF-1 + ユーザヒアリングより*

`rust_core/src/pdp/` モジュール内で4箇所のコード重複を共有ヘルパーに抽出し、
`rayon` を追加して PDP 計算ループを並列化する。
変更はすべて `pdp` モジュール内に閉じており、公開 API（`compute_pdp`, `compute_pdp_2d`, `compute_pdp_from_data`）の
シグネチャは変更しない。

---

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存レイヤードアーキテクチャ + ユーザヒアリングより*

- **パターン**: 既存の `pdp` モジュール内部へのヘルパー関数抽出（外部公開 API 不変）
- **選択理由**: 公開 API が `lib.rs` → WASM バインディングへ接続されており、
  シグネチャ変更はバイナリ互換性を壊す。内部リファクタリングに留める。

---

## コンポーネント構成（変更後） 🔵

**信頼性**: 🔵 *コード直接分析 + ユーザヒアリングより*

```
rust_core/src/pdp/
├── mod.rs          変更なし（pub re-export）
├── types.rs        変更なし（PdpResult1d, PdpResult2d）
├── utils.rs        ★ 拡張: normalize_x_minmax / normalize_y / r_squared / extract_xy 追加
├── api.rs          ★ 変更: extract_xy を使ってDF抽出を共通化
├── ridge_core.rs   ★ 変更: fold スタイル統一（f64::min / f64::max）
├── kriging_core.rs ★ 変更: 正規化・R² ヘルパー呼び出しに置き換え + rayon 並列化
└── tests.rs        変更なし（既存テストで回帰テストとして機能）
```

**変更ファイル数**: 4ファイル  
**変更なしファイル数**: 3ファイル

---

## 設計詳細

### REF-1/2: utils.rs への共通ヘルパー追加 🔵

**信頼性**: 🔵 *コード直接分析・ユーザヒアリングより*

#### 追加関数一覧

```rust
// normalize_x_minmax: 各列の (min, range) と正規化済み行列を返す
// 1回のループで col_stats と x_norm を同時に計算（効率的）
pub(super) fn normalize_x_minmax(
    x_matrix: &[Vec<f64>],
) -> (Vec<(f64, f64)>, Vec<Vec<f64>>)

// normalize_y: y の (mean, std, normalized_y) を返す
pub(super) fn normalize_y(
    y: &[f64],
) -> (f64, f64, Vec<f64>)

// r_squared: ss_tot < EPSILON のゼロ除算ガード付き R²
pub(super) fn r_squared(
    y_actual: &[f64],
    y_pred: &[f64],
) -> f64

// extract_xy: DataFrame から x_matrix と y を一括抽出
// api.rs 内の with_active_df クロージャ内で使用
pub(super) fn extract_xy(
    df: &crate::data::DataFrame,
    param_names: &[String],
    objective_name: &str,
) -> (Vec<Vec<f64>>, Vec<f64>)
```

#### normalize_x_minmax の実装方針

```rust
// col_stats と x_norm を同一イテレーションで計算することで O(N×D) → O(N×D) に保つ
// (2回別ループを避ける)
pub(super) fn normalize_x_minmax(
    x_matrix: &[Vec<f64>],
) -> (Vec<(f64, f64)>, Vec<Vec<f64>>) {
    let n_dims = x_matrix.first().map(|r| r.len()).unwrap_or(0);
    // まず col_stats を先に計算（列走査1回目）
    let col_stats: Vec<(f64, f64)> = (0..n_dims)
        .map(|d| {
            let min = x_matrix.iter().map(|r| r[d]).fold(f64::INFINITY, f64::min);
            let max = x_matrix.iter().map(|r| r[d]).fold(f64::NEG_INFINITY, f64::max);
            (min, (max - min).max(f64::EPSILON))
        })
        .collect();
    // x_norm を計算（行走査）
    let x_norm = x_matrix
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(d, &v)| {
                    let (min, range) = col_stats[d];
                    (v - min) / range
                })
                .collect()
        })
        .collect();
    (col_stats, x_norm)
}
```

---

### REF-3: api.rs の extract_xy 共通化 🔵

**信頼性**: 🔵 *コード直接分析 + ユーザヒアリング（utils.rs 配置）より*

`compute_pdp` と `compute_pdp_2d` の `with_active_df` クロージャ内で同一の抽出ロジックが重複している。
`extract_xy` を `utils.rs` に追加し、両関数から呼ぶ。

```rust
// api.rs リファクタリング後のイメージ

pub fn compute_pdp(param_name: &str, objective_name: &str, n_grid: usize, _n_samples: usize) -> Option<PdpResult1d> {
    crate::dataframe::with_active_df(|df| {
        let param_names = df.param_col_names().to_vec();
        let n = df.row_count();
        let target_idx = param_names.iter().position(|p| p == param_name)?;
        let _ = df.objective_col_names().iter().position(|o| o == objective_name)?;

        // ★ 共通関数に置き換え
        let (x_matrix, y) = extract_xy(df, &param_names, objective_name);

        Some(compute_pdp_from_matrix(&x_matrix, &y, &param_names, objective_name, target_idx, n_grid))
    }).flatten()
}
```

---

### REF-5: ridge_core.rs の fold スタイル統一 🔵

**信頼性**: 🔵 *コード直接分析より*

```rust
// 変更前（クロージャ形式、4箇所）
let min_j = param_col.iter().cloned().fold(f64::INFINITY, |a, b| a.min(b));
let max_j = param_col.iter().cloned().fold(f64::NEG_INFINITY, |a, b| a.max(b));

// 変更後（関数ポインタ形式、kriging_core.rs と統一）
let min_j = param_col.iter().cloned().fold(f64::INFINITY, f64::min);
let max_j = param_col.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
```

---

### PERF-1: rayon 並列化 🔵

**信頼性**: 🔵 *ユーザヒアリング（制限なし・meanループのみ）より*

#### Cargo.toml への追加

```toml
[dependencies]
rayon = "1"
```

#### compute_pdp_1d_kriging_raw の並列化対象

`mean_avg` 計算（N 行のループ）のみ並列化する。
`var_centroid` は単点評価（O(N²)）なので変更不要。

```rust
// 変更前
let mean_avg: f64 = {
    let sum: f64 = x_norm
        .iter()
        .map(|row_norm| {
            let mut pt = row_norm.clone();
            pt[target_param_idx] = v_norm;
            gaussian_process::predict_mean(&model, &pt)
        })
        .sum();
    sum / n as f64
};

// 変更後（par_iter）
use rayon::prelude::*;

let mean_avg: f64 = x_norm
    .par_iter()
    .map(|row_norm| {
        let mut pt = row_norm.clone();
        pt[target_param_idx] = v_norm;
        gaussian_process::predict_mean(&model, &pt)
    })
    .sum::<f64>()
    / n as f64;
```

**Sync 確認**: `GpModel` は `Vec<f64>` / `Vec<Vec<f64>>` / `f64` のみで構成 → 自動的に `Sync`。
参照（`&model`）をクロージャ間で共有可能。

#### compute_pdp_1d_sparse_kriging_raw の並列化対象

グリッドループ自体を並列化する（各グリッド点の計算が独立しているため）。

```rust
// 変更前
let mut pdp_values = Vec::with_capacity(n_grid);
let mut y_upper_vec = Vec::with_capacity(n_grid);
let mut y_lower_vec = Vec::with_capacity(n_grid);

for &v in &grid {
    // mean_norm 計算（N × M ops）
    let mean_norm = x_norm.iter().map(...).sum::<f64>() / n as f64;
    // var_avg 計算（N × M² ops）
    let var_avg = x_norm.iter().map(...).sum::<f64>() / n as f64;
    pdp_values.push(...);
    y_upper_vec.push(...);
    y_lower_vec.push(...);
}

// 変更後（par_iter + collect でタプル収集）
use rayon::prelude::*;

let results: Vec<(f64, f64, f64)> = grid
    .par_iter()
    .map(|&v| {
        let v_norm = (v - min_j) / range_j;

        let mean_norm: f64 = x_norm
            .iter()
            .map(|row| {
                let mut pt = row.clone();
                pt[target_param_idx] = v_norm;
                sparse_fitc::fitc_predict_mean(&fitc_model, &pt)
            })
            .sum::<f64>()
            / n as f64;

        let var_avg: f64 = x_norm
            .iter()
            .map(|row| {
                let mut pt = row.clone();
                pt[target_param_idx] = v_norm;
                sparse_fitc::fitc_predict_variance(&fitc_model, &pt).max(0.0)
            })
            .sum::<f64>()
            / n as f64;

        let pdp_orig = mean_norm * y_std + y_mean;
        let std_orig = var_avg.sqrt() * y_std;
        (pdp_orig, pdp_orig + 1.96 * std_orig, pdp_orig - 1.96 * std_orig)
    })
    .collect();

// アンパック（順序保証）
let (pdp_values, y_upper_vec, y_lower_vec) = results.into_iter().fold(
    (Vec::with_capacity(n_grid), Vec::with_capacity(n_grid), Vec::with_capacity(n_grid)),
    |(mut p, mut u, mut l), (pdp, upper, lower)| {
        p.push(pdp); u.push(upper); l.push(lower);
        (p, u, l)
    },
);
```

**Sync 確認**: `SparseFitcModel` は `Vec<f64>` のみで構成 → 自動的に `Sync`。
`x_norm` は `Vec<Vec<f64>>` → `Sync`。

#### kriging_core.rs のインポート追加

```rust
use rayon::prelude::*;
```

---

## 変更ファイル一覧 🔵

**信頼性**: 🔵 *要件定義 + コード分析より*

| ファイル | 変更種別 | 変更内容 |
|---------|---------|---------|
| `rust_core/Cargo.toml` | 追記 | `rayon = "1"` を `[dependencies]` に追加 |
| `rust_core/src/pdp/utils.rs` | 拡張 | `normalize_x_minmax`, `normalize_y`, `r_squared`, `extract_xy` を追加 |
| `rust_core/src/pdp/kriging_core.rs` | リファクタリング + 性能改善 | 正規化/R² をヘルパー呼び出しに変更、rayon 並列化追加 |
| `rust_core/src/pdp/api.rs` | リファクタリング | `extract_xy` 呼び出しに変更 |
| `rust_core/src/pdp/ridge_core.rs` | スタイル統一 | fold クロージャ → 関数ポインタ形式 |

---

## 技術的制約と注意事項

### rayon + WASM の共存 🔵

**信頼性**: 🔵 *ユーザヒアリング（rust_core は WASM と分離）より*

- `rust_core` は `rlib` であり WASM バイナリに直接リンクしない
- WASM バインディングは別クレートで管理されており、そちらで `rayon` を除外する場合は
  feature flag を検討すること（本リファクタリングのスコープ外）
- 現時点では `rust_core` 側は制限なしで `rayon = "1"` を追加してよい

### par_iter の順序保証 🔵

**信頼性**: 🔵 *rayon 公開 API・Rustドキュメントより*

- `par_iter().map(...).collect::<Vec<_>>()` は元の順序を保持する（rayon の仕様）
- `pdp_values[i]` が `grid[i]` に対応することが保証される

### クロージャ内の `mut pt` 🔵

**信頼性**: 🔵 *Rust 所有権ルール + コード分析より*

- 各スレッドが `row_norm.clone()` で独自の `pt` を所有するため、
  `pt[target_param_idx] = v_norm;` の変更は他スレッドに影響しない

### r_squared の入力長チェック 🟡

**信頼性**: 🟡 *防御的プログラミングから妥当な推測*

- `y_actual.len() != y_pred.len()` の場合のパニックを防ぐため、
  呼び出し側が同一長であることを保証する（呼び出しパターンから安全と判断）

---

## 信頼性レベルサマリー

- 🔵 青信号: 15件 (88%)
- 🟡 黄信号: 2件 (12%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **設計ヒアリング**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/pdp-maintainability/requirements.md)
- **前フェーズ アーキテクチャ**: [../kriging-performance-optimization/architecture.md](../kriging-performance-optimization/architecture.md)
