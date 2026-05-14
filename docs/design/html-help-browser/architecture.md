# HTML Help Browser アーキテクチャ設計

**作成日**: 2026-05-14
**関連要件定義**: [requirements.md](../../spec/html-help-browser/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書・ユーザヒアリングより*

ヘルプドキュメントの表示方式を、egui内カスタムMarkdownパーサーからビルド時HTML生成＋ブラウザ表示に変更する。数式のLaTeX記法移行、KaTeXインライン埋め込みによるリッチな数式レンダリング、日英両対応、アプリ内言語切替を実現する。eguiモーダル表示は完全に廃止する。

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *要件定義書・ユーザヒアリングより*

- **パターン**: Build-time HTML Generation + OS Browser Launch
- **選択理由**:
  - egui内でのMarkdown/数式レンダリングは機能的に限界がある
  - ビルド時変換により実行時オーバーヘッドをゼロにする
  - OSデフォルトブラウザはKaTeXのフル機能を利用可能
  - `include_str!` パターンを踏襲し、単一バイナリ配布を維持

## コンポーネント構成

### ビルドパイプライン 🔵

**信頼性**: 🔵 *要件定義 REQ-001〜004・既存 build.rs パターンより*

```
theory/*.md ──→ [build.rs] ──→ OUT_DIR/*.html ──→ include_str! ──→ バイナリ埋め込み
                   │
                   ├── pulldown-cmark (md→html)
                   ├── KaTeX CSS/JS インライン注入
                   └── ライトテーマ CSS 注入
```

**build.rs 拡張内容**:
- `pulldown-cmark` を `[build-dependencies]` に追加
- `theory/en/` と `theory/ja/` の全 `.md` をスキャン
- 各ファイルをスタンドアロンHTMLに変換
- KaTeX v0.16 の minified CSS + JS をインライン埋め込み
- 生成HTMLを `OUT_DIR` に配置、Rust側は `include_str!` で参照

### ランタイムコンポーネント 🔵

**信頼性**: 🔵 *既存アーキテクチャパターン・要件定義より*

```
┌──────────────────────────────────────────────────┐
│  TunnyApp                                        │
│  ├── AppState                                    │
│  │   └── help_language: HelpLanguage    ◀ 新規   │
│  ├── WidgetStates                                │
│  │   └── help_state: HelpState         ◀ 変更   │
│  └── UI                                          │
│      ├── layout.rs                               │
│      │   └── show_language_menu()      ◀ 新規   │
│      └── help/                                   │
│          ├── mod.rs                    ◁ 変更    │
│          ├── help_content.rs           ◁ 変更    │
│          ├── help_launcher.rs          ◀ 新規    │
│          ├── help_types.rs             ◁ 変更    │
│          ├── md_renderer.rs            ✕ 削除    │
│          └── help_modal.rs             ✕ 削除    │
└──────────────────────────────────────────────────┘
```

**凡例**: ◀ 新規追加、◁ 変更、✕ 削除

### ヘルプ起動フロー 🔵

**信頼性**: 🔵 *要件定義 REQ-020〜023・既存 CellToolbarAction パターンより*

```
[ユーザー: Help クリック]
        │
        ▼
CellToolbarAction::Help(panel_item)
        │
        ▼
help_launcher::open_help(panel_item, help_language)
        │
        ├── 1. panel_item → HTMLコンテンツ選択
        │      (include_str! で埋め込み済みのHTML文字列)
        │
        ├── 2. 一時ファイルにHTML書き出し
        │      std::env::temp_dir() / "tunny-help-{widget}-{lang}.html"
        │
        └── 3. open::that(path) でブラウザ起動
               失敗時 → egui トーストでエラー表示
```

## ディレクトリ構造（変更後）🔵

**信頼性**: 🔵 *既存プロジェクト構造・要件定義より*

```
egui-app/
├── build.rs                    ◁ 変更: md→html 変換追加
├── Cargo.toml                  ◁ 変更: pulldown-cmark 追加
├── src/
│   ├── app.rs                  ◁ 変更: 言語切替統合
│   ├── state/
│   │   ├── app_state.rs        ◁ 変更: help_language フィールド追加
│   │   └── messages.rs         ◁ 変更: HelpLaunchFailed 追加
│   └── ui/
│       ├── layout.rs           ◁ 変更: show_help_modal → 言語メニュー
│       ├── help/
│       │   ├── mod.rs          ◁ 変更: モジュール構成変更
│       │   ├── help_types.rs   ◁ 変更: HelpState, HelpLanguage
│       │   ├── help_content.rs ◁ 変更: HTML参照ベース
│       │   └── help_launcher.rs◀ 新規: ブラウザ起動ロジック
│       └── widget_states.rs    ◁ 変更: help_modal → help_state

theory/
├── en/                         ◁ 変更: 数式 LaTeX 記法移行
│   ├── widgets/                ◁ 変更: 数式 LaTeX 記法移行
│   └── ...
└── ja/                         ◁ 変更: 数式 LaTeX 記法移行
    ├── widgets/                ◀ 新規: 日本語ウィジェットヘルプ作成
    └── ...
```

## 新規依存関係 🔵

**信頼性**: 🔵 *要件定義 REQ-002・ユーザヒアリングより*

```toml
# Cargo.toml [build-dependencies] に追加
pulldown-cmark = { version = "0.11", default-features = false }

# 既存依存の活用（追加不要）
# open = "5"           → ブラウザ起動
# tempfile = "3"       → テスト用（dev-dependencies）
```

**選択理由**:
- `pulldown-cmark`: Rustエコシステムで最も広く使われるMarkdownパーサー。`default-features = false` でコンパイル時間最小化。
- KaTeXのCSS/JSはビルド時に文字列定数としてインライン化するため、ランタイム依存なし。

## 非機能要件の実現方法

### パフォーマンス 🔵

**信頼性**: 🔵 *NFR-001, NFR-002・既存アーキテクチャより*

- **ビルド時間**: pulldown-cmark の md→html 変換は約60ファイル、+3秒以内を想定
- **ヘルプ起動**: HTMLは `include_str!` でメモリ上に存在、一時ファイル書き出し + `open::that` は 200ms 以内
- **メモリ**: 全HTML埋め込みでバイナリサイズ +200KB 程度（KaTeX含む）、実行時メモリ増加なし
- **ファイルI/O**: 一時ファイルへの書き込みのみ（読み込みなし）

### セキュリティ 🔵

**信頼性**: 🔵 *NFR-010, NFR-011・既存 html_report.rs パターンより*

- **一時ファイル**: `std::env::temp_dir()` に配置、ファイル名はサニタイズ済み
- **HTMLインジェクション**: ビルド時の固定コンテンツのみ（ユーザー入力の埋め込みなし）
- **パストラバーサル**: ファイル名生成に `sanitize_filename()` を使用（既存パターン踏襲）

### ユーザビリティ 🟡

**信頼性**: 🟡 *ヒアリング結果より*

- **言語切替**: 「⋯」メニュー内に配置、egui::mutex で状態共有
- **エラー表示**: `open::that` 失敗時、egui トーストまたはステータスバーに表示
- **ライトテーマ固定**: HTMLは常にライトテーマ（ダークモード追従なし）

### 保守性 🔵

**信頼性**: 🔵 *NFR-030, NFR-031・既存パターンより*

- **コンテンツ追加**: theory/ にmd追加 + help_content.rs にマッピング追加のみ
- **build.rs テスト**: `#[cfg(test)]` でHTML生成ロジックの単体テスト可能
- **LaTeX数式の追加**: Markdown内に `$...$` / `$$...$$` を書くだけ

## 技術的制約

### ビルド時制約 🔵

**信頼性**: 🔵 *既存 build.rs パターンより*

- pulldown-cmark は `default-features = false` で最小構成
- KaTeX CSS/JS は `const` 文字列としてハードコード（CDN不使用）
- OUT_DIR のファイルパスは `env!("OUT_DIR")` マクロで参照

### 実行時制約 🔵

**信頼性**: 🔵 *要件定義・既存実装より*

- ブラウザがインストールされていない環境ではヘルプ表示不可（エラー表示のみ）
- 一時ファイルはOSのテンポラリディレクトリクリーンアップに依存
- 同一ヘルプの連続クリックは同じファイルを上書き（複数タブは開かない）

### 互換性制約 🔵

**信頼性**: 🔵 *要件定義・既存プラットフォームより*

- Windows / macOS / Linux の全プラットフォームで `open::that` が動作
- theory/ja/widgets/ が存在しない場合のフォールバック不要（日本語widgetsも作成）

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **設計ヒアリング**: [design-interview.md](design-interview.md)
- **要件定義**: [requirements.md](../../spec/html-help-browser/requirements.md)

## 信頼性レベルサマリー

- 🔵 青信号: 18件 (90%)
- 🟡 黄信号: 2件 (10%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質 — 全主要設計判断をユーザーヒアリングで確認済み
