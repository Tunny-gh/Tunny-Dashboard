//! 自己完結型レポート出力。
//!
//! Optuna 最適化結果を構造化した [`StudyReport`] に落とし込み、JSON / Markdown /
//! （後続フェーズで HTML）へレンダリングする。レポートモデルは言語非依存の
//! 構造化ファクトを持ち、文章化（en / ja）はレンダラのテンプレートが担当する。
//! Markdown / JSON は将来の MCP サーバーが LLM にそのまま渡す想定。
//!
//! ## モジュール構成
//!
//! - [`model`] — [`StudyReport`] の構造体ツリー（`serde::Serialize`）
//! - [`builder`] — `(StudyMeta, DataFrame, StudyExtras)` → [`StudyReport`]
//! - [`findings`] — Key Findings（まとめ）の決定論的生成
//! - [`markdown`] — Markdown レンダラ（LLM 向け主用途）

pub mod builder;
pub mod findings;
pub mod markdown;
pub mod model;
pub mod svg;
pub mod theme;

pub use builder::build_study_report;
pub use markdown::render_markdown;
pub use model::*;

/// レンダリング言語。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReportLang {
    /// 英語。
    #[default]
    En,
    /// 日本語。
    Ja,
}

/// レポート生成オプション。
#[derive(Debug, Clone, Copy)]
pub struct ReportOptions {
    /// 既定のレンダリング言語（`render_*` は明示引数で上書き可能）。
    pub lang: ReportLang,
    /// 上位表の件数（既定 10）。
    pub top_n: usize,
    /// 相関ヒートマップの最大パラメータ数（既定 15）。
    pub max_heatmap_params: usize,
}

impl Default for ReportOptions {
    fn default() -> Self {
        ReportOptions {
            lang: ReportLang::En,
            top_n: 10,
            max_heatmap_params: 15,
        }
    }
}

/// 呼び出し側が渡すソース情報。
///
/// `storage_display` は RDB URL の場合、**必ず** マスク済み（`RdbUrl::masked()`）の
/// 文字列を渡すこと（レポートに生パスワードを残さない）。`generated_at_unix` は
/// core が時計を持たないため呼び出し側が与える。`None` なら日時欄を省略する。
#[derive(Debug, Clone)]
pub struct ReportSource {
    /// ストレージ表示名（RDB はマスク済み）。
    pub storage_display: String,
    /// 生成日時（unix 秒）。`None` なら省略。
    pub generated_at_unix: Option<i64>,
}

/// f64 を有効4桁で整形する（レンダラ共通フォーマッタ）。
///
/// - 整数値はそのまま整数表示（末尾 `.0` を付けない）
/// - 非整数は有効4桁に丸め、末尾の余分な 0 とドットを除去する
/// - `NaN` / `±inf` はそれぞれ `"NaN"` / `"inf"` / `"-inf"`
pub fn format_number(value: f64) -> String {
    format_sig(value, 4)
}

fn format_sig(value: f64, sig: usize) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    if value == 0.0 {
        return "0".to_string();
    }
    // 整数値は整数表示。
    if value.fract() == 0.0 && value.abs() < 1e15 {
        return format!("{}", value as i64);
    }
    let magnitude = value.abs().log10().floor() as i32;
    let decimals = (sig as i32 - 1 - magnitude).max(0) as usize;
    let s = format!("{value:.decimals$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

#[cfg(test)]
mod format_tests {
    use super::format_number;

    #[test]
    fn integers_render_without_decimals() {
        assert_eq!(format_number(0.0), "0");
        assert_eq!(format_number(42.0), "42");
        assert_eq!(format_number(-7.0), "-7");
        assert_eq!(format_number(1000.0), "1000");
    }

    #[test]
    fn four_significant_digits_no_trailing_zeros() {
        assert_eq!(format_number(1.23456), "1.235");
        assert_eq!(format_number(0.001234), "0.001234");
        assert_eq!(format_number(12345.6), "12346");
        assert_eq!(format_number(1.5), "1.5");
        assert_eq!(format_number(0.5), "0.5");
    }

    #[test]
    fn non_finite() {
        assert_eq!(format_number(f64::NAN), "NaN");
        assert_eq!(format_number(f64::INFINITY), "inf");
        assert_eq!(format_number(f64::NEG_INFINITY), "-inf");
    }
}
