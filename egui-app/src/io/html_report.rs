//! REQ-005: HTML レポート生成（スタンドアロン・外部リソース参照なし）
//!
//! HTML インジェクション防止: ユーザーデータは必ず `escape_html()` を通す

use std::collections::HashMap;

// ============================================================
// 型定義
// ============================================================

#[derive(Debug, Clone)]
pub struct HtmlReportSnapshot {
    pub study_name: String,
    pub objective_names: Vec<String>,
    pub param_names: Vec<String>,
    pub total_trials: usize,
    pub pareto_count: usize,
    pub selected_trials: Vec<HtmlTrialRow>,
    pub statistics: TrialStatistics,
}

#[derive(Debug, Clone)]
pub struct HtmlTrialRow {
    pub trial_id: u32,
    pub trial_number: u32,
    pub params: HashMap<String, f64>,
    pub objectives: Vec<f64>,
    pub pareto_rank: u32,
}

#[derive(Debug, Clone)]
pub struct TrialStatistics {
    pub objective_means: Vec<f64>,
    pub objective_variances: Vec<f64>,
    pub pareto_count: usize,
}

// ============================================================
// HTML エスケープ（インジェクション防止）
// ============================================================

/// ユーザーデータを HTML に埋め込む前に < > & " をエスケープする
pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

// ============================================================
// sanitize_filename
// ============================================================

/// ファイル名として安全な文字列に変換する（パス区切り文字等を除去）
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .collect()
}

// ============================================================
// SVG チャート（散布図）
// ============================================================

/// 選択試行の目的関数散布図 SVG を生成する（obj[0] vs obj[1]）
pub fn render_scatter_svg(snapshot: &HtmlReportSnapshot) -> String {
    const WIDTH: f64 = 400.0;
    const HEIGHT: f64 = 300.0;
    const MARGIN: f64 = 40.0;

    if snapshot.objective_names.len() < 2 || snapshot.selected_trials.is_empty() {
        return format!(
            r#"<svg width="{WIDTH}" height="{HEIGHT}"><text x="10" y="20" font-size="12">No data</text></svg>"#
        );
    }

    let xs: Vec<f64> = snapshot
        .selected_trials
        .iter()
        .filter_map(|t| t.objectives.first().copied())
        .collect();
    let ys: Vec<f64> = snapshot
        .selected_trials
        .iter()
        .filter_map(|t| t.objectives.get(1).copied())
        .collect();

    if xs.is_empty() {
        return format!(r#"<svg width="{WIDTH}" height="{HEIGHT}"></svg>"#);
    }

    let x_min = xs.iter().cloned().fold(f64::INFINITY, f64::min);
    let x_max = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let y_min = ys.iter().cloned().fold(f64::INFINITY, f64::min);
    let y_max = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let x_range = (x_max - x_min).max(1e-10);
    let y_range = (y_max - y_min).max(1e-10);

    let plot_w = WIDTH - 2.0 * MARGIN;
    let plot_h = HEIGHT - 2.0 * MARGIN;

    let map_x = |v: f64| MARGIN + (v - x_min) / x_range * plot_w;
    let map_y = |v: f64| MARGIN + plot_h - (v - y_min) / y_range * plot_h;

    let mut svg =
        format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{HEIGHT}">"#);
    svg.push_str(&format!(
        "<rect x=\"{MARGIN}\" y=\"{MARGIN}\" width=\"{plot_w}\" height=\"{plot_h}\" fill=\"white\" stroke=\"#ccc\"/>"
    ));

    for (row, (&x, &y)) in snapshot
        .selected_trials
        .iter()
        .zip(xs.iter().zip(ys.iter()))
    {
        let color = if row.pareto_rank == 0 {
            "#e74c3c"
        } else {
            "#3498db"
        };
        svg.push_str(&format!(
            r#"<circle cx="{:.1}" cy="{:.1}" r="4" fill="{color}" opacity="0.8"/>"#,
            map_x(x),
            map_y(y)
        ));
    }

    // 軸ラベル（エスケープ済み）
    let x_label = escape_html(
        snapshot
            .objective_names
            .first()
            .map(|s| s.as_str())
            .unwrap_or(""),
    );
    let y_label = escape_html(
        snapshot
            .objective_names
            .get(1)
            .map(|s| s.as_str())
            .unwrap_or(""),
    );
    svg.push_str(&format!(
        r#"<text x="{}" y="{}" text-anchor="middle" font-size="11">{x_label}</text>"#,
        WIDTH / 2.0,
        HEIGHT - 5.0
    ));
    svg.push_str(&format!(
        r#"<text x="12" y="{}" text-anchor="middle" font-size="11" transform="rotate(-90 12 {})">{y_label}</text>"#,
        HEIGHT / 2.0,
        HEIGHT / 2.0
    ));

    svg.push_str("</svg>");
    svg
}

// ============================================================
// HTML セクションビルダー
// ============================================================

fn write_study_summary(html: &mut String, snap: &HtmlReportSnapshot) {
    html.push_str("<h1>Tunny Dashboard Report</h1>\n");
    html.push_str("<div class=\"summary\">\n");

    let study = escape_html(&snap.study_name);
    html.push_str(&format!(
        "<div class=\"card\"><strong>Study</strong><br>{study}</div>\n"
    ));
    html.push_str(&format!(
        "<div class=\"card\"><strong>Total Trials</strong><br>{}</div>\n",
        snap.total_trials
    ));
    html.push_str(&format!(
        "<div class=\"card\"><strong>Pareto Optimal</strong><br>{}</div>\n",
        snap.pareto_count
    ));
    html.push_str(&format!(
        "<div class=\"card\"><strong>Selected</strong><br>{}</div>\n",
        snap.selected_trials.len()
    ));
    html.push_str("</div>\n");
}

fn write_scatter_svg_section(html: &mut String, snap: &HtmlReportSnapshot) {
    if snap.objective_names.len() >= 2 {
        html.push_str("<h2>Objective Space</h2>\n");
        html.push_str(&render_scatter_svg(snap));
        html.push('\n');
    }
}

fn write_trial_table(html: &mut String, snap: &HtmlReportSnapshot) {
    html.push_str("<h2>Selected Trials</h2>\n");
    html.push_str("<table>\n<thead><tr>");
    html.push_str("<th>Trial</th>");
    for name in &snap.objective_names {
        html.push_str(&format!("<th>{}</th>", escape_html(name)));
    }
    for name in &snap.param_names {
        html.push_str(&format!("<th>{}</th>", escape_html(name)));
    }
    html.push_str("<th>Pareto Rank</th>");
    html.push_str("</tr></thead>\n<tbody>\n");

    for row in &snap.selected_trials {
        html.push_str("<tr>");
        html.push_str(&format!("<td>{}</td>", row.trial_number));
        for &v in &row.objectives {
            html.push_str(&format!("<td>{:.6}</td>", v));
        }
        for name in &snap.param_names {
            let v = row.params.get(name).copied().unwrap_or(f64::NAN);
            html.push_str(&format!("<td>{:.6}</td>", v));
        }
        html.push_str(&format!("<td>{}</td>", row.pareto_rank));
        html.push_str("</tr>\n");
    }

    html.push_str("</tbody>\n</table>\n");
}

fn write_statistics(html: &mut String, snap: &HtmlReportSnapshot) {
    html.push_str("<h2>Statistics</h2>\n");
    html.push_str("<table>\n<thead><tr><th>Objective</th><th>Mean</th><th>Variance</th></tr></thead>\n<tbody>\n");
    for ((name, mean), var) in snap
        .objective_names
        .iter()
        .zip(&snap.statistics.objective_means)
        .zip(&snap.statistics.objective_variances)
    {
        html.push_str(&format!(
            "<tr><td>{}</td><td>{:.6}</td><td>{:.6}</td></tr>\n",
            escape_html(name),
            mean,
            var
        ));
    }
    html.push_str("</tbody>\n</table>\n");
}

// ============================================================
// build_html_report
// ============================================================

/// スタンドアロン HTML レポートを生成する（外部リソース参照なし）
pub fn build_html_report(snapshot: &HtmlReportSnapshot) -> String {
    let mut html = String::with_capacity(64 * 1024);

    html.push_str(
        r#"<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="UTF-8">
<title>Tunny Dashboard Report</title>
<style>
body { font-family: sans-serif; margin: 20px; }
table { border-collapse: collapse; width: 100%; }
th, td { border: 1px solid #ccc; padding: 6px 8px; text-align: right; }
th { background: #f0f0f0; }
h1, h2 { color: #333; }
.summary { display: flex; gap: 20px; margin-bottom: 20px; }
.card { border: 1px solid #ddd; border-radius: 4px; padding: 12px; min-width: 120px; }
</style>
</head>
<body>
"#,
    );

    write_study_summary(&mut html, snapshot);
    write_scatter_svg_section(&mut html, snapshot);
    write_trial_table(&mut html, snapshot);
    write_statistics(&mut html, snapshot);

    html.push_str("</body>\n</html>\n");
    html
}

// ============================================================
// build_and_send_report — StudyContext から直接レポートを送信
// ============================================================

pub fn build_and_send_report(
    ctx: &crate::state::types::StudyContext,
    selected_indices: &[u32],
    tx: std::sync::mpsc::SyncSender<crate::state::messages::AppMessage>,
) {
    let trial_map: std::collections::HashMap<u32, &crate::state::types::TrialRow> =
        ctx.trial_rows.iter().map(|r| (r.trial_id, r)).collect();
    let snap = HtmlReportSnapshot {
        study_name: ctx.meta.name.clone(),
        objective_names: ctx.meta.objective_names.clone(),
        param_names: ctx.meta.param_names.clone(),
        total_trials: ctx.trial_rows.len(),
        pareto_count: ctx.pareto_indices.len(),
        selected_trials: selected_indices
            .iter()
            .filter_map(|&id| trial_map.get(&id).copied())
            .map(|r| HtmlTrialRow {
                trial_id: r.trial_id,
                trial_number: r.trial_number,
                params: r.params.clone(),
                objectives: r.objectives.clone(),
                pareto_rank: r.pareto_rank,
            })
            .collect(),
        statistics: TrialStatistics {
            objective_means: vec![0.0; ctx.meta.objective_names.len()],
            objective_variances: vec![0.0; ctx.meta.objective_names.len()],
            pareto_count: ctx.pareto_indices.len(),
        },
    };
    generate_html_report_async(snap, tx);
}

// ============================================================
// generate_html_report_async — バックグラウンド生成
// ============================================================

/// HTML レポートを非同期で生成し、`HtmlReportDone` を送信する
pub fn generate_html_report_async(
    snapshot: HtmlReportSnapshot,
    tx: std::sync::mpsc::SyncSender<crate::state::messages::AppMessage>,
) {
    let filename = format!("{}.html", sanitize_filename(&snapshot.study_name));
    crate::app::spawn_task(tx, move || {
        let html = build_html_report(&snapshot);
        crate::state::messages::AppMessage::HtmlReportDone {
            html,
            suggested_filename: filename,
        }
    });
}

// ============================================================
// テスト
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snapshot() -> HtmlReportSnapshot {
        HtmlReportSnapshot {
            study_name: "test_study".to_string(),
            objective_names: vec!["obj1".to_string(), "obj2".to_string()],
            param_names: vec!["x".to_string()],
            total_trials: 10,
            pareto_count: 3,
            selected_trials: vec![HtmlTrialRow {
                trial_id: 0,
                trial_number: 0,
                params: [("x".to_string(), 0.5)].into_iter().collect(),
                objectives: vec![1.0, 2.0],
                pareto_rank: 0,
            }],
            statistics: TrialStatistics {
                objective_means: vec![1.0, 2.0],
                objective_variances: vec![0.1, 0.2],
                pareto_count: 3,
            },
        }
    }

    #[test]
    fn test_escape_html_basic() {
        assert_eq!(escape_html("<script>"), "&lt;script&gt;");
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html(r#"say "hi""#), "say &quot;hi&quot;");
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("foo/bar"), "foo_bar");
        assert_eq!(sanitize_filename("a:b*c?"), "a_b_c_");
        assert_eq!(sanitize_filename("normal"), "normal");
    }

    #[test]
    fn test_build_html_report_contains_study_name() {
        let snap = make_snapshot();
        let html = build_html_report(&snap);
        assert!(html.contains("test_study"));
    }

    #[test]
    fn test_build_html_report_is_valid_html() {
        let snap = make_snapshot();
        let html = build_html_report(&snap);
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("</html>"));
    }

    #[test]
    fn test_build_html_report_no_raw_user_data_injection() {
        let mut snap = make_snapshot();
        snap.study_name = "<script>alert(1)</script>".to_string();
        let html = build_html_report(&snap);
        // 生の <script> タグが HTML 本文に存在しないことを確認
        // ただしヘッダーの <style> タグは除く
        let body_start = html.find("<body>").unwrap_or(0);
        let body = &html[body_start..];
        assert!(!body.contains("<script>"));
    }

    #[test]
    fn test_render_scatter_svg_two_objectives() {
        let snap = make_snapshot();
        let svg = render_scatter_svg(&snap);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("<circle"));
    }

    #[test]
    fn test_render_scatter_svg_single_objective_no_circles() {
        let mut snap = make_snapshot();
        snap.objective_names = vec!["obj1".to_string()];
        let svg = render_scatter_svg(&snap);
        // 1目的関数のみの場合は "No data" SVG
        assert!(svg.contains("No data"));
    }

    // TASK-2128 integration tests

    #[test]
    fn task2128_build_html_report_contains_doctype() {
        let snap = make_snapshot();
        let html = build_html_report(&snap);
        assert!(html.contains("<!DOCTYPE html>") || html.contains("<html"));
        assert!(html.contains("test_study"));
    }

    #[test]
    fn task2128_build_html_report_escapes_xss() {
        let mut snap = make_snapshot();
        snap.study_name = "<script>alert('xss')</script>".to_string();
        let html = build_html_report(&snap);
        // <script>alert() が生のまま body に含まれないこと
        let body_start = html.find("<body>").unwrap_or(0);
        let body = &html[body_start..];
        assert!(
            !body.contains("<script>alert"),
            "XSS インジェクションを許容してはならない"
        );
        assert!(
            html.contains("&lt;script&gt;") || html.contains("&lt;"),
            "< は &lt; にエスケープされること"
        );
    }

    #[test]
    fn task2128_html_report_done_message_channel() {
        use crate::state::messages::AppMessage;
        use std::sync::mpsc;

        let (tx, rx) = mpsc::sync_channel::<AppMessage>(8);
        let html = "<html>test</html>".to_string();
        let filename = "test_study.html".to_string();

        tx.send(AppMessage::HtmlReportDone {
            html: html.clone(),
            suggested_filename: filename.clone(),
        })
        .unwrap();

        match rx.recv().unwrap() {
            AppMessage::HtmlReportDone {
                html: received_html,
                suggested_filename,
            } => {
                assert_eq!(received_html, html);
                assert_eq!(suggested_filename, filename);
            }
            _ => panic!("予期しないメッセージタイプ"),
        }
    }

    #[test]
    fn task2128_build_html_with_multiple_trials() {
        let snap = HtmlReportSnapshot {
            study_name: "multi_trial".to_string(),
            objective_names: vec!["f1".to_string()],
            param_names: vec!["x".to_string()],
            total_trials: 3,
            pareto_count: 1,
            selected_trials: vec![
                HtmlTrialRow {
                    trial_id: 0,
                    trial_number: 0,
                    params: [("x".to_string(), 0.1)].into(),
                    objectives: vec![1.0],
                    pareto_rank: 0,
                },
                HtmlTrialRow {
                    trial_id: 1,
                    trial_number: 1,
                    params: [("x".to_string(), 0.5)].into(),
                    objectives: vec![2.0],
                    pareto_rank: 1,
                },
                HtmlTrialRow {
                    trial_id: 2,
                    trial_number: 2,
                    params: [("x".to_string(), 0.9)].into(),
                    objectives: vec![0.5],
                    pareto_rank: 0,
                },
            ],
            statistics: TrialStatistics {
                objective_means: vec![1.17],
                objective_variances: vec![0.36],
                pareto_count: 1,
            },
        };
        let html = build_html_report(&snap);
        assert!(html.contains("multi_trial"));
        assert!(html.contains("</html>"));
    }
}
