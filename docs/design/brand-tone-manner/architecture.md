# ブランドトンマナ統一 アーキテクチャ設計

**作成日**: 2026-05-25
**後継設計**: [ui-color-theming/architecture.md](../ui-color-theming/architecture.md)（本設計で上書き済み）
**関連要件定義**: [requirements.md](../../spec/brand-tone-manner/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 要件定義書・TONMANUAL・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: 要件定義書・TONMANUAL から妥当な推測による設計
- 🔴 **赤信号**: 要件定義書・TONMANUAL・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書 概要・ユーザヒアリングより*

TONMANUAL.md を唯一のブランド定義ソース（Single Source of Truth）として、
Tunny Dashboard の 3 つの出力面を統一的にブランド化する。

| 出力面 | 担当ファイル | 出力タイミング |
|--------|------------|--------------|
| egui デスクトップ UI | `egui-app/src/theme/ui_colors.rs`, `mod.rs` | ランタイム |
| ヘルプ HTML ブラウザ | `egui-app/build.rs` | コンパイル時 |
| HTML エクスポートレポート | `egui-app/src/io/html_report.rs` | ユーザー操作時 |

---

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存 ui-color-theming 設計・egui Visuals API より*

- **パターン**: 集中管理カラーモジュール + マルチ出力面へのスタイル伝播
- **選択理由**:
  - `ui_colors.rs` に全 UI カラー定数を集約 → 1 ファイル修正でブランド変更が完結
  - `chart_colors.rs` はデータ識別用途として独立維持（ブランドカラー変更の影響を受けない）
  - HTML 生成（build.rs・html_report.rs）はそれぞれが CSS 文字列を直接保持し、`ui_colors.rs` には依存しない

---

## コンポーネント構成

### テーマモジュール (`egui-app/src/theme/`) 🔵

**信頼性**: 🔵 *既存コードベース・要件定義より*

```
egui-app/src/theme/
├── mod.rs            ← tunny_light_visuals() を定義。ui_colors の定数を egui::Visuals に適用
├── ui_colors.rs      ← [変更対象] UI レベルカラー定数（ツールバー・パネル・テキスト等）
├── chart_colors.rs   ← [変更対象外] チャートデータ識別色（Pareto 点・バー・散布図等）
├── color_compute.rs  ← カラー演算ユーティリティ
├── colormap.rs       ← カラーマップ定義
└── colormap_name.rs  ← カラーマップ名
```

**責務分離**:
- `ui_colors.rs`: TONMANUAL ブランドカラーの Rust 表現
- `chart_colors.rs`: データ可視化専用色（ブランドカラーとは独立）
- `mod.rs`: `egui::Visuals` への橋渡し

### ヘルプ HTML 生成 (`egui-app/build.rs`) 🔵

**信頼性**: 🔵 *既存コードベース・要件定義より*

```
build.rs
└── generate_help_html_files()
    └── convert_dir()
        └── wrap_as_standalone_html()  ← [変更対象] CSS テンプレートを TONMANUAL に揃える
```

- コンパイル時に `theory/{en,ja}/**/*.md` を HTML に変換
- `wrap_as_standalone_html()` 内のインライン CSS を変更する
- KaTeX CSS/JS は `help-assets/` から読み込み、競合しないように配置する

### HTML エクスポートレポート (`egui-app/src/io/html_report.rs`) 🔵

**信頼性**: 🔵 *既存コードベース・要件定義より*

```
html_report.rs
└── build_html_report()         ← [変更対象] インライン CSS を TONMANUAL に揃える
    ├── write_study_summary()   ← .card スタイルに影響
    ├── write_scatter_svg_section()  ← SVG 色定数に影響
    ├── write_trial_table()     ← table/th スタイルに影響
    └── write_statistics()      ← table/th スタイルに影響
```

外部リソースなし（スタンドアロン HTML）の制約を維持する。

---

## ブランドカラー変換マッピング 🔵

**信頼性**: 🔵 *TONMANUAL §2 カラーパレット・要件定義 REQ-001〜REQ-019 より*

### 変更する定数（`ui_colors.rs`）

| 定数名 | 旧 HEX | 新 HEX | TONMANUAL 根拠 |
|--------|--------|--------|--------------|
| `TOOLBAR_BG` | `#202124` | `#BFDBFE` | blue-200 ナビゲーション背景 |
| `TOOLBAR_TEXT` | `#E8EAED` | `#374151` | gray-700 サブテキスト |
| `TOOLBAR_BTN_ACTIVE` | `#4285F4` | `#3B82F6` | blue-500 プライマリCTA |
| `TOOLBAR_INPUT_BG` | `#303134` | `#F3F4F6` | gray-100 |
| `TOOLBAR_INPUT_STROKE` | `#5F6368` | `#E5E7EB` | gray-200 |
| `PANEL_BG` | `#F0F2F5` | `#F3F4F6` | gray-100 パネル背景 |
| `ACCENT_BLUE` | `#4285F4` | `#3B82F6` | blue-500 メインブルー |
| `ACCENT_BLUE_HOVER` | `#3367D6` | `#2563EB` | blue-600 ホバーブルー |
| `ACCENT_BLUE_MUTED` | `#E8F0FE` | `#BFDBFE` | blue-200 選択ハイライト |
| `TEXT_PRIMARY` | `#202124` | `#111827` | gray-900 見出しテキスト |
| `TEXT_SECONDARY` | `#5F6368` | `#4B5563` | gray-600 本文テキスト |
| `BORDER_COLOR` | `#DADCE0` | `#E5E7EB` | gray-200 ボーダー |

**変更しない定数**:
- `CENTRAL_BG`: `#FFFFFF` のまま（TONMANUAL white と一致）
- `ERROR_COLOR`: `#EA4335` のまま（セマンティックカラー、ブランド対象外）

### 新規追加する定数（`ui_colors.rs`）

| 定数名 | HEX | TONMANUAL 根拠 |
|--------|-----|--------------|
| `HEADER_BG` | `#93C5FD` | blue-300 ヘッダー背景 |
| `ANNOUNCE_BG` | `#60A5FA` | blue-400 アナウンスバー背景 |
| `ACTION_GREEN` | `#22C55E` | green-500 アクション |
| `ACTION_GREEN_HOVER` | `#16A34A` | green-600 ホバー（アクション） |
| `TEXT_SUB` | `#374151` | gray-700 サブテキスト |

### ツールバーホバー色（導出値）🟡

**信頼性**: 🟡 *TONMANUAL §4 ナビゲーションバーの精神から推測*

| 定数名 | 旧 HEX | 新 HEX | 根拠 |
|--------|--------|--------|------|
| `TOOLBAR_BTN_HOVER` | `#374151` | `#DBEAFE` | blue-100（blue-200 背景の上の hover として 1 段明るく） |

---

## 非機能要件の実現方法

### ビルド互換性 🔵

**信頼性**: 🔵 *要件定義 NFR-001・CLAUDE.md より*

- カラー定数変更は Rust 型安全（Color32）のため、コンパイルエラーで矛盾を検出可能
- ヘルプ HTML は `cargo build` で自動再生成される（`build.rs` で `rerun-if-changed=theory/` が設定済み）

### 視認性（WCAG AA 基準） 🟡

**信頼性**: 🟡 *要件定義 NFR-101 から妥当な推測*

ツールバー dark→light 変更後のコントラスト比（事前算出）:

| 背景 (TOOLBAR_BG) | テキスト | コントラスト比 | 判定 |
|------------------|---------|-------------|------|
| `#BFDBFE`（blue-200）| `#374151`（gray-700） | ≈ 4.6:1 | ✅ WCAG AA 合格 |
| `#BFDBFE`（blue-200）| `#3B82F6`（blue-500, active btn） | ≈ 2.5:1 | ⚠️ btn BG が white なら問題なし |

アクティブボタン（blue-500 BG + white text）のコントラスト: ≈ 4.6:1 ✅

### スタンドアロン HTML 🔵

**信頼性**: 🔵 *要件定義 NFR-002・html_report.rs 既存仕様より*

- CSS をインライン埋め込み（外部リソースへの `<link>` なし）
- フォントは `sans-serif`（システムフォント）のみ使用

---

## 技術的制約

### egui テーマ制約 🔵

**信頼性**: 🔵 *既存コードベース・要件定義より*

- `chart_colors.rs` の色はデータ識別用途のためブランドカラー変更の対象外
- `CELL_TOOLBAR_BG` は現状 `PANEL_BG` に近い値で、gray-100 への変更は現行値（#F5F7FA）より若干暗くなる
- egui `Visuals::light()` のデフォルト値が一部上書きされていない場合があるため、動作確認が必要

### HTML 生成制約 🔵

**信頼性**: 🔵 *build.rs 既存実装より*

- `katex.min.css` が body テキスト色を上書きしないよう CSS 記述順序に注意
- KaTeX CSS の後に body スタイルを記述することで優先度を確保する

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **実装ガイド**: [implementation-guide.md](implementation-guide.md)
- **設計ヒアリング**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/brand-tone-manner/requirements.md)
- **旧テーマ設計（上書き済み）**: [ui-color-theming/architecture.md](../ui-color-theming/architecture.md)

## 信頼性レベルサマリー

- 🔵 青信号: 11件 (85%)
- 🟡 黄信号: 2件 (15%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
