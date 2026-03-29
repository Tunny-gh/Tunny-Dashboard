#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

pub mod clustering;
pub mod dataframe;
pub mod export;
pub mod filter;
pub mod journal_parser;
pub mod live_update;
pub mod pareto;
pub mod pdp;
pub mod sampling;
pub mod sensitivity;

/// Documentation.
#[cfg(feature = "wasm")]
#[wasm_bindgen(start)]
pub fn main() {
    // Documentation.
    console_error_panic_hook();
}

#[cfg(feature = "wasm")]
fn console_error_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&format!("WASM Panic: {}", info).into());
    }));
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
/// Study: { studyId, name, directions, completedTrials, totalTrials,
///           paramNames, objectiveNames, userAttrNames, hasConstraints }
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "parseJournal")]
pub fn wasm_parse_journal(data: &[u8]) -> Result<JsValue, JsValue> {
    use crate::journal_parser::OptimizationDirection;
    use serde::Serialize;

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

    let result = crate::journal_parser::parse_journal(data).map_err(|e| JsValue::from_str(&e))?;

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

    js_result
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Documentation.
///
/// Documentation.
///           sizes: ArrayBuffer, trialCount: number }
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "selectStudy")]
pub fn wasm_select_study(study_id: u32) -> Result<JsValue, JsValue> {
    let result = crate::dataframe::select_study(study_id).map_err(|e| JsValue::from_str(&e))?;

    let gpu = result.gpu_buffer_data;

    // Documentation.
    let pos_arr = js_sys::Float32Array::new_with_length(gpu.positions.len() as u32);
    pos_arr.copy_from(&gpu.positions);

    let pos3d_arr = js_sys::Float32Array::new_with_length(gpu.positions3d.len() as u32);
    pos3d_arr.copy_from(&gpu.positions3d);

    let sizes_arr = js_sys::Float32Array::new_with_length(gpu.sizes.len() as u32);
    sizes_arr.copy_from(&gpu.sizes);

    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"positions".into(), &pos_arr.buffer()).map_err(|e| e)?;
    js_sys::Reflect::set(&obj, &"positions3d".into(), &pos3d_arr.buffer()).map_err(|e| e)?;
    js_sys::Reflect::set(&obj, &"sizes".into(), &sizes_arr.buffer()).map_err(|e| e)?;
    js_sys::Reflect::set(
        &obj,
        &"trialCount".into(),
        &JsValue::from(gpu.trial_count as u32),
    )
    .map_err(|e| e)?;

    Ok(obj.into())
}

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "filterByRanges")]
pub fn wasm_filter_by_ranges(ranges_json: &str) -> Result<JsValue, JsValue> {
    let result = crate::filter::filter_by_ranges(ranges_json);
    let arr = js_sys::Uint32Array::new_with_length(result.len() as u32);
    arr.copy_from(&result);
    Ok(arr.into())
}

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
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

/// Documentation.
///
/// Documentation.
/// Documentation.
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

/// Documentation.
///
/// Documentation.
/// Documentation.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "appendJournalDiff")]
pub fn wasm_append_journal_diff(data: &[u8]) -> Result<JsValue, JsValue> {
    let result = crate::live_update::append_journal_diff(data);

    let obj = js_sys::Object::new();
    js_sys::Reflect::set(
        &obj,
        &"new_completed".into(),
        &JsValue::from(result.new_completed as u32),
    )
    .map_err(|e| e)?;
    js_sys::Reflect::set(
        &obj,
        &"consumed_bytes".into(),
        &JsValue::from(result.consumed_bytes as u32),
    )
    .map_err(|e| e)?;

    Ok(obj.into())
}

/// Documentation.
///
/// Documentation.
/// Documentation.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "computeReportStats")]
pub fn wasm_compute_report_stats() -> Result<JsValue, JsValue> {
    let result = crate::export::compute_report_stats();
    Ok(JsValue::from_str(&result))
}

/// Documentation.
///
/// Documentation.
/// Documentation.
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
                // Documentation.
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

                // Documentation.
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

    trials
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn lib_compiles() {
        // Documentation.
        assert!(true);
    }

    #[test]
    fn wasm_get_trials_no_active_study_returns_empty() {
        // Documentation.
        // Documentation.
        // Documentation.
        // Documentation.
        let result = crate::dataframe::with_active_df(|_df| 42usize);
        assert!(
            result.is_none(),
            "translated Study translated None translated"
        );
    }

    #[test]
    fn wasm_get_trials_with_dataframe() {
        use crate::dataframe::{
            select_study, store_dataframes, with_active_df, DataFrame, TrialRow,
        };
        use std::collections::HashMap;

        // Documentation.
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

        // Documentation.
        store_dataframes(vec![df]);
        select_study(0).unwrap();

        // Documentation.
        let param_count = with_active_df(|df| df.param_col_names().len()).unwrap_or(0);
        assert_eq!(param_count, 1, "parametertranslated");

        let row_count = with_active_df(|df| df.row_count()).unwrap_or(0);
        assert_eq!(row_count, 2, "translated");

        // Documentation.
        let x_values: Vec<f64> = with_active_df(|df| {
            df.get_numeric_column("x")
                .map(|col| col.to_vec())
                .unwrap_or_default()
        })
        .unwrap_or_default();
        assert_eq!(x_values, vec![1.5, 2.5], "x parametertranslated");

        // Documentation.
        let obj_values: Vec<f64> = with_active_df(|df| {
            df.get_numeric_column("obj0")
                .map(|col| col.to_vec())
                .unwrap_or_default()
        })
        .unwrap_or_default();
        assert_eq!(obj_values, vec![10.0, 5.0], "objectivetranslated");
    }
}
