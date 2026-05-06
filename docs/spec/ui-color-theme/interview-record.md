# UIカラー設定一元化 ヒアリング記録

**作成日**: 2026-05-07
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

現状のコードベースを分析し、色の一元化スコープ・ディレクトリ構成・境界ケースを明確化するためのヒアリングを実施。

---

## 質問と回答

### Q1: 色の一元化スコープ

**カテゴリ**: 未定義部分詳細化
**背景**: `theme.rs`（UIテーマ）と `render/colormap.rs`（チャートグラデーション）は既に分離されているが、各ウィジェットにもハードコードされた色が多数存在する。どこまでを一元化対象とするか不明確だった。

**回答**: 「UI＋チャートデータ両方」
→ UIテーマ色だけでなく、ウィジェット内の固有チャート色（COLOR_PARETO 等）もすべて theme に集約する。

**信頼性への影響**:
- REQ-041, REQ-042 の信頼性が 🔴 → 🔵 に向上

---

### Q2: colormap.rs の扱い

**カテゴリ**: 既存設計確認
**背景**: `render/colormap.rs` はグラデーション管理ロジックとして機能しており、`render/` モジュールに存在する意味があるかどうか確認が必要だった。

**回答**: 「theme/colormap.rs に移動」
→ `egui-app/src/theme/colormap.rs` に移動し、theme モジュールとして統合する。

**信頼性への影響**:
- REQ-004, REQ-021〜024 の信頼性が 🔴 → 🔵 に向上

---

### Q3: theme ディレクトリ構成

**カテゴリ**: 未定義部分詳細化
**背景**: 色を一元化する際に、単一ファイル（theme.rs 拡張）か、ディレクトリ分割かで保守性が変わる。

**回答**: 「theme/mod.rs + サブファイル分割（推奨）」を選択
→ `theme/mod.rs`（UIテーマ）+ `theme/colormap.rs`（グラデーション）+ `theme/chart_colors.rs`（チャート固有色）の 3 ファイル構成。

**信頼性への影響**:
- REQ-001〜005 の信頼性が 🟡 → 🔵 に向上

---

### Q4: mcdm_scatter_chart.rs のスコア色の扱い

**カテゴリ**: 影響範囲確認
**背景**: `mcdm_scatter_chart.rs` には `COLOR_RED / COLOR_ORANGE / COLOR_YELLOW / COLOR_GRAY` がモジュール内定数として定義されており、スコア段階（良/中/低/なし）を表す意味的な色。theme に移動すべきか確認が必要だった。

**回答**: 「theme に移動する」
→ `theme/chart_colors.rs` に統合し、意味的な名前を維持する。

**信頼性への影響**:
- REQ-033 の信頼性が 🟡 → 🔵 に向上

---

## ヒアリング結果サマリー

### 確認できた事項
- 一元化対象はUIテーマ色とチャートデータ色の両方（全色）
- `render/colormap.rs` は `theme/colormap.rs` に移動
- 3 ファイル分割構成（mod.rs / colormap.rs / chart_colors.rs）
- mcdm スコア色も theme に移動する

### 追加/変更要件
- REQ-013: `ERROR_COLOR` セマンティック定数を追加（ヒアリング前は未定義）
- REQ-035: `optimization_history.rs` 試行線色の定数化を追加

### 残課題
- `normalize_trial` / `compute_chart_colors` の循環依存確認は実装フェーズで解決（EDGE-001）
- `egui` 組み込み定数（TRANSPARENT/WHITE/BLACK）の theme 化は任意（REQ-043）

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 5件
- 🟡 黄信号: 12件
- 🔴 赤信号: 8件

**ヒアリング後**:
- 🔵 青信号: 20件 (+15)
- 🟡 黄信号: 7件 (-5)
- 🔴 赤信号: 0件 (-8)

## 関連文書

- **要件定義書**: [requirements.md](requirements.md)
- **ユーザストーリー**: [user-stories.md](user-stories.md)
- **受け入れ基準**: [acceptance-criteria.md](acceptance-criteria.md)
