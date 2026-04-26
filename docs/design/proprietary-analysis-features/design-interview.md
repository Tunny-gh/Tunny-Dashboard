# プロプライエタリ分析ツール不足機能 設計ヒアリング記録

**作成日**: 2026-04-26  
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

---

## ヒアリング目的

既存の要件定義書 (`docs/spec/proprietary-analysis-features/`) と実装コード調査結果を照合し、設計上の不明点・判断ポイントについてユーザーにヒアリングを実施しました。

---

## 質問と回答

### Q1: Trade-off Navigator の重み変更 → スコアリング計算フロー

**質問日時**: 2026-04-26  
**カテゴリ**: アーキテクチャ  
**背景**: コード調査で `score_tradeoff_navigator(weights: &[f64], is_minimize: &[bool]) -> Vec<u32>` が rust_core に実装済みであることを確認。ただし UI ↔ 計算のフローが未設計。スライダー変更のたびに非同期計算が必要か（NFR-001: 50,000 試行で 100ms 以内）、同期でも問題ないか、設計判断が必要。

**選択肢**:
- A. `AppMessage` 経由（非同期）— `spawn_task` でバックグラウンドスレッド実行
- B. UI 直接呼び出し（同期）— スライダー変更時に UI スレッドで直接実行

**回答**: A. AppMessage 経由（非同期）

**信頼性への影響**:
- この回答により、`AppMessage::TradeoffDone { sorted_indices: Vec<u32> }` バリアント追加が確定（信頼性: 🔵）
- `left_panel.rs` の `show_tradeoff_navigator()` が `spawn_task` 経由で呼び出すフローが確定
- NFR-001（100ms 以内）を UI ブロックなしで達成できることが確定

---

### Q2: 分析セッション保存（.tdash）の実装方針

**質問日時**: 2026-04-26  
**カテゴリ**: データモデル  
**背景**: コード調査で `egui-app/src/io/session.rs` に既存の `SessionSnapshot` 構造体が存在することを確認。しかし現状は `study_name`・`filter_ranges`・`selected_indices`・`saved_at` の 4 フィールドのみで、REQ-004 が要求する `tradeoff_weights`・`layout_config`・`cluster_config`・`color_mode`・`pinned_trials` が含まれていない。

**選択肢**:
- A. `SessionSnapshot` 拡張（推奨）— 既存構造体にフィールドを追加し、拡張子のみ `.tdash` に変更
- B. `TdashSession` 新規作成 — 別ファイルに独立した新構造体を作成

**回答**: A. SessionSnapshot 拡張（推奨）

**信頼性への影響**:
- 既存 `io/session.rs` への追加変更が確定（信頼性: 🔵）
- `SessionSnapshot` への追加フィールドが REQ-004-B の `.tdash` 仕様を満たす形で確定
- `serde(default)` による後方互換（既存の古い SessionSnapshot も読み込み可能）が設計方針に確定

---

### Q3: 複数 Study 比較モードのアーキテクチャ

**質問日時**: 2026-04-26  
**カテゴリ**: アーキテクチャ  
**背景**: REQ-006 が比較モード（2〜4 Study 同時選択）を要求。現在の `LayoutMode` enum は `MultiObjective`・`VariableSpace`・`ConvergenceAnalysis`・`FreeLayout` の 4 種。比較モードを既存のレイアウトモード切り替えパターンに統合するか、サイドバー方式にするか設計判断が必要。

**選択肢**:
- A. `LayoutMode::Comparison` 追加（推奨）— 既存の `LayoutMode` enum にバリアント追加、固定 4 分割レイアウト
- B. サイドバー方式 — `FreeLayout` の横にサイドバーを追加して比較ビューを表示

**回答**: A. LayoutMode::Comparison 追加（推奨）

**信頼性への影響**:
- `layout_state.rs` の `LayoutMode` enum への `Comparison` バリアント追加が確定（信頼性: 🔵）
- `AppState` への `comparison_mode: bool`・`comparison_studies: Vec<StudyContext>`・`comparison_colors: Vec<egui::Color32>` フィールド追加が確定
- `egui-app/src/ui/comparison_panel.rs` 新規ファイル作成が確定
- Toolbar の `LayoutMode` ボタン群に「Comparison」ボタン追加が確定

---

### Q4: HTML レポート生成時のチャート SVG キャプチャ方法

**質問日時**: 2026-04-26  
**カテゴリ**: 技術選択  
**背景**: REQ-005 が HTML レポートに現在表示中チャートの SVG キャプチャを要求。egui の Immediate Mode レンダリングは GPU への直接描画のため、SVG キャプチャには 2 つのアプローチが考えられる:
- A. `egui Shape → SVG 変換` — `Painter` + off-screen `Context` でチャートをレンダリングし `epaint::Shape` を SVG タグに変換
- B. `データ → SVG 直接生成` — チャートのデータ（`StudyContext::trial_rows`）を元にシンプルな SVG を独立生成

**回答**: A. egui Shape → SVG 変換（推奨）

**信頼性への影響**:
- `io/html_report.rs` に `EguiSvgExporter` 実装が確定（信頼性: 🟡）
  - egui の `epaint::Shape` を SVG タグに変換するパスを `HtmlReportBuilder` に組み込む
  - ただし egui の Immediate Mode の性質上、完全な Shape 収集には制約があるため、実装段階でデータドリブン SVG 生成へのフォールバックを検討する（信頼性: 🟡）

---

## ヒアリング結果サマリー

### 確認できた事項
- Trade-off Navigator は非同期パターン（`spawn_task` + `AppMessage::TradeoffDone`）で統一
- セッション保存は既存 `SessionSnapshot` を拡張（新規構造体不要）
- 比較モードは `LayoutMode::Comparison` として既存パターンに統合
- HTML レポートの SVG は `egui Shape → SVG 変換` アプローチ（実装段階でデータドリブンへのフォールバック可）

### 設計方針の決定事項
- 全 9 機能を既存の 4 層メッセージパッシング・ステートマシンパターンに統合
- 新規ファイルは `html_report.rs`・`artifacts.rs`・`comparison_panel.rs`・`artifact_modal.rs` の 4 ファイルのみ
- `SessionSnapshot` を REQ-004 の `.tdash` 形式として拡張（後方互換あり）
- `LayoutMode::Comparison` 追加で比較専用レイアウトを実現

### 残課題
- `egui Shape → SVG 変換` の実装詳細（epaint::Shape のすべてのバリアントを SVG に変換する工数評価が必要）
- `comparison_studies` の最大メモリ使用量（4 Study × 50,000 試行 × 30 変数の場合の見積もり）

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 20件
- 🟡 黄信号: 8件
- 🔴 赤信号: 0件

**ヒアリング後**:
- 🔵 青信号: 28件 (+8件)
- 🟡 黄信号: 4件 (-4件)
- 🔴 赤信号: 0件

---

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **型定義（Rust）**: [interfaces.rs](interfaces.rs)
- **要件定義**: [requirements.md](../../spec/proprietary-analysis-features/requirements.md)
- **ユーザーストーリー**: [user-stories.md](../../spec/proprietary-analysis-features/user-stories.md)
