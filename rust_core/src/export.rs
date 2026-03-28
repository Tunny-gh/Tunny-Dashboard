//! CSV シリアライズ・レポート統計 (TASK-1101)
//!
//! 【役割】: DataFrame の指定行・列を CSV 文字列にシリアライズする
//! 【設計方針】:
//!   - `serialize_csv(indices, columns_json)`: 指定インデックス + 列名リストで CSV を生成
//!   - `compute_report_stats()`: HTMLレポート向けサマリー統計（TASK-1102 で使用）
//!   - 列値が文字列 or 数値かは DataFrame の列種別から自動判定
//!   - CSV フォーマット: RFC 4180 準拠（ヘッダ行 + データ行、UTF-8）
//!
//! REQ-150: serialize_csv() — CSV シリアライズ
//! REQ-151: UTF-8 エンコーディング保証
//! REQ-152: 指定列のみ出力
//! REQ-153: 指定インデックスのみ出力
//!
//! 参照: docs/tasks/tunny-dashboard-tasks.md TASK-1101

// =============================================================================
// 定数
// =============================================================================

/// 【CSV 区切り文字】: RFC 4180 標準のカンマ区切り
const CSV_DELIMITER: char = ',';

/// 【CSV エスケープ必要文字】: カンマ・改行・ダブルクォートを含む場合にクォーティング
const NEEDS_QUOTING_CHARS: [char; 3] = [',', '\n', '"'];

// =============================================================================
// 内部ヘルパー
// =============================================================================

/// 【CSV フィールドエスケープ】: RFC 4180 準拠でフィールド値をクォート処理する
///
/// 【処理方針】:
///   - カンマ・改行・ダブルクォートを含む場合はフィールド全体をダブルクォートで囲む
///   - フィールド内のダブルクォートは二重化する（"" エスケープ）🟢
fn escape_csv_field(s: &str) -> String {
    if s.chars().any(|c| NEEDS_QUOTING_CHARS.contains(&c)) {
        // 【RFC 4180 エスケープ】: " → "" にして全体を " で囲む
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// 【数値フォーマット】: f64 を CSV 出力用文字列に変換する
///
/// 【設計方針】:
///   - NaN / Inf は空文字列として出力（Excel 互換）🟡
///   - 整数値は小数点なしで出力（1.0 → "1"）
///   - 小数は最大 10 桁まで表示（後続ゼロ除去）
fn format_f64(v: f64) -> String {
    if v.is_nan() || v.is_infinite() {
        return String::new();
    }
    // 【整数判定】: 整数値なら小数点なしで表示
    if v.fract() == 0.0 && v.abs() < 1e15 {
        return format!("{}", v as i64);
    }
    // 【小数表示】: 後続ゼロを除去した最大 10 桁表示
    let s = format!("{:.10}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

// =============================================================================
// 公開 API
// =============================================================================

/// 【レポート統計計算】: 全数値列のサマリー統計 JSON を返す
///
/// 【戻り値】: `{"col_name":{"min":f64,"max":f64,"mean":f64,"std":f64,"count":usize}, ...}`
///   - 有限値のみ集計（NaN / Inf を除外）
///   - DataFrame が未ロードまたは空の場合は `"{}"` を返す
///
/// 🟢 REQ-154〜REQ-155 (TASK-1102 HTMLレポート向け統計サマリー)
pub fn compute_report_stats() -> String {
    let result = crate::dataframe::with_active_df(|df| compute_report_stats_from_df(df));
    result.unwrap_or_else(|| "{}".to_string())
}

/// 【レポート統計計算（内部）】: DataFrame から直接統計を計算する (テスト用)
pub(crate) fn compute_report_stats_from_df(df: &crate::dataframe::DataFrame) -> String {
    if df.row_count() == 0 {
        return "{}".to_string();
    }

    let mut entries: Vec<String> = Vec::new();

    for col_name in df.column_names() {
        // 【数値列のみ対象】: 文字列列は統計対象外
        if let Some(vals) = df.get_numeric_column(&col_name) {
            // 【有限値のみ集計】: NaN / Inf を除外してから統計を計算
            let finite: Vec<f64> = vals.iter().copied().filter(|v| v.is_finite()).collect();
            if finite.is_empty() {
                continue;
            }

            let count = finite.len();
            let min = finite.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = finite.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let mean = finite.iter().sum::<f64>() / count as f64;

            // 【不偏分散】: count > 1 のとき (n-1) で除算、count=1 のとき 0
            let std = if count > 1 {
                let variance =
                    finite.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (count - 1) as f64;
                variance.sqrt()
            } else {
                0.0
            };

            // 【JSON エントリ生成】: キーのダブルクォートをエスケープ
            let safe_name = col_name.replace('"', "\\\"");
            let entry = format!(
                r#""{}":{{"min":{},"max":{},"mean":{},"std":{},"count":{}}}"#,
                safe_name,
                format_f64(min),
                format_f64(max),
                format_f64(mean),
                format_f64(std),
                count
            );
            entries.push(entry);
        }
    }

    format!("{{{}}}", entries.join(","))
}

/// 【CSV シリアライズ】: 指定行・列を CSV 文字列に変換する
///
/// 【引数】:
///   indices      - 出力対象の行インデックス（0ベース DataFrame 行番号）
///   columns_json - 出力する列名の JSON 配列（例: `["x1","obj0","trial_id"]`）
///                  `"trial_id"` は特殊列として DataFrame の Optuna trial_id を出力
///
/// 【戻り値】: UTF-8 CSV 文字列（ヘッダ行 + データ行）
///   - 列が見つからない場合は空文字列セル
///   - 行が範囲外の場合は空行をスキップ（出力しない）
///
/// 🟢 REQ-150〜REQ-153
pub fn serialize_csv(indices: &[u32], columns_json: &str) -> String {
    // 【列名パース】: JSON 配列から列名リストを取得
    let columns: Vec<String> = parse_columns_json(columns_json);
    if columns.is_empty() {
        return String::new();
    }

    // 【DataFrame アクセス】: アクティブ DataFrame を参照して CSV を構築
    let result = crate::dataframe::with_active_df(|df| {
        serialize_csv_from_df(df, indices, &columns)
    });

    result.unwrap_or_default()
}

/// 【CSV シリアライズ（内部）】: DataFrame から直接シリアライズする
///
/// 【pub(crate) 公開】: テスト用に DataFrame を直接受け取る版
pub(crate) fn serialize_csv_from_df(
    df: &crate::dataframe::DataFrame,
    indices: &[u32],
    columns: &[String],
) -> String {
    let n = df.row_count();
    let mut out = String::with_capacity(indices.len() * columns.len() * 10);

    // 【ヘッダ行出力】: 列名をエスケープして CSV 出力
    let header_fields: Vec<String> = columns.iter().map(|c| escape_csv_field(c)).collect();
    out.push_str(&header_fields.join(&CSV_DELIMITER.to_string()));
    out.push('\n');

    // 【データ行出力】: 各インデックスの行を出力
    for &idx in indices {
        let row = idx as usize;
        if row >= n {
            // 【範囲外スキップ】: 範囲外インデックスは無視
            continue;
        }

        let mut fields = Vec::with_capacity(columns.len());
        for col in columns {
            let field = get_cell_value(df, row, col);
            fields.push(escape_csv_field(&field));
        }
        out.push_str(&fields.join(&CSV_DELIMITER.to_string()));
        out.push('\n');
    }

    out
}

/// 【セル値取得】: DataFrame の指定行・列の値を文字列として返す
///
/// 【優先順位】:
///   1. "trial_id" 特殊列 → Optuna trial_id
///   2. 数値列 → format_f64()
///   3. 文字列列 → そのまま
///   4. 見つからない場合 → 空文字列
fn get_cell_value(df: &crate::dataframe::DataFrame, row: usize, col: &str) -> String {
    // 【特殊列: trial_id】: Optuna の実際の trial_id を返す 🟢
    if col == "trial_id" {
        return df
            .get_trial_id(row)
            .map(|id| id.to_string())
            .unwrap_or_default();
    }

    // 【数値列】: get_numeric_column で値を取得
    if let Some(vals) = df.get_numeric_column(col) {
        if let Some(&v) = vals.get(row) {
            return format_f64(v);
        }
        return String::new();
    }

    // 【文字列列】: get_string_column で値を取得
    if let Some(vals) = df.get_string_column(col) {
        return vals.get(row).cloned().unwrap_or_default();
    }

    // 【列なし】: 空文字列を返す
    String::new()
}

// =============================================================================
// JSON パース
// =============================================================================

/// 【列名 JSON パース】: `["col1","col2",...]` 形式の JSON 配列をパースする
///
/// 【設計方針】: serde_json 非依存のシンプルな実装（文字列配列のみ対応）🟡
fn parse_columns_json(json: &str) -> Vec<String> {
    let trimmed = json.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return vec![];
    }

    let inner = &trimmed[1..trimmed.len() - 1];
    if inner.trim().is_empty() {
        return vec![];
    }

    // 【シンプルパース】: ","区切りで分割し、前後の空白・クォートを除去
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;

    for ch in inner.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            ',' if !in_string => {
                let s = current.trim().to_string();
                if !s.is_empty() {
                    result.push(s);
                }
                current.clear();
            }
            _ if in_string => current.push(ch),
            _ => {} // 文字列外の空白などは無視
        }
    }
    let s = current.trim().to_string();
    if !s.is_empty() {
        result.push(s);
    }

    result
}

// =============================================================================
// テスト
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{DataFrame, TrialRow};
    use std::collections::HashMap;

    /// テスト用 DataFrame を構築するヘルパー
    fn make_test_df() -> DataFrame {
        let rows = vec![
            TrialRow {
                trial_id: 0,
                param_display: {
                    let mut m = HashMap::new();
                    m.insert("x1".to_string(), 1.5);
                    m.insert("x2".to_string(), 2.0);
                    m
                },
                param_category_label: HashMap::new(),
                objective_values: vec![10.0, 20.0],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            },
            TrialRow {
                trial_id: 5,
                param_display: {
                    let mut m = HashMap::new();
                    m.insert("x1".to_string(), 3.0);
                    m.insert("x2".to_string(), 4.5);
                    m
                },
                param_category_label: HashMap::new(),
                objective_values: vec![30.0, 40.0],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            },
        ];

        DataFrame::from_trials(
            &rows,
            &["x1".to_string(), "x2".to_string()],
            &["obj0".to_string(), "obj1".to_string()],
            &[],
            &[],
            0,
        )
    }

    // TC-1101-01: 全インデックス・全列でヘッダ行が生成される
    #[test]
    fn tc_1101_01_csv_header_row() {
        // 【テスト目的】: CSV の最初の行がヘッダ（列名）になることを確認 🟢
        let df = make_test_df();
        let csv = serialize_csv_from_df(&df, &[0, 1], &["trial_id".to_string(), "x1".to_string()]);

        let lines: Vec<&str> = csv.lines().collect();
        // 【確認内容】: ヘッダ行が "trial_id,x1" であること
        assert_eq!(lines[0], "trial_id,x1", "ヘッダ行が一致しない");
    }

    // TC-1101-02: trial_id 特殊列が Optuna trial_id を返す
    #[test]
    fn tc_1101_02_trial_id_column() {
        // 【テスト目的】: "trial_id" 列が DataFrame の actual trial_id を返すことを確認 🟢
        let df = make_test_df();
        let csv = serialize_csv_from_df(&df, &[0, 1], &["trial_id".to_string()]);

        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[1], "0", "row=0 の trial_id が 0 であるべき");
        assert_eq!(lines[2], "5", "row=1 の trial_id が 5 であるべき");
    }

    // TC-1101-03: 数値列の値が正しく出力される
    #[test]
    fn tc_1101_03_numeric_column_values() {
        // 【テスト目的】: 数値列が適切にフォーマットされて出力されることを確認 🟢
        let df = make_test_df();
        let csv =
            serialize_csv_from_df(&df, &[0], &["x1".to_string(), "obj0".to_string()]);

        let lines: Vec<&str> = csv.lines().collect();
        // 【確認内容】: "1.5,10" が2行目に出力されること
        assert_eq!(lines[1], "1.5,10", "数値が正しくフォーマットされていない");
    }

    // TC-1101-04: 指定インデックスのみが出力される
    #[test]
    fn tc_1101_04_index_filtering() {
        // 【テスト目的】: 指定したインデックスの行のみ出力されることを確認 🟢
        let df = make_test_df();
        let csv = serialize_csv_from_df(&df, &[1], &["x1".to_string()]);

        let lines: Vec<&str> = csv.lines().collect();
        // 【確認内容】: データ行が1行だけ（ヘッダ除く）
        assert_eq!(lines.len(), 2, "index=[1] なら出力は2行（ヘッダ+データ1行）");
        assert_eq!(lines[1], "3", "row=1 の x1=3.0 → 整数表示で '3'");
    }

    // TC-1101-05: 範囲外インデックスはスキップされる
    #[test]
    fn tc_1101_05_out_of_range_index_skipped() {
        // 【テスト目的】: 存在しないインデックスが無視されることを確認 🟢
        let df = make_test_df();
        let csv = serialize_csv_from_df(&df, &[99], &["x1".to_string()]);

        let lines: Vec<&str> = csv.lines().collect();
        // 【確認内容】: ヘッダ行のみでデータ行なし
        assert_eq!(lines.len(), 1, "範囲外インデックスはスキップされること");
    }

    // TC-1101-06: カンマ含む文字列は RFC 4180 でクォートされる
    #[test]
    fn tc_1101_06_csv_field_escaping() {
        // 【テスト目的】: カンマを含む列値が正しくクォートされることを確認 🟢
        assert_eq!(escape_csv_field("hello,world"), "\"hello,world\"");
        assert_eq!(escape_csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(escape_csv_field("normal"), "normal");
    }

    // TC-1101-07: 列名 JSON パースが正しく動作する
    #[test]
    fn tc_1101_07_parse_columns_json() {
        // 【テスト目的】: JSON 列名配列が正しくパースされることを確認 🟢
        let cols = parse_columns_json("[\"x1\", \"obj0\", \"trial_id\"]");
        assert_eq!(cols, vec!["x1", "obj0", "trial_id"]);
    }

    // TC-1101-08: 空インデックスで CSV はヘッダのみ
    #[test]
    fn tc_1101_08_empty_indices_header_only() {
        // 【テスト目的】: 空のインデックスリストでヘッダのみ出力されることを確認 🟢
        let df = make_test_df();
        let csv = serialize_csv_from_df(&df, &[], &["x1".to_string()]);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 1, "空インデックスはヘッダ行のみ");
    }

    // TC-1101-09: format_f64 が整数・小数・NaN を正しく処理する
    #[test]
    fn tc_1101_09_format_f64_cases() {
        // 【テスト目的】: 数値フォーマット関数の各ケースを確認 🟢
        assert_eq!(format_f64(1.0), "1");
        assert_eq!(format_f64(1.5), "1.5");
        assert_eq!(format_f64(0.1 + 0.2), "0.3"); // 浮動小数点誤差を含む
        assert_eq!(format_f64(f64::NAN), "");
        assert_eq!(format_f64(f64::INFINITY), "");
    }

    // TC-1101-10: 存在しない列は空文字列セルになる
    #[test]
    fn tc_1101_10_nonexistent_column_empty() {
        // 【テスト目的】: 存在しない列名は空文字列セルになることを確認 🟢
        let df = make_test_df();
        let csv =
            serialize_csv_from_df(&df, &[0], &["nonexistent".to_string()]);
        let lines: Vec<&str> = csv.lines().collect();
        // 【確認内容】: データ行は空セル1つ
        assert_eq!(lines[1], "", "存在しない列は空文字列");
    }

    // TC-1102-01: compute_report_stats が数値列の JSON を返す
    #[test]
    fn tc_1102_01_report_stats_numeric_columns() {
        // 【テスト目的】: compute_report_stats_from_df が数値列の統計 JSON を返すことを確認 🟢
        let df = make_test_df();
        let json = compute_report_stats_from_df(&df);

        // 【確認内容】: JSON が空でないこと
        assert!(!json.is_empty(), "統計 JSON が空であってはならない");
        // 【確認内容】: x1 列が含まれること
        assert!(json.contains("\"x1\""), "x1 列が JSON に含まれるべき");
        // 【確認内容】: min/max/mean/std/count キーが含まれること
        assert!(json.contains("\"min\""), "min フィールドが JSON に含まれるべき");
        assert!(json.contains("\"max\""), "max フィールドが JSON に含まれるべき");
        assert!(json.contains("\"mean\""), "mean フィールドが JSON に含まれるべき");
        assert!(json.contains("\"std\""), "std フィールドが JSON に含まれるべき");
        assert!(json.contains("\"count\""), "count フィールドが JSON に含まれるべき");
    }

    // TC-1102-02: 空 DataFrame では "{}" を返す
    #[test]
    fn tc_1102_02_report_stats_empty_df() {
        // 【テスト目的】: 空の DataFrame では空 JSON オブジェクトを返すことを確認 🟢
        let empty_df = DataFrame::from_trials(&[], &[], &[], &[], &[], 0);
        let json = compute_report_stats_from_df(&empty_df);
        // 【確認内容】: "{}" が返ること
        assert_eq!(json, "{}", "空 DataFrame は {{}} を返すべき");
    }

    // TC-1102-03: min/max/mean が正しく計算される
    #[test]
    fn tc_1102_03_report_stats_correct_values() {
        // 【テスト目的】: x1 列 (1.5, 3.0) の統計値が正しく計算されることを確認 🟢
        let df = make_test_df();
        let json = compute_report_stats_from_df(&df);

        // x1 列: 値は [1.5, 3.0] → min=1.5, max=3, mean=2.25
        // 【確認内容】: min が 1.5 であること
        assert!(json.contains("\"min\":1.5"), "x1 の min=1.5 であるべき: {}", json);
        // 【確認内容】: max が 3 であること
        assert!(json.contains("\"max\":3"), "x1 の max=3 であるべき: {}", json);
        // 【確認内容】: count が 2 であること
        assert!(json.contains("\"count\":2"), "x1 の count=2 であるべき: {}", json);
    }

    // TC-1102-04: 有効な JSON 構造 (波括弧で囲まれている)
    #[test]
    fn tc_1102_04_report_stats_valid_json_structure() {
        // 【テスト目的】: 返り値が { } で囲まれた有効な JSON 形式であることを確認 🟢
        let df = make_test_df();
        let json = compute_report_stats_from_df(&df);
        // 【確認内容】: { で始まり } で終わること
        assert!(json.starts_with('{'), "JSON が {{ で始まるべき");
        assert!(json.ends_with('}'), "JSON が }} で終わるべき");
    }
}
