use super::RidgeResult;

fn transpose_and_standardize(x_matrix: &[Vec<f64>], n: usize, p: usize) -> Vec<f64> {
    let mut x_cols = vec![0.0f64; n * p];

    for (i, row) in x_matrix.iter().enumerate() {
        for (j, &v) in row.iter().enumerate() {
            x_cols[j * n + i] = v;
        }
    }

    let nf = n as f64;

    for j in 0..p {
        let col = &mut x_cols[j * n..(j + 1) * n];
        let mean: f64 = col.iter().sum::<f64>() / nf;
        let std_dev = (col.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / nf).sqrt();
        let std_dev = if std_dev < f64::EPSILON { 1.0 } else { std_dev };

        for v in col.iter_mut() {
            *v = (*v - mean) / std_dev;
        }
    }

    x_cols
}

pub(super) fn gaussian_elimination(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let p = b.len();
    if p == 0 {
        return Some(vec![]);
    }

    for col in 0..p {
        let pivot_row = (col..p)
            .max_by(|&i, &j| {
                a[i][col]
                    .abs()
                    .partial_cmp(&a[j][col].abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(col);

        a.swap(col, pivot_row);
        b.swap(col, pivot_row);

        let pivot = a[col][col];
        if pivot.abs() < 1e-12 {
            return None;
        }

        for row in (col + 1)..p {
            let factor = a[row][col] / pivot;
            for k in col..p {
                let v = a[col][k] * factor;
                a[row][k] -= v;
            }
            b[row] -= b[col] * factor;
        }
    }

    let mut x = vec![0.0f64; p];
    for i in (0..p).rev() {
        let mut sum = b[i];
        for j in (i + 1)..p {
            sum -= a[i][j] * x[j];
        }
        x[i] = sum / a[i][i];
    }

    Some(x)
}

pub fn compute_ridge(x_matrix: &[Vec<f64>], y: &[f64], alpha: f64) -> RidgeResult {
    let n = y.len();
    let empty = RidgeResult {
        beta: vec![],
        r_squared: 0.0,
    };

    if n < 2 || x_matrix.len() != n {
        return empty;
    }
    let p = x_matrix[0].len();
    if p == 0 {
        return empty;
    }

    let x_cols = transpose_and_standardize(x_matrix, n, p);

    let y_mean: f64 = y.iter().sum::<f64>() / n as f64;
    let y_c: Vec<f64> = y.iter().map(|&yi| yi - y_mean).collect();

    let mut xtx_flat = vec![0.0f64; p * p];
    for i in 0..p {
        for j in i..p {
            let col_i = &x_cols[i * n..(i + 1) * n];
            let col_j = &x_cols[j * n..(j + 1) * n];
            let val: f64 = col_i.iter().zip(col_j.iter()).map(|(a, b)| a * b).sum();
            xtx_flat[i * p + j] = val;
            xtx_flat[j * p + i] = val;
        }
    }
    for i in 0..p {
        xtx_flat[i * p + i] += alpha;
    }

    let mut xty = vec![0.0f64; p];
    for j in 0..p {
        let col_j = &x_cols[j * n..(j + 1) * n];
        xty[j] = col_j.iter().zip(y_c.iter()).map(|(xij, yi)| xij * yi).sum();
    }

    let xtx_2d: Vec<Vec<f64>> = (0..p)
        .map(|i| xtx_flat[i * p..(i + 1) * p].to_vec())
        .collect();
    let beta = match gaussian_elimination(xtx_2d, xty) {
        Some(b) => b,
        None => vec![0.0; p],
    };

    let y_hat: Vec<f64> = (0..n)
        .map(|i| (0..p).map(|j| x_cols[j * n + i] * beta[j]).sum())
        .collect();

    let ss_res: f64 = y_c
        .iter()
        .zip(y_hat.iter())
        .map(|(yi, yhi)| (yi - yhi).powi(2))
        .sum();
    let ss_tot: f64 = y_c.iter().map(|&yi| yi.powi(2)).sum();

    let r_squared = if ss_tot < f64::EPSILON {
        0.0
    } else {
        (1.0 - ss_res / ss_tot).max(0.0)
    };

    RidgeResult { beta, r_squared }
}
