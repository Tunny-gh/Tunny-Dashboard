//! HTML / SVG report shared color palette (verified tokens).
//!
//! The values are sourced from the color tokens listed in the "HTML
//! Renderer" section of the design document "Short-term 2: Self-contained
//! report output", and this module fixes them as `pub const &str`. To
//! achieve light/dark support without JS, the design references values not
//! directly but through CSS custom properties (in the form `--foo`) from
//! SVG / HTML, and [`css_variables`] generates that declaration block
//! (`:root` + `@media (prefers-color-scheme: dark)`) as a string.
//!
//! # How the derived palettes (sequential / diverging) are computed
//!
//! The design document specifies only the two ends of the sequential scale
//! (100 and 700), the two poles of the diverging scale (blue / red), and
//! the neutral color; the intermediate steps are interpolated and defined
//! in this module. Naively interpolating sRGB byte values linearly makes
//! intermediate colors of differing hue look muddy (desaturated), so
//! interpolation is performed in [OKLab](https://bottosson.github.io/posts/oklab/)
//! color space (designed to be perceptually uniform) and converted back to
//! sRGB. The concrete procedure is as follows (a reference implementation
//! of the same procedure lives in `#[cfg(test)]`, and tests assert that the
//! constant values below match that implementation's output).
//!
//! 1. Convert the sRGB hex values at both ends to linear RGB, then to OKLab.
//! 2. Linearly interpolate `L`, `a`, and `b` independently in OKLab space.
//! 3. Convert back OKLab -> linear RGB -> sRGB, round to 8 bits, and format
//!    as a hex string.
//!
//! The sequential scale (`SEQ_100..=SEQ_700`) has 7 steps at increments of
//! 100 (`t = (step - 100) / 600`); each arm of the diverging scale
//! (`DIV_NEG_*` / `DIV_POS_*`) has 5 steps with `t = 0` at the neutral color
//! and `t = 1` at the pole color, using `t = i / 5` (`i = 1..=5`)
//! (corresponding to the design document's "five steps x two arms").
//!
//! The ink color (black / white) used for direct labels on the diverging
//! scale is precomputed from the relative luminance (WCAG contrast ratio)
//! of each step, and [`diverging_ink_var`] returns the appropriate CSS
//! variable name for each step.

// ============================================================
// Surface
// ============================================================

/// Light-theme surface color.
pub const SURFACE_LIGHT: &str = "#fcfcfb";
/// Dark-theme surface color.
pub const SURFACE_DARK: &str = "#1a1a19";

// ============================================================
// Ink (text)
// ============================================================

/// Light-theme primary ink color (body text, headings).
pub const INK_PRIMARY_LIGHT: &str = "#0b0b0b";
/// Dark-theme primary ink color.
pub const INK_PRIMARY_DARK: &str = "#ffffff";
/// Light-theme secondary ink color (captions, etc.).
pub const INK_SECONDARY_LIGHT: &str = "#52514e";
/// Dark-theme secondary ink color.
pub const INK_SECONDARY_DARK: &str = "#c3c2b7";
/// Muted ink color (for axis labels and auxiliary text).
///
/// The design document specifies a single value shared by both themes
/// (this gray retains sufficient contrast against the surface color of
/// both themes, so it has no per-theme variant).
pub const INK_MUTED: &str = "#898781";

// ============================================================
// Grid / Axis
// ============================================================

/// Light-theme grid line color (hairline).
pub const GRID_LIGHT: &str = "#e1e0d9";
/// Dark-theme grid line color.
pub const GRID_DARK: &str = "#2c2c2a";
/// Light-theme axis line color.
pub const AXIS_LIGHT: &str = "#c3c2b7";
/// Dark-theme axis line color.
pub const AXIS_DARK: &str = "#383835";

// ============================================================
// Categorical series (1-6, fixed order, no cycling)
// ============================================================

/// Series 1 (blue), light.
pub const SERIES_1_LIGHT: &str = "#2a78d6";
/// Series 1 (blue), dark.
pub const SERIES_1_DARK: &str = "#3987e5";
/// Series 2 (aqua), light.
pub const SERIES_2_LIGHT: &str = "#1baf7a";
/// Series 2 (aqua), dark.
pub const SERIES_2_DARK: &str = "#199e70";
/// Series 3 (yellow), light.
pub const SERIES_3_LIGHT: &str = "#eda100";
/// Series 3 (yellow), dark.
pub const SERIES_3_DARK: &str = "#c98500";
/// Series 4 (green), light.
///
/// The design document specifies only a single value; the dark theme uses
/// the same value.
pub const SERIES_4_LIGHT: &str = "#008300";
/// Series 4 (green), dark (same value as light).
pub const SERIES_4_DARK: &str = "#008300";
/// Series 5 (violet), light.
pub const SERIES_5_LIGHT: &str = "#4a3aa7";
/// Series 5 (violet), dark.
pub const SERIES_5_DARK: &str = "#9085e9";
/// Series 6 (red), light.
pub const SERIES_6_LIGHT: &str = "#e34948";
/// Series 6 (red), dark.
pub const SERIES_6_DARK: &str = "#e66767";

/// Categorical series colors (light). Index 0 corresponds to series 1.
/// Cycling is not allowed (no design should require a 7th series or beyond).
pub const SERIES_LIGHT: [&str; 6] = [
    SERIES_1_LIGHT,
    SERIES_2_LIGHT,
    SERIES_3_LIGHT,
    SERIES_4_LIGHT,
    SERIES_5_LIGHT,
    SERIES_6_LIGHT,
];
/// Categorical series colors (dark).
pub const SERIES_DARK: [&str; 6] = [
    SERIES_1_DARK,
    SERIES_2_DARK,
    SERIES_3_DARK,
    SERIES_4_DARK,
    SERIES_5_DARK,
    SERIES_6_DARK,
];

// ============================================================
// Sequential (monotonic quantities such as histograms; single
// theme-independent palette)
// ============================================================

/// Sequential 100 (lightest). The design document's end value verbatim.
pub const SEQ_100: &str = "#cde2fb";
/// Sequential 200 (OKLab interpolation, `t=1/6`).
pub const SEQ_200: &str = "#abc3e2";
/// Sequential 300 (OKLab interpolation, `t=2/6`).
pub const SEQ_300: &str = "#8aa6ca";
/// Sequential 400 (OKLab interpolation, `t=3/6`). Default histogram color.
pub const SEQ_400: &str = "#6a89b2";
/// Sequential 500 (OKLab interpolation, `t=4/6`).
pub const SEQ_500: &str = "#4b6c9a";
/// Sequential 600 (OKLab interpolation, `t=5/6`).
pub const SEQ_600: &str = "#2d5182";
/// Sequential 700 (darkest). The design document's end value verbatim.
pub const SEQ_700: &str = "#0d366b";

// ============================================================
// Diverging (correlation heatmap; [-1,1] quantized into 5+5+neutral)
// ============================================================

/// Diverging neutral color (light, `v=0`).
pub const DIV_NEUTRAL_LIGHT: &str = "#f0efec";
/// Diverging neutral color (dark, `v=0`).
pub const DIV_NEUTRAL_DARK: &str = "#383835";

/// Negative side (blue direction) step 1 (closest to neutral), light.
pub const DIV_NEG_1_LIGHT: &str = "#c9d8ea";
/// Negative side step 2, light.
pub const DIV_NEG_2_LIGHT: &str = "#a3c1e7";
/// Negative side step 3, light.
pub const DIV_NEG_3_LIGHT: &str = "#7da9e2";
/// Negative side step 4, light.
pub const DIV_NEG_4_LIGHT: &str = "#5691dd";
/// Negative side step 5 (`v=-1`, the blue pole itself), light.
pub const DIV_NEG_5_LIGHT: &str = "#2a78d6";

/// Positive side (red direction) step 1 (closest to neutral), light.
pub const DIV_POS_1_LIGHT: &str = "#f3d0cb";
/// Positive side step 2, light.
pub const DIV_POS_2_LIGHT: &str = "#f2b1aa";
/// Positive side step 3, light.
pub const DIV_POS_3_LIGHT: &str = "#f09289";
/// Positive side step 4, light.
pub const DIV_POS_4_LIGHT: &str = "#ea7069";
/// Positive side step 5 (`v=1`, the red pole itself), light.
pub const DIV_POS_5_LIGHT: &str = "#e34948";

/// Negative side step 1, dark.
pub const DIV_NEG_1_DARK: &str = "#3b4856";
/// Negative side step 2, dark.
pub const DIV_NEG_2_DARK: &str = "#3d5878";
/// Negative side step 3, dark.
pub const DIV_NEG_3_DARK: &str = "#3e689b";
/// Negative side step 4, dark.
pub const DIV_NEG_4_DARK: &str = "#3c77bf";
/// Negative side step 5 (`v=-1`), dark.
pub const DIV_NEG_5_DARK: &str = "#3987e5";

/// Positive side step 1, dark.
pub const DIV_POS_1_DARK: &str = "#5a433f";
/// Positive side step 2, dark.
pub const DIV_POS_2_DARK: &str = "#7b4d49";
/// Positive side step 3, dark.
pub const DIV_POS_3_DARK: &str = "#9e5653";
/// Positive side step 4, dark.
pub const DIV_POS_4_DARK: &str = "#c15f5d";
/// Positive side step 5 (`v=1`), dark.
pub const DIV_POS_5_DARK: &str = "#e66767";

// ============================================================
// CSS custom property names (the single definition shared by svg.rs / the
// HTML renderer)
// ============================================================

/// `--surface` variable name.
pub const VAR_SURFACE: &str = "--surface";
/// `--ink-primary` variable name.
pub const VAR_INK_PRIMARY: &str = "--ink-primary";
/// `--ink-secondary` variable name.
pub const VAR_INK_SECONDARY: &str = "--ink-secondary";
/// `--ink-muted` variable name.
pub const VAR_INK_MUTED: &str = "--ink-muted";
/// `--grid` variable name.
pub const VAR_GRID: &str = "--grid";
/// `--axis` variable name.
pub const VAR_AXIS: &str = "--axis";

/// Categorical series variable names (index 0 is series 1).
pub const VAR_SERIES: [&str; 6] = [
    "--series-1",
    "--series-2",
    "--series-3",
    "--series-4",
    "--series-5",
    "--series-6",
];

/// Mapping between sequential steps and variable names (`(step, name)`).
pub const VAR_SEQ: [(u16, &str); 7] = [
    (100, "--seq-100"),
    (200, "--seq-200"),
    (300, "--seq-300"),
    (400, "--seq-400"),
    (500, "--seq-500"),
    (600, "--seq-600"),
    (700, "--seq-700"),
];

/// Diverging neutral color variable name.
pub const VAR_DIV_NEUTRAL: &str = "--div-neutral";
/// Diverging neutral color direct-label ink variable name.
pub const VAR_DIV_NEUTRAL_INK: &str = "--div-neutral-ink";
/// Diverging negative side (blue direction) variable names (index 0 is step 1).
pub const VAR_DIV_NEG: [&str; 5] = [
    "--div-neg-1",
    "--div-neg-2",
    "--div-neg-3",
    "--div-neg-4",
    "--div-neg-5",
];
/// Diverging negative side direct-label ink variable names.
pub const VAR_DIV_NEG_INK: [&str; 5] = [
    "--div-neg-1-ink",
    "--div-neg-2-ink",
    "--div-neg-3-ink",
    "--div-neg-4-ink",
    "--div-neg-5-ink",
];
/// Diverging positive side (red direction) variable names.
pub const VAR_DIV_POS: [&str; 5] = [
    "--div-pos-1",
    "--div-pos-2",
    "--div-pos-3",
    "--div-pos-4",
    "--div-pos-5",
];
/// Diverging positive side direct-label ink variable names.
pub const VAR_DIV_POS_INK: [&str; 5] = [
    "--div-pos-1-ink",
    "--div-pos-2-ink",
    "--div-pos-3-ink",
    "--div-pos-4-ink",
    "--div-pos-5-ink",
];

/// Rounds a correlation-heatmap value into a quantized `-5..=5` step
/// (`0` is neutral, `-5`/`5` are the poles).
///
/// Quantization rule: clamp `mag = ceil(min(|v|, 1) * 5)` to `1..=5` and
/// apply the sign of `v`. `v == 0.0` returns neutral (`0`). The boundaries
/// are `(0.0, 0.2] -> 1, (0.2, 0.4] -> 2, ..., (0.8, 1.0] -> 5`, so for
/// example `v = 0.5` and `v = 0.51` quantize to the same step (3) (whether
/// to show the numeric label is decided separately by
/// [`diverging_show_label`]'s `|v| > 0.5` check, so the quantization itself
/// does not switch exactly at 0.5).
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

/// Shows a numeric direct label only for cells with `|v| > 0.5` (design
/// document rule).
pub fn diverging_show_label(v: f64) -> bool {
    v.abs() > 0.5
}

/// Returns the CSS variable name for the fill color corresponding to a
/// quantized step (the return value of [`diverging_bin`], `-5..=5`).
pub fn diverging_var(bin: i32) -> &'static str {
    match bin {
        0 => VAR_DIV_NEUTRAL,
        b if (1..=5).contains(&b) => VAR_DIV_POS[(b - 1) as usize],
        b if (-5..=-1).contains(&b) => VAR_DIV_NEG[(-b - 1) as usize],
        _ => VAR_DIV_NEUTRAL,
    }
}

/// Returns the CSS variable name for the direct-label ink color
/// corresponding to a quantized step.
///
/// The ink color itself was chosen ahead of time based on the precomputed
/// WCAG contrast ratio against each step's cell color (all steps use black
/// in the light theme; in the dark theme only the two poles (`-5`/`5`) use
/// black and the rest use white). The returned value is not
/// `--ink-primary` (i.e. [`VAR_INK_PRIMARY`]) but a dedicated variable
/// fixed to either black or white depending on the theme (defined by
/// `css_variables`).
pub fn diverging_ink_var(bin: i32) -> &'static str {
    match bin {
        0 => VAR_DIV_NEUTRAL_INK,
        b if (1..=5).contains(&b) => VAR_DIV_POS_INK[(b - 1) as usize],
        b if (-5..=-1).contains(&b) => VAR_DIV_NEG_INK[(-b - 1) as usize],
        _ => VAR_DIV_NEUTRAL_INK,
    }
}

/// Generates the CSS custom property declaration block to embed in the
/// HTML `<style>` tag.
///
/// Returns a string consisting of two blocks: `:root { ... }` (light
/// default) and `@media (prefers-color-scheme: dark) { :root { ... } }`
/// (dark override). The HTML renderer is expected to embed this directly
/// inside `<style>` (all values originate from this module's constants;
/// do not add hex values directly here).
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
        // Sequential is theme-independent (the design document specifies
        // only a single palette).
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
    // Dark-theme negative-side label ink: black only at step 5 (pole),
    // white for steps 1-4.
    let div_neg_ink_dark = [
        INK_PRIMARY_DARK,
        INK_PRIMARY_DARK,
        INK_PRIMARY_DARK,
        INK_PRIMARY_DARK,
        INK_PRIMARY_LIGHT,
    ];
    for i in 0..5 {
        push(div_neg_light[i], div_neg_dark[i], VAR_DIV_NEG[i]);
        // Light theme uses black at every step.
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
    // Dark-theme positive-side label ink: black at steps 4-5 (near the
    // pole), white for steps 1-3.
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
    // Reference implementation of the OKLab interpolation (a direct
    // implementation of the procedure described in the module doc comment).
    // Verifying that the `pub const` values above match this
    // implementation's output ensures the hex literals were derived
    // without manual calculation errors.
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
        // In the light theme, black ink gives higher contrast across all
        // 11 steps.
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
