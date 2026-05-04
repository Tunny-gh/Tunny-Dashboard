# pdp-maintainability ユーザストーリー

**作成日**: 2026-05-04
**関連要件定義**: [requirements.md](requirements.md)
**ヒアリング記録**: [interview-record.md](interview-record.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なストーリー
- 🟡 **黄信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるストーリー
- 🔴 **赤信号**: PRD・EARS要件定義書・設計文書・ユーザヒアリングにない推測によるストーリー

---

## エピック1: コード重複の排除（REF系）

### ストーリー 1.1: 正規化ヘルパーを提供する 🔵

**信頼性**: 🔵 *コード直接分析・ユーザヒアリングより*

**私は** `pdp` モジュールのメンテナー **として**
**kriging_core.rs の正規化コードブロックを1か所に集約したい**
**そうすることで** バグ修正や仕様変更が1箇所で済み、見落としが減る

**関連要件**: REQ-101, REQ-102, REQ-103

**詳細シナリオ**:
1. `rust_core/src/pdp/utils.rs` に `normalize_x_minmax` と `normalize_y` を実装する
2. `kriging_core.rs` の `compute_pdp_1d_kriging_raw` が新ヘルパーを呼ぶよう置き換える
3. `kriging_core.rs` の `compute_pdp_1d_sparse_kriging_raw` が新ヘルパーを呼ぶよう置き換える
4. `cargo test` が全件パスすることを確認する

**前提条件**:
- 現在の `utils.rs` は `col_mean_std` のみを公開している

**制約事項**:
- ヘルパーは `pub(super)` スコープに留める（モジュール外へ公開しない）
- `range == 0` 列は `f64::EPSILON` でクランプする

**優先度**: Must Have

---

### ストーリー 1.2: R² 計算を共通関数にまとめる 🔵

**信頼性**: 🔵 *コード直接分析・ユーザヒアリングより*

**私は** `pdp` モジュールのメンテナー **として**
**R² の計算式を1か所に集約したい**
**そうすることで** ゼロ除算ガードの漏れや定義の揺れを防げる

**関連要件**: REQ-201, REQ-202

**詳細シナリオ**:
1. `utils.rs` に `r_squared(y_actual: &[f64], y_pred: &[f64]) -> f64` を実装する
2. `ss_tot < f64::EPSILON` のとき `1.0` を返す処理を含める
3. `compute_pdp_1d_kriging_raw`、`compute_pdp_2d_kriging_raw`、`compute_pdp_2d_sparse_kriging_raw` を新関数に置き換える
4. `cargo test` が全件パスすることを確認する

**前提条件**:
- 3関数の R² 計算ロジックは等価（`ss_tot`/`ss_res` の構造が同一）

**優先度**: Must Have

---

### ストーリー 1.3: DataFrame からの xy 抽出を共通関数にまとめる 🔵

**信頼性**: 🔵 *コード直接分析・ユーザヒアリングより*

**私は** `pdp` モジュールのメンテナー **として**
**`api.rs` で重複している DataFrame → Vec の変換コードを1か所に集約したい**
**そうすることで** 列マッピングのロジック変更が1か所で済む

**関連要件**: REQ-301, REQ-302

**詳細シナリオ**:
1. `api.rs` に `fn extract_xy(df: &impl DfAccessor, param_names: &[String], objective_name: &str) -> (Vec<Vec<f64>>, Vec<f64>)` を実装する
2. `compute_pdp` の内部 closure が新関数を使うよう置き換える
3. `compute_pdp_2d` の内部 closure が新関数を使うよう置き換える
4. `cargo test` が全件パスすることを確認する

**前提条件**:
- `with_active_df` のクロージャ内で呼べること（借用チェックを考慮）

**優先度**: Must Have

---

### ストーリー 1.4: min/max の fold スタイルを統一する 🔵

**信頼性**: 🔵 *コード直接分析より*

**私は** `pdp` モジュールのメンテナー **として**
**`ridge_core.rs` の `fold(f64::INFINITY, |a, b| a.min(b))` を `fold(f64::INFINITY, f64::min)` に統一したい**
**そうすることで** `kriging_core.rs` と一貫したスタイルになりレビューが容易になる

**関連要件**: REQ-601

**詳細シナリオ**:
1. `ridge_core.rs` の `compute_pdp_from_matrix` と `compute_pdp_2d_from_matrix` の fold クロージャをすべて関数形式に変換する
2. `cargo clippy` が警告なしで通ることを確認する

**優先度**: Should Have

---

## エピック2: rayon による並列化（PERF系）

### ストーリー 2.1: Sparse Kriging PDP ループを並列化する 🔵

**信頼性**: 🔵 *ユーザヒアリング・rayon の公開 API より*

**私は** 大量データ（N > 500）でダッシュボードを使うユーザー **として**
**Sparse Kriging の PDP 計算を高速化したい**
**そうすることで** インタラクティブな操作で待ち時間が短縮される

**関連要件**: REQ-501, REQ-502

**詳細シナリオ**:
1. `Cargo.toml` に `rayon = "1"` を追加する
2. `kriging_core.rs` の先頭に `use rayon::prelude::*;` を追加する
3. `compute_pdp_1d_sparse_kriging_raw` の PDP ループ（`for &v in &grid`）を  
   `grid.par_iter().map(|&v| { ... }).collect()` 形式に変換する
4. 各グリッド点の `mean_norm`・`var_avg` を並列で計算し、結果をタプルで集める
5. `cargo test` が全件パスすることを確認する
6. Release ビルドでのパフォーマンス測定を行う（N=1000, n_grid=50）

**前提条件**:
- `rayon` がインストール済みであること
- `fitc_predict_mean`・`fitc_predict_variance` が `Send + Sync` であること

**制約事項**:
- WASM バイナリに `rayon` が含まれないよう、WASM ラッパークレート側は `wasm-bindgen-rayon` または feature flag で管理する（rust_core 側は制限なし）

**優先度**: Must Have

---

### ストーリー 2.2: Standard Kriging の mean ループを並列化する 🔵

**信頼性**: 🔵 *ユーザヒアリングより*

**私は** 中規模データ（N = 100〜300）でダッシュボードを使うユーザー **として**
**Standard Kriging の PDP 計算を高速化したい**
**そうすることで** `n_grid=50` 程度でもスムーズに結果が表示される

**関連要件**: REQ-501, REQ-503

**詳細シナリオ**:
1. `compute_pdp_1d_kriging_raw` の mean_avg 計算（`x_norm.iter().map(...)` による N 回ループ）を  
   `x_norm.par_iter().map(...)` で並列化する
2. variance 計算（centroid 単点のみ）は変更不要
3. `cargo test` が全件パスすることを確認する

**優先度**: Must Have

---

## ストーリーマップ

```
エピック1: コード重複排除
├── ストーリー 1.1 正規化ヘルパー      (🔵 Must Have)
├── ストーリー 1.2 R² 共通関数          (🔵 Must Have)
├── ストーリー 1.3 DataFrame 抽出共通化 (🔵 Must Have)
└── ストーリー 1.4 min/max スタイル統一 (🔵 Should Have)

エピック2: rayon 並列化
├── ストーリー 2.1 Sparse Kriging 並列化 (🔵 Must Have)
└── ストーリー 2.2 Standard Kriging 並列化 (🔵 Must Have)
```

## 信頼性レベルサマリー

- 🔵 青信号: 6件 (100%)
- 🟡 黄信号: 0件 (0%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
