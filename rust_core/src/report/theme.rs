//! HTML / SVG レポート共通カラーパレット（検証済みトークン）。
//!
//! 値の出典は設計書「短期2: 自己完結型レポート出力」の
//! 「HTML レンダラ」節に記載のカラートークンで、本モジュールはそれを
//! `pub const &str` として固定化したものである。ライト/ダーク両対応の
//! ページを JS 無しで実現するため、値そのものではなく CSS custom
//! property（`--foo` の形）を経由して SVG / HTML から参照する設計とし、
//! [`css_variables`] がその宣言ブロック（`:root` + `@media
//! (prefers-color-scheme: dark)`）を文字列として生成する。
//!
//! # 派生パレットの導出方法（シーケンシャル / ダイバージング）
//!
//! 設計書はシーケンシャルの両端（100 と 700）とダイバージングの両極
//! （blue / red）と中立色のみを規定しており、中間段階は本モジュールで
//! 補間して定義する。単純に sRGB のバイト値を線形補間すると、色相の
//! 異なる中間色が濁って（低彩度に）見える問題があるため、
//! [OKLab](https://bottosson.github.io/posts/oklab/) 色空間（知覚的に
//! 均等になるよう設計された空間）で補間してから sRGB に戻す。
//! 具体的な手順は以下（`#[cfg(test)]` 内に同じ手順の参照実装があり、
//! 下記の定数値がその実装の出力と一致することをテストで担保している）。
//!
//! 1. 両端の sRGB 16 進値をリニア RGB → OKLab に変換する。
//! 2. OKLab 空間で `L`, `a`, `b` をそれぞれ線形補間する。
//! 3. OKLab → リニア RGB → sRGB に逆変換し、8bit に丸めて 16 進文字列化する。
//!
//! シーケンシャル（`SEQ_100..=SEQ_700`）は 100 刻みの 7 段階
//! （`t = (step - 100) / 600`）、ダイバージングの各腕（`DIV_NEG_*` /
//! `DIV_POS_*`）は中立色を `t = 0`、極色を `t = 1` として
//! `t = i / 5`（`i = 1..=5`）の 5 段階で補間している
//! （設計書の「五段階 × 2 腕」に対応）。
//!
//! ダイバージングの直接ラベルのインク色（黒 / 白）は各段階の相対輝度
//! （WCAG のコントラスト比）から事前計算済みで、[`diverging_ink_var`]
//! が段階ごとに適切な CSS 変数名を返す。

// ============================================================
// Surface（面）
// ============================================================

/// ライトテーマの面色。
pub const SURFACE_LIGHT: &str = "#fcfcfb";
/// ダークテーマの面色。
pub const SURFACE_DARK: &str = "#1a1a19";

// ============================================================
// Ink（テキスト）
// ============================================================

/// ライトテーマの主要インク色（本文・見出し）。
pub const INK_PRIMARY_LIGHT: &str = "#0b0b0b";
/// ダークテーマの主要インク色。
pub const INK_PRIMARY_DARK: &str = "#ffffff";
/// ライトテーマの副次インク色（キャプション等）。
pub const INK_SECONDARY_LIGHT: &str = "#52514e";
/// ダークテーマの副次インク色。
pub const INK_SECONDARY_DARK: &str = "#c3c2b7";
/// ミュートインク色（軸ラベル・補助テキスト用）。
///
/// 設計書はライト/ダークで単一値のみを規定している
/// （このグレーは両テーマの面色に対して十分なコントラストを保てるため、
/// テーマ別の値を持たない）。
pub const INK_MUTED: &str = "#898781";

// ============================================================
// Grid / Axis
// ============================================================

/// ライトテーマのグリッド線色（hairline）。
pub const GRID_LIGHT: &str = "#e1e0d9";
/// ダークテーマのグリッド線色。
pub const GRID_DARK: &str = "#2c2c2a";
/// ライトテーマの軸線色。
pub const AXIS_LIGHT: &str = "#c3c2b7";
/// ダークテーマの軸線色。
pub const AXIS_DARK: &str = "#383835";

// ============================================================
// Categorical series（1〜6、この順番固定・循環禁止）
// ============================================================

/// 系列1（blue）ライト。
pub const SERIES_1_LIGHT: &str = "#2a78d6";
/// 系列1（blue）ダーク。
pub const SERIES_1_DARK: &str = "#3987e5";
/// 系列2（aqua）ライト。
pub const SERIES_2_LIGHT: &str = "#1baf7a";
/// 系列2（aqua）ダーク。
pub const SERIES_2_DARK: &str = "#199e70";
/// 系列3（yellow）ライト。
pub const SERIES_3_LIGHT: &str = "#eda100";
/// 系列3（yellow）ダーク。
pub const SERIES_3_DARK: &str = "#c98500";
/// 系列4（green）ライト。
///
/// 設計書は単一値のみ規定しており、ダークテーマでも同値を用いる。
pub const SERIES_4_LIGHT: &str = "#008300";
/// 系列4（green）ダーク（ライトと同値）。
pub const SERIES_4_DARK: &str = "#008300";
/// 系列5（violet）ライト。
pub const SERIES_5_LIGHT: &str = "#4a3aa7";
/// 系列5（violet）ダーク。
pub const SERIES_5_DARK: &str = "#9085e9";
/// 系列6（red）ライト。
pub const SERIES_6_LIGHT: &str = "#e34948";
/// 系列6（red）ダーク。
pub const SERIES_6_DARK: &str = "#e66767";

/// カテゴリカル系列色（ライト）。インデックス 0 が系列1に対応する。
/// 循環利用は禁止（7系列目以降が必要な設計は行わない）。
pub const SERIES_LIGHT: [&str; 6] = [
    SERIES_1_LIGHT,
    SERIES_2_LIGHT,
    SERIES_3_LIGHT,
    SERIES_4_LIGHT,
    SERIES_5_LIGHT,
    SERIES_6_LIGHT,
];
/// カテゴリカル系列色（ダーク）。
pub const SERIES_DARK: [&str; 6] = [
    SERIES_1_DARK,
    SERIES_2_DARK,
    SERIES_3_DARK,
    SERIES_4_DARK,
    SERIES_5_DARK,
    SERIES_6_DARK,
];

// ============================================================
// Sequential（ヒストグラム等の単調量。テーマ非依存の単一パレット）
// ============================================================

/// シーケンシャル 100（最淡）。設計書の両端値そのもの。
pub const SEQ_100: &str = "#cde2fb";
/// シーケンシャル 200（OKLab 補間、`t=1/6`）。
pub const SEQ_200: &str = "#abc3e2";
/// シーケンシャル 300（OKLab 補間、`t=2/6`）。
pub const SEQ_300: &str = "#8aa6ca";
/// シーケンシャル 400（OKLab 補間、`t=3/6`）。ヒストグラムの既定色。
pub const SEQ_400: &str = "#6a89b2";
/// シーケンシャル 500（OKLab 補間、`t=4/6`）。
pub const SEQ_500: &str = "#4b6c9a";
/// シーケンシャル 600（OKLab 補間、`t=5/6`）。
pub const SEQ_600: &str = "#2d5182";
/// シーケンシャル 700（最濃）。設計書の両端値そのもの。
pub const SEQ_700: &str = "#0d366b";

// ============================================================
// Diverging（相関ヒートマップ、[-1,1] を 5+5+neutral に量子化）
// ============================================================

/// ダイバージング中立色（ライト、`v=0`）。
pub const DIV_NEUTRAL_LIGHT: &str = "#f0efec";
/// ダイバージング中立色（ダーク、`v=0`）。
pub const DIV_NEUTRAL_DARK: &str = "#383835";

/// 負側（blue 方向）段階1（中立寄り）ライト。
pub const DIV_NEG_1_LIGHT: &str = "#c9d8ea";
/// 負側段階2ライト。
pub const DIV_NEG_2_LIGHT: &str = "#a3c1e7";
/// 負側段階3ライト。
pub const DIV_NEG_3_LIGHT: &str = "#7da9e2";
/// 負側段階4ライト。
pub const DIV_NEG_4_LIGHT: &str = "#5691dd";
/// 負側段階5（`v=-1`、blue 極そのもの）ライト。
pub const DIV_NEG_5_LIGHT: &str = "#2a78d6";

/// 正側（red 方向）段階1（中立寄り）ライト。
pub const DIV_POS_1_LIGHT: &str = "#f3d0cb";
/// 正側段階2ライト。
pub const DIV_POS_2_LIGHT: &str = "#f2b1aa";
/// 正側段階3ライト。
pub const DIV_POS_3_LIGHT: &str = "#f09289";
/// 正側段階4ライト。
pub const DIV_POS_4_LIGHT: &str = "#ea7069";
/// 正側段階5（`v=1`、red 極そのもの）ライト。
pub const DIV_POS_5_LIGHT: &str = "#e34948";

/// 負側段階1ダーク。
pub const DIV_NEG_1_DARK: &str = "#3b4856";
/// 負側段階2ダーク。
pub const DIV_NEG_2_DARK: &str = "#3d5878";
/// 負側段階3ダーク。
pub const DIV_NEG_3_DARK: &str = "#3e689b";
/// 負側段階4ダーク。
pub const DIV_NEG_4_DARK: &str = "#3c77bf";
/// 負側段階5（`v=-1`）ダーク。
pub const DIV_NEG_5_DARK: &str = "#3987e5";

/// 正側段階1ダーク。
pub const DIV_POS_1_DARK: &str = "#5a433f";
/// 正側段階2ダーク。
pub const DIV_POS_2_DARK: &str = "#7b4d49";
/// 正側段階3ダーク。
pub const DIV_POS_3_DARK: &str = "#9e5653";
/// 正側段階4ダーク。
pub const DIV_POS_4_DARK: &str = "#c15f5d";
/// 正側段階5（`v=1`）ダーク。
pub const DIV_POS_5_DARK: &str = "#e66767";

// ============================================================
// CSS custom property 名（svg.rs / html レンダラが共有する唯一の定義）
// ============================================================

/// `--surface` 変数名。
pub const VAR_SURFACE: &str = "--surface";
/// `--ink-primary` 変数名。
pub const VAR_INK_PRIMARY: &str = "--ink-primary";
/// `--ink-secondary` 変数名。
pub const VAR_INK_SECONDARY: &str = "--ink-secondary";
/// `--ink-muted` 変数名。
pub const VAR_INK_MUTED: &str = "--ink-muted";
/// `--grid` 変数名。
pub const VAR_GRID: &str = "--grid";
/// `--axis` 変数名。
pub const VAR_AXIS: &str = "--axis";

/// カテゴリカル系列の変数名（インデックス 0 が系列1）。
pub const VAR_SERIES: [&str; 6] = [
    "--series-1",
    "--series-2",
    "--series-3",
    "--series-4",
    "--series-5",
    "--series-6",
];

/// シーケンシャル段階と変数名の対応（`(段階, 変数名)`）。
pub const VAR_SEQ: [(u16, &str); 7] = [
    (100, "--seq-100"),
    (200, "--seq-200"),
    (300, "--seq-300"),
    (400, "--seq-400"),
    (500, "--seq-500"),
    (600, "--seq-600"),
    (700, "--seq-700"),
];

/// ダイバージング中立色の変数名。
pub const VAR_DIV_NEUTRAL: &str = "--div-neutral";
/// ダイバージング中立色の直接ラベルインク変数名。
pub const VAR_DIV_NEUTRAL_INK: &str = "--div-neutral-ink";
/// ダイバージング負側（blue 方向）の変数名（インデックス 0 が段階1）。
pub const VAR_DIV_NEG: [&str; 5] = [
    "--div-neg-1",
    "--div-neg-2",
    "--div-neg-3",
    "--div-neg-4",
    "--div-neg-5",
];
/// ダイバージング負側の直接ラベルインク変数名。
pub const VAR_DIV_NEG_INK: [&str; 5] = [
    "--div-neg-1-ink",
    "--div-neg-2-ink",
    "--div-neg-3-ink",
    "--div-neg-4-ink",
    "--div-neg-5-ink",
];
/// ダイバージング正側（red 方向）の変数名。
pub const VAR_DIV_POS: [&str; 5] = [
    "--div-pos-1",
    "--div-pos-2",
    "--div-pos-3",
    "--div-pos-4",
    "--div-pos-5",
];
/// ダイバージング正側の直接ラベルインク変数名。
pub const VAR_DIV_POS_INK: [&str; 5] = [
    "--div-pos-1-ink",
    "--div-pos-2-ink",
    "--div-pos-3-ink",
    "--div-pos-4-ink",
    "--div-pos-5-ink",
];

/// 相関ヒートマップの値を `-5..=5` の量子化段階に丸める
/// （`0` が中立、`-5`/`5` が両極）。
///
/// 量子化規則: `mag = ceil(min(|v|, 1) * 5)` を `1..=5` にクランプし、
/// `v` の符号を付与する。`v == 0.0` は中立（`0`）を返す。
/// 境界は `(0.0, 0.2] -> 1, (0.2, 0.4] -> 2, ..., (0.8, 1.0] -> 5` であり、
/// たとえば `v = 0.5` と `v = 0.51` は同じ段階（3）に量子化される
/// （数値ラベルの表示要否は別途 [`diverging_show_label`] の `|v| > 0.5`
/// で判定するため、量子化そのものは 0.5 ちょうどでは切り替わらない）。
pub fn diverging_bin(v: f64) -> i32 {
    if v == 0.0 {
        return 0;
    }
    let mag = (v.abs().min(1.0) * 5.0).ceil().clamp(1.0, 5.0) as i32;
    if v < 0.0 {
        -mag
    } else {
        mag
    }
}

/// `|v| > 0.5` のセルにのみ数値の直接ラベルを表示する（設計書の規則）。
pub fn diverging_show_label(v: f64) -> bool {
    v.abs() > 0.5
}

/// 量子化段階（[`diverging_bin`] の戻り値、`-5..=5`）に対応する
/// 塗り色の CSS 変数名を返す。
pub fn diverging_var(bin: i32) -> &'static str {
    match bin {
        0 => VAR_DIV_NEUTRAL,
        b if (1..=5).contains(&b) => VAR_DIV_POS[(b - 1) as usize],
        b if (-5..=-1).contains(&b) => VAR_DIV_NEG[(-b - 1) as usize],
        _ => VAR_DIV_NEUTRAL,
    }
}

/// 量子化段階に対応する直接ラベルインク色の CSS 変数名を返す。
///
/// インク色自体は各段階のセル色に対する WCAG コントラスト比を
/// 事前計算して選定済み（ライトテーマは全段階で黒、ダークテーマは
/// 両極（`-5`/`5`）のみ黒・それ以外は白）。値は
/// [`VAR_INK_PRIMARY`] を指す `--ink-primary` ではなく、テーマに応じて
/// 黒/白いずれかに固定された専用変数（`css_variables` が定義する）を返す。
pub fn diverging_ink_var(bin: i32) -> &'static str {
    match bin {
        0 => VAR_DIV_NEUTRAL_INK,
        b if (1..=5).contains(&b) => VAR_DIV_POS_INK[(b - 1) as usize],
        b if (-5..=-1).contains(&b) => VAR_DIV_NEG_INK[(-b - 1) as usize],
        _ => VAR_DIV_NEUTRAL_INK,
    }
}

/// HTML `<style>` に埋め込む CSS custom property 宣言ブロックを生成する。
///
/// `:root { ... }`（ライト既定）と
/// `@media (prefers-color-scheme: dark) { :root { ... } }`（ダーク上書き）
/// の2ブロックからなる文字列を返す。HTML レンダラはこれをそのまま
/// `<style>` 内に埋め込むことを想定している（値はすべて本モジュールの
/// 定数由来であり、ここに直接 16 進値を書き足さないこと）。
pub fn css_variables() -> String {
    let mut light = String::new();
    let mut dark = String::new();

    let mut push = |light_val: &str, dark_val: &str, name: &str| {
        light.push_str(&format!("  {name}: {light_val};\n"));
        dark.push_str(&format!("  {name}: {dark_val};\n"));
    };

    push(SURFACE_LIGHT, SURFACE_DARK, VAR_SURFACE);
    push(INK_PRIMARY_LIGHT, INK_PRIMARY_DARK, VAR_INK_PRIMARY);
    push(INK_SECONDARY_LIGHT, INK_SECONDARY_DARK, VAR_INK_SECONDARY);
    push(INK_MUTED, INK_MUTED, VAR_INK_MUTED);
    push(GRID_LIGHT, GRID_DARK, VAR_GRID);
    push(AXIS_LIGHT, AXIS_DARK, VAR_AXIS);

    for i in 0..6 {
        push(SERIES_LIGHT[i], SERIES_DARK[i], VAR_SERIES[i]);
    }

    let seq_light = [
        SEQ_100, SEQ_200, SEQ_300, SEQ_400, SEQ_500, SEQ_600, SEQ_700,
    ];
    for (idx, (_, name)) in VAR_SEQ.iter().enumerate() {
        // シーケンシャルはテーマ非依存（設計書が単一パレットのみ規定）。
        push(seq_light[idx], seq_light[idx], name);
    }

    push(DIV_NEUTRAL_LIGHT, DIV_NEUTRAL_DARK, VAR_DIV_NEUTRAL);
    push(INK_PRIMARY_LIGHT, INK_PRIMARY_DARK, VAR_DIV_NEUTRAL_INK);

    let div_neg_light = [
        DIV_NEG_1_LIGHT,
        DIV_NEG_2_LIGHT,
        DIV_NEG_3_LIGHT,
        DIV_NEG_4_LIGHT,
        DIV_NEG_5_LIGHT,
    ];
    let div_neg_dark = [
        DIV_NEG_1_DARK,
        DIV_NEG_2_DARK,
        DIV_NEG_3_DARK,
        DIV_NEG_4_DARK,
        DIV_NEG_5_DARK,
    ];
    // ダークテーマの負側ラベルインク: 段階5(極)のみ黒、1〜4は白。
    let div_neg_ink_dark = [
        INK_PRIMARY_DARK,
        INK_PRIMARY_DARK,
        INK_PRIMARY_DARK,
        INK_PRIMARY_DARK,
        INK_PRIMARY_LIGHT,
    ];
    for i in 0..5 {
        push(div_neg_light[i], div_neg_dark[i], VAR_DIV_NEG[i]);
        // ライトテーマは全段階黒。
        push(INK_PRIMARY_LIGHT, div_neg_ink_dark[i], VAR_DIV_NEG_INK[i]);
    }

    let div_pos_light = [
        DIV_POS_1_LIGHT,
        DIV_POS_2_LIGHT,
        DIV_POS_3_LIGHT,
        DIV_POS_4_LIGHT,
        DIV_POS_5_LIGHT,
    ];
    let div_pos_dark = [
        DIV_POS_1_DARK,
        DIV_POS_2_DARK,
        DIV_POS_3_DARK,
        DIV_POS_4_DARK,
        DIV_POS_5_DARK,
    ];
    // ダークテーマの正側ラベルインク: 段階4・5(極付近)は黒、1〜3は白。
    let div_pos_ink_dark = [
        INK_PRIMARY_DARK,
        INK_PRIMARY_DARK,
        INK_PRIMARY_DARK,
        INK_PRIMARY_LIGHT,
        INK_PRIMARY_LIGHT,
    ];
    for i in 0..5 {
        push(div_pos_light[i], div_pos_dark[i], VAR_DIV_POS[i]);
        push(INK_PRIMARY_LIGHT, div_pos_ink_dark[i], VAR_DIV_POS_INK[i]);
    }

    format!(
        ":root {{\n{light}}}\n\n@media (prefers-color-scheme: dark) {{\n  :root {{\n{}  }}\n}}\n",
        dark.lines().map(|l| format!("  {l}\n")).collect::<String>()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------
    // OKLab 補間の参照実装（モジュール doc の手順をそのまま実装したもの）。
    // 上記の `pub const` 群がこの実装の出力と一致することを検証する
    // ことで、16進リテラルが手計算ミスなく導出されたことを担保する。
    // ------------------------------------------------------------

    fn srgb_to_linear(c: f64) -> f64 {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    fn linear_to_srgb(c: f64) -> f64 {
        if c <= 0.0031308 {
            c * 12.92
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        }
    }

    fn hex_to_rgb(h: &str) -> (f64, f64, f64) {
        let h = h.trim_start_matches('#');
        let r = u8::from_str_radix(&h[0..2], 16).unwrap() as f64;
        let g = u8::from_str_radix(&h[2..4], 16).unwrap() as f64;
        let b = u8::from_str_radix(&h[4..6], 16).unwrap() as f64;
        (r, g, b)
    }

    fn rgb_to_hex(r: f64, g: f64, b: f64) -> String {
        let clamp = |v: f64| -> u8 { v.round().clamp(0.0, 255.0) as u8 };
        format!("#{:02x}{:02x}{:02x}", clamp(r), clamp(g), clamp(b))
    }

    fn linear_srgb_to_oklab(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
        let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
        let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
        let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;
        let l_ = l.cbrt();
        let m_ = m.cbrt();
        let s_ = s.cbrt();
        (
            0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
            1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
            0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
        )
    }

    fn oklab_to_linear_srgb(l: f64, a: f64, b: f64) -> (f64, f64, f64) {
        let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
        let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
        let s_ = l - 0.0894841775 * a - 1.2914855480 * b;
        let l3 = l_ * l_ * l_;
        let m3 = m_ * m_ * m_;
        let s3 = s_ * s_ * s_;
        (
            4.0767416621 * l3 - 3.3077115913 * m3 + 0.2309699292 * s3,
            -1.2684380046 * l3 + 2.6097574011 * m3 - 0.3413193965 * s3,
            -0.0041960863 * l3 - 0.7034186147 * m3 + 1.7076147010 * s3,
        )
    }

    fn hex_to_oklab(h: &str) -> (f64, f64, f64) {
        let (r, g, b) = hex_to_rgb(h);
        linear_srgb_to_oklab(
            srgb_to_linear(r / 255.0),
            srgb_to_linear(g / 255.0),
            srgb_to_linear(b / 255.0),
        )
    }

    fn oklab_to_hex(lab: (f64, f64, f64)) -> String {
        let (r, g, b) = oklab_to_linear_srgb(lab.0, lab.1, lab.2);
        rgb_to_hex(
            linear_to_srgb(r.max(0.0)) * 255.0,
            linear_to_srgb(g.max(0.0)) * 255.0,
            linear_to_srgb(b.max(0.0)) * 255.0,
        )
    }

    fn lerp_oklab_hex(from: &str, to: &str, t: f64) -> String {
        let a = hex_to_oklab(from);
        let b = hex_to_oklab(to);
        oklab_to_hex((
            a.0 + (b.0 - a.0) * t,
            a.1 + (b.1 - a.1) * t,
            a.2 + (b.2 - a.2) * t,
        ))
    }

    fn relative_luminance(h: &str) -> f64 {
        let (r, g, b) = hex_to_rgb(h);
        0.2126 * srgb_to_linear(r / 255.0)
            + 0.7152 * srgb_to_linear(g / 255.0)
            + 0.0722 * srgb_to_linear(b / 255.0)
    }

    fn contrast(h1: &str, h2: &str) -> f64 {
        let l1 = relative_luminance(h1) + 0.05;
        let l2 = relative_luminance(h2) + 0.05;
        if l1 > l2 {
            l1 / l2
        } else {
            l2 / l1
        }
    }

    #[test]
    fn sequential_matches_oklab_derivation() {
        let expect = [
            SEQ_100, SEQ_200, SEQ_300, SEQ_400, SEQ_500, SEQ_600, SEQ_700,
        ];
        for (i, step) in [100, 200, 300, 400, 500, 600, 700].iter().enumerate() {
            let t = (step - 100) as f64 / 600.0;
            assert_eq!(
                lerp_oklab_hex(SEQ_100, SEQ_700, t),
                expect[i],
                "step {step}"
            );
        }
    }

    #[test]
    fn diverging_light_matches_oklab_derivation() {
        let neg = [
            DIV_NEG_1_LIGHT,
            DIV_NEG_2_LIGHT,
            DIV_NEG_3_LIGHT,
            DIV_NEG_4_LIGHT,
            DIV_NEG_5_LIGHT,
        ];
        let pos = [
            DIV_POS_1_LIGHT,
            DIV_POS_2_LIGHT,
            DIV_POS_3_LIGHT,
            DIV_POS_4_LIGHT,
            DIV_POS_5_LIGHT,
        ];
        for i in 1..=5 {
            let t = i as f64 / 5.0;
            assert_eq!(
                lerp_oklab_hex(DIV_NEUTRAL_LIGHT, SERIES_1_LIGHT, t),
                neg[i - 1]
            );
            assert_eq!(
                lerp_oklab_hex(DIV_NEUTRAL_LIGHT, SERIES_6_LIGHT, t),
                pos[i - 1]
            );
        }
    }

    #[test]
    fn diverging_dark_matches_oklab_derivation() {
        let neg = [
            DIV_NEG_1_DARK,
            DIV_NEG_2_DARK,
            DIV_NEG_3_DARK,
            DIV_NEG_4_DARK,
            DIV_NEG_5_DARK,
        ];
        let pos = [
            DIV_POS_1_DARK,
            DIV_POS_2_DARK,
            DIV_POS_3_DARK,
            DIV_POS_4_DARK,
            DIV_POS_5_DARK,
        ];
        for i in 1..=5 {
            let t = i as f64 / 5.0;
            assert_eq!(
                lerp_oklab_hex(DIV_NEUTRAL_DARK, SERIES_1_DARK, t),
                neg[i - 1]
            );
            assert_eq!(
                lerp_oklab_hex(DIV_NEUTRAL_DARK, SERIES_6_DARK, t),
                pos[i - 1]
            );
        }
    }

    #[test]
    fn diverging_label_ink_light_is_always_black() {
        // ライトテーマは全11段階で黒インクの方がコントラストが高い。
        let all = [
            DIV_NEUTRAL_LIGHT,
            DIV_NEG_1_LIGHT,
            DIV_NEG_2_LIGHT,
            DIV_NEG_3_LIGHT,
            DIV_NEG_4_LIGHT,
            DIV_NEG_5_LIGHT,
            DIV_POS_1_LIGHT,
            DIV_POS_2_LIGHT,
            DIV_POS_3_LIGHT,
            DIV_POS_4_LIGHT,
            DIV_POS_5_LIGHT,
        ];
        for c in all {
            assert!(contrast(c, INK_PRIMARY_LIGHT) >= contrast(c, INK_PRIMARY_DARK));
        }
    }

    #[test]
    fn diverging_label_ink_dark_extremes_are_black_middle_is_white() {
        assert!(
            contrast(DIV_NEG_5_DARK, INK_PRIMARY_LIGHT)
                >= contrast(DIV_NEG_5_DARK, INK_PRIMARY_DARK)
        );
        assert!(
            contrast(DIV_POS_5_DARK, INK_PRIMARY_LIGHT)
                >= contrast(DIV_POS_5_DARK, INK_PRIMARY_DARK)
        );
        assert!(
            contrast(DIV_POS_4_DARK, INK_PRIMARY_LIGHT)
                >= contrast(DIV_POS_4_DARK, INK_PRIMARY_DARK)
        );
        for c in [
            DIV_NEUTRAL_DARK,
            DIV_NEG_1_DARK,
            DIV_NEG_2_DARK,
            DIV_NEG_3_DARK,
            DIV_NEG_4_DARK,
            DIV_POS_1_DARK,
            DIV_POS_2_DARK,
            DIV_POS_3_DARK,
        ] {
            assert!(contrast(c, INK_PRIMARY_DARK) >= contrast(c, INK_PRIMARY_LIGHT));
        }
    }

    #[test]
    fn diverging_bin_boundaries() {
        assert_eq!(diverging_bin(-1.0), -5);
        assert_eq!(diverging_bin(-0.51), -3);
        assert_eq!(diverging_bin(-0.5), -3);
        assert_eq!(diverging_bin(0.0), 0);
        assert_eq!(diverging_bin(0.5), 3);
        assert_eq!(diverging_bin(0.51), 3);
        assert_eq!(diverging_bin(1.0), 5);
    }

    #[test]
    fn diverging_show_label_threshold() {
        assert!(!diverging_show_label(0.5));
        assert!(diverging_show_label(0.51));
        assert!(!diverging_show_label(-0.5));
        assert!(diverging_show_label(-0.51));
        assert!(!diverging_show_label(0.0));
    }

    #[test]
    fn diverging_var_and_ink_var_cover_all_bins() {
        for b in -5..=5 {
            let var = diverging_var(b);
            let ink = diverging_ink_var(b);
            assert!(var.starts_with("--div-"));
            assert!(ink.ends_with("-ink") || ink == VAR_DIV_NEUTRAL_INK);
        }
    }

    #[test]
    fn css_variables_contains_all_tokens() {
        let css = css_variables();
        assert!(css.contains(":root"));
        assert!(css.contains("@media (prefers-color-scheme: dark)"));
        assert!(css.contains(VAR_SURFACE));
        for name in VAR_SERIES {
            assert!(css.contains(name), "missing {name}");
        }
        for (_, name) in VAR_SEQ {
            assert!(css.contains(name), "missing {name}");
        }
        for name in VAR_DIV_NEG.iter().chain(VAR_DIV_POS.iter()) {
            assert!(css.contains(name), "missing {name}");
        }
        assert!(css.contains(SURFACE_LIGHT));
        assert!(css.contains(SURFACE_DARK));
    }
}
