# LightGBM Surface Plot 設計ヒアリング記録

**作成日**: 2026-05-01
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

要件定義・既存コードベース調査（`lgbm.rs`, `pdp/api.rs`, `ridge_core.rs`, `pdp_chart.rs`, `pdp_2d.rs`, `chart_registry.rs`）で把握したパターンを元に、設計上の判断が必要な項目をヒアリングした。

## 質問と回答

### Q1: 設計規模

**カテゴリ**: 設計方針  
**背景**: kairo-design の標準ヒアリング  
**回答**: フル設計  
**信頼性への影響**: architecture.md, dataflow.md, design-interview.md をフル作成する方針を確定

---

### Q2: 1D LightGBM PDP の計算方式

**カテゴリ**: アーキテクチャ  
**背景**: 2 種類の設計方針が考えられた
- **全特徴量で学習**: x_matrix 全カラムで LightGBM を学習、各グリッド点で target カラムを固定して他変数を周辺化（真の PDP）
- **2 変数のみ**: target カラムのみで学習（Ridge との差別化が薄い）

**回答**: 全特徴量で学習（推奨）  
**信頼性への影響**: 
- `compute_pdp_1d_lgbm()` の設計を「全特徴量学習 + 全行周辺化」方式に確定（🔵）
- dataflow の 1D フロー「rows = x_matrix.map(|r| r[param_idx] = v)」の設計を確定

---

### Q3: LightGBM 学習イテレーション数（num_iterations）

**カテゴリ**: 技術選択  
**背景**: 2 D の既存実装は `num_iterations: 100`（`LgbmRfConfig::default()` は 64）。統一するか確認

**回答**: 100（現行 2D と同じ）  
**信頼性への影響**: 
- `compute_pdp_1d_lgbm()` の config: `LgbmRfConfig { num_iterations: 100, ..Default::default() }` を確定（🔵）
- 1D/2D で一貫した LightGBM 設定になる

---

## ヒアリング結果サマリー

### 確認できた事項
- 1D PDP は全特徴量学習 + グリッド点ごとの全行周辺化（真の PDP）
- num_iterations = 100（2D と統一）
- ICE ライン・信頼区間は不要（1D: PDP 曲線 + R² のみ）
- 2D はすでに "random_forest" ディスパッチ実装済み、UI と n_grid のみ変更

### 設計方針の決定事項
1. `compute_pdp_1d_lgbm()` の戻り値は `Option<(Vec<f64>, Vec<f64>, f64)>` タプル（循環依存回避）
2. `pdp/api.rs` がタプルを `PdpResult1d` に変換（既存 2D パターンと対称）
3. LightGBM 失敗時は Ridge フォールバック（UI クラッシュなし）
4. n_grid: 1D=30（ModelType::RandomForest のみ）、2D=30（RandomForest のみ、他は 20 維持）

### 残課題
- なし（要件・設計ともに確定）

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 16件
- 🟡 黄信号: 3件
- 🔴 赤信号: 2件（計算方式・イテレーション数）

**ヒアリング後**:
- 🔵 青信号: 20件 (+4)
- 🟡 黄信号: 3件 (+0)
- 🔴 赤信号: 0件 (-2)

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **要件定義**: [requirements.md](../../spec/lightgbm-surface-plot/requirements.md)
