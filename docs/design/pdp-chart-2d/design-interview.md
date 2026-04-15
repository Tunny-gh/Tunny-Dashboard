# PDP Chart 2D 設計ヒアリング記録

**作成日**: 2026-04-15
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

既存コードベースを調査し、「まだ利用不可のグラフがいくつかある」という問題を特定・解決するための設計を明確化するためのヒアリングを実施。

---

## 質問と回答

### Q1: 作業規模の確認

**質問日時**: 2026-04-15
**カテゴリ**: 設計方針

**回答**: フル設計（推奨）

**信頼性への影響**: 全ファイルを対象にした包括的な設計文書を作成する方針確定。

---

### Q2: 既存コード分析の必要性

**質問日時**: 2026-04-15
**カテゴリ**: コード分析方針
**背景**: `chart-implementation` 設計文書（2026-04-12）が既に存在するが、直近コミット「feat: implement 4 chart widgets」以降の実装状況を確認するかどうか。

**回答**: 必要

**信頼性への影響**:
- 実際の実装状況を調査した結果、4チャート（ParallelCoordinates, ScatterMatrix, SensitivityHeatmap, ClusterScatter）は全て実装済みであることが判明
- 未実装は `ParetoScatter3D` と `PdpChart2D`（ChartId未追加・grid_canvas未接続）のみと特定

---

### Q3: 利用不可チャートの設計対象

**質問日時**: 2026-04-15
**カテゴリ**: スコープ確定
**背景**: コード調査で2つの未接続チャートを発見:
1. ParetoScatter3D — wgpu GPU レンダリングが必要、pareto_3d.rs に show() なし
2. PdpChart2D — pdp_2d.rs は実装済みだが ChartId 未定義・grid_canvas 未接続

**回答**: PdpChart2D（ParetoScatter3D は今回スコープ外）

**信頼性への影響**:
- 設計スコープが PdpChart2D のみに確定 → 🔴 → 🔵 に向上
- ParetoScatter3D は今後の設計タスクとして留保

---

### Q4: ParetoScatter3D の GPU アーキテクチャ（参考質問）

**質問日時**: 2026-04-15
**カテゴリ**: 技術選択（将来参考用）
**背景**: 将来 ParetoScatter3D を実装する際のアーキテクチャ方針を事前確認。

**回答**: wgpu 直接（pareto_2d.rs と同パターン）

**信頼性への影響**:
- 今回スコープ外だが、将来の ParetoScatter3D 設計時の方針として記録
- 既存 pareto_2d.rs / scatter_renderer.rs パターンを踏襲

---

### Q5: PdpResult2d 計算トリガー方式

**質問日時**: 2026-04-15
**カテゴリ**: アーキテクチャ
**背景**: `pdp_2d.rs` は `computing: bool` フィールドを持つが、計算をトリガーする仕組みが未実装。既存の 1D PDP は `AppMessage::PdpDone` で非同期計算（ただし app.rs で TODO 状態）。

**回答**: AppMessage 拡張（推奨）— `AppMessage::Pdp2dDone` を新設

**信頼性への影響**:
- 計算パスが `SensitivityDone` / `ClusteringDone` と同一パターンで確定 → 🔴 → 🔵 に向上
- `app.rs::poll_messages()` に `Pdp2dDone` ハンドラを追加する設計が確定

---

### Q6: チャートピッカーへの追加

**質問日時**: 2026-04-15
**カテゴリ**: UI 設計
**背景**: `ChartId::all()` を `left_panel.rs` が使用してチャートリストを表示。`ChartId::PdpChart2D` を追加するだけで自動的にピッカーに表示される構造。

**回答**: 必要（チャートピッカーに「PDP Chart 2D」を表示）

**信頼性への影響**:
- `ChartId` enum への追加のみで対応可能と確認 → 追加コストなし
- `label()` に `"PDP Chart 2D"` を追加する設計確定

---

## ヒアリング結果サマリー

### 確認できた事項

1. **設計対象**: PdpChart2D のみ（ParetoScatter3D は今回スコープ外）
2. **既存実装の活用**: `pdp_2d.rs`・`WidgetStates::pdp_2d`・`PdpResult2d`・`rust_core::pdp::api::compute_pdp_2d` はすべて実装済み。新規ファイル作成不要。
3. **計算パターン**: `AppMessage::Pdp2dDone` 追加 + `spawn_task` パターンを採用
4. **tx 伝播**: `layout.rs` → `main_canvas.rs` → `grid_canvas.rs` → `show_chart()` に `&tx` を渡す変更が必要
5. **obj_names**: 現在 `let _ = obj_names` で未使用 → 目的関数選択 UI と "Run" ボタンの追加が必要

### 設計方針の決定事項

| 項目 | 決定内容 |
|------|----------|
| スコープ | PdpChart2D のみ。ParetoScatter3D は将来タスク |
| 計算トリガー | `pending_compute: Option<Pdp2dComputeRequest>` フィールド + "Run" ボタン |
| 非同期パターン | `spawn_task(tx, || compute_pdp_2d(...) → Pdp2dDone)` |
| チャートピッカー | `ChartId::PdpChart2D` 追加のみで自動対応 |
| tx 伝播 | layout → main_canvas → grid_canvas のシグネチャ変更 |

### 残課題

- `compute_pdp_2d` の `model_type` 選択 UI が pdp_2d.rs に未実装（pdp_chart.rs には実装済み）。実装時に pdp_chart.rs のモデル選択パターンを参照すること。
- 自動再計算（param 変更検知）の要否は実装時に判断。明示的な "Run" ボタンを推奨。
- エラー表示: `load_error` への格納でツールバーにエラーが表示されるが、チャート内インライン表示への変更は実装時判断。

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 4件
- 🟡 黄信号: 3件
- 🔴 赤信号: 4件

**ヒアリング後**:
- 🔵 青信号: 11件 (+7)
- 🟡 黄信号: 2件 (-1)
- 🔴 赤信号: 0件 (-4)

---

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **既存 chart-implementation 設計**: [../chart-implementation/architecture.md](../chart-implementation/architecture.md)
- **参考: pdp_2d.rs**: `egui-app/src/ui/widgets/pdp_2d.rs`
- **参考: pdp_chart.rs（モデル選択 UI）**: `egui-app/src/ui/widgets/pdp_chart.rs`
