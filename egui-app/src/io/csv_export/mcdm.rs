use crate::state::app_state::{AppState, McdmResult};
use tunny_core::export::{CsvField, CsvWriter};

pub(super) fn build_pareto_csv(app_state: &AppState) -> Option<String> {
    let study = app_state.current_study.as_ref()?;
    if study.pareto_indices.is_empty() {
        return None;
    }
    // Export every individual with its Pareto rank (rank 0 = Pareto front).
    // This matches the chart, which plots all trials and colors the front.
    // `StudyView::new` guarantees `pareto_rank` is row-aligned (length == row
    // count), so rank lookups never go out of bounds.
    let row_indices: Vec<usize> = (0..study.view.trial_ids.len()).collect();
    Some(crate::io::export::build_trial_csv_from_view(
        &study.view,
        &row_indices,
        &study.meta.param_names,
        &study.meta.objective_names,
        crate::io::export::TrialCsvColumns {
            pareto_rank: true,
            cluster_id: false,
        },
    ))
}

pub(super) fn build_mcdm_rank_csv(result: &McdmResult, app_state: &AppState) -> Option<String> {
    let trial_ids = &app_state.current_study.as_ref()?.view.trial_ids;
    let method_name = result.method_label();
    let scores = result.primary_scores();
    let ranked = result.ranked_indices();
    let mut w = CsvWriter::new();
    w.header(["trial_id", "rank", "score", "method"]);
    for (rank, &idx) in ranked.iter().enumerate() {
        let i = idx as usize;
        let trial_id = trial_ids.get(i).copied().unwrap_or(i as u32);
        let score = scores.get(i).copied().unwrap_or(f64::NAN);
        w.row([
            CsvField::UInt(trial_id as u64),
            CsvField::UInt((rank + 1) as u64),
            CsvField::Num(score),
            CsvField::Text(method_name),
        ]);
    }
    Some(w.finish())
}

pub(super) fn build_mcdm_scatter_csv(result: &McdmResult, app_state: &AppState) -> Option<String> {
    let trial_ids = &app_state.current_study.as_ref()?.view.trial_ids;
    let scores = result.primary_scores();
    let ranked = result.ranked_indices();
    let mut w = CsvWriter::new();
    w.header(["trial_id", "rank", "primary_score"]);
    for (rank, &idx) in ranked.iter().enumerate() {
        let i = idx as usize;
        let trial_id = trial_ids.get(i).copied().unwrap_or(i as u32);
        let score = scores.get(i).copied().unwrap_or(f64::NAN);
        w.row([
            CsvField::UInt(trial_id as u64),
            CsvField::UInt((rank + 1) as u64),
            CsvField::Num(score),
        ]);
    }
    Some(w.finish())
}

pub(super) fn build_mcdm_table_csv(result: &McdmResult, app_state: &AppState) -> Option<String> {
    let trial_ids = &app_state.current_study.as_ref()?.view.trial_ids;
    let tid = |idx: u32| trial_ids.get(idx as usize).copied().unwrap_or(idx);
    match result {
        McdmResult::Topsis(r) => {
            let mut w = CsvWriter::new();
            w.header(["trial_id", "rank", "topsis_score"]);
            for (rank, &idx) in r.ranked_indices.iter().enumerate() {
                let score = r.scores.get(idx as usize).copied().unwrap_or(f64::NAN);
                w.row([
                    CsvField::UInt(tid(idx) as u64),
                    CsvField::UInt((rank + 1) as u64),
                    CsvField::Num(score),
                ]);
            }
            Some(w.finish())
        }
        McdmResult::Vikor(r) => {
            let mut w = CsvWriter::new();
            w.header(["trial_id", "rank", "s_value", "r_value", "q_value"]);
            for (rank, &idx) in r.ranked_indices.iter().enumerate() {
                let i = idx as usize;
                let s = r.s_values.get(i).copied().unwrap_or(f64::NAN);
                let rv = r.r_values.get(i).copied().unwrap_or(f64::NAN);
                let q = r.q_values.get(i).copied().unwrap_or(f64::NAN);
                w.row([
                    CsvField::UInt(tid(idx) as u64),
                    CsvField::UInt((rank + 1) as u64),
                    CsvField::Num(s),
                    CsvField::Num(rv),
                    CsvField::Num(q),
                ]);
            }
            Some(w.finish())
        }
        McdmResult::PrometheeI(r) => {
            let mut w = CsvWriter::new();
            w.header([
                "trial_id",
                "rank",
                "phi_plus",
                "phi_minus",
                "incomparable_count",
            ]);
            for (rank, &idx) in r.ranked_indices_i.iter().enumerate() {
                let i = idx as usize;
                let phi_plus = r.phi_plus.get(i).copied().unwrap_or(f64::NAN);
                let phi_minus = r.phi_minus.get(i).copied().unwrap_or(f64::NAN);
                let incomparable_count = r.incomparable_counts.get(i).copied().unwrap_or(0);
                w.row([
                    CsvField::UInt(tid(idx) as u64),
                    CsvField::UInt((rank + 1) as u64),
                    CsvField::Num(phi_plus),
                    CsvField::Num(phi_minus),
                    CsvField::UInt(incomparable_count as u64),
                ]);
            }
            Some(w.finish())
        }
        McdmResult::PrometheeII(r) => {
            let mut w = CsvWriter::new();
            w.header(["trial_id", "rank", "phi_net"]);
            for (rank, &idx) in r.ranked_indices_ii.iter().enumerate() {
                let phi_net = r.phi_net.get(idx as usize).copied().unwrap_or(f64::NAN);
                w.row([
                    CsvField::UInt(tid(idx) as u64),
                    CsvField::UInt((rank + 1) as u64),
                    CsvField::Num(phi_net),
                ]);
            }
            Some(w.finish())
        }
    }
}
