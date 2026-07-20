//! MCDM section construction (TOPSIS / VIKOR / PROMETHEE consensus).

use std::collections::BTreeSet;

use crate::mcdm::{promethee, topsis, vikor};

use super::VIKOR_V;
use crate::report::model::*;

/// Takes the already-computed TOPSIS ranking ([`front_topsis`]) and
/// additionally computes VIKOR / PROMETHEE to build the MCDM section.
/// TOPSIS itself is shared with the caller (`build_outcome_multi`) and is
/// not recomputed here.
#[allow(clippy::too_many_arguments)]
pub(super) fn build_mcdm(
    ts: &topsis::TopsisResult,
    values: &[f64],
    k: usize,
    weights: &[f64],
    is_minimize: &[bool],
    m: usize,
    front_rows: &[usize],
    trial_numbers: &[u32],
    objectives: &[Vec<f64>],
) -> Option<McdmSection> {
    let vk = vikor::compute_vikor(values, k, m, weights, is_minimize, VIKOR_V).ok()?;
    let pr = promethee::compute_promethee(values, k, m, weights, is_minimize).ok()?;

    let entry = |ranked: &[u32], rank_i: usize| -> McdmEntry {
        let row = front_rows[ranked[rank_i] as usize];
        McdmEntry {
            rank: rank_i + 1,
            trial_number: trial_numbers[row],
            objectives: objectives[row].clone(),
        }
    };
    let top_entries = |ranked: &[u32]| -> Vec<McdmEntry> {
        (0..ranked.len().min(5)).map(|i| entry(ranked, i)).collect()
    };
    let top_set = |ranked: &[u32]| -> BTreeSet<u32> {
        ranked
            .iter()
            .take(10)
            .map(|&sub| trial_numbers[front_rows[sub as usize]])
            .collect()
    };

    let t10 = top_set(&ts.ranked_indices);
    let v10 = top_set(&vk.ranked_indices);
    let p10 = top_set(&pr.ranked_indices_ii);
    let consensus_trials: Vec<u32> = t10
        .iter()
        .filter(|x| v10.contains(x) && p10.contains(x))
        .copied()
        .collect();

    Some(McdmSection {
        weight_scheme: "equal".to_string(),
        weights: weights.to_vec(),
        topsis_top: top_entries(&ts.ranked_indices),
        vikor_top: top_entries(&vk.ranked_indices),
        promethee_top: top_entries(&pr.ranked_indices_ii),
        consensus_trials,
    })
}
