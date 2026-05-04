# pdp-maintainability 設計ヒアリング記録

**作成日**: 2026-05-04
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

要件定義書・既存コード・前フェーズ設計文書を確認し、技術的な設計方針を決定するためのヒアリングを実施。

---

## 質問と回答

### Q1: 設計規模について

**質問日時**: 2026-05-04
**カテゴリ**: 優先順位
**背景**: フル設計（アーキテクチャ・データフロー・型定義・すべて）か軽量版かを確認

**回答**: フル設計（推奨）

**信頼性への影響**:
- すべての設計項目（architecture.md, dataflow.md, interfaces.rs, design-interview.md）を作成対象に
- 各設計要素が 🔵 確実な設計として記録される

---

### Q2: compute_pdp_1d_kriging_raw の rayon 並列化範囲

**質問日時**: 2026-05-04
**カテゴリ**: 技術選択
**背景**: `compute_pdp_1d_kriging_raw` には以下2種の並列化候補があった:
- **グリッドループ並列化**: `for &v in &grid` 全体を `par_iter()` にする（n_grid × N × N² の粒度）
- **meanループのみ並列化**: `x_norm.iter()` を `par_iter()` にする（グリッドループは逐次, N のみ並列）

グリッドループ並列化は粒度が大きく効果が高いが、クロージャ内に `centroid_norm.clone()` と variance 計算があり、コードが複雑になる。
meanループのみであれば最小変更で済む（variance の `centroid_pt` は単点なので元のまま）。

**回答**: meanループのみ並列化（推奨）

**信頼性への影響**:
- REQ-503 の実装方針が確定（🔵）
- グリッドループ並列化は将来の最適化として残す

---

### Q3: extract_xy の配置場所

**質問日時**: 2026-05-04
**カテゴリ**: アーキテクチャ
**背景**: `extract_xy` を以下2か所に配置する候補があった:
- **api.rs 内**: `with_active_df` クロージャ内で直接利用でき、DF型への依存が局所化される
- **utils.rs に移動**: 他モジュールから再利用しやすい反面、`utils.rs` が `data::DataFrame` に依存する

現状 `utils.rs` は `core::math::stats` にのみ依存しており、`data::DataFrame` を追加すると依存グラフが広がる。
一方 `api.rs` に閉じれば既存の依存関係を変えない。

**回答**: utils.rs に移動

**信頼性への影響**:
- REQ-301 の実装方針が確定（🔵）
- `utils.rs` が `crate::data::DataFrame` に依存するようになる（許容済み）

---

## ヒアリング結果サマリー

### 確認できた事項
- フル設計で進める
- `compute_pdp_1d_kriging_raw` は mean ループのみ並列化（variance はセントロイド単点のため不要）
- `extract_xy` は `utils.rs` に配置（`data::DataFrame` への依存を追加）
- `compute_pdp_1d_sparse_kriging_raw` のグリッドループは全体を `par_iter()` で並列化

### 設計方針の決定事項
1. **rayon 導入**: `Cargo.toml` に `rayon = "1"` を追加
2. **kriging_core.rs**: `use rayon::prelude::*;` を追加
3. **mean ループのみ**: `x_norm.par_iter()` で mean_avg 計算を並列化
4. **sparse_kriging**: グリッドループ全体を `grid.par_iter()` で並列化し、タプル `(pdp, upper, lower)` で収集
5. **utils.rs**: 4関数（`normalize_x_minmax`, `normalize_y`, `r_squared`, `extract_xy`）を追加
6. **ridge_core.rs**: fold クロージャを `f64::min`/`f64::max` の関数ポインタに統一

### 残課題
- `SparseFitcModel` の `Sync` 確認が実装時に必要（分析上は全フィールドが `Vec<f64>` で問題なし）
- WASM バインディング側が rayon を受け入れるか、実際のビルドで確認が必要

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 8件
- 🟡 黄信号: 4件
- 🔴 赤信号: 4件

**ヒアリング後**:
- 🔵 青信号: 16件 (+8)
- 🟡 黄信号: 2件 (-2)
- 🔴 赤信号: 0件 (-4)

---

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **要件定義**: [requirements.md](../../spec/pdp-maintainability/requirements.md)
