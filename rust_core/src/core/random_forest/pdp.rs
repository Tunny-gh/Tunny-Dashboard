use super::forest::{extract_columns, train_rf_on_columns};

type Pdp2dRfResult = (Vec<f64>, Vec<f64>, Vec<Vec<f64>>, f64);

/// Compute the 2D partial dependence surface using a Random Forest.
///
/// Extracts columns `param1_idx` and `param2_idx` from `x_matrix`, trains a
/// Random Forest on those 2 features, and evaluates on a `n_grid × n_grid` grid.
pub(crate) fn compute_pdp_2d_rf(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    param1_idx: usize,
    param2_idx: usize,
    n_grid: usize,
) -> Option<Pdp2dRfResult> {
    let n = y.len();
    if n < 2 || x_matrix.is_empty() || n_grid == 0 {
        return None;
    }
    let p = x_matrix[0].len();
    if param1_idx >= p || param2_idx >= p {
        return None;
    }

    let x2d = extract_columns(x_matrix, &[param1_idx, param2_idx])?;
    let rf = train_rf_on_columns(x_matrix, y, &[param1_idx, param2_idx], 100, 10, 2, 42)?;

    let col1: Vec<f64> = x2d.iter().map(|row| row[0]).collect();
    let col2: Vec<f64> = x2d.iter().map(|row| row[1]).collect();
    let min1 = col1.iter().cloned().fold(f64::INFINITY, f64::min);
    let max1 = col1.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min2 = col2.iter().cloned().fold(f64::INFINITY, f64::min);
    let max2 = col2.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let grid1 = linspace(min1, max1, n_grid);
    let grid2 = linspace(min2, max2, n_grid);

    let values: Vec<Vec<f64>> = grid1
        .iter()
        .map(|&value1| {
            grid2
                .iter()
                .map(|&value2| rf.predict(&[value1, value2]))
                .collect()
        })
        .collect();

    let y_pred: Vec<f64> = x2d.iter().map(|xi| rf.predict(xi)).collect();
    let y_mean = y.iter().sum::<f64>() / n as f64;
    let ss_res: f64 = y
        .iter()
        .zip(y_pred.iter())
        .map(|(&yi, &yp)| (yi - yp).powi(2))
        .sum();
    let ss_tot: f64 = y.iter().map(|&yi| (yi - y_mean).powi(2)).sum();
    let r_squared = if ss_tot < f64::EPSILON {
        1.0
    } else {
        1.0 - ss_res / ss_tot
    };

    Some((grid1, grid2, values, r_squared))
}

fn linspace(min: f64, max: f64, n: usize) -> Vec<f64> {
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![(min + max) / 2.0];
    }
    (0..n)
        .map(|i| min + (max - min) * i as f64 / (n - 1) as f64)
        .collect()
}
