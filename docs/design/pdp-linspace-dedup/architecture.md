# pdp-linspace-dedup アーキテクチャ設計

**作成日**: 2026-05-01
**関連要件定義**: [requirements.md](../../spec/pdp-linspace-dedup/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 要件定義書・既存実装を参考にした確実な設計
- 🟡 **黄信号**: 要件定義書・既存実装から妥当な推測による設計
- 🔴 **赤信号**: 要件定義書・既存実装にない推測による設計

---

## 変更概要 🔵

**信頼性**: 🔵 *要件定義書 REQ-001~004 より*

`linspace` 関数を `core::math` モジュールに集約し、`pdp::utils` と `core::lgbm` の両方から参照する。既存の `pdp` → `core` 依存方向を維持し、逆依存は発生しない。

## 変更ファイル一覧 🔵

**信頼性**: 🔵 *既存コードベース調査より*

| ファイル | 変更内容 |
|---|---|
| `rust_core/src/core/math/grid.rs` | **新規**: `linspace` 関数（`pub(crate)`） |
| `rust_core/src/core/math/mod.rs` | `pub(crate) mod grid;` 追加 |
| `rust_core/src/core/lgbm.rs` | `pdp_linspace` 定義を削除、`use crate::core::math::grid::linspace;` に置換 |
| `rust_core/src/pdp/utils.rs` | `linspace` 定義を削除、`use crate::core::math::grid::linspace;` に置換 |

## 詳細設計

### 1. core::math::grid モジュールの新規作成 🔵

**信頼性**: 🔵 *既存 core::math/mod.rs 構造より*

```rust
// rust_core/src/core/math/grid.rs

/// 等間隔グリッドを生成する
pub(crate) fn linspace(min: f64, max: f64, n: usize) -> Vec<f64> {
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![(min + max) / 2.0];
    }
    (0..n)
        .map(|i| min + (max - min) * i as f64 / (n - 1) as f64)
        .collect()
}
```

`rust_core/src/core/math/mod.rs` に `pub(crate) mod grid;` を追加。

### 2. lgbm.rs の修正 🔵

**信頼性**: 🔵 *既存 lgbm.rs の pdp_linspace 使用箇所（3件）より*

- `fn pdp_linspace(...)` 定義（行363-373）を削除
- ファイル先頭に `use crate::core::math::grid::linspace;` を追加
- 呼び出し箇所（`pdp_linspace` → `linspace`）を置換（3箇所: 行343, 344, 409）

### 3. pdp::utils の修正 🔵

**信頼性**: 🔵 *既存 pdp/utils.rs の linspace 使用箇所より*

- `pub(super) fn linspace(...)` 定義（行23-33）を削除
- `use crate::core::math::grid::linspace;` を追加
- `ridge_core.rs` と `kriging_core.rs` の `use super::utils::{..., linspace};` は変更不要（re-export 不要、直接インポート可能）

### 呼び出し元への影響分析 🔵

**信頼性**: 🔵 *既存コード grep 調査より*

| 呼び出し元 | 現在の import | 変更後 |
|---|---|---|
| `pdp/ridge_core.rs` | `use super::utils::{col_mean_std, linspace};` | `use crate::core::math::grid::linspace;` + `use super::utils::col_mean_std;` に分割 |
| `pdp/kriging_core.rs` | `use super::utils::linspace;` | `use crate::core::math::grid::linspace;` に変更 |

**注意**: `pdp::utils` から `linspace` が削除されるため、`ridge_core.rs` は `col_mean_std` と `linspace` の import を分離する必要がある 🔵

## 技術的制約

### 循環依存の回避 🔵

**信頼性**: 🔵 *モジュール構造 pdp/mod.rs・core/mod.rs より*

- `core` は `pdp` に依存しない（現状維持）
- `pdp` → `core::math::grid` の依存は既存の `pdp` → `core` 方向に沿う
- 新たな循環依存は発生しない

### 可視性 🔵

**信頼性**: 🔵 *既存 core::math/mod.rs の `pub(crate)` パターンより*

- `core::math::grid::linspace` は `pub(crate)` で crate 内全域からアクセス可能
- `pdp::utils::col_mean_std` は `pub(super)` のままで変更なし

## 信頼性レベルサマリー

- 🔵 青信号: 8件 (100%)
- 🟡 黄信号: 0件 (0%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質 — 全設計項目が既存コード調査に基づく
