//! Objective-statistics section construction.

use crate::io::journal::parser::StudyMeta;
use crate::statistics::histogram::sturges_bins;
use crate::statistics::{compute_histogram, quantile, BinRule};

use super::MAX_HIST_BINS;
use crate::report::model::*;

pub(super) fn build_objective_stats(
    objectives: &[Vec<f64>],
    meta: &StudyMeta,
    directions: &[Direction],
    m: usize,
) -> Vec<ObjectiveStats> {
    (0..m)
        .map(|j| {
            let mut finite: Vec<f64> = objectives
                .iter()
                .map(|o| o[j])
                .filter(|v| v.is_finite())
                .collect();
            let name = meta.objective_names[j].clone();
            let direction = directions[j];
            if finite.is_empty() {
                return ObjectiveStats {
                    name,
                    direction,
                    n: 0,
                    mean: 0.0,
                    std: 0.0,
                    min: 0.0,
                    q1: 0.0,
                    median: 0.0,
                    q3: 0.0,
                    max: 0.0,
                    histogram: None,
                };
            }
            finite.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let cnt = finite.len();
            let mean = finite.iter().sum::<f64>() / cnt as f64;
            let var = finite.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / cnt as f64;
            let bins = sturges_bins(cnt).clamp(1, MAX_HIST_BINS);
            let histogram =
                compute_histogram(&finite, BinRule::Manual(bins)).map(|h| HistogramData {
                    bin_edges: h.bin_edges,
                    counts: h.counts,
                });
            ObjectiveStats {
                name,
                direction,
                n: cnt,
                mean,
                std: var.sqrt(),
                min: finite[0],
                q1: quantile(&finite, 0.25),
                median: quantile(&finite, 0.5),
                q3: quantile(&finite, 0.75),
                max: finite[cnt - 1],
                histogram,
            }
        })
        .collect()
}
