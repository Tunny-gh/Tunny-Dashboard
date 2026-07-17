//! Demo harness for visually reviewing `report::svg` / `report::theme`.
//!
//! Generates all 5 chart kinds from plausible-looking synthetic data (a 60-trial
//! convergence curve, a 2-objective Pareto point cloud, 8-parameter importance,
//! a 20-bin histogram, an 8x2 correlation heatmap), then writes them out as a
//! self-contained HTML file with the color tokens from
//! [`tunny_core::report::theme::css_variables`] embedded.
//! Meant to be opened in a browser to visually confirm the look of both light and
//! dark themes (contains no analysis logic whatsoever — just checks `report::svg`'s
//! rendering output).
//!
//! Run: `cargo run -p tunny-core --example report_svg_demo -- [output_path]`
//! (defaults to `/tmp/report_svg_demo.html` if output_path is omitted)

use tunny_core::report::svg::{self, HBarItem, HistBin, LinePoint, ScatterPoint};
use tunny_core::report::theme;

/// Deterministic pseudo-random generator (xorshift64*). Used for reproducibility of demo data.
struct Rng(u64);
impl Rng {
    fn next_f64(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let v = self.0.wrapping_mul(0x2545_F491_4F6C_DD1D);
        (v >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn convergence_data() -> (Vec<LinePoint>, Vec<usize>) {
    let mut rng = Rng(0x5EED_2234_ABCD_0001);
    // Build a raw trial-value sequence (not best-so-far) that mimics a typical
    // convergence curve: large variance early in the search, with improvements
    // becoming less likely later on.
    let raw: Vec<f64> = (0..60)
        .map(|i| {
            let trend = 5.0 * (-(i as f64) / 18.0).exp() + 0.3;
            let noise = (rng.next_f64() - 0.5) * 2.0;
            (trend + noise).max(0.05)
        })
        .collect();

    let mut best = f64::INFINITY;
    let mut points = Vec::with_capacity(raw.len());
    let mut improvements = Vec::new();
    for (i, &v) in raw.iter().enumerate() {
        if v < best {
            best = v;
            improvements.push(points.len());
        }
        points.push(LinePoint {
            trial_number: i as i64,
            value: best,
        });
    }
    (points, improvements)
}

fn pareto_data() -> (Vec<ScatterPoint>, Vec<ScatterPoint>) {
    let mut rng = Rng(0x5EED_2234_ABCD_0002);
    let mut background = Vec::with_capacity(60);
    for i in 0..60 {
        let x = rng.next_f64() * 10.0;
        let y = rng.next_f64() * 10.0;
        background.push(ScatterPoint {
            trial_number: i,
            x,
            y,
            feasible: true,
        });
    }
    // Non-dominated front: 12 monotonic points where larger x means smaller y.
    let mut front = Vec::with_capacity(12);
    for i in 0..12 {
        let t = i as f64 / 11.0;
        front.push(ScatterPoint {
            trial_number: 1000 + i,
            x: t * 9.5 + 0.2,
            y: (1.0 - t).powf(1.3) * 9.5 + 0.1,
            feasible: true,
        });
    }
    (background, front)
}

fn importance_data() -> Vec<HBarItem> {
    vec![
        HBarItem {
            label: "learning_rate".to_string(),
            value: 0.82,
        },
        HBarItem {
            label: "num_leaves_for_gradient_boosting_model".to_string(),
            value: 0.61,
        },
        HBarItem {
            label: "max_depth".to_string(),
            value: 0.45,
        },
        HBarItem {
            label: "subsample".to_string(),
            value: 0.33,
        },
        HBarItem {
            label: "colsample_bytree".to_string(),
            value: 0.28,
        },
        HBarItem {
            label: "reg_alpha".to_string(),
            value: 0.15,
        },
        HBarItem {
            label: "reg_lambda".to_string(),
            value: 0.09,
        },
        HBarItem {
            label: "min_child_weight".to_string(),
            value: 0.04,
        },
    ]
}

fn histogram_data() -> Vec<HistBin> {
    let mut rng = Rng(0x5EED_2234_ABCD_0003);
    let mut counts = [0u64; 20];
    for _ in 0..400 {
        // Build a peak at the center using a simple normal approximation (an
        // Irwin-Hall-like sum).
        let v: f64 = (0..6).map(|_| rng.next_f64()).sum::<f64>() / 6.0;
        let bin = ((v * 20.0) as usize).min(19);
        counts[bin] += 1;
    }
    (0..20)
        .map(|i| HistBin {
            lower: i as f64 / 20.0,
            upper: (i + 1) as f64 / 20.0,
            count: counts[i],
        })
        .collect()
}

fn heatmap_data() -> (Vec<Vec<f64>>, Vec<String>, Vec<String>) {
    // Spearman correlation for 8 parameters x 2 objectives. Deliberately mixes in
    // boundary values (-1, -0.51, -0.5, 0, 0.5, 0.51, 1) so the look of quantization
    // can be verified.
    let matrix = vec![
        vec![1.0, -1.0],
        vec![0.51, -0.51],
        vec![0.5, -0.5],
        vec![0.0, 0.2],
        vec![-0.3, 0.63],
        vec![0.77, -0.12],
        vec![-0.9, 0.4],
        vec![0.05, -0.05],
    ];
    let row_labels = vec![
        "learning_rate".to_string(),
        "num_leaves".to_string(),
        "max_depth".to_string(),
        "subsample".to_string(),
        "colsample_bytree".to_string(),
        "reg_alpha".to_string(),
        "reg_lambda".to_string(),
        "min_child_weight".to_string(),
    ];
    let col_labels = vec!["accuracy".to_string(), "latency_ms".to_string()];
    (matrix, row_labels, col_labels)
}

fn main() {
    let out_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/report_svg_demo.html".to_string());

    let (conv_points, conv_improvements) = convergence_data();
    let (background, front) = pareto_data();
    let importance = importance_data();
    let hist_bins = histogram_data();
    let (heat_matrix, heat_rows, heat_cols) = heatmap_data();

    let line_svg = svg::line_chart(&conv_points, &conv_improvements, 640.0, 240.0);
    let scatter_svg = svg::scatter_chart(
        &background,
        &front,
        "objective 1",
        "objective 2",
        640.0,
        400.0,
    );
    let hbar_svg = svg::hbar_chart(&importance, 640.0);
    let hist_svg = svg::histogram(&hist_bins, 640.0, 220.0);
    let heatmap_svg = svg::heatmap(&heat_matrix, &heat_rows, &heat_cols, 640.0);

    let css_vars = theme::css_variables();

    let html = format!(
        r#"<!doctype html>
<html lang="ja">
<head>
<meta charset="utf-8" />
<title>report::svg デモ</title>
<style>
{css_vars}
body {{
  background: var(--surface);
  color: var(--ink-primary);
  font-family: system-ui, -apple-system, "Segoe UI", sans-serif;
  max-width: 720px;
  margin: 0 auto;
  padding: 24px;
}}
h2 {{
  color: var(--ink-primary);
  border-bottom: 1px solid var(--grid);
  padding-bottom: 4px;
  margin-top: 40px;
}}
figure {{
  margin: 0;
}}
</style>
</head>
<body>
<h1>report::svg チャートデモ</h1>
<p style="color: var(--ink-secondary)">
OS のライト/ダーク設定を切り替えて両テーマの見た目を確認してください
（JS 不使用・外部リソース参照ゼロの自己完結ページ）。
</p>

<h2>line_chart（収束カーブ、60 試行）</h2>
<figure>{line_svg}</figure>

<h2>scatter_chart（2目的 Pareto 点群、60点中12点フロント）</h2>
<figure>{scatter_svg}</figure>

<h2>hbar_chart（パラメータ重要度、8項目・長い名前を含む）</h2>
<figure>{hbar_svg}</figure>

<h2>histogram（目的値分布、20ビン）</h2>
<figure>{hist_svg}</figure>

<h2>heatmap（相関ヒートマップ、8パラメータ×2目的）</h2>
<figure>{heatmap_svg}</figure>

</body>
</html>
"#
    );

    std::fs::write(&out_path, html).expect("failed to write demo HTML");
    println!("wrote {out_path}");
}
