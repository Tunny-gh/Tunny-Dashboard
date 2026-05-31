//! メモリ計測ベンチマーク基盤（TASK-2343 / TASK-2344）
//!
//! Optuna Journal ログをロードし、memory-efficiency リファクタの中核である
//! 「列指向 `Arc<DataFrame>` + `StudyView` 並行配列」常駐表現と、旧実装
//! （`main` ブランチ）が `StudyContext` に永続保持していた行指向
//! `Vec<TrialRow>` 表現とを、`dhat` ヒーププロファイラで定量比較する。
//!
//! 計測指標（NFR-001/002/003 / MEM-001/004 検証用）:
//!   - 列指向常駐ヒープ（全 study の `DataFrame` を共有ストアに常駐）
//!   - `StudyView` 並行配列の追加オーバーヘッド
//!   - 旧 行指向 `Vec<TrialRow>` 常駐ヒープ（per-row HashMap 込み）
//!   - パース時ピークヒープ（ロードピーク; NFR-003）
//!   - 列指向 / 行指向の削減率（NFR-001: 定常 -50% 以上の主証拠）
//!
//! 実行:
//!   cargo run --release --example mem_probe -- <journal.log>
//!   （引数省略時はカレントの mem_eff.log を使用）
//!
//! dhat は `dhat-heap.json` も出力する（`dhat-view` で詳細閲覧可）。

use tunny_core::dataframe::{select_study, snapshot};
use tunny_core::io::journal::parser::parse_journal;
use tunny_desktop::state::types::{StudyView, TrialRow, TrialState};

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

/// 現在の生存ヒープ量（bytes）。
fn live() -> usize {
    dhat::HeapStats::get().curr_bytes
}

fn mib(bytes: i128) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn main() {
    // `.testing()` で HeapStats::get() を有効化する。
    let _profiler = dhat::Profiler::builder().testing().build();

    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "mem_eff.log".to_string());

    // ファイル読込み前のベースライン（生ファイルバッファを含まない真の基準）。
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

    // --- パース（全 study を共有ストアへ常駐化） ---
    let parse_start = std::time::Instant::now();
    let result = match parse_journal(&data) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("parse error: {e}");
            std::process::exit(1);
        }
    };
    let parse_ms = parse_start.elapsed().as_secs_f64() * 1000.0;
    // パース直後（生ファイルバッファ data がまだ生存）のピークを記録。
    let peak_with_buffer = dhat::HeapStats::get().max_bytes;

    // 生ファイルバッファを解放して常駐ストアのみを残す。
    drop(data);

    let after_parse = live();
    // 生ファイルバッファ解放後の常駐 = 全 DataFrame 列データ（共有ストア）。
    let store_resident = after_parse as i128 - baseline as i128;

    let studies = &result.studies;
    let total_trials: usize = studies.iter().map(|s| s.completed_trials as usize).sum();

    // --- 新表現: 全 study の StudyView を構築（共有 DataFrame + 並行配列） ---
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
    // 新表現の定常常駐 = 列指向ストア + StudyView 並行配列。
    let new_resident = after_views as i128 - baseline as i128;

    // 各 StudyView は Arc<DataFrame> をクローン参照するだけ（実データ複製なし）。
    let view_rows: usize = views.iter().map(|v| v.row_count()).sum();

    // --- 旧表現: 全 study を行指向 Vec<TrialRow> として再構築 ---
    // これは main ブランチが StudyContext.trial_rows に永続保持していた表現と
    // 等価（per-row HashMap<params> + Vec<objectives>）。
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
                let objectives: Vec<f64> = v
                    .df
                    .objective_col_names()
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
                    state: v.state.get(i).cloned().unwrap_or(TrialState::Complete),
                    user_attrs: std::collections::HashMap::new(),
                }
            })
            .collect();
        legacy_rows.push(rows);
    }
    let after_rows = live();
    let legacy_resident = after_rows as i128 - before_rows as i128;
    let legacy_row_count: usize = legacy_rows.iter().map(|r| r.len()).sum();

    // --- 削減率（行指向→列指向）---
    // 公平な定常比較: 旧 = 行 Vec、新 = 列指向ストア + StudyView 配列。
    let reduction_pct = if legacy_resident > 0 {
        (legacy_resident - new_resident) as f64 / legacy_resident as f64 * 100.0
    } else {
        0.0
    };
    // バイト/試行の比較（表現密度）。
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

    // 生存維持（最適化で解放されないように）。
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
        studies.first().map(|s| s.objective_names.len()).unwrap_or(0),
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
