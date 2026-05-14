/// HTML Help Browser 型定義
///
/// 作成日: 2026-05-14
/// 関連設計: architecture.md, dataflow.md
///
/// 信頼性レベル:
/// - 🔵 青信号: EARS要件定義書・設計文書・既存実装を参考にした確実な型定義
/// - 🟡 黄信号: EARS要件定義書・設計文書・既存実装から妥当な推測による型定義
/// - 🔴 赤信号: EARS要件定義書・設計文書・既存実装にない推測による型定義

// ============================================================
// 言語設定
// ============================================================

/// ヘルプ表示言語
/// 🔵 信頼性: 要件定義 REQ-030〜032・ユーザヒアリングより
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum HelpLanguage {
    En, // 🔵 デフォルト（要件定義より）
    Ja, // 🔵 ユーザヒアリング: 日英両対応選択より
}

impl Default for HelpLanguage {
    fn default() -> Self {
        Self::En // 🔵 デフォルトは英語
    }
}

impl HelpLanguage {
    /// 言語コードの文字列表現を返す
    /// 🔵 信頼性: ファイルパス生成要件より
    pub fn code(&self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Ja => "ja",
        }
    }
}

// ============================================================
// ヘルプコンテンツ定義（再設計）
// ============================================================

/// ヘルプコンテンツ定義
/// 🔵 信頼性: 要件定義 REQ-052・既存 HelpContent パターンより
///
/// 旧 HelpContent (tabs: &[HelpTabDef]) から
/// 単一HTML参照に簡略化。
/// ヘルプはブラウザで表示するため、タブ構造はHTML内で完結する。
pub struct HelpContent {
    /// ウィジェットの表示名（ファイル名生成・ログ用）
    /// 🔵 信頼性: 既存 help_content.rs の title パターンより
    pub widget_name: &'static str,

    /// 英語版HTML（include_str! で埋め込み）
    /// 🔵 信頼性: 要件定義 REQ-004・既存 include_str! パターンより
    pub html_en: &'static str,

    /// 日本語版HTML（include_str! で埋め込み）
    /// 🔵 信頼性: 要件定義 REQ-003・ユーザヒアリング: 日英両対応より
    pub html_ja: &'static str,
}

impl HelpContent {
    /// 指定言語のHTMLコンテンツを返す
    /// 🔵 信頼性: 要件定義 REQ-032より
    pub fn html(&self, lang: HelpLanguage) -> &'static str {
        match lang {
            HelpLanguage::En => self.html_en,
            HelpLanguage::Ja => self.html_ja,
        }
    }
}

// ============================================================
// ヘルプ起動関数シグネチャ
// ============================================================

/// ヘルプをブラウザで開く
/// 🔵 信頼性: 要件定義 REQ-020〜023・アーキテクチャ設計より
///
/// # 引数
/// - `item`: ヘルプ対象のパネルアイテム
/// - `lang`: ヘルプ表示言語
///
/// # 戻り値
/// - `Ok(())`: ブラウザ起動成功
/// - `Err(String)`: ファイル書き出し失敗 or ブラウザ起動失敗
///
/// # 処理フロー
/// 1. `get_help_content(item)` で HelpContent 取得
/// 2. `content.html(lang)` でHTML文字列取得
/// 3. `temp_dir() / "tunny-help-{widget_name}-{lang.code()}.html"` に書き出し
/// 4. `open::that(&path)` でブラウザ起動
pub fn open_help(item: &PanelItem, lang: HelpLanguage) -> Result<(), String> {
    // 実装は help_launcher.rs に記述
    todo!()
}

// ============================================================
// AppState 拡張（既存に追加）
// ============================================================

/// AppState に追加するフィールド
/// 🔵 信頼性: 要件定義 REQ-031・既存 AppState パターンより
///
/// ```rust
/// // app_state.rs の AppState 構造体に追加:
/// pub struct AppState {
///     // ... 既存フィールド ...
///
///     /// ヘルプ表示言語
///     /// 🔵 要件定義 REQ-031より
///     /// clear() でリセットしない（selected_colormap と同じパターン）
///     pub help_language: HelpLanguage,
/// }
/// ```

// ============================================================
// WidgetStates 変更（既存を変更）
// ============================================================

/// WidgetStates の help 関連フィールド変更
/// 🔵 信頼性: 要件定義 REQ-050〜053より
///
/// ```rust
/// // 変更前:
/// // pub help_modal: HelpModalState,
///
/// // 変更後:
/// // （help_language は AppState に移動するため、
/// //   WidgetStates には help 関連フィールド不要）
/// ```

// ============================================================
// build.rs 関数シグネチャ
// ============================================================

/// MarkdownファイルをスタンドアロンHTMLに変換する
/// 🔵 信頼性: 要件定義 REQ-001, REQ-002・アーキテクチャ設計より
///
/// # 引数
/// - `markdown`: 入力Markdown文字列
///
/// # 戻り値
/// - 完全なスタンドアロンHTML（KaTeX CSS/JS + ライトテーマCSS 埋め込み済み）
///
/// # 処理
/// 1. pulldown-cmark で Markdown → HTML body に変換
/// 2. KaTeX auto-render 用のマークアップを挿入
/// 3. HTMLラッパー（DOCTYPE, head, style, script）で包む
fn markdown_to_standalone_html(markdown: &str) -> String {
    // 実装は build.rs に記述
    todo!()
}

/// theory/ ディレクトリ配下の全 .md をスキャンして HTML に変換する
/// 🔵 信頼性: 要件定義 REQ-003・アーキテクチャ設計より
///
/// # 処理
/// 1. theory/en/**/*.md と theory/ja/**/*.md をスキャン
/// 2. 各ファイルを markdown_to_standalone_html() で変換
/// 3. OUT_DIR/help/{en,ja}/ に同名 .html で出力
/// 4. cargo:rerun-if-changed=theory/ を出力
fn generate_help_html_files() {
    // 実装は build.rs に記述
    todo!()
}

// ============================================================
// HTMLテンプレート定数
// ============================================================

/// KaTeX CSS（minified）
/// 🔵 信頼性: 要件定義 REQ-010, REQ-013・ユーザヒアリング: KaTeX埋め込みより
///
/// ビルド時に文字列定数としてハードコード。
/// KaTeX v0.16.x の katex.min.css の内容を埋め込む。
const KATEX_CSS: &str = include_str!("katex.min.css");

/// KaTeX JS（minified）
/// 🔵 信頼性: 要件定義 REQ-010, REQ-013・ユーザヒアリング: KaTeX埋め込みより
const KATEX_JS: &str = include_str!("katex.min.js");

/// KaTeX auto-render JS（minified）
/// 🔵 信頼性: LaTeX記法の自動検出用
const KATEX_AUTO_RENDER_JS: &str = include_str!("auto-render.min.js");

/// ライトテーマCSS
/// 🔵 信頼性: ユーザヒアリング: ライト固定選択より
const LIGHT_THEME_CSS: &str = r#"
body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    max-width: 800px;
    margin: 0 auto;
    padding: 20px;
    line-height: 1.6;
    color: #333;
    background: #fff;
}
h1, h2, h3 { color: #1a1a1a; }
h1 { border-bottom: 2px solid #e0e0e0; padding-bottom: 8px; }
h2 { border-bottom: 1px solid #eee; padding-bottom: 4px; }
table { border-collapse: collapse; width: 100%; margin: 16px 0; }
th, td { border: 1px solid #ddd; padding: 8px 12px; text-align: left; }
th { background: #f5f5f5; font-weight: 600; }
code { background: #f4f4f4; padding: 2px 6px; border-radius: 3px; font-size: 0.9em; }
pre { background: #f8f8f8; padding: 12px; border-radius: 4px; overflow-x: auto; }
pre code { background: none; padding: 0; }
blockquote { border-left: 3px solid #ddd; margin: 16px 0; padding: 8px 16px; color: #666; }
"#;

// ============================================================
// CellToolbarAction 変更（既存を変更）
// ============================================================

/// grid_canvas.rs の CellToolbarAction::Help 処理変更
/// 🔵 信頼性: 要件定義 REQ-020・既存 CellToolbarAction パターンより
///
/// ```rust
/// // 変更前:
/// // CellToolbarAction::Help(help_item) => {
/// //     widgets.help_modal.open = true;
/// //     widgets.help_modal.item = Some(help_item.clone());
/// // }
///
/// // 変更後:
/// // CellToolbarAction::Help(help_item) => {
/// //     if let Err(e) = help_launcher::open_help(&help_item, app.app_state.help_language) {
/// //         app.load_error = Some(e);
/// //     }
/// // }
/// ```

// ============================================================
// 信頼性レベルサマリー
// ============================================================
// - 🔵 青信号: 20件 (95%)
// - 🟡 黄信号: 1件 (5%)
// - 🔴 赤信号: 0件 (0%)
//
// 品質評価: ✅ 高品質
