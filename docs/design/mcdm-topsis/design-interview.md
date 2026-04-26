# MCDM/TOPSIS UI 設計ヒアリング記録

**作成日**: 2026-04-23
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

既存のTOPSIS計算実装（rust_core）とegui-app基盤（AppState, AppMessage等）を確認し、UI配線・可視化方法・インタラクション設計について不明点を明確化した。

## 質問と回答

### Q1: 作業規模

**質問日時**: 2026-04-23
**カテゴリ**: 全体方針
**背景**: 計算ロジックは完成済みでUI配線のみの作業。設計の深度を確認するため。

**回答**: フル設計（推奨）を選択

**信頼性への影響**:
- 全設計文書（アーキテクチャ・データフロー・型定義・ヒアリング記録）を作成

---

### Q2: 計算トリガー

**質問日時**: 2026-04-23
**カテゴリ**: 技術選択
**背景**: theory/topsis.mdに「リアルタイム変更→スコア再計算」の記載があるが、50K×4で100ms以内のためどちらも可能。ImportanceChartとの一貫性も考慮点。

**回答**: オンデマンド（Runボタン）

**信頼性への影響**:
- ImportanceChartパターンを踏襲 → ウィジェット設計が🔵 に
- pending_compute パターンが確定 → chart_registry設計が🔵 に

---

### Q3: 可視化形式

**質問日時**: 2026-04-23
**カテゴリ**: UI設計
**背景**: theory/topsis.mdにはバーチャートとハイライトの記載がある。テーブルやカラーモード追加の要望確認。

**回答**: 全選択（ランキングバーチャート + ランキングテーブル + 散布図カラーモード）

**信頼性への影響**:
- 3つの可視化コンポーネント設計が必要 → interfaces.rs の設計項目増
- ColorMode::TopsisScore 追加が確定 → app_state.rs 変更範囲が🔵 に

---

### Q4: TopsisResult型不一致の解決

**質問日時**: 2026-04-23
**カテゴリ**: データモデル
**背景**: rust_core側に5フィールド（scores, ranked_indices, positive_ideal, negative_ideal, duration_ms）、egui-app側に2フィールド（scores, ranking: Vec<usize>）。フィールド名も型も異なる。

**回答**: rust_coreに統一（推奨）

**信頼性への影響**:
- `results.rs` の TopsisResult を rust_core 定義に置き換え → 型定義が🔵 に
- positive_ideal/negative_ideal もUIで表示可能に

---

### Q5: 重み設定UI

**質問日時**: 2026-04-23
**カテゴリ**: UI設計
**背景**: 目的関数ごとの重み設定方法。均等重み等のプリセットを用意するか、シンプルに個別スライダーのみにするか。

**回答**: 個別スライダー（推奨）

**信頼性への影響**:
- ウィジェット構造が確定 → TopsisChart設計が🔵 に
- 正規化ロジックはUI側で実行（sum(w)で割る）

---

### Q6: バーチャート描画方式

**質問日時**: 2026-04-23
**カテゴリ**: 技術選択
**背景**: egui_plot::BarChart か egui::Painter フルカスタムか。最大20本の横棒なのでどちらでも性能問題なし。

**回答**: egui_plot Bar（推奨）

**信頼性への影響**:
- egui_plot使用確定 → バーチャート描画設計が🔵 に
- PlotResponse.hovered_bar_item でクリック検出パターンが確定

---

### Q7: 上位N件表示

**質問日時**: 2026-04-23
**カテゴリ**: UI設計
**背景**: theory/topsis.mdに「5/10/20件を切り替え」の記載。自由入力にするか固定選択にするか。

**回答**: 5/10/20 トグル（推奨）

**信頼性への影響**:
- トグルボタン実装確定 → ウィジェットUI設計が🔵 に

---

### Q8: 既存コードの詳細分析

**質問日時**: 2026-04-23
**カテゴリ**: 調査深度
**背景**: エージェントによる既存コード調査は完了しているが、ウィジェット実装パターンの詳細確認が必要か。

**回答**: 必要

**信頼性への影響**:
- ImportanceChart・SensitivityHeatmap・ClusterScatterの実装パターンが詳細に判明
- chart_registry・message_handlerのディスパッチロジックが確定
- 全ウィジェット配線パターンが🔵 に

## ヒアリング結果サマリー

### 確認できた事項
- 計算トリガー: オンデマンド（Runボタン）- ImportanceChartパターン踏襲
- 可視化: バーチャート + テーブル + カラーモードの3本柱
- 型: rust_core定義に統一
- 重みUI: 個別スライダー（0.0-1.0、内部正規化）
- 描画: egui_plot::BarChart
- 表示件数: 5/10/20トグル

---

### Q9: MCDM手法の統一UI（追記ヒアリング）

**質問日時**: 2026-04-23
**カテゴリ**: アーキテクチャ
**背景**: 将来的にVIKOR・PROMETHEE IIの実装を予定。ImportanceChartのようにMcdmMethod enumで手法を切替える統一UIが可能か確認。

**回答**: McdmChartに変更（推奨）を選択
- ImportanceChartの `ImportanceMetric` enumパターンを踏襲
- `McdmMethod` enum（初期: Topsisのみ）で手法切替
- `McdmResult` enum で結果をラップ、`primary_scores()` で共通アクセス
- 将来のVIKOR/PROMETHEE II追加はバリアント追加のみ

**信頼性への影響**:
- ウィジェット名が `TopsisChart` → `McdmChart` に変更
- ChartId が `TopsisChart` → `McdmChart` に変更
- ColorMode が `TopsisScore` → `McdmScore` に変更
- AppState が `topsis_result` → `mcdm_result` に変更
- AppMessage が `TopsisDone` → `McdmDone` に変更
- 設計全体の一貫性が向上（手法追加時の変更範囲が最小化）

---

## ヒアリング結果サマリー

### 確認できた事項
- 計算トリガー: オンデマンド（Runボタン）- ImportanceChartパターン踏襲
- 可視化: バーチャート + テーブル + カラーモードの3本柱
- 型: rust_core定義に統一
- 重みUI: 個別スライダー（0.0-1.0、内部正規化）
- 描画: egui_plot::BarChart
- 表示件数: 5/10/20トグル
- UI統一: McdmChart + McdmMethod enum（VIKOR/PROMETHEE II将来対応）

### 設計方針の決定事項
1. 新規ファイル: `egui-app/src/ui/widgets/mcdm_chart.rs`
2. TopsisResult を rust_core 定義に統一（`results.rs` 変更）
3. ChartId::McdmChart 追加（`layout_state.rs` 変更）
4. WidgetStates に mcdm フィールド追加（`widget_states.rs` 変更）
5. chart_registry にディスパッチ追加（`chart_registry.rs` 変更）
6. 右パネルに "MCDM" グループ追加（`right_panel.rs` 変更）
7. ColorMode::McdmScore 追加（`app_state.rs` 変更）
8. McdmMethod enum + McdmResult enum で手法統一（将来拡張対応）

### 残課題
- egui_plot::BarChart のクリックイベントAPIの確認（実装時に検証）🟡
- テーブルとバーチャートのレイアウト比率（ウィジェット内での配置）🟡

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 4件
- 🟡 黄信号: 8件
- 🔴 赤信号: 2件

**ヒアリング後（Q9追記込み）**:
- 🔵 青信号: 24件 (+20)
- 🟡 黄信号: 2件 (-6)
- 🔴 赤信号: 0件 (-2)

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **理論文書**: [theory/topsis.md](../../../theory/topsis.md)
- **egui移行設計**: [../egui-migration/architecture.md](../egui-migration/architecture.md)
