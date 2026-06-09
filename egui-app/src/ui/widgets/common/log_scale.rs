//! Y 軸対数スケール描画の共通ヘルパー。
//!
//! 対数スケールでは値を log10 変換してプロットし、Y 軸ラベルは変換前の元の値
//! （10^mark で復元）を表示する。複数のチャート（最適化履歴・Slice など）で
//! 同じ目盛り・ラベル整形を共有するためにここへ切り出している。

/// log10 変換した軸（プロット座標 = log10(値)）に対し、10 の累乗を主目盛りに
/// 配置する grid spacer。各ディケード（10^k 〜 10^(k+1)）内に 2〜9 倍の補助目盛りを
/// 置き、`step_size` の大小でラインの太さを区別する（主目盛り > 補助目盛り）。
pub fn log10_grid_spacer(input: egui_plot::GridInput) -> Vec<egui_plot::GridMark> {
    let (min, max) = input.bounds;
    if !min.is_finite() || !max.is_finite() || min >= max {
        return Vec::new();
    }

    // 表示範囲（log10 空間）を覆うディケードの整数指数。可視範囲を覆うだけに留め、
    // 範囲外へ目盛り（特にラベル）がはみ出さないようにする。
    let start = min.floor() as i64;
    let end = max.ceil() as i64;

    // ディケード数が多すぎる場合は主目盛り（10 の累乗）のみに間引く。
    let decade_span = end - start;
    let majors_only = decade_span > 12;

    // 可視範囲内の目盛りのみ採用する（端からわずかに外れる分は許容する）。
    let eps = (max - min) * 1e-9;
    let in_bounds = |v: f64| v >= min - eps && v <= max + eps;

    let mut marks = Vec::new();
    for exp in start..=end {
        let decade = 10f64.powi(exp as i32);
        // 主目盛り: 10^exp。step_size はディケード全幅（log10 空間で 1.0）。
        if in_bounds(exp as f64) {
            marks.push(egui_plot::GridMark {
                value: exp as f64,
                step_size: 1.0,
            });
        }
        if majors_only {
            continue;
        }
        // 補助目盛り: 2×, 3×, ... 9× 10^exp。log10 空間での位置は exp + log10(m)。
        for m in 2..=9 {
            let value = (decade * m as f64).log10();
            if in_bounds(value) {
                marks.push(egui_plot::GridMark {
                    value,
                    step_size: 0.1,
                });
            }
        }
    }
    marks
}

/// 対数スケール時の Y 軸ラベルを、元の値（10^mark 復元後）に応じて読みやすく整形する。
/// 大きな値・小さな値は指数表記、中間域は桁数に応じた固定小数で表示する。
pub fn format_log_tick(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    let abs = value.abs();
    if !(1e-4..1e5).contains(&abs) {
        // 表示域外は指数表記（例: 1.2e-5, 3.4e6）
        format!("{value:.1e}")
    } else if abs >= 100.0 {
        format!("{value:.0}")
    } else if abs >= 1.0 {
        format!("{value:.1}")
    } else {
        // 1 未満は有効桁を確保するため小数桁を増やす
        format!("{value:.3}")
    }
}

/// Y 軸対数スケール用の grid spacer / ラベル整形を `plot` に適用する。
/// 主目盛り（10 の累乗）のみラベルを付け、補助目盛り（2〜9 倍）はラインのみとする。
pub fn apply_log_y_axis(plot: egui_plot::Plot<'_>) -> egui_plot::Plot<'_> {
    plot.y_grid_spacer(log10_grid_spacer)
        .y_axis_formatter(|mark, _range| {
            // 主目盛り（10 の累乗 = log10 空間で整数）のみラベルを付け、
            // 補助目盛り（2〜9 倍）はラインのみでラベルは出さない。
            if (mark.value - mark.value.round()).abs() > 1e-6 {
                return String::new();
            }
            let original = 10f64.powf(mark.value.round());
            format_log_tick(original)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_log_tick_restores_original_scale() {
        assert_eq!(format_log_tick(0.0), "0");
        assert_eq!(format_log_tick(1.0), "1.0");
        assert_eq!(format_log_tick(550.0), "550");
        assert_eq!(format_log_tick(2.5), "2.5");
        assert_eq!(format_log_tick(0.001), "0.001");
        assert_eq!(format_log_tick(1_000_000.0), "1.0e6");
    }

    #[test]
    fn log10_grid_spacer_places_decades_as_majors() {
        let input = egui_plot::GridInput {
            bounds: (0.0, 3.0),
            base_step_size: 0.01,
        };
        let marks = log10_grid_spacer(input);
        let majors: Vec<f64> = marks
            .iter()
            .filter(|m| m.step_size == 1.0)
            .map(|m| m.value)
            .collect();
        for exp in [0.0, 1.0, 2.0, 3.0] {
            assert!(
                majors.iter().any(|&v| (v - exp).abs() < 1e-9),
                "missing decade major at 10^{exp}"
            );
        }
        assert!(marks.iter().any(|m| m.step_size < 1.0));
    }

    #[test]
    fn log10_grid_spacer_thins_to_majors_for_wide_range() {
        let input = egui_plot::GridInput {
            bounds: (-10.0, 10.0),
            base_step_size: 0.1,
        };
        let marks = log10_grid_spacer(input);
        assert!(marks.iter().all(|m| m.step_size == 1.0));
    }
}
