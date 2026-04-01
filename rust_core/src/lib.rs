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

/// Documentation.
///
/// Documentation.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "computeSensitivity")]
pub fn wasm_compute_sensitivity() -> Result<JsValue, JsValue> {
    use serde::Serialize;
    let start = js_sys::Date::now();
    match sensitivity::compute_sensitivity() {
        Some(result) => {
            let duration_ms = js_sys::Date::now() - start;
            let output = serde_json::json!({
                "spearman": result.spearman,
                "ridge": result.ridge.iter().map(|r| serde_json::json!({"beta": r.beta, "rSquared": r.r_squared})).collect::<Vec<_>>(),
                "paramNames": result.param_names,
                "objectiveNames": result.objective_names,
                "durationMs": duration_ms,
            });
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            output
                .serialize(&serializer)
                .map_err(|e| JsValue::from_str(&e.to_string()))
        }
        None => Err(JsValue::from_str("No active study")),
    }
}

/// Documentation.
///
/// Documentation.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "computeSensitivitySelected")]
pub fn wasm_compute_sensitivity_selected(indices: js_sys::Uint32Array) -> Result<JsValue, JsValue> {
    use serde::Serialize;
    if indices.length() == 0 {
        return Err(JsValue::from_str("Empty selection"));
    }
    let idx_vec: Vec<u32> = indices.to_vec();
    let start = js_sys::Date::now();
    match sensitivity::compute_sensitivity_selected(&idx_vec) {
        Some(result) => {
            let duration_ms = js_sys::Date::now() - start;
            let output = serde_json::json!({
                "spearman": result.spearman,
                "ridge": result.ridge.iter().map(|r| serde_json::json!({"beta": r.beta, "rSquared": r.r_squared})).collect::<Vec<_>>(),
                "paramNames": result.param_names,
                "objectiveNames": result.objective_names,
                "durationMs": duration_ms,
            });
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            output
                .serialize(&serializer)
                .map_err(|e| JsValue::from_str(&e.to_string()))
        }
        None => Err(JsValue::from_str("No active study")),
    }
}

/// Sobol 感度指数（一次・全効果）を計算する
/// n_samples: Saltelli サンプリング数（推奨: 1024）
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "computeSobol")]
pub fn wasm_compute_sobol(n_samples: u32) -> Result<JsValue, JsValue> {
    use serde::Serialize;
    let start = js_sys::Date::now();
    match sensitivity::compute_sobol(n_samples as usize) {
        Some(result) => {
            let duration_ms = js_sys::Date::now() - start;
            let output = serde_json::json!({
                "paramNames":     result.param_names,
                "objectiveNames": result.objective_names,
                "firstOrder":     result.first_order,
                "totalEffect":    result.total_effect,
                "nSamples":       result.n_samples,
                "durationMs":     duration_ms,
            });
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            output
                .serialize(&serializer)
                .map_err(|e| JsValue::from_str(&e.to_string()))
        }
        None => Err(JsValue::from_str("No active study")),
    }
}

/// Documentation.
///
/// Documentation.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "runPca")]
pub fn wasm_run_pca(n_components: u32, space: &str) -> Result<JsValue, JsValue> {
    use serde::Serialize;
    let pca_space = match space {
        "param" => clustering::PcaSpace::Param,
        "objective" => clustering::PcaSpace::Objective,
        "all" => clustering::PcaSpace::All,
        _ => return Err(JsValue::from_str("Invalid space")),
    };
    let start = js_sys::Date::now();
    match clustering::run_pca(n_components as usize, pca_space) {
        Some(result) => {
            let duration_ms = js_sys::Date::now() - start;
            let output = serde_json::json!({
                "projections": result.projections,
                "explainedVariance": result.explained_variance,
                "featureNames": result.feature_names,
                "durationMs": duration_ms,
            });
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            output
                .serialize(&serializer)
                .map_err(|e| JsValue::from_str(&e.to_string()))
        }
        None => Err(JsValue::from_str("Insufficient data for PCA")),
    }
}

/// Documentation.
///
/// Documentation.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "runKmeans")]
pub fn wasm_run_kmeans(
    k: u32,
    data: js_sys::Float64Array,
    n_cols: u32,
) -> Result<JsValue, JsValue> {
    use serde::Serialize;
    let data_vec: Vec<f64> = data.to_vec();
    let start = js_sys::Date::now();
    let result = clustering::run_kmeans(k as usize, &data_vec, n_cols as usize);
    let duration_ms = js_sys::Date::now() - start;
    let output = serde_json::json!({
        "labels": result.labels,
        "centroids": result.centroids,
        "wcss": result.wcss,
        "durationMs": duration_ms,
    });
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    output
        .serialize(&serializer)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Documentation.
///
/// Documentation.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "estimateKElbow")]
pub fn wasm_estimate_k_elbow(
    data: js_sys::Float64Array,
    n_cols: u32,
    max_k: u32,
) -> Result<JsValue, JsValue> {
    use serde::Serialize;
    let data_vec: Vec<f64> = data.to_vec();
    let start = js_sys::Date::now();
    let result = clustering::estimate_k_elbow(&data_vec, n_cols as usize, max_k as usize);
    let duration_ms = js_sys::Date::now() - start;
    let output = serde_json::json!({
        "wcssPerK": result.wcss_per_k,
        "recommendedK": result.recommended_k,
        "durationMs": duration_ms,
    });
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    output
        .serialize(&serializer)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Documentation.
///
/// Documentation.
/// Documentation.
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "computeClusterStats")]
pub fn wasm_compute_cluster_stats(labels: js_sys::Int32Array) -> Result<JsValue, JsValue> {
    use serde::Serialize;
    use std::collections::HashMap;
    let labels_vec: Vec<usize> = labels.to_vec().into_iter().map(|l| l as usize).collect();
    let start = js_sys::Date::now();
    let stats = clustering::compute_cluster_stats(&labels_vec);
    // Get feature names after compute_cluster_stats releases the dataframe borrow
    let feature_names = crate::dataframe::with_active_df(|df| {
        let mut names = df.param_col_names().to_vec();
        names.extend_from_slice(df.objective_col_names());
        names
    })
    .unwrap_or_default();
    let duration_ms = js_sys::Date::now() - start;
    let output = serde_json::json!({
        "stats": stats.iter().map(|s| {
            let centroid_map: HashMap<String, f64> = feature_names.iter()
                .zip(s.centroid.iter())
                .map(|(k, &v)| (k.clone(), v))
                .collect();
            let std_map: HashMap<String, f64> = feature_names.iter()
                .zip(s.std_dev.iter())
                .map(|(k, &v)| (k.clone(), v))
                .collect();
            let significant_diffs: Vec<&String> = feature_names.iter()
                .zip(s.significant_features.iter())
                .filter(|(_, &sig)| sig)
                .map(|(name, _)| name)
                .collect();
            serde_json::json!({
                "clusterId": s.cluster_id,
                "size": s.size,
                "centroid": centroid_map,
                "std": std_map,
                "significantDiffs": significant_diffs,
            })
        }).collect::<Vec<_>>(),
        "durationMs": duration_ms,
    });
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    output
        .serialize(&serializer)
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
