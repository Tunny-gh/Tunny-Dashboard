# sensitivity-refactoring 設計ヒアリング記録

**作成日**: 2026-05-04
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

コード分析・要件定義で特定した設計判断点（Trait ディスパッチ方式、前処理統一、定数配置先）についてユーザーと確認。

---

## 質問と回答

### Q1: TreeMetric Trait のディスパッチ方式

**質問日時**: 2026-05-04
**カテゴリ**: 技術選択
**背景**: `analysis/full.rs` が `TreeMetric` トレイトを通じて MDI/SHAP/RF-ANOVA/PFI を呼び出す際、静的ディスパッチ（ジェネリクス）と動的ディスパッチ（`Box<dyn TreeMetric>`）の2択があった。静的の方が実行時オーバーヘッドがなく、既存コードのスタイルにも合致する。

**回答**: `静的ディスパッチ（推奨）`

**信頼性への影響**:
- `architecture.md` の「静的ディスパッチによる呼び出し」セクションが 🟡 → 🔵 に向上
- `dataflow.md` の「静的ディスパッチの型パラメータフロー」が 🟡 → 🔵 に向上
- `interfaces.rs` の `run_tree_metric_for_all_objectives<M: TreeMetric>` シグネチャが確定

---

### Q2: mdi.rs / shap.rs の `prepare_training_data` 統一

**質問日時**: 2026-05-04
**カテゴリ**: アーキテクチャ
**背景**: `tree_common.rs::prepare_training_data` が既に NaN/Inf フィルタリング・ダウンサンプリング・80/20分割・Fisher-Yatesシャッフルを統一処理として提供済みだが、`mdi.rs` と `shap.rs` はそれを使わずインライン実装していた。統一することで約50〜60行 × 2ファイル = 約100〜120行を削減できる。

**回答**: `統一する（推奨）`

**信頼性への影響**:
- `dataflow.md` の「mdi.rs / shap.rs の前処理統一フロー」が 🟡 → 🔵 に向上
- `architecture.md` の mdi.rs/shap.rs の変更内容が確定

---

### Q3: 定数集約の場所

**質問日時**: 2026-05-04
**カテゴリ**: 技術選択
**背景**: `MAX_ROWS` 等の定数を集約する場所として「`tree_common.rs` に追加」と「`constants.rs` を新規作成」の2択があった。前者はファイル追加なしで済むが、後者は定数の責務を明確に分離できる。

**回答**: `constants.rs を新規作成`

**信頼性への影響**:
- `architecture.md` の `sensitivity/constants.rs` セクションが 🟡 → 🔵 に向上
- `interfaces.rs` の `constants.rs` 定義が確定

---

## ヒアリング結果サマリー

### 確認できた事項

- 静的ディスパッチ（ジェネリクス）で `TreeMetric` を利用する
- mdi.rs / shap.rs は `prepare_training_data` に統一する
- 定数は `sensitivity/constants.rs` に集約する

### 設計方針の決定事項

1. `fn run_tree_metric_for_all_objectives<M: TreeMetric>(...)` を `analysis/full.rs` または `analysis/common.rs` に定義
2. `MdiMetric::compute_importances` は `PreparedData` を受け取り（前処理済み）、LightGBM 訓練と重要度計算のみを担当
3. `constants.rs` は `pub(crate) mod constants;` として `sensitivity/mod.rs` に追加
4. `pdp/utils.rs::col_mean_std` は `core::math::stats::column_mean_std` への委譲として実装（シグネチャ変更なし）

### 残課題

- `run_tree_metric_for_all_objectives` に `PreparedData` を渡す形にすると、`max_rows()` / シードは `prepare_training_data` 呼び出し側（`full.rs`）が管理する必要がある。`TreeMetric` から `max_rows` / シードを分離するか否かはタスク実装時に最終確定する（設計文書では含む形で記載）
- `mdi.rs` 内の既存テスト（`importances_sum_to_one` 等）は `compute_mdi_importances` 関数の公開シグネチャを通じてテストしているため、内部実装の変更後もそのまま利用可能

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 6件
- 🟡 黄信号: 6件
- 🔴 赤信号: 0件

**ヒアリング後**:
- 🔵 青信号: 12件 (+6)
- 🟡 黄信号: 0件 (-6)
- 🔴 赤信号: 0件

---

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型定義（Rust）**: [interfaces.rs](interfaces.rs)
- **要件定義**: [requirements.md](../../spec/sensitivity-refactoring/requirements.md)
