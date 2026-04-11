use super::model::DataFrame;

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
pub(super) fn build_positions(df: &DataFrame, n: usize) -> Vec<f32> {
    let mut positions = vec![0.0f32; n * 2];
    let obj_names = df.objective_col_names();

    match obj_names.len() {
        0 => {}
        1 => {
            let obj0 = df.get_numeric_column(&obj_names[0]).unwrap_or(&[]);
            let x_scale = if n > 1 { (n - 1) as f32 } else { 1.0 };
            for i in 0..n {
                positions[i * 2] = i as f32 / x_scale;
                positions[i * 2 + 1] = obj0.get(i).copied().unwrap_or(f64::NAN) as f32;
            }
        }
        _ => {
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
pub(super) fn build_positions3d(df: &DataFrame, n: usize) -> Vec<f32> {
    let mut positions3d = vec![0.0f32; n * 3];
    let obj_names = df.objective_col_names();

    if obj_names.is_empty() {
        return positions3d;
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
                let x = i as f32 / x_scale;
                let y = obj0.get(i).copied().unwrap_or(f64::NAN) as f32;
                (x, y, 0.0f32)
            }
            2 => {
                let x = obj0.get(i).copied().unwrap_or(f64::NAN) as f32;
                let y = obj1.and_then(|v| v.get(i)).copied().unwrap_or(f64::NAN) as f32;
                (x, y, 0.0f32)
            }
            _ => {
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
