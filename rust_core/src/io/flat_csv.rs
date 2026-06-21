//! フラット CSV（1 行 = 1 トライアル）形式の最適化結果インポート。
//!
//! Optuna Journal とは異なる外部最適化プログラムの出力に対応する。
//! ヘッダのラベル接頭辞で列の役割を判別する:
//!   - `in:<name>`  → パラメータ（変数）
//!   - `out:<name>` → 目的関数の評価値
//!   - `img`        → アーティファクト（CSV と同じディレクトリにある画像等のファイル名）
//!
//! それ以外の列は user_attr として取り込む（数値なら numeric、非数値なら string）。
//! 最適化方向は CSV に情報が無いため全目的を Minimize とみなす（Journal の未知方向と同じ既定）。

use std::collections::HashMap;

use crate::dataframe::{DataFrame, TrialRow};
use crate::io::journal::parser::{OptimizationDirection, StudyMeta};

/// フラット CSV のパース結果。
pub struct FlatCsvParseResult {
    /// 単一 Study のメタ情報（`study_id` は 0 固定）。
    pub meta: StudyMeta,
    /// 構築済み DataFrame。
    pub dataframe: DataFrame,
    /// trial_id → 画像ファイル名（`img` 列）。空セルの行は含まない。
    pub images: Vec<(u32, String)>,
}

/// 列の役割。ヘッダの接頭辞から判別する。
enum ColumnRole {
    Param(String),
    Objective(String),
    /// `img` 列。アーティファクトのファイル名を持つ。
    Image,
    /// `in:`/`out:`/`img` 以外。user_attr として取り込む。
    UserAttr(String),
}

fn classify_header(header: &str) -> ColumnRole {
    let h = header.trim();
    if let Some(name) = h.strip_prefix("in:") {
        ColumnRole::Param(name.trim().to_string())
    } else if let Some(name) = h.strip_prefix("out:") {
        ColumnRole::Objective(name.trim().to_string())
    } else if h.eq_ignore_ascii_case("img") {
        ColumnRole::Image
    } else {
        ColumnRole::UserAttr(h.to_string())
    }
}

/// RFC 4180 準拠の最小 CSV 行パーサ。ダブルクォート囲み・`""` エスケープ・
/// クォート内カンマを扱う。改行はクォート内に現れない前提（1 行 = 1 レコード）。
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                fields.push(std::mem::take(&mut field));
            }
            _ => field.push(c),
        }
    }
    fields.push(field);
    fields
}

/// フラット CSV のバイト列をパースして単一 Study を構築する。
///
/// `study_name` は Study 名（通常はファイル名）。失敗時はエラーメッセージを返す。
pub fn parse_flat_csv(data: &[u8], study_name: &str) -> Result<FlatCsvParseResult, String> {
    let text = String::from_utf8_lossy(data);
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());

    let header_line = lines.next().ok_or("CSV is empty")?;
    let roles: Vec<ColumnRole> = parse_csv_line(header_line)
        .iter()
        .map(|h| classify_header(h))
        .collect();

    // 列順を保ったまま役割ごとの名前リストを作る。
    let mut param_names: Vec<String> = Vec::new();
    let mut objective_names: Vec<String> = Vec::new();
    for role in &roles {
        match role {
            ColumnRole::Param(n) => param_names.push(n.clone()),
            ColumnRole::Objective(n) => objective_names.push(n.clone()),
            _ => {}
        }
    }
    if objective_names.is_empty() {
        return Err(
            "CSV has no objective columns (expected at least one 'out:' header)".to_string(),
        );
    }

    // パラメータ列が数値かカテゴリかを判定するため、全セルを一旦文字列で集める。
    // param_raw[name] = 各行の生文字列。
    let mut param_raw: HashMap<String, Vec<String>> = param_names
        .iter()
        .map(|n| (n.clone(), Vec::new()))
        .collect();
    let mut obj_raw: HashMap<String, Vec<String>> = objective_names
        .iter()
        .map(|n| (n.clone(), Vec::new()))
        .collect();
    // user_attr 列名（出現順）。
    let mut user_attr_names: Vec<String> = Vec::new();
    let mut user_attr_raw: HashMap<String, Vec<String>> = HashMap::new();
    for role in &roles {
        if let ColumnRole::UserAttr(n) = role {
            if !user_attr_raw.contains_key(n) {
                user_attr_names.push(n.clone());
                user_attr_raw.insert(n.clone(), Vec::new());
            }
        }
    }

    let mut images: Vec<(u32, String)> = Vec::new();
    let mut row_count: u32 = 0;

    for line in lines {
        let fields = parse_csv_line(line);
        let trial_id = row_count;
        for (idx, role) in roles.iter().enumerate() {
            let cell = fields.get(idx).map(|s| s.trim()).unwrap_or("");
            match role {
                ColumnRole::Param(n) => param_raw.get_mut(n).unwrap().push(cell.to_string()),
                ColumnRole::Objective(n) => obj_raw.get_mut(n).unwrap().push(cell.to_string()),
                ColumnRole::Image => {
                    if !cell.is_empty() {
                        images.push((trial_id, cell.to_string()));
                    }
                }
                ColumnRole::UserAttr(n) => user_attr_raw.get_mut(n).unwrap().push(cell.to_string()),
            }
        }
        row_count += 1;
    }

    if row_count == 0 {
        return Err("CSV has a header but no data rows".to_string());
    }

    // パラメータ列ごとに数値判定。全行が f64 にパースできれば numeric、さもなくば categorical。
    // param_bounds は numeric 列の観測 min/max を採用する（サロゲート最適化の探索箱に使う）。
    let mut param_numeric: HashMap<String, Vec<f64>> = HashMap::new();
    let mut param_category: HashMap<String, Vec<String>> = HashMap::new();
    let mut param_bounds: HashMap<String, (f64, f64)> = HashMap::new();
    for name in &param_names {
        let raw = &param_raw[name];
        let parsed: Option<Vec<f64>> = raw.iter().map(|s| s.parse::<f64>().ok()).collect();
        match parsed {
            Some(vals) => {
                if let (Some(&lo), Some(&hi)) = (
                    vals.iter().min_by(|a, b| a.total_cmp(b)),
                    vals.iter().max_by(|a, b| a.total_cmp(b)),
                ) {
                    param_bounds.insert(name.clone(), (lo, hi));
                }
                param_numeric.insert(name.clone(), vals);
            }
            None => {
                param_category.insert(name.clone(), raw.clone());
            }
        }
    }

    // user_attr 列ごとに数値/文字列を判定する。
    let mut user_attr_numeric_names: Vec<String> = Vec::new();
    let mut user_attr_string_names: Vec<String> = Vec::new();
    let mut ua_numeric: HashMap<String, Vec<f64>> = HashMap::new();
    let mut ua_string: HashMap<String, Vec<String>> = HashMap::new();
    for name in &user_attr_names {
        let raw = &user_attr_raw[name];
        let parsed: Option<Vec<f64>> = raw.iter().map(|s| s.parse::<f64>().ok()).collect();
        match parsed {
            Some(vals) => {
                user_attr_numeric_names.push(name.clone());
                ua_numeric.insert(name.clone(), vals);
            }
            None => {
                user_attr_string_names.push(name.clone());
                ua_string.insert(name.clone(), raw.clone());
            }
        }
    }

    // 目的列をパースする（非数値は NaN とする）。
    let obj_parsed: HashMap<String, Vec<f64>> = objective_names
        .iter()
        .map(|name| {
            let vals: Vec<f64> = obj_raw[name]
                .iter()
                .map(|s| s.parse::<f64>().unwrap_or(f64::NAN))
                .collect();
            (name.clone(), vals)
        })
        .collect();

    // 行指向 TrialRow を構築する。
    let mut trial_rows: Vec<TrialRow> = Vec::with_capacity(row_count as usize);
    for row in 0..row_count as usize {
        let mut param_display: HashMap<String, f64> = HashMap::new();
        let mut param_category_label: HashMap<String, String> = HashMap::new();
        for name in &param_names {
            if let Some(vals) = param_numeric.get(name) {
                param_display.insert(name.clone(), vals[row]);
            } else if let Some(vals) = param_category.get(name) {
                param_category_label.insert(name.clone(), vals[row].clone());
            }
        }
        let objective_values: Vec<f64> =
            objective_names.iter().map(|n| obj_parsed[n][row]).collect();
        let mut user_attrs_numeric: HashMap<String, f64> = HashMap::new();
        for name in &user_attr_numeric_names {
            user_attrs_numeric.insert(name.clone(), ua_numeric[name][row]);
        }
        let mut user_attrs_string: HashMap<String, String> = HashMap::new();
        for name in &user_attr_string_names {
            user_attrs_string.insert(name.clone(), ua_string[name][row].clone());
        }
        trial_rows.push(TrialRow {
            trial_id: row as u32,
            trial_number: row as u32,
            param_display,
            param_category_label,
            objective_values,
            user_attrs_numeric,
            user_attrs_string,
            constraint_values: Vec::new(),
        });
    }

    // DataFrame は列名をソート済み前提で扱う（finalize_state と同じ規約）。
    let mut sorted_params = param_names.clone();
    sorted_params.sort();
    let mut sorted_uan = user_attr_numeric_names.clone();
    sorted_uan.sort();
    let mut sorted_uas = user_attr_string_names.clone();
    sorted_uas.sort();

    let dataframe = DataFrame::from_trials(
        &trial_rows,
        &sorted_params,
        &objective_names,
        &sorted_uan,
        &sorted_uas,
        0,
    );

    let mut all_user_attr_names = user_attr_names.clone();
    all_user_attr_names.sort();

    let meta = StudyMeta {
        study_id: 0,
        name: study_name.to_string(),
        directions: objective_names
            .iter()
            .map(|_| OptimizationDirection::Minimize)
            .collect(),
        completed_trials: row_count,
        total_trials: row_count,
        param_names: sorted_params,
        objective_names,
        user_attr_names: all_user_attr_names,
        has_constraints: false,
        param_bounds,
    };

    Ok(FlatCsvParseResult {
        meta,
        dataframe,
        images,
    })
}

#[cfg(test)]
mod tests;
