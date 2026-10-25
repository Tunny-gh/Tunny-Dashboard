//! Shared ticks for axes whose data has already been transformed with log10.

use std::cell::RefCell;
use std::rc::Rc;

use egui_plot::{GridInput, GridMark};

#[derive(Default)]
struct LogTicks {
    marks: Vec<GridMark>,
    labels: Vec<(f64, String)>,
}

fn format_log_tick(log_value: f64, log_step: f64) -> String {
    let value = 10f64.powf(log_value);
    let digits = ((-log_step.log10()).ceil() as usize + 2).clamp(2, 17);
    if !(1e-4..1e5).contains(&value) {
        let text = format!("{value:.precision$e}", precision = digits - 1);
        let (mantissa, exponent) = text.split_once('e').unwrap();
        format!(
            "{}e{exponent}",
            mantissa.trim_end_matches('0').trim_end_matches('.')
        )
    } else {
        let decimals = (digits as i32 - 1 - value.log10().floor() as i32).max(0) as usize;
        let text = format!("{value:.decimals$}");
        if decimals > 0 {
            text.trim_end_matches('0').trim_end_matches('.').to_owned()
        } else {
            text
        }
    }
}

fn nice_step(step: f64) -> f64 {
    let scale = 10f64.powf(step.log10().floor());
    let fraction = step / scale;
    [1.0, 2.0, 2.5, 5.0, 10.0]
        .into_iter()
        .find(|&n| n >= fraction)
        .unwrap_or(10.0)
        * scale
}

fn log_ticks(input: GridInput, horizontal: bool, text_width: impl Fn(&str) -> f64) -> LogTicks {
    let (min, max) = input.bounds;
    let span = max - min;
    let unit = input.base_step_size / 8.0;
    if !min.is_finite()
        || !max.is_finite()
        || !span.is_finite()
        || span <= 0.0
        || !unit.is_finite()
        || unit <= 0.0
    {
        return LogTicks::default();
    }

    let mut candidates = Vec::new();
    let mut minors = Vec::new();
    if span >= 2.0 {
        for exponent in (min.floor().max(-324.0) as i32)..=(max.ceil().min(309.0) as i32) {
            candidates.push(f64::from(exponent));
            if span <= 12.0 {
                for mantissa in 2..=9 {
                    minors.push(f64::from(exponent) + f64::from(mantissa).log10());
                }
            }
        }
    } else {
        // Normalize before rounding: neither huge nor subnormal original values
        // should overflow/underflow the nice-step calculation.
        let exponent = min.floor();
        let low = 10f64.powf(min - exponent);
        let width = low * (span * std::f64::consts::LN_10).exp_m1();
        let target = (span / unit / if horizontal { 100.0 } else { 40.0 }).max(2.0);
        let step = nice_step(width / target);
        if step.is_finite() && step > 0.0 {
            for (spacing, output) in [(step, &mut candidates), (step / 5.0, &mut minors)] {
                let first = (low / spacing).ceil();
                let count = (width / spacing).ceil().min(2048.0) as usize;
                for i in 0..=count {
                    let value = (first + i as f64) * spacing;
                    output.push(exponent + value.log10());
                }
            }
        }
    }

    let valid = |v: &f64| {
        *v >= min && *v <= max && {
            let original = 10f64.powf(*v);
            original.is_finite() && original > 0.0
        }
    };
    candidates.retain(valid);
    candidates.sort_by(f64::total_cmp);
    candidates.dedup();
    let precision_step = candidates
        .windows(2)
        .map(|w| w[1] - w[0])
        .fold(span, f64::min);
    let mut ticks = LogTicks::default();
    let mut previous: Option<(f64, f64)> = None;
    for &value in candidates.iter().rev() {
        let label = format_log_tick(value, precision_step);
        let extent = if horizontal { text_width(&label) } else { 16.0 };
        let inset = (extent / 2.0 + 4.0) * unit;
        if value < min + inset || value > max - inset {
            continue;
        }
        if let Some((last, last_extent)) = previous {
            if last - value
                < ((extent + last_extent) / 2.0 + 12.0).max(if horizontal { 84.0 } else { 32.0 })
                    * unit
            {
                continue;
            }
        }
        previous = Some((value, extent));
        ticks.labels.push((value, label));
    }
    ticks.labels.reverse();

    // egui_plot uses step_size for label visibility as well as grid opacity.
    // Use the nearest selected major, not the unthinned candidate interval.
    for (i, (value, _)) in ticks.labels.iter().enumerate() {
        let before = i.checked_sub(1).map(|j| value - ticks.labels[j].0);
        let after = ticks.labels.get(i + 1).map(|next| next.0 - value);
        let step_size = before.into_iter().chain(after).fold(span, f64::min);
        ticks.marks.push(GridMark {
            value: *value,
            step_size,
        });
    }
    minors.retain(valid);
    minors.sort_by(f64::total_cmp);
    minors.dedup();
    let mut last_minor = f64::NEG_INFINITY;
    for value in minors {
        if value - last_minor < 12.0 * unit
            || ticks
                .labels
                .iter()
                .any(|(major, _)| (major - value).abs() < 10.0 * unit)
        {
            continue;
        }
        ticks.marks.push(GridMark {
            value,
            step_size: 12.0 * unit,
        });
        last_minor = value;
    }
    ticks.marks.sort_by(|a, b| a.value.total_cmp(&b.value));
    ticks
}

fn apply_log_axis<'a>(
    plot: egui_plot::Plot<'a>,
    ui: &egui::Ui,
    horizontal: bool,
) -> egui_plot::Plot<'a> {
    let ticks = Rc::new(RefCell::new(LogTicks::default()));
    let grid_ticks = Rc::clone(&ticks);
    let ctx = ui.ctx().clone();
    let font = egui::TextStyle::Body.resolve(ui.style());
    let spacer = move |input| {
        let plan = log_ticks(input, horizontal, |text| {
            ctx.fonts_mut(|fonts| {
                f64::from(
                    fonts
                        .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE)
                        .size()
                        .x,
                )
            })
        });
        let marks = plan.marks.clone();
        *grid_ticks.borrow_mut() = plan;
        marks
    };
    // A grid mark has no major/minor flag. Share the selected labels with the
    // formatter rather than inferring major status from integer log coordinates.
    let formatter = move |mark: GridMark, _: &std::ops::RangeInclusive<f64>| {
        ticks
            .borrow()
            .labels
            .iter()
            .find(|(value, _)| *value == mark.value)
            .map(|(_, label)| label.clone())
            .unwrap_or_default()
    };
    if horizontal {
        plot.x_grid_spacer(spacer).x_axis_formatter(formatter)
    } else {
        plot.y_grid_spacer(spacer).y_axis_formatter(formatter)
    }
}

pub fn apply_log_y_axis<'a>(plot: egui_plot::Plot<'a>, ui: &egui::Ui) -> egui_plot::Plot<'a> {
    apply_log_axis(plot, ui, false)
}

pub fn apply_log_x_axis<'a>(plot: egui_plot::Plot<'a>, ui: &egui::Ui) -> egui_plot::Plot<'a> {
    apply_log_axis(plot, ui, true)
}

#[cfg(test)]
mod tests;
