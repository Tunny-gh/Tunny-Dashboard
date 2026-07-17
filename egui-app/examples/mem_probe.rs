//! Memory measurement benchmark harness (TASK-2343 / TASK-2344)
//!
//! Loads an Optuna Journal log and quantitatively compares, via the `dhat` heap
//! profiler, the resident representation at the core of the memory-efficiency
//! refactor — "column-oriented `Arc<DataFrame>` + `StudyView` parallel arrays" — against
//! the row-oriented `Vec<TrialRow>` representation the old implementation (the `main`
//! branch) kept persistently held in `StudyContext`.
//!
//! Measurement metrics (for verifying NFR-001/002/003 / MEM-001/004):
//!   - Column-oriented resident heap (all studies' `DataFrame`s resident in the shared
//!     store)
//!   - Additional overhead of `StudyView` parallel arrays
//!   - Old row-oriented `Vec<TrialRow>` resident heap (including per-row HashMap)
//!   - Peak heap during parsing (load peak; NFR-003)
//!   - Reduction rate from row-oriented to column-oriented (NFR-001: the primary
//!     evidence for a steady-state reduction of -50% or more)
//!
//! Usage:
//!   cargo run --release --example mem_probe -- <journal.log>
//!   (uses mem_eff.log in the current directory if the argument is omitted)
//!
//! dhat also outputs `dhat-heap.json` (viewable in detail with `dhat-view`).

use tunny_core::dataframe::{select_study, snapshot};
use tunny_core::io::journal::parser::parse_journal;
use tunny_desktop::state::types::StudyView;

/// A local reproduction of the old implementation's (`main` branch) trial state enum.
/// It's been removed from the app itself, so it's defined here to keep the memory
/// measurement faithful.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
enum TrialState {
    #[default]
    Complete,
}

/// A local reproduction of the row-oriented representation the old implementation kept
/// persistently held in `StudyContext.trial_rows`.
/// The app itself has migrated to the column-oriented `StudyView` (MEM-001).
/// The fields are never read; they're held purely to reproduce the heap footprint.
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct TrialRow {
    trial_id: u32,
    trial_number: u32,
    params: std::collections::HashMap<String, f64>,
    objectives: Vec<f64>,
    pareto_rank: u32,
    cluster_id: Option<i32>,
    state: TrialState,
    user_attrs: std::collections::HashMap<String, String>,
}

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

/// The current live heap size (bytes).
fn live() -> usize {
    dhat::HeapStats::get().curr_bytes
}

fn mib(bytes: i128) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn main() {
    // Enable HeapStats::get() with `.testing()`.
    let _profiler = dhat::Profiler::builder().testing().build();

    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "mem_eff.log".to_string());

    // Baseline before reading the file (a true baseline that excludes the raw file
    // buffer).
    let baseline = live();

    let read_start = std::time::Instant::now();
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("failed to read {path}: {e}");
            std::process::exit(1);
        }
    };
    let file_bytes = data.len();
    let read_ms = read_start.elapsed().as_secs_f64() * 1000.0;

    // --- Parse (makes all studies resident in the shared store) ---
    let parse_start = std::time::Instant::now();
    let result = match parse_journal(&data) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("parse error: {e}");
            std::process::exit(1);
        }
    };
    let parse_ms = parse_start.elapsed().as_secs_f64() * 1000.0;
    // Record the peak right after parsing (while the raw file buffer `data` is still
    // alive).
    let peak_with_buffer = dhat::HeapStats::get().max_bytes;

    // Free the raw file buffer, leaving only the resident store.
    drop(data);

    let after_parse = live();
    // Resident memory after freeing the raw file buffer = all DataFrame column data
    // (the shared store).
    let store_resident = after_parse as i128 - baseline as i128;

    let studies = &result.studies;
    let total_trials: usize = studies.iter().map(|s| s.completed_trials as usize).sum();

    // --- New representation: build StudyViews for all studies (shared DataFrame +
    // parallel arrays) ---
    let before_views = live();
    let mut views: Vec<StudyView> = Vec::with_capacity(studies.len());
    for s in studies {
        if select_study(s.study_id).is_err() {
            continue;
        }
        if let Some(df) = snapshot(s.study_id) {
            let n = df.row_count();
            views.push(StudyView::new(df, vec![0u32; n]));
        }
    }
    let after_views = live();
    let view_overhead = after_views as i128 - before_views as i128;
    // Steady-state resident memory of the new representation = column-oriented store +
    // StudyView parallel arrays.
    let new_resident = after_views as i128 - baseline as i128;

    // Each StudyView only clones an Arc<DataFrame> reference (no actual data
    // duplication).
    let view_rows: usize = views.iter().map(|v| v.row_count()).sum();

    // --- Old representation: rebuild all studies as row-oriented Vec<TrialRow> ---
    // This is equivalent to the representation the main branch kept persistently held in
    // StudyContext.trial_rows (per-row HashMap<params> + Vec<objectives>).
    let before_rows = live();
    let mut legacy_rows: Vec<Vec<TrialRow>> = Vec::with_capacity(views.len());
    for v in &views {
        let rows: Vec<TrialRow> = (0..v.row_count())
            .map(|i| {
                let mut params = std::collections::HashMap::new();
                for name in v.df.param_col_names() {
                    if let Some(col) = v.df.get_numeric_column(name) {
                        if let Some(val) = col.get(i) {
                            params.insert(name.to_string(), *val);
                        }
                    }
                }
                let objectives: Vec<f64> =
                    v.df.objective_col_names()
                        .iter()
                        .map(|name| {
                            v.df.get_numeric_column(name)
                                .and_then(|c| c.get(i).copied())
                                .unwrap_or(0.0)
                        })
                        .collect();
                TrialRow {
                    trial_id: v.trial_ids.get(i).copied().unwrap_or(i as u32),
                    trial_number: i as u32,
                    params,
                    objectives,
                    pareto_rank: v.pareto_rank.get(i).copied().unwrap_or(0),
                    cluster_id: v.cluster_id.get(i).copied().flatten(),
                    state: TrialState::Complete,
                    user_attrs: std::collections::HashMap::new(),
                }
            })
            .collect();
        legacy_rows.push(rows);
    }
    let after_rows = live();
    let legacy_resident = after_rows as i128 - before_rows as i128;
    let legacy_row_count: usize = legacy_rows.iter().map(|r| r.len()).sum();

    // --- Reduction rate (row-oriented -> column-oriented) ---
    // A fair steady-state comparison: old = row Vec, new = column-oriented store +
    // StudyView arrays.
    let reduction_pct = if legacy_resident > 0 {
        (legacy_resident - new_resident) as f64 / legacy_resident as f64 * 100.0
    } else {
        0.0
    };
    // Bytes-per-trial comparison (representation density).
    let bytes_per_row_legacy = if legacy_row_count > 0 {
        legacy_resident as f64 / legacy_row_count as f64
    } else {
        0.0
    };
    let bytes_per_row_new = if view_rows > 0 {
        new_resident as f64 / view_rows as f64
    } else {
        0.0
    };

    // Keep alive (so the optimizer doesn't free them).
    std::hint::black_box(&views);
    std::hint::black_box(&legacy_rows);

    let n_cols = studies
        .first()
        .map(|s| s.param_names.len() + s.objective_names.len())
        .unwrap_or(0);

    println!("================ mem_probe (memory-efficiency 検証) ================");
    println!("journal             : {path}");
    println!("file size           : {:.1} MiB", mib(file_bytes as i128));
    println!("read time           : {read_ms:.0} ms");
    println!("parse time          : {parse_ms:.0} ms");
    println!("studies             : {}", studies.len());
    println!("total trials         : {total_trials}");
    println!(
        "columns (param+obj) : {n_cols}  (例: {} params + {} obj)",
        studies.first().map(|s| s.param_names.len()).unwrap_or(0),
        studies
            .first()
            .map(|s| s.objective_names.len())
            .unwrap_or(0),
    );
    println!("-------------------------------------------------------------------");
    println!("[load peak]");
    println!(
        "  parse peak (raw buffer 込み) : {:.1} MiB",
        mib(peak_with_buffer as i128)
    );
    println!("-------------------------------------------------------------------");
    println!("[steady state — 新表現: 列指向 Arc<DataFrame> + StudyView]");
    println!(
        "  共有ストア常駐 (全 DataFrame)   : {:.1} MiB",
        mib(store_resident)
    );
    println!(
        "  StudyView 並行配列 追加分       : {:.1} MiB",
        mib(view_overhead)
    );
    println!(
        "  新 定常常駐 合計                : {:.1} MiB  ({:.0} bytes/trial)",
        mib(new_resident),
        bytes_per_row_new
    );
    println!("-------------------------------------------------------------------");
    println!("[steady state — 旧表現: 行指向 Vec<TrialRow> (main 相当)]");
    println!(
        "  旧 行 Vec 常駐 合計            : {:.1} MiB  ({:.0} bytes/trial)",
        mib(legacy_resident),
        bytes_per_row_legacy
    );
    println!("-------------------------------------------------------------------");
    println!(
        "[削減率] 定常メモリ: 旧 {:.1} MiB → 新 {:.1} MiB  = -{reduction_pct:.1}%  (NFR-001 目標: -50% 以上)",
        mib(legacy_resident),
        mib(new_resident),
    );
    let verdict = if reduction_pct >= 50.0 {
        "PASS (>= -50%)"
    } else {
        "REVIEW (< -50%)"
    };
    println!("[NFR-001 判定] {verdict}");
    println!("===================================================================");
}
