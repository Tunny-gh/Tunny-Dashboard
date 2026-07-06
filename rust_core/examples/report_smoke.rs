//! 自己完結レポート出力（HTML / Markdown / JSON）の E2E 動作確認用 CLI。
//!
//! ```text
//! cargo run -p tunny-core --example report_smoke -- \
//!     <storage> <study_id> <out_dir> [--lang en|ja] [--top-n N]
//! ```
//!
//! - `<storage>`: SQLite ファイルパスか RDB URL（`postgresql://` 系）。
//!   `RdbUrl::parse` が URL とみなせば `parse_single_study_url` へ、そうでなければ
//!   ローカルファイルとみなし `sqlite::parse_single_study` へディスパッチする。
//! - `<study_id>`: 対象 study の ID。
//! - `<out_dir>`: 出力先ディレクトリ（`report_{study}.{html,md,json}` の3形式を書く）。
//! - `--lang`: 文章言語（既定 `en`）。
//! - `--top-n`: 上位表の件数（既定 10）。
//!
//! `storage_display` は URL の場合 `RdbUrl::masked()`（生パスワードを残さない）、
//! ファイルの場合はパスそのものを用いる。`generated_at_unix` は `SystemTime` から
//! 取得する（`core` ライブラリは時計を持たないが、example では利用してよい）。

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use tunny_core::data::dataframe::DataFrame;
use tunny_core::data::extras::StudyExtras;
use tunny_core::io::rdb::{parse_single_study_url, RdbUrl};
use tunny_core::io::sqlite;
use tunny_core::journal_parser::StudyMeta;
use tunny_core::report::{render_html, render_markdown};
use tunny_core::{build_study_report, ReportLang, ReportOptions, ReportSource};

/// パース済みコマンドライン引数。
struct Args {
    storage: String,
    study_id: u32,
    out_dir: PathBuf,
    lang: ReportLang,
    top_n: usize,
}

fn parse_args(argv: &[String]) -> Result<Args, String> {
    let mut positional: Vec<&String> = Vec::new();
    let mut lang = ReportLang::En;
    let mut top_n = 10usize;

    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--lang" => {
                let v = argv
                    .get(i + 1)
                    .ok_or_else(|| "--lang requires a value (en|ja)".to_string())?;
                lang = match v.as_str() {
                    "en" => ReportLang::En,
                    "ja" => ReportLang::Ja,
                    other => return Err(format!("invalid --lang: {other} (expected en|ja)")),
                };
                i += 2;
            }
            "--top-n" => {
                let v = argv
                    .get(i + 1)
                    .ok_or_else(|| "--top-n requires a value".to_string())?;
                top_n = v
                    .parse::<usize>()
                    .map_err(|_| format!("invalid --top-n: {v}"))?;
                i += 2;
            }
            other => {
                positional.push(&argv[i]);
                let _ = other;
                i += 1;
            }
        }
    }

    if positional.len() != 3 {
        return Err("expected <storage> <study_id> <out_dir>".to_string());
    }
    let study_id = positional[1]
        .parse::<u32>()
        .map_err(|_| format!("invalid study_id: {}", positional[1]))?;

    Ok(Args {
        storage: positional[0].clone(),
        study_id,
        out_dir: PathBuf::from(positional[2]),
        lang,
        top_n,
    })
}

/// storage をディスパッチして `(meta, df, extras, storage_display)` を得る。
fn load_study(
    storage: &str,
    study_id: u32,
) -> Result<(StudyMeta, DataFrame, StudyExtras, String), String> {
    match RdbUrl::parse(storage) {
        Some(url) => {
            let masked = url.masked();
            let (meta, df, extras) = parse_single_study_url(&url, study_id)?;
            Ok((meta, df, extras, masked))
        }
        None => {
            let (meta, df, extras) = sqlite::parse_single_study(Path::new(storage), study_id)?;
            Ok((meta, df, extras, storage.to_string()))
        }
    }
}

/// ファイル名に使えない文字を `_` に置換し、study 名をサニタイズする。
fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('_');
    if trimmed.is_empty() {
        "study".to_string()
    } else {
        trimmed.to_string()
    }
}

fn run(args: &Args) -> Result<(), String> {
    let (meta, df, extras, storage_display) = load_study(&args.storage, args.study_id)?;

    let generated_at_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64);

    let source = ReportSource {
        storage_display,
        generated_at_unix,
    };
    let opts = ReportOptions {
        lang: args.lang,
        top_n: args.top_n,
        ..ReportOptions::default()
    };

    let report = build_study_report(&meta, &df, Some(&extras), &source, &opts);

    let html = render_html(&report, args.lang);
    let markdown = render_markdown(&report, args.lang);
    let json = serde_json::to_string_pretty(&report)
        .map_err(|e| format!("JSON serialization failed: {e}"))?;

    std::fs::create_dir_all(&args.out_dir)
        .map_err(|e| format!("failed to create out_dir {}: {e}", args.out_dir.display()))?;

    let stem = format!("report_{}", sanitize(&meta.name));
    let write = |ext: &str, contents: &str| -> Result<PathBuf, String> {
        let path = args.out_dir.join(format!("{stem}.{ext}"));
        std::fs::write(&path, contents)
            .map_err(|e| format!("failed to write {}: {e}", path.display()))?;
        Ok(path)
    };

    let html_path = write("html", &html)?;
    let md_path = write("md", &markdown)?;
    let json_path = write("json", &json)?;

    println!("html={}", html_path.display());
    println!("md={}", md_path.display());
    println!("json={}", json_path.display());

    Ok(())
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse_args(&argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error={e}");
            eprintln!(
                "usage: report_smoke <storage> <study_id> <out_dir> [--lang en|ja] [--top-n N]"
            );
            return ExitCode::FAILURE;
        }
    };

    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error={e}");
            ExitCode::FAILURE
        }
    }
}
