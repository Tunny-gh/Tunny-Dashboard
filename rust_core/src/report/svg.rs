//! HTML レポート埋め込み用の SVG チャート描画プリミティブ。
//!
//! ここに定義する関数は**純粋な文字列生成器**であり、外部クレートに依存せず、
//! 統計・集計などの分析処理は一切行わない（呼び出し側 `builder.rs` が既に
//! 計算済みのデータ系列を渡す前提）。色はすべて [`super::theme`] が定義する
//! CSS custom property（`var(--foo)`）経由で参照し、SVG 内に生の 16 進数
//! カラーコードを一切書き出さない（ライト/ダークテーマへの自動追従のため）。
//!
//! # 共通仕様
//!
//! - `viewBox` ベースでサイズ指定し、`width="100%"` によりレスポンシブに
//!   表示される（ページ幅に追従してスケールする）。
//! - 目盛りは 1/2/5 × 10^n の「nice number」アルゴリズム（[`nice_ticks`]）
//!   で決定する。
//! - グリッド線は hairline（1px、`var(--grid)`）、軸ラベルはミュートインク
//!   （`var(--ink-muted)`）。
//! - すべてのデータマークには `<title>` 子要素を付与し、ブラウザネイティブの
//!   ツールチップとして機能させる（JS 不使用の設計方針に対応）。
//! - `<text>` には `font-family="inherit"` を指定し、SVG 内にフォント名を
//!   埋め込まない（HTML ページ側の CSS からフォントを継承させる）。数値
//!   ラベルには `font-variant-numeric: tabular-nums` を付与する。
//! - 系列が1本のみのチャートは凡例を出さない（`<title>` が系列名を兼ねる）。
//! - 描画要素は `write!`（[`std::fmt::Write`]）で出力バッファへ直接書き込み、
//!   要素ごとの使い捨て `String` 確保を避ける。

use std::fmt::Write as _;

use super::theme;

// ================================================================
// 共通インフラ
// ================================================================

/// 全テキスト共通のフォントサイズ（viewBox 単位）。
///
/// SVG 側で明示しないとページの既定サイズ（16px 前後）で描画され、
/// 文字幅ベースのマージン計算が破綻するため、必ずこの値を `font-size`
/// 属性として出力する（フォント**ファミリ**は `inherit` のままページに従う）。
const FONT_SIZE: f64 = 12.0;

/// マージン計算に使う1文字あたりの推定幅（px、`FONT_SIZE` 時）。
///
/// system-ui 系フォントの英数字の平均幅より少し広めに取ってあり、
/// truncation 済みラベルがマージンからはみ出さないための安全側の値。
const CHAR_W: f64 = 7.0;

/// プロット領域の余白。ラベルの有無から決定する。
struct Margins {
    top: f64,
    right: f64,
    bottom: f64,
    left: f64,
}

/// XML テキストコンテンツ / 属性値として安全な文字列にエスケープする。
///
/// `&` を最初に処理しないと、後続の置換で生成した実体参照
/// （`&amp;` 等）を二重エスケープしてしまうため、処理順序が重要。
///
/// エスケープ対象（`& < > " '`）は HTML テキストノード / 二重引用符属性値
/// としても安全な集合であり、`html` レンダラもこの実装を共有する
/// （`&apos;` は HTML5 では有効な実体参照であるため差分にならない）。
pub(crate) fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// SVG 座標・サイズ用の数値フォーマッタ（小数点以下2桁固定）。
///
/// データ値の表示用フォーマッタ（[`fmt_sig4`]）とは目的が異なる
/// （こちらは描画座標であり、有効桁数の概念を持たない）。
fn coord(v: f64) -> String {
    format!("{v:.2}")
}

/// データ値表示用の有効4桁フォーマッタ（`%.4g` 相当）。
///
/// レポート全体で使う共通フォーマッタ（`crate::report::format_number` 想定）
/// とは意図的に重複実装している（`report::svg` を他モジュールから
/// 疎結合に保つため。設計書参照）。
fn fmt_sig4(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v.is_infinite() {
        return if v > 0.0 {
            "inf".to_string()
        } else {
            "-inf".to_string()
        };
    }

    let sign = if v.is_sign_negative() { "-" } else { "" };
    let av = v.abs();
    let exponent = av.log10().floor() as i32;

    if !(-4..4).contains(&exponent) {
        // 指数表記: 仮数部を有効4桁（小数点以下3桁）に丸める。
        let mantissa = av / 10f64.powi(exponent);
        let mantissa_str = trim_trailing_zeros(&format!("{mantissa:.3}"));
        let exp_sign = if exponent >= 0 { "+" } else { "-" };
        format!("{sign}{mantissa_str}e{exp_sign}{}", exponent.abs())
    } else {
        let decimals = (3 - exponent).max(0) as usize;
        let s = trim_trailing_zeros(&format!("{av:.decimals$}"));
        format!("{sign}{s}")
    }
}

/// 小数点以下の末尾ゼロ（および不要な小数点）を取り除く。
fn trim_trailing_zeros(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    let trimmed = s.trim_end_matches('0');
    trimmed.trim_end_matches('.').to_string()
}

/// nice number（1 / 2 / 5 × 10^n）を計算する。
///
/// `round` が真なら最も近い nice number、偽なら `range` 以上の
/// 最小の nice number を返す（目盛間隔の決定に使う）。
fn nice_num(range: f64, round: bool) -> f64 {
    if range <= 0.0 {
        return 1.0;
    }
    let exponent = range.log10().floor();
    let fraction = range / 10f64.powf(exponent);
    let nice_fraction = if round {
        if fraction < 1.5 {
            1.0
        } else if fraction < 3.0 {
            2.0
        } else if fraction < 7.0 {
            5.0
        } else {
            10.0
        }
    } else if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice_fraction * 10f64.powf(exponent)
}

/// nice-number アルゴリズムで目盛り位置を計算する（Heckbert 方式）。
///
/// `min == max`（退化ケース）の場合は値を中心に人為的な幅を設けてから
/// 通常のアルゴリズムを適用する（`min == max == 0.0` なら `[-1, 1]`、
/// それ以外は値の絶対値の半分を幅とする）ため、単一点データや
/// 定数系列でも panic / NaN を起こさず常に2点以上の目盛りを返す。
fn nice_ticks(min: f64, max: f64, target_count: usize) -> Vec<f64> {
    let (min, max) = if min > max { (max, min) } else { (min, max) };
    let (min, max) = if (max - min).abs() < f64::EPSILON {
        let pad = if min == 0.0 { 1.0 } else { min.abs() * 0.5 };
        (min - pad, max + pad)
    } else {
        (min, max)
    };

    let target = target_count.max(2);
    let range = nice_num(max - min, false);
    let spacing = nice_num(range / (target - 1) as f64, true);
    let nice_min = (min / spacing).floor() * spacing;
    let nice_max = (max / spacing).ceil() * spacing;

    let n = ((nice_max - nice_min) / spacing).round() as i64;
    (0..=n).map(|i| nice_min + spacing * i as f64).collect()
}

/// 整数目盛り（試行番号の X 軸用）を計算する。
///
/// [`nice_ticks`] の結果を四捨五入・重複除去し、2点未満になった場合は
/// 実際の最小・最大値（整数化）にフォールバックする。
fn nice_ticks_integer(min: f64, max: f64, target_count: usize) -> Vec<i64> {
    let mut ticks: Vec<i64> = nice_ticks(min, max, target_count)
        .into_iter()
        .map(|t| t.round() as i64)
        .collect();
    ticks.dedup();
    if ticks.len() >= 2 {
        return ticks;
    }
    let lo = min.floor() as i64;
    let hi = max.ceil() as i64;
    if lo == hi {
        vec![lo]
    } else {
        vec![lo, hi]
    }
}

/// `<line>` によるヘアライングリッド線を `body` へ書き込む。
fn hairline(body: &mut String, x1: f64, y1: f64, x2: f64, y2: f64) {
    let _ = writeln!(
        body,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"var({grid})\" stroke-width=\"1\" />",
        coord(x1),
        coord(y1),
        coord(x2),
        coord(y2),
        grid = theme::VAR_GRID
    );
}

/// `<line>` による軸線を `body` へ書き込む。
fn axis_line(body: &mut String, x1: f64, y1: f64, x2: f64, y2: f64) {
    let _ = writeln!(
        body,
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"var({axis})\" stroke-width=\"1\" />",
        coord(x1),
        coord(y1),
        coord(x2),
        coord(y2),
        axis = theme::VAR_AXIS
    );
}

/// ミュートインクのテキスト（軸目盛・カテゴリラベル用）を `body` へ書き込む。
fn text_muted(body: &mut String, x: f64, y: f64, anchor: &str, content: &str, numeric: bool) {
    let style = if numeric {
        " style=\"font-variant-numeric: tabular-nums\""
    } else {
        ""
    };
    let _ = writeln!(
        body,
        "<text x=\"{}\" y=\"{}\" text-anchor=\"{anchor}\" font-family=\"inherit\" font-size=\"{fs}\" fill=\"var({muted})\"{style}>{}</text>",
        coord(x),
        coord(y),
        escape_xml(content),
        fs = FONT_SIZE,
        muted = theme::VAR_INK_MUTED
    );
}

/// 副次インクの直接データラベル（最終値・バー端の値など）を `body` へ書き込む。
fn text_secondary(body: &mut String, x: f64, y: f64, anchor: &str, content: &str) {
    let _ = writeln!(
        body,
        "<text x=\"{}\" y=\"{}\" text-anchor=\"{anchor}\" font-family=\"inherit\" font-size=\"{fs}\" fill=\"var({secondary})\" style=\"font-variant-numeric: tabular-nums\">{}</text>",
        coord(x),
        coord(y),
        escape_xml(content),
        fs = FONT_SIZE,
        secondary = theme::VAR_INK_SECONDARY
    );
}

/// SVG ルート要素でチャート本体をラップする（viewBox ベースで responsive）。
fn svg_wrap(width: f64, height: f64, body: &str) -> String {
    format!(
        "<svg viewBox=\"0 0 {w} {h}\" width=\"100%\" height=\"auto\" preserveAspectRatio=\"xMinYMin meet\" xmlns=\"http://www.w3.org/2000/svg\">\n{body}</svg>\n",
        w = coord(width),
        h = coord(height)
    )
}

/// データが空の場合のプレースホルダ表示。
fn empty_message(width: f64, height: f64) -> String {
    let mut body = String::new();
    text_muted(
        &mut body,
        width / 2.0,
        height / 2.0,
        "middle",
        "no data",
        false,
    );
    svg_wrap(width, height, &body)
}

/// カテゴリラベルを指定文字数で truncate し、超過時は `…` を付与する。
fn truncate_label(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else {
        let head: String = chars[..max_chars.saturating_sub(1)].iter().collect();
        format!("{head}…")
    }
}

// ================================================================
// line_chart（best-so-far / HV history）
// ================================================================

/// [`line_chart`] の1点。X = 試行番号、Y = 値。
#[derive(Debug, Clone, Copy)]
pub struct LinePoint {
    /// 試行番号。
    pub trial_number: i64,
    /// 値（best-so-far または HV）。
    pub value: f64,
}

/// 収束カーブ（best-so-far / HV history）の折れ線チャートを描画する。
///
/// 単一系列・塗りなし・stroke 2px。`improvement_marks` は `points` への
/// インデックス集合で、best 更新点にのみ 4px マーカーを付ける
/// （呼び出し側が判定済みの値を渡す。ここでは判定しない）。最終点には
/// 直接ラベル（値）を必ず表示する（最終点が `improvement_marks` に
/// 含まれない場合はマーカーも追加する）。X 軸は整数（試行番号）目盛り。
pub fn line_chart(
    points: &[LinePoint],
    improvement_marks: &[usize],
    width: f64,
    height: f64,
) -> String {
    if points.is_empty() {
        return empty_message(width, height);
    }

    let x_min = points[0].trial_number as f64;
    let x_max = points[points.len() - 1].trial_number as f64;
    let y_min_raw = points.iter().map(|p| p.value).fold(f64::INFINITY, f64::min);
    let y_max_raw = points
        .iter()
        .map(|p| p.value)
        .fold(f64::NEG_INFINITY, f64::max);

    let y_ticks = nice_ticks(y_min_raw, y_max_raw, 5);
    let y_min = y_ticks[0];
    let y_max = y_ticks[y_ticks.len() - 1];
    let x_ticks = nice_ticks_integer(x_min, x_max, 6);

    let last_idx = points.len() - 1;
    let final_label = fmt_sig4(points[last_idx].value);

    let max_y_tick_chars = y_ticks
        .iter()
        .map(|t| fmt_sig4(*t).chars().count())
        .max()
        .unwrap_or(1);
    let m = Margins {
        top: 14.0,
        right: 20.0 + final_label.chars().count() as f64 * CHAR_W,
        bottom: 30.0,
        left: 16.0 + max_y_tick_chars as f64 * CHAR_W,
    };
    let plot_w = (width - m.left - m.right).max(1.0);
    let plot_h = (height - m.top - m.bottom).max(1.0);

    let sx = |tn: f64| -> f64 {
        if (x_max - x_min).abs() < f64::EPSILON {
            m.left + plot_w / 2.0
        } else {
            m.left + (tn - x_min) / (x_max - x_min) * plot_w
        }
    };
    let sy = |v: f64| -> f64 {
        if (y_max - y_min).abs() < f64::EPSILON {
            m.top + plot_h / 2.0
        } else {
            m.top + plot_h - (v - y_min) / (y_max - y_min) * plot_h
        }
    };

    let mut body = String::new();

    for t in &y_ticks {
        let y = sy(*t);
        hairline(&mut body, m.left, y, m.left + plot_w, y);
        text_muted(&mut body, m.left - 8.0, y + 3.0, "end", &fmt_sig4(*t), true);
    }
    axis_line(
        &mut body,
        m.left,
        m.top + plot_h,
        m.left + plot_w,
        m.top + plot_h,
    );
    for t in &x_ticks {
        let x = sx(*t as f64);
        text_muted(
            &mut body,
            x,
            m.top + plot_h + 18.0,
            "middle",
            &t.to_string(),
            true,
        );
    }

    let path_points = points
        .iter()
        .map(|p| {
            format!(
                "{},{}",
                coord(sx(p.trial_number as f64)),
                coord(sy(p.value))
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let _ = writeln!(
        body,
        "<polyline points=\"{path_points}\" fill=\"none\" stroke=\"var({series1})\" stroke-width=\"2\" />",
        series1 = theme::VAR_SERIES[0]
    );

    let marker = |idx: usize, body: &mut String| {
        let p = &points[idx];
        let cx = sx(p.trial_number as f64);
        let cy = sy(p.value);
        let title = escape_xml(&format!(
            "trial #{} = {}",
            p.trial_number,
            fmt_sig4(p.value)
        ));
        let _ = writeln!(
            body,
            "<circle cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"var({series1})\"><title>{title}</title></circle>",
            coord(cx),
            coord(cy),
            series1 = theme::VAR_SERIES[0]
        );
    };

    for &idx in improvement_marks {
        if idx < points.len() {
            marker(idx, &mut body);
        }
    }
    if !improvement_marks.contains(&last_idx) {
        marker(last_idx, &mut body);
    }

    let last = &points[last_idx];
    text_secondary(
        &mut body,
        sx(last.trial_number as f64) + 8.0,
        sy(last.value) + 4.0,
        "start",
        &final_label,
    );

    svg_wrap(width, height, &body)
}

// ================================================================
// scatter_chart（Pareto フロント）
// ================================================================

/// [`scatter_chart`] の1点。
#[derive(Debug, Clone, Copy)]
pub struct ScatterPoint {
    /// 試行番号（ツールチップ表示用）。
    pub trial_number: i64,
    /// X 座標（先頭の目的値）。
    pub x: f64,
    /// Y 座標（2番目の目的値）。
    pub y: f64,
    /// 全制約を満たす点か（制約なしスタディでは常に `true`）。
    /// `false` の点はツールチップに `[infeasible]` を付記する。
    pub feasible: bool,
}

/// Pareto 散布図を描画する。
///
/// `background` は全 COMPLETE 点（ミュート・半透明・r=4）、`front` は
/// 非劣解点（`series-1`・r=5）。`front` は内部で X 昇順にソートしてから
/// 階段線（1.5px）で接続する（front が2点未満なら階段線は描かない）。
/// 軸ラベルには目的名を渡す。front / dominated の2系列が両方存在する
/// 場合はプロット右上に小さな凡例を表示する（2系列以上は凡例必須の規則）。
pub fn scatter_chart(
    background: &[ScatterPoint],
    front: &[ScatterPoint],
    x_label: &str,
    y_label: &str,
    width: f64,
    height: f64,
) -> String {
    if background.is_empty() && front.is_empty() {
        return empty_message(width, height);
    }

    let all_x: Vec<f64> = background.iter().chain(front.iter()).map(|p| p.x).collect();
    let all_y: Vec<f64> = background.iter().chain(front.iter()).map(|p| p.y).collect();
    let x_min_raw = all_x.iter().copied().fold(f64::INFINITY, f64::min);
    let x_max_raw = all_x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let y_min_raw = all_y.iter().copied().fold(f64::INFINITY, f64::min);
    let y_max_raw = all_y.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    let x_ticks = nice_ticks(x_min_raw, x_max_raw, 5);
    let y_ticks = nice_ticks(y_min_raw, y_max_raw, 5);
    let x_min = x_ticks[0];
    let x_max = x_ticks[x_ticks.len() - 1];
    let y_min = y_ticks[0];
    let y_max = y_ticks[y_ticks.len() - 1];

    // 左マージン = 回転 Y 軸タイトル分（16px）+ 目盛ラベルの最大幅 + 余白。
    // 上マージンは凡例1行分を確保する（front / dominated の2系列があるため
    // 凡例必須。単系列なら凡例なしの共通規則は他チャートに適用）。
    let max_y_tick_chars = y_ticks
        .iter()
        .map(|t| fmt_sig4(*t).chars().count())
        .max()
        .unwrap_or(1);
    let has_legend = !background.is_empty() && !front.is_empty();
    let m = Margins {
        top: if has_legend { 28.0 } else { 14.0 },
        right: 16.0,
        bottom: 44.0,
        left: 16.0 + max_y_tick_chars as f64 * CHAR_W + 12.0,
    };
    let plot_w = (width - m.left - m.right).max(1.0);
    let plot_h = (height - m.top - m.bottom).max(1.0);

    let sx = |v: f64| -> f64 {
        if (x_max - x_min).abs() < f64::EPSILON {
            m.left + plot_w / 2.0
        } else {
            m.left + (v - x_min) / (x_max - x_min) * plot_w
        }
    };
    let sy = |v: f64| -> f64 {
        if (y_max - y_min).abs() < f64::EPSILON {
            m.top + plot_h / 2.0
        } else {
            m.top + plot_h - (v - y_min) / (y_max - y_min) * plot_h
        }
    };

    let mut body = String::new();

    scatter_frame(
        &mut body, &m, plot_w, plot_h, height, &x_ticks, &y_ticks, &sx, &sy, x_label, y_label,
    );
    scatter_points(&mut body, background, front, &sx, &sy);
    if has_legend {
        scatter_legend(&mut body, &m, plot_w);
    }

    svg_wrap(width, height, &body)
}

/// 散布図の枠（グリッド線・目盛ラベル・軸線・軸タイトル）を書き込む。
#[allow(clippy::too_many_arguments)]
fn scatter_frame(
    body: &mut String,
    m: &Margins,
    plot_w: f64,
    plot_h: f64,
    height: f64,
    x_ticks: &[f64],
    y_ticks: &[f64],
    sx: &dyn Fn(f64) -> f64,
    sy: &dyn Fn(f64) -> f64,
    x_label: &str,
    y_label: &str,
) {
    for t in y_ticks {
        let y = sy(*t);
        hairline(body, m.left, y, m.left + plot_w, y);
        text_muted(body, m.left - 8.0, y + 3.0, "end", &fmt_sig4(*t), true);
    }
    for t in x_ticks {
        let x = sx(*t);
        hairline(body, x, m.top, x, m.top + plot_h);
        text_muted(
            body,
            x,
            m.top + plot_h + 18.0,
            "middle",
            &fmt_sig4(*t),
            true,
        );
    }
    axis_line(
        body,
        m.left,
        m.top + plot_h,
        m.left + plot_w,
        m.top + plot_h,
    );
    axis_line(body, m.left, m.top, m.left, m.top + plot_h);

    text_muted(
        body,
        m.left + plot_w / 2.0,
        height - 4.0,
        "middle",
        x_label,
        false,
    );
    // Y 軸タイトル: 左マージン内（目盛ラベルのさらに左）に rotate(-90) で
    // 縦書き配置する。回転中心 = (14, プロット縦中央)。
    let ty = m.top + plot_h / 2.0;
    let _ = writeln!(
        body,
        "<text x=\"14\" y=\"{}\" transform=\"rotate(-90 14 {})\" text-anchor=\"middle\" font-family=\"inherit\" font-size=\"{fs}\" fill=\"var({muted})\">{}</text>",
        coord(ty),
        coord(ty),
        escape_xml(y_label),
        fs = FONT_SIZE,
        muted = theme::VAR_INK_MUTED
    );
}

/// 散布図のデータマーク（背景点・front 階段線・front 点）を書き込む。
fn scatter_points(
    body: &mut String,
    background: &[ScatterPoint],
    front: &[ScatterPoint],
    sx: &dyn Fn(f64) -> f64,
    sy: &dyn Fn(f64) -> f64,
) {
    for p in background {
        let title = escape_xml(&scatter_title(p, false));
        let _ = writeln!(
            body,
            "<circle cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"var({muted})\" fill-opacity=\"0.4\"><title>{title}</title></circle>",
            coord(sx(p.x)),
            coord(sy(p.y)),
            muted = theme::VAR_INK_MUTED
        );
    }

    if front.len() >= 2 {
        let mut sorted_front: Vec<&ScatterPoint> = front.iter().collect();
        sorted_front.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
        let mut path = format!(
            "M {} {}",
            coord(sx(sorted_front[0].x)),
            coord(sy(sorted_front[0].y))
        );
        for w in sorted_front.windows(2) {
            let (prev, cur) = (w[0], w[1]);
            let _ = write!(
                path,
                " L {} {} L {} {}",
                coord(sx(cur.x)),
                coord(sy(prev.y)),
                coord(sx(cur.x)),
                coord(sy(cur.y))
            );
        }
        let _ = writeln!(
            body,
            "<path d=\"{path}\" fill=\"none\" stroke=\"var({series1})\" stroke-width=\"1.5\" />",
            series1 = theme::VAR_SERIES[0]
        );
    }

    for p in front {
        let title = escape_xml(&scatter_title(p, true));
        let _ = writeln!(
            body,
            "<circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"var({series1})\"><title>{title}</title></circle>",
            coord(sx(p.x)),
            coord(sy(p.y)),
            series1 = theme::VAR_SERIES[0]
        );
    }
}

/// 散布図点のツールチップ文字列（front / infeasible の付記込み）。
fn scatter_title(p: &ScatterPoint, on_front: bool) -> String {
    let mut title = format!(
        "trial #{} ({}, {})",
        p.trial_number,
        fmt_sig4(p.x),
        fmt_sig4(p.y)
    );
    if on_front {
        title.push_str(" [front]");
    }
    if !p.feasible {
        title.push_str(" [infeasible]");
    }
    title
}

/// 凡例（front / dominated の2系列があるときのみ）: プロット右上の
/// 上マージン内に「● Pareto front ● dominated」を右詰めで置く。
/// 凡例マーカーはデータマークではないため `<title>` を付けない。
fn scatter_legend(body: &mut String, m: &Margins, plot_w: f64) {
    const LEGEND_FRONT: &str = "Pareto front";
    const LEGEND_BG: &str = "dominated";
    let w_front = LEGEND_FRONT.chars().count() as f64 * CHAR_W;
    let w_bg = LEGEND_BG.chars().count() as f64 * CHAR_W;
    let item_gap = 18.0;
    let marker_w = 12.0; // マーカー直径 + テキストとの間隔
    let total = marker_w + w_front + item_gap + marker_w + w_bg;
    let start_x = (m.left + plot_w - total).max(m.left);
    let cy = 10.0;
    let text_y = cy + 4.0;

    let _ = writeln!(
        body,
        "<circle cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"var({series1})\" />",
        coord(start_x + 4.0),
        coord(cy),
        series1 = theme::VAR_SERIES[0]
    );
    text_muted(
        body,
        start_x + marker_w,
        text_y,
        "start",
        LEGEND_FRONT,
        false,
    );
    let x2 = start_x + marker_w + w_front + item_gap;
    let _ = writeln!(
        body,
        "<circle cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"var({muted})\" fill-opacity=\"0.4\" />",
        coord(x2 + 4.0),
        coord(cy),
        muted = theme::VAR_INK_MUTED
    );
    text_muted(body, x2 + marker_w, text_y, "start", LEGEND_BG, false);
}

// ================================================================
// hbar_chart（パラメータ重要度）
// ================================================================

/// [`hbar_chart`] の1本のバー。
#[derive(Debug, Clone)]
pub struct HBarItem {
    /// カテゴリ名（パラメータ名など）。
    pub label: String,
    /// 値（重要度スコア）。
    pub value: f64,
}

/// 横棒グラフを描画する（高さは項目数から自動決定）。
///
/// バーは `series-1` 単色（ランキングを色で塗り分けない）。バー高さ
/// 20px・角丸 `rx=2`・バー間ギャップ 8px。値をバー右端に直接ラベル、
/// カテゴリ名を左側に表示する（24文字超は `…` で truncate し、
/// 完全なラベルは `<title>` に保持する）。
pub fn hbar_chart(items: &[HBarItem], width: f64) -> String {
    const BAR_H: f64 = 20.0;
    const GAP: f64 = 8.0;
    const MAX_LABEL_CHARS: usize = 24;

    if items.is_empty() {
        return empty_message(width, 40.0);
    }

    let n = items.len();
    let top = 8.0;
    let bottom = 24.0;
    let height = top + bottom + n as f64 * BAR_H + (n.saturating_sub(1)) as f64 * GAP;

    let max_val = items
        .iter()
        .map(|it| it.value)
        .fold(f64::NEG_INFINITY, f64::max)
        .max(0.0);
    let value_ticks = nice_ticks(0.0, max_val, 4);
    let x_max = value_ticks[value_ticks.len() - 1].max(f64::EPSILON);

    let max_label_len = items
        .iter()
        .map(|it| truncate_label(&it.label, MAX_LABEL_CHARS).chars().count())
        .max()
        .unwrap_or(0);
    let max_value_len = items
        .iter()
        .map(|it| fmt_sig4(it.value).chars().count())
        .max()
        .unwrap_or(1);

    // 左マージンは truncation 後の表示ラベル最大文字数 × CHAR_W + 余白で
    // 動的に確保する（ラベルは m.left - 8 に end アンカーで置くため、
    // ラベル全幅 + アンカー余白 + 左端余白が必要）。
    let m = Margins {
        top,
        right: 16.0 + max_value_len as f64 * CHAR_W,
        bottom,
        left: 12.0 + max_label_len as f64 * CHAR_W + 8.0,
    };
    let plot_w = (width - m.left - m.right).max(1.0);

    let mut body = String::new();

    for t in &value_ticks {
        let x = m.left + (t / x_max) * plot_w;
        hairline(&mut body, x, m.top, x, height - m.bottom);
        text_muted(
            &mut body,
            x,
            height - m.bottom + 16.0,
            "middle",
            &fmt_sig4(*t),
            true,
        );
    }

    for (i, item) in items.iter().enumerate() {
        let y_top = m.top + i as f64 * (BAR_H + GAP);
        let y_mid = y_top + BAR_H / 2.0 + 4.0;
        let bar_w = ((item.value.max(0.0) / x_max) * plot_w).max(0.0);

        let full_label_escaped = escape_xml(&item.label);
        let display_label = truncate_label(&item.label, MAX_LABEL_CHARS);
        let _ = writeln!(
            body,
            "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-family=\"inherit\" font-size=\"{fs}\" fill=\"var({muted})\"><title>{full_label_escaped}</title>{}</text>",
            coord(m.left - 8.0),
            coord(y_mid),
            escape_xml(&display_label),
            fs = FONT_SIZE,
            muted = theme::VAR_INK_MUTED
        );

        let title = escape_xml(&format!("{}: {}", item.label, fmt_sig4(item.value)));
        let _ = writeln!(
            body,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" fill=\"var({series1})\"><title>{title}</title></rect>",
            coord(m.left),
            coord(y_top),
            coord(bar_w),
            coord(BAR_H),
            series1 = theme::VAR_SERIES[0]
        );

        text_secondary(
            &mut body,
            m.left + bar_w + 6.0,
            y_mid,
            "start",
            &fmt_sig4(item.value),
        );
    }

    svg_wrap(width, height, &body)
}

// ================================================================
// histogram（目的値分布）
// ================================================================

/// [`histogram`] の1ビン。
#[derive(Debug, Clone, Copy)]
pub struct HistBin {
    /// ビン下端。
    pub lower: f64,
    /// ビン上端。
    pub upper: f64,
    /// 度数。
    pub count: u64,
}

/// ヒストグラムを描画する（シーケンシャル単色、ビン間 2px ギャップ）。
pub fn histogram(bins: &[HistBin], width: f64, height: f64) -> String {
    const GAP: f64 = 2.0;

    if bins.is_empty() {
        return empty_message(width, height);
    }

    let max_count = bins.iter().map(|b| b.count).max().unwrap_or(0);
    let y_ticks = nice_ticks(0.0, max_count as f64, 4);
    let y_max = y_ticks[y_ticks.len() - 1].max(1.0);

    let max_y_tick_chars = y_ticks
        .iter()
        .map(|t| format!("{}", t.round() as i64).chars().count())
        .max()
        .unwrap_or(1);
    let m = Margins {
        top: 12.0,
        right: 12.0,
        bottom: 28.0,
        left: 16.0 + max_y_tick_chars as f64 * CHAR_W,
    };
    let plot_w = (width - m.left - m.right).max(1.0);
    let plot_h = (height - m.top - m.bottom).max(1.0);

    let n = bins.len();
    let bin_w = ((plot_w - GAP * (n.saturating_sub(1)) as f64) / n as f64).max(0.5);

    let mut body = String::new();

    for t in &y_ticks {
        let y = m.top + plot_h - (t / y_max) * plot_h;
        hairline(&mut body, m.left, y, m.left + plot_w, y);
        text_muted(
            &mut body,
            m.left - 8.0,
            y + 3.0,
            "end",
            &format!("{}", t.round() as i64),
            true,
        );
    }

    for (i, bin) in bins.iter().enumerate() {
        let x = m.left + i as f64 * (bin_w + GAP);
        let bar_h = (bin.count as f64 / y_max) * plot_h;
        let y = m.top + plot_h - bar_h;
        let title = escape_xml(&format!(
            "[{}, {}): {}",
            fmt_sig4(bin.lower),
            fmt_sig4(bin.upper),
            bin.count
        ));
        let _ = writeln!(
            body,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var({seq400})\"><title>{title}</title></rect>",
            coord(x),
            coord(y),
            coord(bin_w),
            coord(bar_h),
            seq400 = theme::VAR_SEQ[3].1
        );
    }

    // X 軸の代表目盛り（最初のビン下端 / 中央境界 / 最後のビン上端）。
    let first = bins[0].lower;
    let mid = bins[n / 2].lower;
    let last = bins[n - 1].upper;
    for (val, x) in [
        (first, m.left),
        (mid, m.left + (n / 2) as f64 * (bin_w + GAP)),
        (last, m.left + plot_w),
    ] {
        text_muted(&mut body, x, height - 8.0, "middle", &fmt_sig4(val), true);
    }

    svg_wrap(width, height, &body)
}

// ================================================================
// heatmap（相関ヒートマップ）
// ================================================================

/// 行ラベルの最大表示文字数（超過分は `…` + `<title>`）。
const HEATMAP_MAX_ROW_CHARS: usize = 24;
/// 列ラベルの最大表示文字数。回転ラベルの投影高さ（=上マージン）を
/// 有限に抑えるための上限で、超過分は `…` + `<title>` に退避する。
const HEATMAP_MAX_COL_CHARS: usize = 22;

/// 相関ヒートマップを描画する（ダイバージング配色、値域 `[-1, 1]`）。
///
/// `matrix[row][col]` の形。セル間 2px ギャップ、値をダイバージング
/// 5+5+neutral（[`theme::diverging_bin`]）に量子化して塗り、
/// `|value| > 0.5` のセルのみ数値を直接ラベルする（インクは
/// [`theme::diverging_ink_var`] によりセル明度に応じ黒/白を切り替える）。
/// 右側に量子化段階を示す離散カラーレジェンドを描画する。
///
/// 幅は `width` 引数で指定し、高さは行数から自動決定する（セル 1 行
/// あたり 28px 目安）。`row_labels.len() == matrix.len()` かつ
/// `col_labels.len() == matrix[0].len()` を前提とする。
///
/// 行ラベルは 24 文字、列ラベルは 22 文字で truncate し（`…` 付与・
/// 完全なラベルは `<title>` に保持）、左マージンは表示ラベル幅から、
/// 上マージンは rotate(-40°) した列ラベルの垂直投影高さから動的に決める。
pub fn heatmap(
    matrix: &[Vec<f64>],
    row_labels: &[String],
    col_labels: &[String],
    width: f64,
) -> String {
    const GAP: f64 = 2.0;
    const CELL_H: f64 = 28.0;
    const LEGEND_W: f64 = 90.0;
    /// rotate(-40°) した列ラベルの、1文字あたりの垂直投影高さ
    /// （`CHAR_W × sin(40°) ≈ 7.0 × 0.643`）。
    const COL_LABEL_RISE: f64 = 4.5;

    let rows = matrix.len();
    let cols = matrix.first().map(Vec::len).unwrap_or(0);
    if rows == 0 || cols == 0 {
        return empty_message(width, 40.0);
    }

    // 表示（truncation 後）ラベルの最大文字数からマージンを動的に決める。
    let max_row_chars = row_labels
        .iter()
        .map(|s| truncate_label(s, HEATMAP_MAX_ROW_CHARS).chars().count())
        .max()
        .unwrap_or(0);
    let max_col_chars = col_labels
        .iter()
        .map(|s| truncate_label(s, HEATMAP_MAX_COL_CHARS).chars().count())
        .max()
        .unwrap_or(0);
    let m = Margins {
        top: 24.0 + max_col_chars as f64 * COL_LABEL_RISE,
        right: LEGEND_W + 16.0,
        bottom: 8.0,
        left: 12.0 + max_row_chars as f64 * CHAR_W + 8.0,
    };

    let height = m.top + m.bottom + rows as f64 * CELL_H + (rows.saturating_sub(1)) as f64 * GAP;
    let plot_w = (width - m.left - m.right).max(1.0);
    let plot_h = rows as f64 * CELL_H + (rows.saturating_sub(1)) as f64 * GAP;
    let cell_w = ((plot_w - GAP * (cols.saturating_sub(1)) as f64) / cols as f64).max(0.5);

    let mut body = String::new();

    heatmap_col_labels(&mut body, col_labels, cols, &m, cell_w, GAP);
    heatmap_cells(
        &mut body, matrix, row_labels, col_labels, rows, cols, &m, cell_w, CELL_H, GAP,
    );
    heatmap_legend(&mut body, width, &m, plot_h, LEGEND_W);

    svg_wrap(width, height, &body)
}

/// ヒートマップの列ラベル（rotate(-40°)、truncation + `<title>` 退避）を書き込む。
fn heatmap_col_labels(
    body: &mut String,
    col_labels: &[String],
    cols: usize,
    m: &Margins,
    cell_w: f64,
    gap: f64,
) {
    for (c, label) in col_labels.iter().enumerate().take(cols) {
        let cx = m.left + c as f64 * (cell_w + gap) + cell_w / 2.0;
        let cy = m.top - 8.0;
        let display = truncate_label(label, HEATMAP_MAX_COL_CHARS);
        // truncation したときのみ完全ラベルを <title> で保持する。
        let title = if display == *label {
            String::new()
        } else {
            format!("<title>{}</title>", escape_xml(label))
        };
        let _ = writeln!(
            body,
            "<text x=\"{}\" y=\"{}\" transform=\"rotate(-40 {} {})\" text-anchor=\"start\" font-family=\"inherit\" font-size=\"{fs}\" fill=\"var({muted})\">{title}{}</text>",
            coord(cx),
            coord(cy),
            coord(cx),
            coord(cy),
            escape_xml(&display),
            fs = FONT_SIZE,
            muted = theme::VAR_INK_MUTED
        );
    }
}

/// ヒートマップの行ラベルとセル（量子化塗り + 高相関セルの直接ラベル）を書き込む。
#[allow(clippy::too_many_arguments)]
fn heatmap_cells(
    body: &mut String,
    matrix: &[Vec<f64>],
    row_labels: &[String],
    col_labels: &[String],
    rows: usize,
    cols: usize,
    m: &Margins,
    cell_w: f64,
    cell_h: f64,
    gap: f64,
) {
    for (r, row) in matrix.iter().enumerate().take(rows) {
        let y = m.top + r as f64 * (cell_h + gap);
        let row_label = row_labels.get(r).map(String::as_str).unwrap_or("");
        let display = truncate_label(row_label, HEATMAP_MAX_ROW_CHARS);
        let title = if display == row_label {
            String::new()
        } else {
            format!("<title>{}</title>", escape_xml(row_label))
        };
        let _ = writeln!(
            body,
            "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-family=\"inherit\" font-size=\"{fs}\" fill=\"var({muted})\">{title}{}</text>",
            coord(m.left - 8.0),
            coord(y + cell_h / 2.0 + 4.0),
            escape_xml(&display),
            fs = FONT_SIZE,
            muted = theme::VAR_INK_MUTED
        );

        for (c, &value) in row.iter().enumerate().take(cols) {
            let x = m.left + c as f64 * (cell_w + gap);
            let clamped = value.clamp(-1.0, 1.0);
            let bin = theme::diverging_bin(clamped);
            let fill_var = theme::diverging_var(bin);
            let col_label = col_labels.get(c).map(String::as_str).unwrap_or("");
            let title = escape_xml(&format!("{row_label} × {col_label}: {}", fmt_sig4(value)));
            let _ = writeln!(
                body,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var({fill_var})\"><title>{title}</title></rect>",
                coord(x),
                coord(y),
                coord(cell_w),
                coord(cell_h)
            );

            if theme::diverging_show_label(value) {
                let ink_var = theme::diverging_ink_var(bin);
                let _ = writeln!(
                    body,
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"inherit\" font-size=\"{fs}\" fill=\"var({ink_var})\" style=\"font-variant-numeric: tabular-nums\">{}</text>",
                    coord(x + cell_w / 2.0),
                    coord(y + cell_h / 2.0 + 4.0),
                    escape_xml(&fmt_sig4(value)),
                    fs = FONT_SIZE
                );
            }
        }
    }
}

/// 離散カラーレジェンド（-5..=5 の11段階を縦に並べる）を書き込む。
fn heatmap_legend(body: &mut String, width: f64, m: &Margins, plot_h: f64, legend_w: f64) {
    let legend_x = width - legend_w + 8.0;
    let swatch_h = (plot_h / 11.0).max(6.0);
    for (i, bin) in (-5..=5).rev().enumerate() {
        let y = m.top + i as f64 * swatch_h;
        let fill_var = theme::diverging_var(bin);
        let range_desc = diverging_bin_range_desc(bin);
        let _ = writeln!(
            body,
            "<rect x=\"{}\" y=\"{}\" width=\"14\" height=\"{}\" fill=\"var({fill_var})\"><title>{}</title></rect>",
            coord(legend_x),
            coord(y),
            coord(swatch_h.max(1.0)),
            escape_xml(&range_desc)
        );
        if bin == 5 || bin == 0 || bin == -5 {
            let label = if bin == 5 {
                "1"
            } else if bin == -5 {
                "-1"
            } else {
                "0"
            };
            text_muted(
                body,
                legend_x + 18.0,
                y + swatch_h / 2.0 + 3.0,
                "start",
                label,
                true,
            );
        }
    }
}

/// レジェンドの `<title>` 用に量子化段階が表す値域を説明する文字列を作る。
fn diverging_bin_range_desc(bin: i32) -> String {
    match bin {
        0 => "0".to_string(),
        b if b > 0 => {
            let lower = (b - 1) as f64 / 5.0;
            let upper = b as f64 / 5.0;
            format!("{lower} < ρ ≤ {upper}")
        }
        b => {
            let lower = b as f64 / 5.0;
            let upper = (b + 1) as f64 / 5.0;
            format!("{lower} ≤ ρ < {upper}")
        }
    }
}

// ================================================================
// テスト
// ================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// `fill="..."` / `stroke="..."` 属性値の中に `#` が含まれていないことを
    /// 確認する（生の16進カラーコード禁止。`var(--foo)` には `#` を含まない）。
    fn assert_no_raw_hex_in_color_attrs(svg: &str) {
        for attr in ["fill=\"", "stroke=\""] {
            let mut rest = svg;
            while let Some(pos) = rest.find(attr) {
                let after = &rest[pos + attr.len()..];
                let end = after.find('"').expect("unterminated attribute");
                let value = &after[..end];
                assert!(
                    !value.contains('#'),
                    "raw hex color found in {attr}: {value}"
                );
                rest = &after[end..];
            }
        }
    }

    /// `<title>` 要素の出現回数を数える。
    fn count_titles(svg: &str) -> usize {
        svg.matches("<title>").count()
    }

    fn count(svg: &str, needle: &str) -> usize {
        svg.matches(needle).count()
    }

    // ---------------- nice_ticks ----------------

    #[test]
    fn nice_ticks_basic_range() {
        let ticks = nice_ticks(0.0, 95.0, 5);
        assert!(ticks.len() >= 2);
        assert!(ticks[0] <= 0.0);
        assert!(*ticks.last().unwrap() >= 95.0);
        for w in ticks.windows(2) {
            assert!(w[1] > w[0]);
        }
    }

    #[test]
    fn nice_ticks_negative_range() {
        let ticks = nice_ticks(-50.0, -3.0, 5);
        assert!(ticks[0] <= -50.0);
        assert!(*ticks.last().unwrap() >= -3.0);
        assert!(ticks.windows(2).all(|w| w[1] > w[0]));
    }

    #[test]
    fn nice_ticks_straddling_zero() {
        let ticks = nice_ticks(-1.5, 2.5, 5);
        assert!(ticks[0] <= -1.5);
        assert!(*ticks.last().unwrap() >= 2.5);
    }

    #[test]
    fn nice_ticks_tiny_range() {
        let ticks = nice_ticks(0.000_010, 0.000_030, 5);
        assert!(ticks[0] <= 0.000_010);
        assert!(*ticks.last().unwrap() >= 0.000_030);
        assert!(ticks.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn nice_ticks_degenerate_min_equals_max_nonzero() {
        let ticks = nice_ticks(5.0, 5.0, 5);
        assert!(ticks.len() >= 2);
        assert!(ticks.iter().all(|v| v.is_finite()));
        assert!(ticks[0] <= 5.0 && *ticks.last().unwrap() >= 5.0);
    }

    #[test]
    fn nice_ticks_degenerate_min_equals_max_zero() {
        let ticks = nice_ticks(0.0, 0.0, 5);
        assert!(ticks.len() >= 2);
        assert!(ticks.iter().all(|v| v.is_finite()));
        assert!(ticks[0] <= 0.0 && *ticks.last().unwrap() >= 0.0);
    }

    #[test]
    fn nice_ticks_integer_produces_distinct_integers() {
        let ticks = nice_ticks_integer(0.0, 59.0, 6);
        assert!(ticks.len() >= 2);
        let mut sorted = ticks.clone();
        sorted.dedup();
        assert_eq!(sorted.len(), ticks.len());
    }

    #[test]
    fn nice_ticks_integer_degenerate() {
        // 退化ケース（min == max）でも panic せず、値を含む昇順の整数列を返す。
        let ticks = nice_ticks_integer(3.0, 3.0, 6);
        assert!(!ticks.is_empty());
        assert!(ticks.windows(2).all(|w| w[1] > w[0]));
        assert!(ticks[0] <= 3 && *ticks.last().unwrap() >= 3);
    }

    // ---------------- escaping ----------------

    #[test]
    fn escape_xml_handles_all_special_chars() {
        assert_eq!(
            escape_xml("<script>&\"'</script>"),
            "&lt;script&gt;&amp;&quot;&apos;&lt;/script&gt;"
        );
    }

    #[test]
    fn hbar_chart_escapes_malicious_label() {
        let items = vec![HBarItem {
            label: "<script>alert(1)</script>".to_string(),
            value: 1.0,
        }];
        let svg = hbar_chart(&items, 400.0);
        assert!(!svg.contains("<script>alert"));
        assert!(svg.contains("&lt;script&gt;"));
    }

    // ---------------- fmt_sig4 ----------------

    #[test]
    fn fmt_sig4_basic_cases() {
        assert_eq!(fmt_sig4(0.0), "0");
        assert_eq!(fmt_sig4(1234.0), "1234");
        assert_eq!(fmt_sig4(1234.5678), "1235");
        assert_eq!(fmt_sig4(0.00012345), "0.0001234");
        assert_eq!(fmt_sig4(-12.3456), "-12.35");
        assert_eq!(fmt_sig4(0.1), "0.1");
        assert_eq!(fmt_sig4(100.0), "100");
    }

    // ---------------- line_chart ----------------

    #[test]
    fn line_chart_mark_and_title_counts() {
        let points: Vec<LinePoint> = (0..10)
            .map(|i| LinePoint {
                trial_number: i,
                value: (i as f64).sin(),
            })
            .collect();
        let improvement = vec![0usize, 3, 7];
        let svg = line_chart(&points, &improvement, 400.0, 200.0);

        // improvement marks (3) + final point marker (idx 9 not in improvement) = 4 circles.
        assert_eq!(count(&svg, "<circle"), 4);
        assert_eq!(count_titles(&svg), 4);
        assert_no_raw_hex_in_color_attrs(&svg);
    }

    #[test]
    fn line_chart_final_point_already_improvement_no_duplicate_marker() {
        let points: Vec<LinePoint> = (0..5)
            .map(|i| LinePoint {
                trial_number: i,
                value: i as f64,
            })
            .collect();
        let improvement = vec![0usize, 4];
        let svg = line_chart(&points, &improvement, 300.0, 150.0);
        assert_eq!(count(&svg, "<circle"), 2);
        assert_eq!(count_titles(&svg), 2);
    }

    #[test]
    fn line_chart_empty_points_no_panic() {
        let svg = line_chart(&[], &[], 300.0, 150.0);
        assert!(svg.contains("no data"));
    }

    // ---------------- scatter_chart ----------------

    #[test]
    fn scatter_chart_mark_and_title_counts() {
        let background: Vec<ScatterPoint> = (0..20)
            .map(|i| ScatterPoint {
                trial_number: i,
                x: i as f64,
                y: (20 - i) as f64,
                feasible: true,
            })
            .collect();
        let front: Vec<ScatterPoint> = (0..5)
            .map(|i| ScatterPoint {
                trial_number: i,
                x: i as f64 * 2.0,
                y: (5 - i) as f64,
                feasible: true,
            })
            .collect();
        let svg = scatter_chart(&background, &front, "obj1", "obj2", 400.0, 300.0);

        // データマーク + 凡例マーカー2個（凡例マーカーには <title> を付けない）。
        assert_eq!(count(&svg, "<circle"), background.len() + front.len() + 2);
        assert_eq!(count_titles(&svg), background.len() + front.len());
        assert!(svg.contains("<path"));
        // 2系列（front / dominated）があるため凡例必須。
        assert!(svg.contains("Pareto front"));
        assert!(svg.contains("dominated"));
        assert_no_raw_hex_in_color_attrs(&svg);
    }

    #[test]
    fn scatter_chart_no_legend_when_single_series() {
        // background のみ（単系列）なら凡例なし（共通規則）。
        let background: Vec<ScatterPoint> = (0..5)
            .map(|i| ScatterPoint {
                trial_number: i,
                x: i as f64,
                y: i as f64,
                feasible: true,
            })
            .collect();
        let svg = scatter_chart(&background, &[], "x", "y", 300.0, 200.0);
        assert!(!svg.contains("Pareto front"));
        assert_eq!(count(&svg, "<circle"), background.len());
    }

    #[test]
    fn scatter_chart_single_front_point_no_staircase_path() {
        let background = vec![ScatterPoint {
            trial_number: 0,
            x: 1.0,
            y: 1.0,
            feasible: true,
        }];
        let front = vec![ScatterPoint {
            trial_number: 0,
            x: 1.0,
            y: 1.0,
            feasible: true,
        }];
        let svg = scatter_chart(&background, &front, "x", "y", 300.0, 200.0);
        assert!(!svg.contains("<path"));
    }

    #[test]
    fn scatter_chart_marks_infeasible_in_tooltip() {
        // feasible=false の点はツールチップに [infeasible] を付記する。
        // feasible な点には付かない。
        let background = vec![
            ScatterPoint {
                trial_number: 0,
                x: 1.0,
                y: 2.0,
                feasible: false,
            },
            ScatterPoint {
                trial_number: 1,
                x: 2.0,
                y: 1.0,
                feasible: true,
            },
        ];
        let svg = scatter_chart(&background, &[], "x", "y", 300.0, 200.0);
        assert_eq!(count(&svg, "[infeasible]"), 1);
        assert!(svg.contains("trial #0 (1, 2) [infeasible]"));
        assert!(svg.contains("trial #1 (2, 1)</title>"));
    }

    // ---------------- hbar_chart ----------------

    #[test]
    fn hbar_chart_mark_and_title_counts() {
        let items = vec![
            HBarItem {
                label: "alpha".to_string(),
                value: 0.8,
            },
            HBarItem {
                label: "a_very_long_parameter_name_that_exceeds_limit".to_string(),
                value: 0.5,
            },
            HBarItem {
                label: "gamma".to_string(),
                value: 0.2,
            },
        ];
        let svg = hbar_chart(&items, 400.0);
        assert_eq!(count(&svg, "<rect"), items.len());
        // rect titles + label <text> titles (both counted) = 2 * items.len()
        assert_eq!(count_titles(&svg), items.len() * 2);
        assert_no_raw_hex_in_color_attrs(&svg);
    }

    #[test]
    fn hbar_chart_truncates_long_label_with_ellipsis() {
        let long_name = "a_very_long_parameter_name_that_exceeds_limit";
        let items = vec![HBarItem {
            label: long_name.to_string(),
            value: 1.0,
        }];
        let svg = hbar_chart(&items, 400.0);
        assert!(svg.contains('…'));
        // full label preserved in title
        assert!(svg.contains(long_name));
    }

    // ---------------- histogram ----------------

    #[test]
    fn histogram_mark_and_title_counts() {
        let bins: Vec<HistBin> = (0..20)
            .map(|i| HistBin {
                lower: i as f64,
                upper: (i + 1) as f64,
                count: (i % 7) as u64,
            })
            .collect();
        let svg = histogram(&bins, 500.0, 200.0);
        assert_eq!(count(&svg, "<rect"), bins.len());
        assert_eq!(count_titles(&svg), bins.len());
        assert_no_raw_hex_in_color_attrs(&svg);
    }

    // ---------------- heatmap ----------------

    #[test]
    fn heatmap_mark_and_title_counts() {
        let matrix = vec![vec![-1.0, -0.51, -0.5, 0.0], vec![0.5, 0.51, 1.0, 0.2]];
        let row_labels = vec!["p1".to_string(), "p2".to_string()];
        let col_labels = vec![
            "o1".to_string(),
            "o2".to_string(),
            "o3".to_string(),
            "o4".to_string(),
        ];
        let svg = heatmap(&matrix, &row_labels, &col_labels, 500.0);

        let cell_count = 2 * 4;
        assert_eq!(count(&svg, "<rect"), cell_count + 11); // cells + legend swatches
                                                           // title count = cell titles + legend titles (11)
        assert_eq!(count_titles(&svg), cell_count + 11);
        assert_no_raw_hex_in_color_attrs(&svg);
    }

    #[test]
    fn heatmap_quantization_boundaries() {
        // -1, -0.51, -0.5, 0, 0.5, 0.51, 1 の境界値で量子化段階と
        // ラベル表示可否が理論値と一致することを確認する。
        let cases = [
            (-1.0, -5, true),
            (-0.51, -3, true),
            (-0.5, -3, false),
            (0.0, 0, false),
            (0.5, 3, false),
            (0.51, 3, true),
            (1.0, 5, true),
        ];
        for (v, expected_bin, expected_label) in cases {
            assert_eq!(theme::diverging_bin(v), expected_bin, "bin for {v}");
            assert_eq!(
                theme::diverging_show_label(v),
                expected_label,
                "label for {v}"
            );
        }
    }

    #[test]
    fn heatmap_empty_matrix_no_panic() {
        let svg = heatmap(&[], &[], &[], 400.0);
        assert!(svg.contains("no data"));
    }

    #[test]
    fn heatmap_truncates_long_labels_with_title() {
        let long_row = "an_extremely_long_row_parameter_name_here";
        let long_col = "an_extremely_long_objective_name";
        let matrix = vec![vec![0.3]];
        let svg = heatmap(
            &matrix,
            &[long_row.to_string()],
            &[long_col.to_string()],
            500.0,
        );
        // 表示は truncate され、完全ラベルは <title> に退避される。
        assert!(svg.contains('…'));
        assert!(svg.contains(long_row));
        assert!(svg.contains(long_col));
    }

    #[test]
    fn truncate_label_respects_max_chars() {
        let s = "0123456789012345678901234567"; // 29 chars
        let t = truncate_label(s, 24);
        assert_eq!(t.chars().count(), 24);
        assert!(t.ends_with('…'));
        assert_eq!(truncate_label("short", 24), "short");
    }
}
