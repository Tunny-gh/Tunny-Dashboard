# ブランドトンマナ統一 設計ヒアリング記録

**作成日**: 2026-05-25
**ヒアリング実施**: step4 既存情報ベースの差分ヒアリング

## ヒアリング目的

要件定義書・TONMANUAL.md・既存コードベース（ui_colors.rs, chart_colors.rs, build.rs, html_report.rs）を確認し、設計上の曖昧点を解消するためのヒアリングを実施しました。

---

## 質問と回答

### Q1: 設計規模について

**質問日時**: 2026-05-25
**カテゴリ**: 優先順位
**背景**: ブランドトンマナ統一は複数ファイルにまたがる変更であり、アーキテクチャ・データフロー・実装ガイドの詳細度を決定する必要があった。

**回答**: フル設計（アーキテクチャ・データフロー・実装ガイド含む）

**信頼性への影響**:
- 全設計要素を詳細に記述する方針が確定（🔵）

---

### Q2: chart_colors.rs のスコープについて

**質問日時**: 2026-05-25
**カテゴリ**: 技術選択
**背景**: `chart_colors.rs` には `COLOR_NON_PARETO`・`COLOR_BAR_PRIMARY`・`COLOR_SCATTER_DOT`・`COLOR_LINK` など多数の青系定数（Google Blue `#4285F4`）があり、これらも TONMANUAL blue-500 に揃えるかどうか要件定義では明確にされていなかった。

**回答**: スコープに含めない（チャート用色はデータ識別用途として現行維持）

**信頼性への影響**:
- `chart_colors.rs` 関連の設計項目を除外確定（🔵）
- `ui_colors.rs` の `ACCENT_BLUE` 等と `chart_colors.rs` の `COLOR_NON_PARETO` 等が異なる青を持つことが正式に許容される設計方針が確定

---

### Q3: 既存 ui-color-theming 設計の扱いについて

**質問日時**: 2026-05-25
**カテゴリ**: アーキテクチャ
**背景**: `docs/design/ui-color-theming/` には以前のカラーテーマ設計（ダークネイビーツールバー `#1a2332` 基準）が存在する。brand-tone-manner 設計との関係を明確にする必要があった。

**回答**: 上書き済みとして扱う（brand-tone-manner が ui-color-theming の後継）

**信頼性への影響**:
- brand-tone-manner 設計が新しいカラーテーマの正式ドキュメントになることが確定（🔵）
- ツールバーが dark (#202124) → light (#BFDBFE) に変わる大きな変更が正式に承認

---

## ヒアリング結果サマリー

### 確認できた事項

- `chart_colors.rs` はスコープ外（データ識別用途として独立）
- `ui-color-theming` 設計は本設計で上書き済み
- ツールバーの dark → light 変更は意図的なデザイン方針変更として受け入れ

### 設計方針の決定事項

1. **カラー管理分離**: `ui_colors.rs`（ブランド色）と `chart_colors.rs`（データ識別色）は独立して管理する
2. **実装ガイドの形式**: TypeScript 型定義ではなく Rust Color32 定数と CSS の実装ガイド（`implementation-guide.md`）を作成する
3. **DB スキーマ・API エンドポイントなし**: デスクトップアプリのカラーテーマ変更のため不要
4. **ツールバーホバー色**: TONMANUAL に明示なし → blue-100 (#DBEAFE) を 🟡 として採用

### 残課題

- `TOOLBAR_BTN_HOVER`（#DBEAFE = blue-100）が blue-200 背景上で適切な視認性を持つか、実装後に目視確認が必要
- `ACCENT_BLUE_MUTED` を blue-200 (#BFDBFE) に変更することで選択ハイライトが TOOLBAR_BG と同色になるため、選択状態の識別性を実装後に確認が必要

### 信頼性レベル分布

**ヒアリング前**:
- 🔵 青信号: 20件
- 🟡 黄信号: 10件
- 🔴 赤信号: 0件

**ヒアリング後**:
- 🔵 青信号: 25件 (+5)
- 🟡 黄信号: 8件 (-2)
- 🔴 赤信号: 0件 (変化なし)

---

## 関連文書

- **アーキテクチャ設計**: [architecture.md](architecture.md)
- **データフロー**: [dataflow.md](dataflow.md)
- **実装ガイド**: [implementation-guide.md](implementation-guide.md)
- **要件定義**: [requirements.md](../../spec/brand-tone-manner/requirements.md)
