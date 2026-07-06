//! RDB バックエンド (SQLite/PostgreSQL/MySQL) の E2E 動作確認用 CLI。
//!
//! ```text
//! cargo run -p tunny-core --example rdb_smoke -- <url-or-sqlite-path> [study_id]
//! ```
//!
//! - `study_id` 省略時: study 一覧（id, name, directions, completed/total）を出力する。
//! - `study_id` 指定時: 確定メタ・行データ・extras（trial state 集計 / 中間値件数）・
//!   フィンガープリントを出力する。
//!
//! `<url-or-sqlite-path>` は `postgresql://`/`mysql://` 系 URL か、SQLite ファイルの
//! パス（`is_rdb_url` が偽の場合はローカルファイルとみなし `io::sqlite` へフォールバック）
//! のいずれも受け付ける。3 バックエンドを同一データで突き合わせる手元 E2E 検証のための
//! ハーネスであり、出力は `key=value` 形式・1 項目 1 行で他ツールとの diff に耐える形にする。

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use tunny_core::io::rdb::{
    parse_single_study_rows_url, scan_study_list_url, study_fingerprint_url, RdbUrl,
};
use tunny_core::io::sqlite;
use tunny_core::journal_parser::OptimizationDirection;

fn direction_label(direction: &OptimizationDirection) -> &'static str {
    match direction {
        OptimizationDirection::Minimize => "MINIMIZE",
        OptimizationDirection::Maximize => "MAXIMIZE",
    }
}

fn format_floats(values: &[f64]) -> String {
    values
        .iter()
        .map(f64::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn print_study_list(target: &str) -> Result<(), String> {
    let studies = match RdbUrl::parse(target) {
        Some(url) => scan_study_list_url(&url)?,
        None => sqlite::scan_study_list(Path::new(target))?,
    };
    for study in studies {
        let directions = study
            .directions
            .iter()
            .map(direction_label)
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "study_id={} name={} directions={} completed={} total={}",
            study.study_id, study.name, directions, study.completed_trials, study.total_trials
        );
    }
    Ok(())
}

fn print_study_detail(target: &str, study_id: u32) -> Result<(), String> {
    let is_url = RdbUrl::parse(target);

    let rows = match &is_url {
        Some(url) => parse_single_study_rows_url(url, study_id)?,
        None => sqlite::parse_single_study_rows(Path::new(target), study_id)?,
    };

    println!("study_id={}", rows.meta.study_id);
    println!("name={}", rows.meta.name);
    let directions = rows
        .meta
        .directions
        .iter()
        .map(direction_label)
        .collect::<Vec<_>>()
        .join(",");
    println!("directions={directions}");
    println!("param_names={}", rows.param_names.join(","));
    println!("objective_names={}", rows.objective_names.join(","));
    println!("row_count={}", rows.rows.len());

    // extras: trial state 集計 / 中間値総数。
    let mut state_counts: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut intermediate_total: u64 = 0;
    for trial in &rows.extras.trials {
        *state_counts.entry(trial.state.label()).or_insert(0) += 1;
        intermediate_total += trial.intermediate_values.len() as u64;
    }
    let state_counts_str = state_counts
        .iter()
        .map(|(state, count)| format!("{state}:{count}"))
        .collect::<Vec<_>>()
        .join(",");
    println!("state_counts={state_counts_str}");
    println!("intermediate_total={intermediate_total}");

    if let Some(first) = rows.rows.first() {
        println!("first_trial_id={}", first.trial_id);
        println!(
            "first_objective_values={}",
            format_floats(&first.objective_values)
        );
    }
    if let Some(last) = rows.rows.last() {
        println!("last_trial_id={}", last.trial_id);
        println!(
            "last_objective_values={}",
            format_floats(&last.objective_values)
        );
    }

    // フィンガープリント（ライブ更新ポーリング用）。nonzero であること・
    // 2 回連続で呼んでも同じ値であることを呼び出し側（オーケストレータ）が確認する。
    let fingerprint = match &is_url {
        Some(url) => study_fingerprint_url(url, study_id)?,
        None => sqlite::study_fingerprint(Path::new(target), study_id)?,
    };
    println!("fingerprint_total_trials={}", fingerprint.total_trials);
    println!(
        "fingerprint_completed_trials={}",
        fingerprint.completed_trials
    );
    println!("fingerprint_max_trial_id={}", fingerprint.max_trial_id);
    println!(
        "fingerprint_intermediate_count={}",
        fingerprint.intermediate_count
    );
    println!("fingerprint_state_digest={}", fingerprint.state_digest);

    Ok(())
}

fn run(target: &str, study_id: Option<u32>) -> Result<(), String> {
    match study_id {
        None => print_study_list(target),
        Some(id) => print_study_detail(target, id),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        eprintln!("usage: rdb_smoke <url-or-sqlite-path> [study_id]");
        return ExitCode::FAILURE;
    }
    let target = &args[1];
    let study_id = match args.get(2) {
        Some(s) => match s.parse::<u32>() {
            Ok(v) => Some(v),
            Err(_) => {
                eprintln!("error=invalid study_id: {s}");
                return ExitCode::FAILURE;
            }
        },
        None => None,
    };

    match run(target, study_id) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error={e}");
            ExitCode::FAILURE
        }
    }
}
