# sensitivity-all-models 設計ヒアリング記録

**作成日**: 2026-04-15

## ヒアリング目的

感度分析の全メトリクス対応・計算トリガー追加の仕様を確認。

---

## 質問と回答

### Q1: 「すべての手法」の範囲

**カテゴリ**: アーキテクチャ
**背景**: ImportanceChart には Spearman/Ridge/Sobol のメトリクスが定義済みだが計算・表示されているのは Spearman のみ。また計算トリガーが UI に存在しない。

**回答**: 計算トリガー追加 + 全メトリクス表示の両方

**信頼性への影響**:
- Run ボタン追加が確定 → PdpChart2D の `pending_compute` パターンを採用
- ImportanceChart が SobolResult を受け取る必要があることが確定

---

### Q2: Run ボタンの配置

**カテゴリ**: UI/UX
**背景**: PdpChart2D はウィジェット内に Run ボタンを持つ。左パネル/ツールバーの選択肢もあった。

**回答**: ウィジェット内（PdpChart2D と同じパターン）

**信頼性への影響**:
- `pending_compute: Option<ImportanceMetric>` フィールドを `ImportanceChart` に追加することが確定

---

### Q3: Sobol の計算タイミング

**カテゴリ**: パフォーマンス
**背景**: Spearman/Ridge/RF ANOVA は `compute_sensitivity()` で一括計算されるが、Sobol は別関数・計算コストが高い。

**回答**: 選択している手法のみ計算対象とする（一括ではなくメトリクス別に計算）

**信頼性への影響**:
- rust_core のリファクタリングが必要であることが確定
- `SensitivityMetric` 列挙型と `compute_sensitivity_for(metric)` の新設が確定

---

### Q4: ImportanceMetric への RF ANOVA 追加

**カテゴリ**: データモデル
**背景**: `ImportanceMetric` 列挙型には現在 Spearman/Ridge/Sobol しかなく、RF ANOVA が欠落していることが調査で判明。rust_core には RF ANOVA 計算が実装済み。

**回答**: RF ANOVA も ImportanceMetric に追加する

**信頼性への影響**:
- `ImportanceMetric::RfAnova` の追加が確定
- 既存テスト（`importance_metric_labels_not_empty`）に RfAnova ケースの追加が必要

---

## ヒアリング結果サマリー

### 確認できた事項
- Run ボタンはウィジェット内（PdpChart2D パターン）
- 選択中のメトリクスのみ計算（メトリクス別の個別計算関数が必要）
- Sobol は別計算関数（`compute_sobol`）のまま
- RF ANOVA を ImportanceMetric に追加する

### 設計方針の決定事項
- rust_core: `SensitivityMetric` 列挙型 + `compute_sensitivity_for(metric)` を追加
- rust_core: `full.rs` に `compute_spearman_only` / `compute_ridge_only` / `compute_rf_anova_only` を追加
- egui-app: `SensitivityResult` に `ridge` / `rf_anova` フィールドを追加
- egui-app: `ImportanceMetric::RfAnova` を追加
- egui-app: `ImportanceChart` に `pending_compute` フィールドと Run ボタンを追加
- egui-app: `grid_canvas.rs` でメトリクス別に `spawn_task` を分岐
- 後方互換: `compute_sensitivity()` / `compute_sensitivity_all()` は変更なし

### 残課題
- なし（設計として十分に定義済み）

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 4件
- 🟡 黄信号: 4件
- 🔴 赤信号: 2件

**ヒアリング後**:
- 🔵 青信号: 10件 (+6)
- 🟡 黄信号: 2件 (-2)
- 🔴 赤信号: 0件 (-2)

---

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
