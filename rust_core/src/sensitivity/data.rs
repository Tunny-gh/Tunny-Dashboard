use crate::core::math::rng::SeededRng;
use crate::dataframe::DataFrame;

pub(super) fn get_param_numeric_values(
    df: &DataFrame,
    param_name: &str,
    n: usize,
) -> Option<Vec<f64>> {
    if let Some(col) = df.get_numeric_column(param_name) {
        return Some(col.iter().take(n).copied().collect());
    }

    if let Some(col) = df.get_string_column(param_name) {
        use std::collections::HashMap;

        let mut label_to_id: HashMap<String, f64> = HashMap::new();
        let mut next_id = 0.0f64;
        let mut out = Vec::with_capacity(n);

        for label in col.iter().take(n) {
            let id = match label_to_id.get(label) {
                Some(v) => *v,
                None => {
                    let v = next_id;
                    label_to_id.insert(label.clone(), v);
                    next_id += 1.0;
                    v
                }
            };
            out.push(id);
        }

        return Some(out);
    }

    None
}

pub(super) fn sample_rows(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    max_rows: usize,
    seed: u64,
) -> (Vec<Vec<f64>>, Vec<f64>) {
    let n = y.len();
    let mut indices: Vec<usize> = (0..n).collect();
    let mut rng = SeededRng::from_seed(seed);
    rng.shuffle(&mut indices);
    indices.truncate(max_rows);
    (
        indices.iter().map(|&i| x_matrix[i].clone()).collect(),
        indices.iter().map(|&i| y[i]).collect(),
    )
}
