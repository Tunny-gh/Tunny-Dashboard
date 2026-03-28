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

#[cfg(test)]
mod tests {
    #[test]
    fn lib_compiles() {
        // 基本的なコンパイル確認
        assert!(true);
    }
}
