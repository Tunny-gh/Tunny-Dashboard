#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

pub mod journal_parser;
pub mod dataframe;
pub mod filter;
pub mod pareto;
pub mod clustering;
pub mod sensitivity;
pub mod pdp;
pub mod sampling;
pub mod export;
pub mod live_update;

/// WASM初期化時にパニックハンドラをセットアップする
#[cfg(feature = "wasm")]
#[wasm_bindgen(start)]
pub fn main() {
    // パニック時のデバッグ情報をコンソールに出力（リリースビルドでも有効）
    console_error_panic_hook();
}

#[cfg(feature = "wasm")]
fn console_error_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&format!("WASM Panic: {}", info).into());
    }));
}

// =============================================================================
// WASM公開API
// =============================================================================

/// Optuna Journal ファイルをパースして Study 一覧を JS オブジェクトで返す
///
/// 戻り値: { studies: Study[], durationMs: number }
/// Study: { studyId, name, directions, completedTrials, totalTrials,
///           paramNames, objectiveNames, userAttrNames, hasConstraints }
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "parseJournal")]
pub fn wasm_parse_journal(data: &[u8]) -> Result<JsValue, JsValue> {
    use serde::Serialize;
    use crate::journal_parser::OptimizationDirection;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct JsStudy {
        study_id: u32,
        name: String,
        directions: Vec<String>,
        completed_trials: u32,
        total_trials: u32,
        param_names: Vec<String>,
        objective_names: Vec<String>,
        user_attr_names: Vec<String>,
        has_constraints: bool,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct JsParseResult {
        studies: Vec<JsStudy>,
        duration_ms: f64,
    }

    let result = crate::journal_parser::parse_journal(data)
        .map_err(|e| JsValue::from_str(&e))?;

    let js_result = JsParseResult {
        duration_ms: result.duration_ms,
        studies: result
            .studies
            .into_iter()
            .map(|s| JsStudy {
                study_id: s.study_id,
                name: s.name,
                directions: s
                    .directions
                    .iter()
                    .map(|d| match d {
                        OptimizationDirection::Minimize => "minimize".to_string(),
                        OptimizationDirection::Maximize => "maximize".to_string(),
                    })
                    .collect(),
                completed_trials: s.completed_trials,
                total_trials: s.total_trials,
                param_names: s.param_names,
                objective_names: s.objective_names,
                user_attr_names: s.user_attr_names,
                has_constraints: s.has_constraints,
            })
            .collect(),
    };

    serde_wasm_bindgen::to_value(&js_result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// アクティブ Study を切り替え、GPU バッファを JS オブジェクトで返す
///
/// 戻り値: { positions: ArrayBuffer, positions3d: ArrayBuffer,
///           sizes: ArrayBuffer, trialCount: number }
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "selectStudy")]
pub fn wasm_select_study(study_id: u32) -> Result<JsValue, JsValue> {
    let result = crate::dataframe::select_study(study_id)
        .map_err(|e| JsValue::from_str(&e))?;

    let gpu = result.gpu_buffer_data;

    // Vec<f32> → Float32Array（データをコピー）→ .buffer() で ArrayBuffer を取得
    let pos_arr = js_sys::Float32Array::new_with_length(gpu.positions.len() as u32);
    pos_arr.copy_from(&gpu.positions);

    let pos3d_arr = js_sys::Float32Array::new_with_length(gpu.positions3d.len() as u32);
    pos3d_arr.copy_from(&gpu.positions3d);

    let sizes_arr = js_sys::Float32Array::new_with_length(gpu.sizes.len() as u32);
    sizes_arr.copy_from(&gpu.sizes);

    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"positions".into(), &pos_arr.buffer())
        .map_err(|e| e)?;
    js_sys::Reflect::set(&obj, &"positions3d".into(), &pos3d_arr.buffer())
        .map_err(|e| e)?;
    js_sys::Reflect::set(&obj, &"sizes".into(), &sizes_arr.buffer())
        .map_err(|e| e)?;
    js_sys::Reflect::set(&obj, &"trialCount".into(), &JsValue::from(gpu.trial_count as u32))
        .map_err(|e| e)?;

    Ok(obj.into())
}

/// 範囲条件 JSON を受け取り、条件を満たす trial の行インデックスを Uint32Array で返す
///
/// 引数: ranges_json — `{"col": {"min": 2.0, "max": 8.0}}` 形式の JSON 文字列
/// 戻り値: 条件を満たす行の 0 ベースインデックス（昇順）
/// 存在しない列名は無視、min > max の場合は空配列を返す
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "filterByRanges")]
pub fn wasm_filter_by_ranges(ranges_json: &str) -> Result<JsValue, JsValue> {
    let result = crate::filter::filter_by_ranges(ranges_json);
    let arr = js_sys::Uint32Array::new_with_length(result.len() as u32);
    arr.copy_from(&result);
    Ok(arr.into())
}

/// 選択された試行インデックスと列名 JSON から CSV 文字列を生成して返す
///
/// 引数: indices — 出力対象の行インデックス（JS Array<number>）
///        columns_json — 列名配列 JSON（`[]` の場合は全列）
/// 戻り値: RFC 4180 準拠の UTF-8 CSV 文字列
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "serializeCsv")]
pub fn wasm_serialize_csv(indices: js_sys::Array, columns_json: &str) -> Result<JsValue, JsValue> {
    let indices: Vec<u32> = indices
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0) as u32)
        .collect();
    let result = crate::export::serialize_csv(&indices, columns_json);
    Ok(JsValue::from_str(&result))
}

/// Hypervolume 推移を計算して trialIds / hvValues を返す
///
/// 引数: is_minimize — 各目的関数の最小化フラグ配列（JS Array<boolean>）
/// 戻り値: { trialIds: Uint32Array, hvValues: Float64Array }
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "computeHvHistory")]
pub fn wasm_compute_hv_history(is_minimize: js_sys::Array) -> Result<JsValue, JsValue> {
    let is_minimize: Vec<bool> = is_minimize
        .iter()
        .map(|v| v.as_bool().unwrap_or(true))
        .collect();
    let result = crate::pareto::compute_hypervolume_history(&is_minimize);

    let trial_ids_arr = js_sys::Uint32Array::new_with_length(result.trial_ids.len() as u32);
    trial_ids_arr.copy_from(&result.trial_ids);

    let hv_values_arr = js_sys::Float64Array::new_with_length(result.hv_values.len() as u32);
    hv_values_arr.copy_from(&result.hv_values);

    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"trialIds".into(), &trial_ids_arr).map_err(|e| e)?;
    js_sys::Reflect::set(&obj, &"hvValues".into(), &hv_values_arr).map_err(|e| e)?;

    Ok(obj.into())
}

/// Journal ファイルの差分データを追記し、新規完了試行数と消費バイト数を返す
///
/// 引数: data — Journal ファイルの追加バイト列（Uint8Array）
/// 戻り値: { new_completed: number, consumed_bytes: number }
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "appendJournalDiff")]
pub fn wasm_append_journal_diff(data: &[u8]) -> Result<JsValue, JsValue> {
    let result = crate::live_update::append_journal_diff(data);

    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"new_completed".into(), &JsValue::from(result.new_completed as u32)).map_err(|e| e)?;
    js_sys::Reflect::set(&obj, &"consumed_bytes".into(), &JsValue::from(result.consumed_bytes as u32)).map_err(|e| e)?;

    Ok(obj.into())
}

/// レポート生成用のサマリー統計 JSON を返す
///
/// 戻り値: JSON 文字列 — 各数値列の min/max/mean/std/count
/// アクティブ Study 未選択の場合は `"{}"` を返す
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "computeReportStats")]
pub fn wasm_compute_report_stats() -> Result<JsValue, JsValue> {
    let result = crate::export::compute_report_stats();
    Ok(JsValue::from_str(&result))
}

/// アクティブ Study の全完了トライアルをパラメータ・目的関数値付きで返す
///
/// 戻り値: TrialData[] = [{ trialId, params: Record<string,number>, values: number[], paretoRank: null }]
/// アクティブ Study が未選択の場合は空配列 [] を返す
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "getTrials")]
pub fn wasm_get_trials() -> Result<JsValue, JsValue> {
    use serde::Serialize;
    use std::collections::HashMap;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct JsTrialData {
        trial_id: u32,
        params: HashMap<String, f64>,
        values: Vec<f64>,
        pareto_rank: Option<u32>,
    }

    let trials: Vec<JsTrialData> = crate::dataframe::with_active_df(|df| {
        let param_names = df.param_col_names();
        let obj_names = df.objective_col_names();
        let n = df.row_count();

        (0..n)
            .map(|row| {
                // パラメータ列を走査して HashMap を構築
                let params: HashMap<String, f64> = param_names
                    .iter()
                    .map(|name| {
                        let val = df
                            .get_numeric_column(name)
                            .and_then(|col| col.get(row).copied())
                            .unwrap_or(0.0);
                        (name.clone(), val)
                    })
                    .collect();

                // 目的関数列を走査して Vec を構築
                let values: Vec<f64> = obj_names
                    .iter()
                    .map(|name| {
                        df.get_numeric_column(name)
                            .and_then(|col| col.get(row).copied())
                            .unwrap_or(0.0)
                    })
                    .collect();

                JsTrialData {
                    trial_id: df.get_trial_id(row).unwrap_or(row as u32),
                    params,
                    values,
                    pareto_rank: None,
                }
            })
            .collect()
    })
    .unwrap_or_default();

    serde_wasm_bindgen::to_value(&trials).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn lib_compiles() {
        // 基本的なコンパイル確認
        assert!(true);
    }

    #[test]
    fn wasm_get_trials_no_active_study_returns_empty() {
        // 【テスト目的】: アクティブ Study が未選択のとき with_active_df は None を返し
        //                 unwrap_or_default() で空 Vec になることを確認
        // WASM 環境外では wasm_get_trials() を直接呼べないため、
        // with_active_df が None を返すことを確認する
        let result = crate::dataframe::with_active_df(|_df| 42usize);
        assert!(result.is_none(), "アクティブ Study がない場合は None を返す");
    }

    #[test]
    fn wasm_get_trials_with_dataframe() {
        use crate::dataframe::{DataFrame, TrialRow, store_dataframes, select_study, with_active_df};
        use std::collections::HashMap;

        // テスト用 DataFrame を構築してアクティブにする
        let rows = vec![
            TrialRow {
                trial_id: 0,
                param_display: [("x".to_string(), 1.5)].iter().cloned().collect(),
                param_category_label: HashMap::new(),
                objective_values: vec![10.0],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            },
            TrialRow {
                trial_id: 1,
                param_display: [("x".to_string(), 2.5)].iter().cloned().collect(),
                param_category_label: HashMap::new(),
                objective_values: vec![5.0],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            },
        ];

        let df = DataFrame::from_trials(
            &rows,
            &["x".to_string()],
            &["obj0".to_string()],
            &[],
            &[],
            0,
        );

        // DataFrame を保存してアクティブにする
        store_dataframes(vec![df]);
        select_study(0).unwrap();

        // with_active_df でパラメータ列を確認
        let param_count = with_active_df(|df| df.param_col_names().len()).unwrap_or(0);
        assert_eq!(param_count, 1, "パラメータ列数が一致する");

        let row_count = with_active_df(|df| df.row_count()).unwrap_or(0);
        assert_eq!(row_count, 2, "行数が一致する");

        // パラメータ値の確認
        let x_values: Vec<f64> = with_active_df(|df| {
            df.get_numeric_column("x")
                .map(|col| col.to_vec())
                .unwrap_or_default()
        })
        .unwrap_or_default();
        assert_eq!(x_values, vec![1.5, 2.5], "x パラメータ値が一致する");

        // 目的関数値の確認
        let obj_values: Vec<f64> = with_active_df(|df| {
            df.get_numeric_column("obj0")
                .map(|col| col.to_vec())
                .unwrap_or_default()
        })
        .unwrap_or_default();
        assert_eq!(obj_values, vec![10.0, 5.0], "目的関数値が一致する");
    }
}
