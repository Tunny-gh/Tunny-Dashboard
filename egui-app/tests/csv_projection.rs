mod common;
use common::try_csv as parse_flat_csv;
use tdashboard_analysis::projection::OptimizationDirection;

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
         0.5,8,4,55.73,26.81,2784,925,\"a,b.png\"\n\
         1.0,8,4,57,25,2700,920,\n"
    );
    assert_eq!(
        tdashboard_io::flat_csv::parse_flat_csv(csv.as_bytes(), "s")
            .unwrap()
            .images,
        vec![(0, "a.png".to_string()), (1, "a,b.png".to_string())]
    );
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
fn errors_on_empty_input() {
    for csv in ["", "in:x,in:y\n1,2\n", "in:x,out:f\n"] {
        assert!(parse_flat_csv(csv.as_bytes(), "s").is_err(), "{csv:?}");
    }
}
