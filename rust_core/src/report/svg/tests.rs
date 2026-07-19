use super::*;

/// Confirms that `fill="..."` / `stroke="..."` attribute values contain
/// no `#` (raw hex color codes are forbidden; `var(--foo)` never
/// contains `#`).
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

/// Counts occurrences of the `<title>` element.
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
    // Even in the degenerate case (min == max), this must not panic
    // and must return an ascending integer sequence containing the
    // value.
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

    // data marks + 2 legend markers (legend markers get no <title>).
    assert_eq!(count(&svg, "<circle"), background.len() + front.len() + 2);
    assert_eq!(count_titles(&svg), background.len() + front.len());
    assert!(svg.contains("<path"));
    // legend is required because there are two series (front / dominated).
    assert!(svg.contains("Pareto front"));
    assert!(svg.contains("dominated"));
    assert_no_raw_hex_in_color_attrs(&svg);
}

#[test]
fn scatter_chart_no_legend_when_single_series() {
    // background only (single series) means no legend (shared rule).
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
    // Points with feasible=false get [infeasible] appended to their
    // tooltip. Feasible points do not get it.
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
    // Confirm that at the boundary values -1, -0.51, -0.5, 0, 0.5,
    // 0.51, 1, the quantization step and label-visibility both match
    // the expected theoretical values.
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
    // The display is truncated, and the full label is preserved in
    // <title>.
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
