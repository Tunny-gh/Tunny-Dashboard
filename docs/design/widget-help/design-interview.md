# Widget Help 設計ヒアリング記録

**作成日**: 2026-05-08
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

要件定義書で確定した機能要件に基づき、技術的な実装方式（コンテンツ埋め込み・モーダル描画位置・Markdown レンダリング）について不明点を明確化するためのヒアリングを実施しました。

## 質問と回答

### Q1: Theory フォルダの内容をどうやって egui に表示するか

**質問日時**: 2026-05-08
**カテゴリ**: 技術選択
**背景**: egui には標準の Markdown レンダラがないため、コンテンツの表示方式が技術的な制約となる。3つの方式を提示して選択を求めた

**回答**: include_str!＋軽量MDレンダラ（推奨）

**信頼性への影響**:
- アーキテクチャに `md_renderer.rs` モジュールを追加（信頼性レベル: 🔵）
- `include_str!` によるコンパイル時埋め込み方式が確定
- NFR-002（実行時 I/O ゼロ）の実現方法が確定

---

### Q2: ヘルプモーダルはどこで render するか

**質問日時**: 2026-05-08
**カテゴリ**: アーキテクチャ
**背景**: artifact_modal は app.rs の update ループ内で render されている。ヘルプモーダルも同パターンにするか、各セル内で render するかで実装方針が変わる

**回答**: app.rs ループ（推奨）

**信頼性への影響**:
- artifact_modal と同じパターンで実装（信頼性レベル: 🔵）
- `app.rs` の `update()` メソッドに1行追加のみで対応可能
- 各セルからの状態設定は `WidgetStates.help_modal` を経由

---

## ヒアリング結果サマリー

### 確認できた事項
- コンテンツ表示: `include_str!` + 軽量 Markdown→egui レンダラ
- モーダル描画位置: `app.rs` の `update()` ループ内
- 既存 artifact_modal パターンの完全な再利用が可能

### 設計方針の決定事項
- 新規モジュール `egui-app/src/ui/help/` を作成
- `md_renderer.rs` は見出し・リスト・テーブル・コードブロック・プレーンテキスト数式に対応
- `PanelItem::help_content()` メソッドでコンテンツルックアップ
- `WidgetStates` に `help_modal: HelpModalState` を追加

### 残課題
- 軽量 Markdown レンダラのテーブル対応の詳細（egui TableBuilder vs シンプルラベル）
- 英語版 Theory コンテンツの分量が確定後のバイナリサイズ確認

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 15
- 🟡 黄信号: 5
- 🔴 赤信号: 0

**ヒアリング後**:
- 🔵 青信号: 20 (+5)
- 🟡 黄信号: 0 (-5)
- 🔴 赤信号: 0

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **要件定義**: [requirements.md](../../spec/widget-help/requirements.md)
