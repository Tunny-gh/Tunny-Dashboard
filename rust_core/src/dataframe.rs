//! Module documentation.
//!
//! Module documentation.
//! Module documentation.
//! Module documentation.
//!
//! Reference: docs/implements/TASK-102/dataframe-requirements.md

use std::cell::RefCell;
use std::collections::HashMap;

// =============================================================================
// trial data transfer type（journal_parser.rs → DataFrame for construction）
// =============================================================================

/// Documentation.
/// Documentation.
#[derive(Clone)]
pub struct TrialRow {
    /// Documentation.
    pub trial_id: u32,
    /// Documentation.
    pub param_display: HashMap<String, f64>,
    /// Documentation.
    pub param_category_label: HashMap<String, String>,
    /// objectivevalue list（obj0, obj1, ...）
    pub objective_values: Vec<f64>,
    /// user_attr numeric type（REQ-012）
    pub user_attrs_numeric: HashMap<String, f64>,
    /// user_attr string type（REQ-012）
    pub user_attrs_string: HashMap<String, String>,
    /// constraints value list（REQ-013）
    pub constraint_values: Vec<f64>,
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
#[derive(Debug, Clone)]
pub struct DataFrameInfo {
    pub row_count: usize,
    pub column_names: Vec<String>,
    pub param_columns: Vec<String>,
    pub objective_columns: Vec<String>,
    pub user_attr_columns: Vec<String>,
    pub constraint_columns: Vec<String>,
    /// Documentation.
    pub derived_columns: Vec<String>,
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
#[derive(Debug)]
pub struct GpuBufferData {
    /// Documentation.
    pub positions: Vec<f32>,
    /// Documentation.
    pub positions3d: Vec<f32>,
    /// Documentation.
    pub sizes: Vec<f32>,
    pub trial_count: usize,
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
#[derive(Debug)]
pub struct SelectStudyResult {
    pub data_frame_info: DataFrameInfo,
    pub gpu_buffer_data: GpuBufferData,
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
/// Documentation.
#[derive(Clone)]
pub struct DataFrame {
    row_count: usize,
    /// Documentation.
    trial_ids: Vec<u32>,
    /// Documentation.
    numeric_cols: Vec<(String, Vec<f64>)>,
    /// Documentation.
    string_cols: Vec<(String, Vec<String>)>,
    /// Documentation.
    param_col_names: Vec<String>,
    objective_col_names: Vec<String>,
    user_attr_numeric_col_names: Vec<String>,
    user_attr_string_col_names: Vec<String>,
    constraint_col_names: Vec<String>,
    /// derived columns: is_feasible, constraint_sum 🟢
    derived_col_names: Vec<String>,
}

impl DataFrame {
    /// Documentation.
    pub fn empty() -> Self {
        DataFrame {
            row_count: 0,
            trial_ids: vec![],
            numeric_cols: vec![],
            string_cols: vec![],
            param_col_names: vec![],
            objective_col_names: vec![],
            user_attr_numeric_col_names: vec![],
            user_attr_string_col_names: vec![],
            constraint_col_names: vec![],
            derived_col_names: vec![],
        }
    }

    /// Documentation.
    ///
    /// Documentation.
    /// Documentation.
    /// Documentation.
    /// Documentation.
    /// Documentation.
    /// Documentation.
    /// Documentation.
    /// Documentation.
    pub fn from_trials(
        trial_rows: &[TrialRow],
        param_names: &[String],
        objective_names: &[String],
        user_attr_numeric_names: &[String],
        user_attr_string_names: &[String],
        max_constraints: usize,
    ) -> Self {
        let n = trial_rows.len();
        if n == 0 {
            return DataFrame::empty();
        }

        // Documentation.
        let trial_ids: Vec<u32> = trial_rows.iter().map(|r| r.trial_id).collect();

        let mut numeric_cols: Vec<(String, Vec<f64>)> = Vec::new();
        let mut string_cols: Vec<(String, Vec<String>)> = Vec::new();
        let mut param_col_names = Vec::new();
        let mut objective_col_names = Vec::new();
        let mut user_attr_numeric_col_names = Vec::new();
        let mut user_attr_string_col_names = Vec::new();
        let mut constraint_col_names = Vec::new();
        let mut derived_col_names = Vec::new();

        // Documentation.
        // Documentation.
        // Documentation.
        for name in param_names {
            let has_label = trial_rows
                .iter()
                .any(|r| r.param_category_label.contains_key(name));
            if has_label {
                let vals: Vec<String> = trial_rows
                    .iter()
                    .map(|r| {
                        r.param_category_label
                            .get(name)
                            .cloned()
                            .unwrap_or_default()
                    })
                    .collect();
                string_cols.push((name.clone(), vals));
            } else {
                // Documentation.
                let vals: Vec<f64> = trial_rows
                    .iter()
                    .map(|r| *r.param_display.get(name).unwrap_or(&0.0))
                    .collect();
                numeric_cols.push((name.clone(), vals));
            }
            param_col_names.push(name.clone());
        }

        // Documentation.
        // Documentation.
        for (i, name) in objective_names.iter().enumerate() {
            let vals: Vec<f64> = trial_rows
                .iter()
                .map(|r| r.objective_values.get(i).copied().unwrap_or(f64::NAN))
                .collect();
            numeric_cols.push((name.clone(), vals));
            objective_col_names.push(name.clone());
        }

        // Documentation.
        for name in user_attr_numeric_names {
            let vals: Vec<f64> = trial_rows
                .iter()
                .map(|r| *r.user_attrs_numeric.get(name).unwrap_or(&f64::NAN))
                .collect();
            numeric_cols.push((name.clone(), vals));
            user_attr_numeric_col_names.push(name.clone());
        }

        // Documentation.
        for name in user_attr_string_names {
            let vals: Vec<String> = trial_rows
                .iter()
                .map(|r| r.user_attrs_string.get(name).cloned().unwrap_or_default())
                .collect();
            string_cols.push((name.clone(), vals));
            user_attr_string_col_names.push(name.clone());
        }

        // Documentation.
        if max_constraints > 0 {
            for ci in 0..max_constraints {
                let col_name = format!("c{}", ci + 1);
                // Documentation.
                let vals: Vec<f64> = trial_rows
                    .iter()
                    .map(|r| r.constraint_values.get(ci).copied().unwrap_or(0.0))
                    .collect();
                numeric_cols.push((col_name.clone(), vals));
                constraint_col_names.push(col_name);
            }

            // Documentation.
            let is_feasible_vals: Vec<f64> = trial_rows
                .iter()
                .map(|r| {
                    if r.constraint_values.iter().all(|&c| c <= 0.0) {
                        1.0
                    } else {
                        0.0
                    }
                })
                .collect();
            numeric_cols.push(("is_feasible".to_string(), is_feasible_vals));
            derived_col_names.push("is_feasible".to_string());

            // Documentation.
            let sum_vals: Vec<f64> = trial_rows
                .iter()
                .map(|r| r.constraint_values.iter().sum())
                .collect();
            numeric_cols.push(("constraint_sum".to_string(), sum_vals));
            derived_col_names.push("constraint_sum".to_string());
        }

        DataFrame {
            row_count: n,
            trial_ids,
            numeric_cols,
            string_cols,
            param_col_names,
            objective_col_names,
            user_attr_numeric_col_names,
            user_attr_string_col_names,
            constraint_col_names,
            derived_col_names,
        }
    }

    /// Documentation.
    pub fn get_trial_id(&self, row: usize) -> Option<u32> {
        self.trial_ids.get(row).copied()
    }

    /// Documentation.
    pub fn param_col_names(&self) -> &[String] {
        &self.param_col_names
    }
    pub fn objective_col_names(&self) -> &[String] {
        &self.objective_col_names
    }
    pub fn user_attr_numeric_col_names(&self) -> &[String] {
        &self.user_attr_numeric_col_names
    }
    pub fn user_attr_string_col_names(&self) -> &[String] {
        &self.user_attr_string_col_names
    }
    pub fn constraint_col_names(&self) -> &[String] {
        &self.constraint_col_names
    }

    /// Documentation.
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// Documentation.
    pub fn column_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.numeric_cols.iter().map(|(n, _)| n.clone()).collect();
        names.extend(self.string_cols.iter().map(|(n, _)| n.clone()));
        names
    }

    /// Documentation.
    pub fn get_numeric_column(&self, name: &str) -> Option<&[f64]> {
        self.numeric_cols
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_slice())
    }

    /// Documentation.
    pub fn get_string_column(&self, name: &str) -> Option<&[String]> {
        self.string_cols
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_slice())
    }

    /// Documentation.
    pub fn info(&self) -> DataFrameInfo {
        // Documentation.
        let mut all_user_attr: Vec<String> = self.user_attr_numeric_col_names.clone();
        all_user_attr.extend(self.user_attr_string_col_names.iter().cloned());

        DataFrameInfo {
            row_count: self.row_count,
            column_names: self.column_names(),
            param_columns: self.param_col_names.clone(),
            objective_columns: self.objective_col_names.clone(),
            user_attr_columns: all_user_attr,
            constraint_columns: self.constraint_col_names.clone(),
            derived_columns: self.derived_col_names.clone(),
        }
    }

    /// Documentation.
    pub fn gpu_buffers(&self) -> GpuBufferData {
        let n = self.row_count;
        let positions = build_positions(self, n);
        let positions3d = build_positions3d(self, n);
        // Documentation.
        let sizes = vec![1.0f32; n];
        GpuBufferData {
            positions,
            positions3d,
            sizes,
            trial_count: n,
        }
    }
}

// =============================================================================
// Documentation.
// =============================================================================

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
fn build_positions(df: &DataFrame, n: usize) -> Vec<f32> {
    let mut positions = vec![0.0f32; n * 2];
    let obj_names = &df.objective_col_names;

    match obj_names.len() {
        0 => {} // Documentation.
        1 => {
            // Documentation.
            let obj0 = df.get_numeric_column(&obj_names[0]).unwrap_or(&[]);
            // Documentation.
            let x_scale = if n > 1 { (n - 1) as f32 } else { 1.0 };
            for i in 0..n {
                positions[i * 2] = i as f32 / x_scale;
                positions[i * 2 + 1] = obj0.get(i).copied().unwrap_or(f64::NAN) as f32;
            }
        }
        _ => {
            // x: obj0, y: obj1 🟢
            let obj0 = df.get_numeric_column(&obj_names[0]).unwrap_or(&[]);
            let obj1 = df.get_numeric_column(&obj_names[1]).unwrap_or(&[]);
            for i in 0..n {
                positions[i * 2] = obj0.get(i).copied().unwrap_or(f64::NAN) as f32;
                positions[i * 2 + 1] = obj1.get(i).copied().unwrap_or(f64::NAN) as f32;
            }
        }
    }
    positions
}

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
fn build_positions3d(df: &DataFrame, n: usize) -> Vec<f32> {
    let mut positions3d = vec![0.0f32; n * 3];
    let obj_names = &df.objective_col_names;

    if obj_names.is_empty() {
        return positions3d; // zeros
    }

    let obj0 = df.get_numeric_column(&obj_names[0]).unwrap_or(&[]);
    let obj1 = obj_names
        .get(1)
        .and_then(|name| df.get_numeric_column(name));
    let obj2 = obj_names
        .get(2)
        .and_then(|name| df.get_numeric_column(name));
    let x_scale = if n > 1 { (n - 1) as f32 } else { 1.0 };

    for i in 0..n {
        let (x, y, z) = match obj_names.len() {
            1 => {
                // Documentation.
                let x = i as f32 / x_scale;
                let y = obj0.get(i).copied().unwrap_or(f64::NAN) as f32;
                (x, y, 0.0f32)
            }
            2 => {
                // x: obj0, y: obj1, z: 0.0 🟢
                let x = obj0.get(i).copied().unwrap_or(f64::NAN) as f32;
                let y = obj1.and_then(|v| v.get(i)).copied().unwrap_or(f64::NAN) as f32;
                (x, y, 0.0f32)
            }
            _ => {
                // x: obj0, y: obj1, z: obj2 🟢
                let x = obj0.get(i).copied().unwrap_or(f64::NAN) as f32;
                let y = obj1.and_then(|v| v.get(i)).copied().unwrap_or(f64::NAN) as f32;
                let z = obj2.and_then(|v| v.get(i)).copied().unwrap_or(0.0) as f32;
                (x, y, z)
            }
        };
        positions3d[i * 3] = x;
        positions3d[i * 3 + 1] = y;
        positions3d[i * 3 + 2] = z;
    }
    positions3d
}

// =============================================================================
// Documentation.
// =============================================================================

struct GlobalState {
    dataframes: Vec<DataFrame>,
    /// Documentation.
    active_study_id: Option<u32>,
}

thread_local! {
/// Documentation.
/// Documentation.
    static GLOBAL_STATE: RefCell<GlobalState> = RefCell::new(GlobalState {
        dataframes: vec![],
        active_study_id: None,
    });
}

/// Documentation.
///
/// Documentation.
/// Documentation.
pub fn store_dataframes(dfs: Vec<DataFrame>) {
    GLOBAL_STATE.with(|state| {
        let mut s = state.borrow_mut();
        s.dataframes = dfs;
        s.active_study_id = None; // Documentation.
    });
}

/// Documentation.
///
/// Documentation.
/// Documentation.
pub fn select_study(study_id: u32) -> Result<SelectStudyResult, String> {
    GLOBAL_STATE.with(|state| {
        let mut s = state.borrow_mut();
        let result = s
            .dataframes
            .get(study_id as usize)
            .map(|df| SelectStudyResult {
                data_frame_info: df.info(),
                gpu_buffer_data: df.gpu_buffers(),
            })
            .ok_or_else(|| {
                format!(
                    "study_id {} not found (total: {})",
                    study_id,
                    s.dataframes.len()
                )
            });
        if result.is_ok() {
            s.active_study_id = Some(study_id);
        }
        result
    })
}

/// Documentation.
///
/// Documentation.
pub fn with_active_df<T, F: FnOnce(&DataFrame) -> T>(f: F) -> Option<T> {
    GLOBAL_STATE.with(|state| {
        let s = state.borrow();
        let idx = s.active_study_id? as usize;
        s.dataframes.get(idx).map(f)
    })
}

/// Documentation.
pub fn with_df<T, F: FnOnce(&DataFrame) -> T>(study_id: u32, f: F) -> Option<T> {
    GLOBAL_STATE.with(|state| {
        let s = state.borrow();
        s.dataframes.get(study_id as usize).map(f)
    })
}

// =============================================================================
// Documentation.
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Documentation.
    // -------------------------------------------------------------------------

    /// Documentation.
    fn make_trial(params: &[(&str, f64)], objective_values: Vec<f64>) -> TrialRow {
        TrialRow {
            trial_id: 0,
            param_display: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            param_category_label: HashMap::new(),
            objective_values,
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        }
    }

    fn to_bytes(s: &str) -> Vec<u8> {
        s.as_bytes().to_vec()
    }

    // =========================================================================
    // Documentation.
    // =========================================================================

    #[test]
    fn tc_102_01_row_count_single_trial() {
        // Documentation.
        let rows = vec![make_trial(&[("x", 0.5)], vec![1.0])];
        let df = DataFrame::from_trials(
            &rows,
            &["x".to_string()],
            &["obj0".to_string()],
            &[],
            &[],
            0,
        );
        assert_eq!(df.row_count(), 1); // Documentation.
    }

    #[test]
    fn tc_102_02_param_column_values() {
        // Documentation.
        let rows = vec![
            make_trial(&[("x", 0.5), ("y", 2.0)], vec![1.0]),
            make_trial(&[("x", 1.5), ("y", 3.0)], vec![2.0]),
        ];
        let param_names = vec!["x".to_string(), "y".to_string()];
        let df = DataFrame::from_trials(&rows, &param_names, &["obj0".to_string()], &[], &[], 0);
        let x_col = df.get_numeric_column("x").expect("xtranslated");
        assert!((x_col[0] - 0.5).abs() < 1e-9); // Documentation.
        assert!((x_col[1] - 1.5).abs() < 1e-9); // Documentation.
    }

    #[test]
    fn tc_102_03_objective_column_values() {
        // Documentation.
        let rows = vec![make_trial(&[], vec![0.1, 0.9])];
        let obj_names = vec!["obj0".to_string(), "obj1".to_string()];
        let df = DataFrame::from_trials(&rows, &[], &obj_names, &[], &[], 0);
        let obj0 = df.get_numeric_column("obj0").expect("obj0translated");
        let obj1 = df.get_numeric_column("obj1").expect("obj1translated");
        assert!((obj0[0] - 0.1).abs() < 1e-9); // Documentation.
        assert!((obj1[0] - 0.9).abs() < 1e-9); // Documentation.
    }

    #[test]
    fn tc_102_04_user_attr_numeric() {
        // Documentation.
        let mut row = make_trial(&[], vec![1.0]);
        row.user_attrs_numeric.insert("loss".to_string(), 0.123);
        let df = DataFrame::from_trials(
            &[row],
            &[],
            &["obj0".to_string()],
            &["loss".to_string()],
            &[],
            0,
        );
        let loss = df.get_numeric_column("loss").expect("losstranslated");
        assert!((loss[0] - 0.123).abs() < 1e-9); // Documentation.
    }

    #[test]
    fn tc_102_05_user_attr_string() {
        // Documentation.
        let mut row = make_trial(&[], vec![1.0]);
        row.user_attrs_string
            .insert("tag".to_string(), "run_a".to_string());
        let df = DataFrame::from_trials(
            &[row],
            &[],
            &["obj0".to_string()],
            &[],
            &["tag".to_string()],
            0,
        );
        let tag = df.get_string_column("tag").expect("tagtranslated");
        assert_eq!(tag[0], "run_a"); // Documentation.
    }

    #[test]
    fn tc_102_06_constraint_columns() {
        // Documentation.
        let mut row = make_trial(&[], vec![1.0]);
        row.constraint_values = vec![-0.5, 0.3];
        let df = DataFrame::from_trials(&[row], &[], &["obj0".to_string()], &[], &[], 2);
        let c1 = df.get_numeric_column("c1").expect("c1translated");
        let c2 = df.get_numeric_column("c2").expect("c2translated");
        let is_feas = df
            .get_numeric_column("is_feasible")
            .expect("is_feasibletranslated");
        let csum = df
            .get_numeric_column("constraint_sum")
            .expect("constraint_sumtranslated");
        assert!((c1[0] - (-0.5)).abs() < 1e-9); // Documentation.
        assert!((c2[0] - 0.3).abs() < 1e-9); // Documentation.
        assert!((is_feas[0] - 0.0).abs() < 1e-9); // Documentation.
        assert!((csum[0] - (-0.2)).abs() < 1e-6); // Documentation.
    }

    #[test]
    fn tc_102_07_positions_buffer_size() {
        // Documentation.
        let rows = vec![
            make_trial(&[], vec![1.0, 2.0]),
            make_trial(&[], vec![3.0, 4.0]),
            make_trial(&[], vec![5.0, 6.0]),
        ];
        let obj_names = vec!["obj0".to_string(), "obj1".to_string()];
        let df = DataFrame::from_trials(&rows, &[], &obj_names, &[], &[], 0);
        let gpu = df.gpu_buffers();
        assert_eq!(gpu.positions.len(), 6); // N=3, 3×2=6 🟢
        assert_eq!(gpu.trial_count, 3); // trial_count=3 🟢
    }

    #[test]
    fn tc_102_08_positions3d_buffer_size() {
        // Documentation.
        let rows = vec![
            make_trial(&[], vec![1.0, 2.0]),
            make_trial(&[], vec![3.0, 4.0]),
            make_trial(&[], vec![5.0, 6.0]),
        ];
        let obj_names = vec!["obj0".to_string(), "obj1".to_string()];
        let df = DataFrame::from_trials(&rows, &[], &obj_names, &[], &[], 0);
        let gpu = df.gpu_buffers();
        assert_eq!(gpu.positions3d.len(), 9); // N=3, 3×3=9 🟢
    }

    #[test]
    fn tc_102_09_sizes_buffer() {
        // Documentation.
        let rows = vec![
            make_trial(&[], vec![1.0]),
            make_trial(&[], vec![2.0]),
            make_trial(&[], vec![3.0]),
        ];
        let df = DataFrame::from_trials(&rows, &[], &["obj0".to_string()], &[], &[], 0);
        let gpu = df.gpu_buffers();
        assert_eq!(gpu.sizes.len(), 3); // N=3 🟢
        assert!(gpu.sizes.iter().all(|&s| (s - 1.0f32).abs() < 1e-6)); // Documentation.
    }

    #[test]
    fn tc_102_10_positions_two_objectives() {
        // Documentation.
        let rows = vec![make_trial(&[], vec![1.0, 2.0])];
        let obj_names = vec!["obj0".to_string(), "obj1".to_string()];
        let df = DataFrame::from_trials(&rows, &[], &obj_names, &[], &[], 0);
        let gpu = df.gpu_buffers();
        // positions[0]=obj0[0]=1.0, positions[1]=obj1[0]=2.0 🟢
        assert!((gpu.positions[0] - 1.0f32).abs() < 1e-6);
        assert!((gpu.positions[1] - 2.0f32).abs() < 1e-6);
    }

    #[test]
    fn tc_102_11_positions_single_objective() {
        // Documentation.
        // N=3, x_scale=2.0 → idx/2.0 =[0.0, 0.5, 1.0]
        let rows = vec![
            make_trial(&[], vec![1.0]),
            make_trial(&[], vec![2.0]),
            make_trial(&[], vec![3.0]),
        ];
        let df = DataFrame::from_trials(&rows, &[], &["obj0".to_string()], &[], &[], 0);
        let gpu = df.gpu_buffers();
        // trial 0: [0.0, 1.0], trial 1: [0.5, 2.0], trial 2: [1.0, 3.0]
        assert!((gpu.positions[0] - 0.0f32).abs() < 1e-6); // idx=0 🟢
        assert!((gpu.positions[1] - 1.0f32).abs() < 1e-6); // obj=1.0 🟢
        assert!((gpu.positions[2] - 0.5f32).abs() < 1e-6); // idx=1/2 🟢
        assert!((gpu.positions[3] - 2.0f32).abs() < 1e-6); // obj=2.0 🟢
        assert!((gpu.positions[4] - 1.0f32).abs() < 1e-6); // idx=2/2 🟢
        assert!((gpu.positions[5] - 3.0f32).abs() < 1e-6); // obj=3.0 🟢
    }

    #[test]
    fn tc_102_12_dataframe_info_column_classification() {
        // Documentation.
        let mut row = make_trial(&[("x", 0.5)], vec![1.0]);
        row.user_attrs_numeric.insert("loss".to_string(), 0.1);
        row.constraint_values = vec![-0.5];
        let df = DataFrame::from_trials(
            &[row],
            &["x".to_string()],
            &["obj0".to_string()],
            &["loss".to_string()],
            &[],
            1,
        );
        let info = df.info();
        assert_eq!(info.param_columns, vec!["x"]); // Documentation.
        assert_eq!(info.objective_columns, vec!["obj0"]); // Documentation.
        assert_eq!(info.user_attr_columns, vec!["loss"]); // Documentation.
        assert_eq!(info.constraint_columns, vec!["c1"]); // Documentation.
        assert!(info.derived_columns.contains(&"is_feasible".to_string())); // Documentation.
        assert!(info.derived_columns.contains(&"constraint_sum".to_string())); // Documentation.
    }

    #[test]
    fn tc_102_13_select_study_returns_result() {
        // Documentation.
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
            "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"x\",",
            "\"param_value_internal\":0.5,",
            "\"distribution\":{\"name\":\"FloatDistribution\",\"low\":0.0,\"high\":1.0,\"log\":false}}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5]}\n"
        ));
        crate::journal_parser::parse_journal(&data).expect("translated");
        let result = select_study(0).expect("select_study(0)translated");
        assert!(result.data_frame_info.row_count >= 1); // Documentation.
    }

    #[test]
    fn tc_102_14_select_study_multiple_studies() {
        // Documentation.
        // Documentation.
        let data = to_bytes(concat!(
            // Study A
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"A\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[1.0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":1,\"state\":1,\"values\":[2.0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":2,\"state\":1,\"values\":[3.0]}\n",
            // Study B
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"B\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":1,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":3,\"state\":1,\"values\":[4.0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":1,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":4,\"state\":1,\"values\":[5.0]}\n"
        ));
        crate::journal_parser::parse_journal(&data).expect("translated");
        let result_b = select_study(1).expect("StudyBretrieval");
        assert_eq!(result_b.data_frame_info.row_count, 2); // Documentation.
    }

    // =========================================================================
    // Documentation.
    // =========================================================================

    #[test]
    fn tc_102_e01_invalid_study_id_returns_err() {
        // Documentation.
        let data = to_bytes(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        );
        crate::journal_parser::parse_journal(&data).expect("translated");
        let result = select_study(99);
        assert!(result.is_err()); // Documentation.
    }

    #[test]
    fn tc_102_e02_all_running_returns_empty() {
        // Documentation.
        let data = to_bytes(concat!(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
            "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
            "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":0,\"values\":null}\n"
        ));
        crate::journal_parser::parse_journal(&data).expect("translated");
        let result = select_study(0).expect("translated Ok translated");
        assert_eq!(result.data_frame_info.row_count, 0); // Documentation.
    }

    // =========================================================================
    // Documentation.
    // =========================================================================

    #[test]
    fn tc_102_b01_three_objectives_positions() {
        // Documentation.
        let rows = vec![make_trial(&[], vec![0.1, 0.2, 0.3])];
        let obj_names = vec!["obj0".to_string(), "obj1".to_string(), "obj2".to_string()];
        let df = DataFrame::from_trials(&rows, &[], &obj_names, &[], &[], 0);
        let gpu = df.gpu_buffers();
        // positions: [obj0, obj1] = [0.1, 0.2] 🟢
        assert!((gpu.positions[0] - 0.1f32).abs() < 1e-6);
        assert!((gpu.positions[1] - 0.2f32).abs() < 1e-6);
        // positions3d: [obj0, obj1, obj2] = [0.1, 0.2, 0.3] 🟢
        assert!((gpu.positions3d[0] - 0.1f32).abs() < 1e-6);
        assert!((gpu.positions3d[1] - 0.2f32).abs() < 1e-6);
        assert!((gpu.positions3d[2] - 0.3f32).abs() < 1e-6);
    }

    #[test]
    fn tc_102_b02_study_with_no_complete_trials() {
        // Documentation.
        let data = to_bytes(
            "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        );
        crate::journal_parser::parse_journal(&data).expect("translated");
        let result = select_study(0).expect("translatedStudytranslated Ok translated");
        assert_eq!(result.data_frame_info.row_count, 0); // Documentation.
        assert_eq!(result.gpu_buffer_data.trial_count, 0); // Documentation.
    }

    // =========================================================================
    // Documentation.
    // =========================================================================

    #[test]
    fn tc_102_p01_performance_50000_trials() {
        // Documentation.
        use std::time::Instant;

        // Documentation.
        let mut lines = Vec::with_capacity(100_001);
        lines.push(
            r#"{"op_code":0,"worker_id":"w","study_name":"perf","directions":[0]}"#.to_string(),
        );
        for i in 0u32..50_000 {
            lines.push(
                "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}".to_string()
            );
            lines.push(format!(
                "{{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":{i},\"state\":1,\"values\":[{v}]}}",
                v = (i as f64) * 0.001
            ));
        }
        let data = lines.join("\n").into_bytes();

        crate::journal_parser::parse_journal(&data).expect("translated");

        let start = Instant::now();
        let result = select_study(0).expect("select_study translated");
        let elapsed_ms = start.elapsed().as_millis();

        assert_eq!(result.data_frame_info.row_count, 50_000); // Documentation.
        assert!(
            elapsed_ms < 100,
            "select_study translated 100ms translated: {}ms",
            elapsed_ms
        ); // 🟢
    }
}
