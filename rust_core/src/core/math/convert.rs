#[cfg(test)]
mod tests {
    fn faer_to_ndarray(mat: &faer::Mat<f64>) -> ndarray::Array2<f64> {
        ndarray::Array2::from_shape_fn((mat.nrows(), mat.ncols()), |(i, j)| mat[(i, j)])
    }

    fn ndarray_to_faer(arr: &ndarray::Array2<f64>) -> faer::Mat<f64> {
        faer::Mat::from_fn(arr.nrows(), arr.ncols(), |i, j| arr[[i, j]])
    }

    #[test]
    fn roundtrip_faer_ndarray_faer() {
        let mat = faer::Mat::<f64>::from_fn(3, 4, |i, j| (i * 4 + j) as f64);
        let arr = faer_to_ndarray(&mat);
        let mat2 = ndarray_to_faer(&arr);
        for i in 0..3 {
            for j in 0..4 {
                assert!((mat[(i, j)] - mat2[(i, j)]).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn roundtrip_ndarray_faer_ndarray() {
        let arr = ndarray::Array2::from_shape_fn((2, 5), |(i, j)| (i * 5 + j) as f64);
        let mat = ndarray_to_faer(&arr);
        let arr2 = faer_to_ndarray(&mat);
        for i in 0..2 {
            for j in 0..5 {
                assert!((arr[[i, j]] - arr2[[i, j]]).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn empty_matrix() {
        let mat = faer::Mat::<f64>::zeros(0, 0);
        let arr = faer_to_ndarray(&mat);
        assert_eq!(arr.nrows(), 0);
        assert_eq!(arr.ncols(), 0);
    }

    #[test]
    fn one_by_one_matrix() {
        let mat = faer::Mat::<f64>::from_fn(1, 1, |_, _| 42.0);
        let arr = faer_to_ndarray(&mat);
        assert!((arr[[0, 0]] - 42.0).abs() < 1e-12);
        let mat2 = ndarray_to_faer(&arr);
        assert!((mat2[(0, 0)] - 42.0).abs() < 1e-12);
    }
}
