use super::*;

const COLIBRI_HEADER: &str =
    "in:ShadeDepth,in:X-grid,in:Y-grid,out:UDI,out:ASE,out:1F-MeanLux,out:2F-MeanLux,img";

#[test]
fn parses_colibri_like_csv() {
    let csv = format!(
        "{COLIBRI_HEADER}\n\
         0.0,8,4,53.44,31.26,3495,926,ShadeDepth0.0_X-grid8_Y-grid4_Perspective.png\n\
         0.5,8,4,55.73,26.81,2784,925,ShadeDepth0.5_X-grid8_Y-grid4_Perspective.png\n"
    );
    let result = parse_flat_csv(csv.as_bytes(), "colibri").unwrap();

    assert_eq!(result.meta.name, "colibri");
    assert_eq!(result.meta.completed_trials, 2);
    assert_eq!(result.meta.total_trials, 2);
    // Parameters are sorted.
    assert_eq!(
        result.meta.param_names,
        vec![
            "ShadeDepth".to_string(),
            "X-grid".to_string(),
            "Y-grid".to_string()
        ]
    );
    // Objectives preserve column order.
    assert_eq!(
        result.meta.objective_names,
        vec![
            "UDI".to_string(),
            "ASE".to_string(),
            "1F-MeanLux".to_string(),
            "2F-MeanLux".to_string()
        ]
    );
    // Direction defaults to Minimize for all, since there's no such info.
    assert_eq!(result.meta.directions.len(), 4);
    assert!(result
        .meta
        .directions
        .iter()
        .all(|d| matches!(d, OptimizationDirection::Minimize)));
    assert!(!result.meta.has_constraints);
}

#[test]
fn maps_images_per_trial() {
    let csv = format!(
        "{COLIBRI_HEADER}\n\
         0.0,8,4,53.44,31.26,3495,926,a.png\n\
         0.5,8,4,55.73,26.81,2784,925,b.png\n"
    );
    let result = parse_flat_csv(csv.as_bytes(), "s").unwrap();
    assert_eq!(
        result.images,
        vec![(0, "a.png".to_string()), (1, "b.png".to_string())]
    );
}

#[test]
fn skips_empty_image_cells() {
    let csv = "in:x,out:f,img\n1,2,a.png\n3,4,\n";
    let result = parse_flat_csv(csv.as_bytes(), "s").unwrap();
    assert_eq!(result.images, vec![(0, "a.png".to_string())]);
}

#[test]
fn computes_param_bounds_from_observed_range() {
    let csv = "in:x,in:y,out:f\n0.0,8,1\n2.0,40,2\n1.0,24,3\n";
    let result = parse_flat_csv(csv.as_bytes(), "s").unwrap();
    let bx = result.meta.param_bounds.get("x").unwrap();
    assert_eq!(*bx, (0.0, 2.0));
    let by = result.meta.param_bounds.get("y").unwrap();
    assert_eq!(*by, (8.0, 40.0));
}

#[test]
fn dataframe_row_count_matches_data_rows() {
    let csv = "in:x,out:f\n1,10\n2,20\n3,30\n";
    let result = parse_flat_csv(csv.as_bytes(), "s").unwrap();
    assert_eq!(result.dataframe.row_count(), 3);
}

#[test]
fn categorical_param_falls_back_to_label_column() {
    // A non-numeric parameter column is treated as categorical, and gets no bounds.
    let csv = "in:material,out:f\nsteel,1\nwood,2\n";
    let result = parse_flat_csv(csv.as_bytes(), "s").unwrap();
    assert!(!result.meta.param_bounds.contains_key("material"));
    assert_eq!(result.dataframe.row_count(), 2);
}

#[test]
fn non_prefixed_column_becomes_user_attr() {
    let csv = "in:x,out:f,note,score\n1,10,hello,3.5\n2,20,world,4.5\n";
    let result = parse_flat_csv(csv.as_bytes(), "s").unwrap();
    // note (string) and score (numeric) are ingested as user_attrs.
    assert!(result.meta.user_attr_names.contains(&"note".to_string()));
    assert!(result.meta.user_attr_names.contains(&"score".to_string()));
}

#[test]
fn handles_quoted_fields_with_commas() {
    let csv = "in:x,out:f,img\n1,10,\"a,b.png\"\n";
    let result = parse_flat_csv(csv.as_bytes(), "s").unwrap();
    assert_eq!(result.images, vec![(0, "a,b.png".to_string())]);
}

#[test]
fn errors_on_empty_input() {
    assert!(parse_flat_csv(b"", "s").is_err());
}

#[test]
fn errors_when_no_objective_columns() {
    let csv = "in:x,in:y\n1,2\n";
    assert!(parse_flat_csv(csv.as_bytes(), "s").is_err());
}

#[test]
fn errors_on_header_only() {
    let csv = "in:x,out:f\n";
    assert!(parse_flat_csv(csv.as_bytes(), "s").is_err());
}
