use super::*;

fn plan(low: f64, high: f64, points: f64, horizontal: bool) -> LogTicks {
    let bounds = (low.log10(), high.log10());
    log_ticks(
        GridInput {
            bounds,
            base_step_size: (bounds.1 - bounds.0) * 8.0 / points,
        },
        horizontal,
        |s| s.len() as f64 * 8.0,
    )
}

#[test]
fn narrow_ranges_have_distinct_original_labels_and_upper_interior_ticks() {
    for (low, high) in [
        (2.0, 8.0),
        (0.002, 0.008),
        (2.01, 2.02),
        (8.0, 12.0),
        (2e-200, 8e-200),
        (2e200, 8e200),
        (2e-320, 8e-320),
        (2.0, 2.00000000001),
        (2e200, 2.00000000001e200),
        (1.0, 10.0),
    ] {
        for horizontal in [false, true] {
            let ticks = plan(low, high, 700.0, horizontal);
            assert!(ticks.labels.len() >= 2, "{low}..{high}: {:?}", ticks.labels);
            let values: Vec<f64> = ticks
                .labels
                .iter()
                .map(|(_, s)| s.parse().unwrap())
                .collect();
            assert!(values.windows(2).all(|w| w[0] < w[1]), "{:?}", ticks.labels);
            let tolerance = ticks
                .labels
                .windows(2)
                .map(|w| (10f64.powf(w[1].0) - 10f64.powf(w[0].0)) * 0.25)
                .fold(f64::INFINITY, f64::min)
                .max(f64::from_bits(1));
            for ((position, _), value) in ticks.labels.iter().zip(values) {
                let restored = 10f64.powf(*position);
                assert!(
                    (value - restored).abs() <= tolerance,
                    "{value} vs {restored}"
                );
                assert!(value > low && value < high);
            }
            let upper = ticks.labels.last().unwrap().0;
            assert!((high.log10() - upper) / (high.log10() - low.log10()) < 0.3);
            assert!(ticks.marks.windows(2).all(|w| w[0].value < w[1].value));
            assert!(ticks
                .marks
                .iter()
                .all(|m| m.step_size.is_finite() && m.step_size > 0.0));
            assert!(ticks.marks.len() > ticks.labels.len());
        }
    }
    let ticks = plan(2.0, 8.0, 400.0, false);
    let upper: f64 = ticks.labels.last().unwrap().1.parse().unwrap();
    assert_eq!(
        upper * 2.0,
        (upper * 2.0).round(),
        "upper tick should be rounded: {upper}"
    );
}

#[test]
fn density_adapts_and_broad_ranges_keep_decades() {
    for horizontal in [false, true] {
        let small = plan(2.0, 8.0, 220.0, horizontal);
        let large = plan(2.0, 8.0, 900.0, horizontal);
        assert!(!small.labels.is_empty());
        assert!(large.labels.len() > small.labels.len());
        for ticks in [small, large] {
            for minor in ticks
                .marks
                .iter()
                .filter(|m| !ticks.labels.iter().any(|(v, _)| *v == m.value))
            {
                assert!(ticks
                    .marks
                    .iter()
                    .filter(|m| ticks.labels.iter().any(|(v, _)| *v == m.value))
                    .all(|major| major.step_size > minor.step_size));
            }
        }
    }
    for (low, high) in [(1e-3, 1e3), (1e-200, 1e200)] {
        let ticks = plan(low, high, 700.0, false);
        assert!(!ticks.labels.is_empty());
        assert!(ticks.labels.iter().all(|(v, _)| *v == v.round()));
        assert!(ticks.labels.len() < 25);
    }
}

#[test]
fn invalid_bounds_do_not_generate_ticks() {
    for bounds in [
        (0.0, 0.0),
        (1.0, -1.0),
        (f64::NAN, 1.0),
        (0.0, f64::INFINITY),
    ] {
        assert!(log_ticks(
            GridInput {
                bounds,
                base_step_size: 0.01
            },
            false,
            |_| 10.0
        )
        .marks
        .is_empty());
    }
}

fn rendered_axis(
    ctx: &egui::Context,
    low: f64,
    high: f64,
    size: egui::Vec2,
    horizontal: bool,
) -> Vec<(String, egui::Rect)> {
    let mut labels = Vec::new();
    // egui_plot remembers measured axis thickness for the next frame.
    for frame in 0..3 {
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                ..Default::default()
            },
            |ui| {
                let plot = egui_plot::Plot::new("log_axis_test").show_axes(if horizontal {
                    [true, false]
                } else {
                    [false, true]
                });
                let plot = if horizontal {
                    apply_log_x_axis(plot, ui)
                } else {
                    apply_log_y_axis(plot, ui)
                };
                plot.show(ui, |plot_ui| {
                    let (min, max) = (low.log10(), high.log10());
                    plot_ui.set_plot_bounds(if horizontal {
                        egui_plot::PlotBounds::from_min_max([min, 0.0], [max, 1.0])
                    } else {
                        egui_plot::PlotBounds::from_min_max([0.0, min], [1.0, max])
                    });
                });
            },
        );
        labels = output
            .shapes
            .iter()
            .filter_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape {
                    let rect = egui::Rect::from_min_size(text.pos, text.galley.size());
                    assert!(
                        frame < 2 || shape.clip_rect.contains_rect(rect),
                        "clipped label: {} at {rect:?} in {:?}",
                        text.galley.text(),
                        shape.clip_rect
                    );
                    Some((text.galley.text().to_owned(), rect))
                } else {
                    None
                }
            })
            .collect();
    }
    labels
}

#[test]
fn rendered_labels_fit_without_overlap_in_both_themes_and_axes() {
    for dark in [false, true] {
        for horizontal in [false, true] {
            let ctx = egui::Context::default();
            ctx.set_visuals(if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
            for size in [egui::vec2(320.0, 240.0), egui::vec2(900.0, 650.0)] {
                for (low, high) in [
                    (2.0, 8.0),
                    (0.002, 0.008),
                    (2.01, 2.02),
                    (8.0, 12.0),
                    (2e-200, 8e-200),
                    (2e200, 8e200),
                    (2.0, 2.00000000001),
                    (1e-3, 1e3),
                ] {
                    let labels = rendered_axis(&ctx, low, high, size, horizontal);
                    assert!(
                        !labels.is_empty(),
                        "{low}..{high}, horizontal={horizontal}, size={size:?}"
                    );
                    for (i, (text, rect)) in labels.iter().enumerate() {
                        let value: f64 = text.parse().unwrap();
                        assert!(value >= low && value <= high, "{text}");
                        assert!(
                            rect.min.x >= 0.0
                                && rect.min.y >= 0.0
                                && rect.max.x <= size.x
                                && rect.max.y <= size.y,
                            "clipped {text}: {rect:?}, {size:?}"
                        );
                        for (_, other) in &labels[i + 1..] {
                            assert!(!rect.intersects(*other), "overlap {labels:?}");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn rendered_labels_follow_resizing_zoom_and_pan_bounds() {
    for horizontal in [false, true] {
        let ctx = egui::Context::default();
        let small = rendered_axis(&ctx, 2.0, 8.0, egui::vec2(320.0, 240.0), horizontal);
        let large = rendered_axis(&ctx, 2.0, 8.0, egui::vec2(900.0, 650.0), horizontal);
        assert!(large.len() > small.len());
        for (low, high) in [(2.01, 2.02), (6.01, 6.02), (0.002, 0.008), (1e-3, 1e3)] {
            let labels = rendered_axis(&ctx, low, high, egui::vec2(900.0, 650.0), horizontal);
            assert!(labels.len() >= 2, "{labels:?}");
            let mut values: Vec<f64> = labels.iter().map(|(s, _)| s.parse().unwrap()).collect();
            values.sort_by(f64::total_cmp);
            assert!(values.windows(2).all(|w| w[0] < w[1]));
            assert!(values.iter().all(|v| *v > low && *v < high));
            assert!(
                (high.log10() - values.last().unwrap().log10()) / (high.log10() - low.log10())
                    < 0.3
            );
        }
    }
}
