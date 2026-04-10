fn rank(values: &[f64]) -> Vec<f64> {
    let n = values.len();
    if n == 0 {
        return vec![];
    }

    let mut indices: Vec<usize> = (0..n).collect();
    indices.sort_by(|&a, &b| {
        let va = values[a];
        let vb = values[b];
        match (va.is_nan(), vb.is_nan()) {
            (true, _) => std::cmp::Ordering::Greater,
            (_, true) => std::cmp::Ordering::Less,
            _ => va.partial_cmp(&vb).unwrap(),
        }
    });

    let mut ranks = vec![0.0f64; n];
    let mut i = 0;

    while i < n {
        let val = values[indices[i]];
        if val.is_nan() {
            let avg = (i as f64 + 1.0 + n as f64) / 2.0;
            for k in i..n {
                ranks[indices[k]] = avg;
            }
            break;
        }

        let mut j = i + 1;
        while j < n && values[indices[j]] == val {
            j += 1;
        }

        let avg_rank = (i as f64 + 1.0 + j as f64) / 2.0;
        for k in i..j {
            ranks[indices[k]] = avg_rank;
        }
        i = j;
    }

    ranks
}

fn pearson_correlation(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len();
    if n < 2 {
        return 0.0;
    }
    let nf = n as f64;

    let mean_x: f64 = x.iter().sum::<f64>() / nf;
    let mean_y: f64 = y.iter().sum::<f64>() / nf;

    let mut cov = 0.0f64;
    let mut var_x = 0.0f64;
    let mut var_y = 0.0f64;

    for (&xi, &yi) in x.iter().zip(y.iter()) {
        let dx = xi - mean_x;
        let dy = yi - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    let denom = (var_x * var_y).sqrt();
    if denom < f64::EPSILON {
        return 0.0;
    }
    cov / denom
}

pub fn compute_spearman(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 2 {
        return 0.0;
    }

    let rx = rank(&x[..n]);
    let ry = rank(&y[..n]);

    pearson_correlation(&rx, &ry)
}
