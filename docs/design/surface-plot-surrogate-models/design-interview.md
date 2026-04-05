# 3D Surface Plot サロゲートモデル拡張 設計ヒアリング記録

**作成日**: 2026-04-05
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

既存の `SurfacePlot3D` コンポーネントと `computePdp2d` WASM 関数の実装状況を確認し、Random Forest および Kriging のサロゲートモデル実装方針を明確化するためのヒアリングを実施した。

## ヒアリング前の既存実装調査結果

- `rust_core/src/pdp.rs`: Ridge 回帰のみ実装。`compute_pdp_2d_from_matrix()` が解析的 PDP を計算
- `rust_core/src/lib.rs`: `wasm_compute_pdp_2d(p1, p2, obj, n_grid)` — model_type 引数なし
- `frontend/src/stores/analysisStore.ts`: `surrogateModelType` を cache key に含めるが **WASM には渡していない**（バグ）
- `frontend/src/components/charts/SurfacePlot3D.tsx`: RF/Kriging オプションは `disabled: true` のスタブ

## 質問と回答

### Q1: この設計の作業規模について教えてください

**質問日時**: 2026-04-05
**カテゴリ**: 設計規模
**背景**: 設計の深さ・出力ファイル範囲を確定するため

**回答**: フル設計（推奨）— 包括的なアーキテクチャ設計、詳細なデータフロー、完全な型定義を含む

**信頼性への影響**:

- すべての設計ファイルを作成するスコープが確定（信頼性: 🔵）

---

### Q2: Random Forest の実装方法を教えてください

**質問日時**: 2026-04-05
**カテゴリ**: 技術選択
**背景**: RF の実装手段として純 Rust スクラッチ・linfa クレート・軽量アンサンブルの 3 案があった。WASM バイナリサイズと依存関係の観点で選択が必要

**回答**: Rust 純粋実装（推奨）— rust_core 内に RF をスクラッチ実装。依存なし・ブラウザ完結

**信頼性への影響**:

- `rust_core/src/rf.rs` を新規作成し、CART 決定木 + Bagging をスクラッチ実装する設計が確定（信頼性: 🔵）
- linfa クレートや ONNX モデルファイルの読み込みは設計スコープ外（信頼性: 🔵）

---

### Q3: Kriging（ガウス過程回帰）の実装範囲を教えてください

**質問日時**: 2026-04-05
**カテゴリ**: 技術選択
**背景**: Kriging（GP 回帰）には実装コストが高い完全 GP、近似法、次フェーズ送りの 3 案があった

**回答**: 完全な GP とするが、ARD（Automatic Relevance Determination）で自動関連度推定を行い GP のコストを下げてください

**補足解釈**: ARD は各特徴量次元に独立した長さスケールを持つ RBF カーネル。2D 部分空間 GP（2 次元のみを入力とする）と組み合わせることで計算コストを抑制する。

**信頼性への影響**:

- `rust_core/src/kriging.rs` を新規作成し、ARD RBF カーネル + Cholesky 分解 + 勾配降下法ハイパーパラメータ最適化を実装する設計が確定（信頼性: 🔵）
- カーネル: `k(x1, x2) = σ_f² · exp(−½ · Σ_d ((x1_d − x2_d) / l_d)²)`（信頼性: 🔵）

---

### Q4: `computePdp2d` にモデル種別パラメータを追加する設計を採用しますか？

**質問日時**: 2026-04-05
**カテゴリ**: アーキテクチャ・API設計
**背景**: 既存の `computePdp2d(p1, p2, obj, nGrid)` に model_type を追加する案と、モデルごとに別関数とする案があった

**回答**: model_type 引数を追加（推奨）— 1 関数で全モデル対応

**信頼性への影響**:

- `wasm_compute_pdp_2d(p1, p2, obj, n_grid, model_type: &str)` とシグネチャを変更する設計が確定（信頼性: 🔵）
- TypeScript 側も `computePdp2d(p1, p2, obj, nGrid, modelType: string)` に更新（信頼性: 🔵）

---

### Q5: Random Forest の具体的なアルゴリズムを教えてください

**質問日時**: 2026-04-05
**カテゴリ**: アルゴリズム選択
**背景**: CART + Bagging・Extremely Randomized Trees・PCA+Ridge の 3 案があった

**回答**: CART 決定木 + Bagging（推奨）— 展開消失分散ベースの CART を Bootstrap サンプルで複数本学習

**信頼性への影響**:

- RF アルゴリズム: Bootstrap サンプリング + MSE 最小化 CART + 100 本アンサンブルの設計が確定（信頼性: 🔵）
- ハイパーパラメータ: `n_trees=100, max_depth=10, min_samples_leaf=2`（信頼性: 🟡 推奨値から推測）

---

### Q6: GP Kriging の訓練データ量とパフォーマンス目標を教えてください

**質問日時**: 2026-04-05
**カテゴリ**: パフォーマンス
**背景**: GP は O(N³) のため N が大きいと遅くなる。n≤1000 全件 vs n≤5000 近似法の 2 案があった

**回答**: n≤5000 導向（近似法考慮）— n > 1000 時は Sparse GP 等の近似法を並用

**信頼性への影響**:

- n > 1000 のとき訓練データを 1000 点にランダムサブサンプリングする設計が確定（信頼性: 🔵）
- n > 5000 はサポート範囲外（エラーまたは上限クリップ）（信頼性: 🟡）

---

## ヒアリング結果サマリー

### 確認できた事項

- Random Forest: CART（MSE 最小化 CART 決定木）+ Bagging、純 Rust 実装、`rf.rs` 新規作成
- Kriging: 完全 GP、ARD RBF カーネル（次元ごと独立長さスケール）、Cholesky 分解、`kriging.rs` 新規作成
- API: `computePdp2d` に `model_type: &str` 引数を追加（既存の 1 関数を拡張）
- パフォーマンス: n > 1000 で 1000 点ランダムサブサンプリング（Kriging のみ）
- 2D 部分空間射影: (param1, param2) 列のみ抽出してモデル学習（計算量を O(N × p) から O(N × 2) に削減）

### 設計方針の決定事項

- `rust_core/src/rf.rs` 新規作成（CART + Bagging）
- `rust_core/src/kriging.rs` 新規作成（ARD RBF + Cholesky + 勾配降下最適化）
- `rust_core/src/pdp.rs`: `compute_pdp_2d()` に `model_type: &str` 追加・ディスパッチ追加
- `rust_core/src/lib.rs`: `wasm_compute_pdp_2d()` に `model_type: &str` 追加
- `frontend/src/wasm/wasmLoader.ts`: `computePdp2d` に `modelType: string` 追加
- `frontend/src/stores/analysisStore.ts`: `surrogateModelType` を WASM に渡すよう修正（既存バグ修正）
- `frontend/src/components/charts/SurfacePlot3D.tsx`: RF/Kriging の `disabled: true` 解除

### 残課題

- Kriging ハイパーパラメータ最適化（勾配降下 100 ステップ）のデフォルト値は実装時に要チューニング
- RF の OOB R² 計算: Bootstrap サンプル外データが少ない場合の訓練 R² フォールバック基準（N_oob の閾値）
- Kriging の Cholesky 分解安定性: ジッター値（`1e-6`）が不十分な場合の対処方法
- WASM バイナリサイズ増加の見積もり（Cholesky / CART 実装追加後に確認必要）

### 信頼性レベル分布

**ヒアリング前（推測段階）:**

- 🔵 青信号: 2件
- 🟡 黄信号: 4件
- 🔴 赤信号: 6件

**ヒアリング後（全 6 問回答後）:**

- 🔵 青信号: 14件 (+12)
- 🟡 黄信号: 3件 (-1)
- 🔴 赤信号: 0件 (-6)

---

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.ts](interfaces.ts)
- **前フェーズ ヒアリング記録**: [../mode-frontier-features/design-interview.md](../mode-frontier-features/design-interview.md)
