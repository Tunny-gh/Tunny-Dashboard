use super::*;

fn create_schema(conn: &Connection) {
    conn.execute_batch(
        "
        CREATE TABLE studies (
            study_id INTEGER PRIMARY KEY,
            study_name VARCHAR(512)
        );
        CREATE TABLE study_directions (
            study_direction_id INTEGER PRIMARY KEY,
            direction VARCHAR(8),
            study_id INTEGER,
            objective INTEGER
        );
        CREATE TABLE trials (
            trial_id INTEGER PRIMARY KEY,
            number INTEGER,
            study_id INTEGER,
            state VARCHAR(8),
            datetime_start TEXT,
            datetime_complete TEXT
        );
        CREATE TABLE trial_values (
            trial_value_id INTEGER PRIMARY KEY,
            trial_id INTEGER,
            objective INTEGER,
            value REAL,
            value_type VARCHAR(7)
        );
        CREATE TABLE trial_params (
            param_id INTEGER PRIMARY KEY,
            trial_id INTEGER,
            param_name VARCHAR(512),
            param_value REAL,
            distribution_json TEXT
        );
        CREATE TABLE trial_user_attributes (
            id INTEGER PRIMARY KEY,
            trial_id INTEGER,
            key VARCHAR(512),
            value_json TEXT
        );
        CREATE TABLE trial_system_attributes (
            id INTEGER PRIMARY KEY,
            trial_id INTEGER,
            key VARCHAR(512),
            value_json TEXT
        );
        CREATE TABLE study_system_attributes (
            id INTEGER PRIMARY KEY,
            study_id INTEGER,
            key VARCHAR(512),
            value_json TEXT
        );
        ",
    )
    .unwrap();
}

/// study_id=1: single-objective minimize study with a float param, an int param,
/// a categorical param, a numeric user attr, and 3 completed trials (one PRUNED excluded).
/// study_id=2: two-objective (min, max) study with metric_names set and a constraint.
fn seed_basic(conn: &Connection) {
    conn.execute(
        "INSERT INTO studies (study_id, study_name) VALUES (1, 'study-a'), (2, 'study-b')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO study_directions (study_id, direction, objective) VALUES (1, 'MINIMIZE', 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO study_directions (study_id, direction, objective) VALUES \
         (2, 'MINIMIZE', 0), (2, 'MAXIMIZE', 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO study_system_attributes (study_id, key, value_json) VALUES \
         (2, 'study:metric_names', '[\"Cost\",\"Quality\"]')",
        [],
    )
    .unwrap();

    // study 1: trials 1..=4 (1,2,3 complete; 4 pruned)
    conn.execute_batch(
        "
        INSERT INTO trials (trial_id, number, study_id, state) VALUES
            (1, 0, 1, 'COMPLETE'),
            (2, 1, 1, 'COMPLETE'),
            (3, 2, 1, 'COMPLETE'),
            (4, 3, 1, 'PRUNED');
        ",
    )
    .unwrap();

    conn.execute_batch(
        "
        INSERT INTO trial_values (trial_id, objective, value, value_type) VALUES
            (1, 0, 1.5, 'FINITE'),
            (2, 0, 2.5, 'FINITE'),
            (3, 0, 3.5, 'FINITE');
        ",
    )
    .unwrap();

    let float_dist = r#"{"name": "FloatDistribution", "attributes": {"step": null, "low": -5.0, "high": 5.0, "log": false}}"#;
    let int_dist = r#"{"name": "IntDistribution", "attributes": {"log": false, "step": 1, "low": 1, "high": 10}}"#;
    let cat_dist = r#"{"name": "CategoricalDistribution", "attributes": {"choices": ["steel", "aluminum", "wood"]}}"#;

    for (trial_id, x, n, cat_idx) in [(1, 0.5, 3.0, 0.0), (2, 1.5, 5.0, 1.0), (3, -2.0, 7.0, 2.0)] {
        conn.execute(
            "INSERT INTO trial_params (trial_id, param_name, param_value, distribution_json) \
             VALUES (?1, 'x', ?2, ?3)",
            rusqlite::params![trial_id, x, float_dist],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO trial_params (trial_id, param_name, param_value, distribution_json) \
             VALUES (?1, 'n', ?2, ?3)",
            rusqlite::params![trial_id, n, int_dist],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO trial_params (trial_id, param_name, param_value, distribution_json) \
             VALUES (?1, 'material', ?2, ?3)",
            rusqlite::params![trial_id, cat_idx, cat_dist],
        )
        .unwrap();
    }

    conn.execute(
        "INSERT INTO trial_user_attributes (trial_id, key, value_json) VALUES (1, 'score', '12.5')",
        [],
    )
    .unwrap();

    // study 2: trials 5,6 complete (2 objectives), trial 7 running (excluded)
    conn.execute_batch(
        "
        INSERT INTO trials (trial_id, number, study_id, state) VALUES
            (5, 0, 2, 'COMPLETE'),
            (6, 1, 2, 'COMPLETE'),
            (7, 2, 2, 'RUNNING');
        ",
    )
    .unwrap();
    conn.execute_batch(
        "
        INSERT INTO trial_values (trial_id, objective, value, value_type) VALUES
            (5, 0, 1.0, 'FINITE'),
            (5, 1, 2.0, 'FINITE'),
            (6, 0, 0.0, 'INF_POS'),
            (6, 1, 0.0, 'INF_NEG');
        ",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO trial_system_attributes (trial_id, key, value_json) VALUES (5, 'constraints', '[1.0, -2.0]')",
        [],
    )
    .unwrap();
}

#[test]
fn scan_study_list_reads_names_directions_and_metric_names() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let conn = Connection::open(file.path()).unwrap();
    create_schema(&conn);
    seed_basic(&conn);
    drop(conn);

    let studies = scan_study_list(file.path()).unwrap();
    assert_eq!(studies.len(), 2);

    let s1 = studies.iter().find(|s| s.study_id == 1).unwrap();
    assert_eq!(s1.name, "study-a");
    assert_eq!(s1.directions, vec![OptimizationDirection::Minimize]);
    assert_eq!(s1.objective_names, vec!["obj0".to_string()]);
    assert_eq!(s1.completed_trials, 3);
    assert_eq!(s1.total_trials, 4);
    assert!(!s1.has_constraints);

    let s2 = studies.iter().find(|s| s.study_id == 2).unwrap();
    assert_eq!(
        s2.directions,
        vec![
            OptimizationDirection::Minimize,
            OptimizationDirection::Maximize
        ]
    );
    assert_eq!(
        s2.objective_names,
        vec!["Cost".to_string(), "Quality".to_string()]
    );
    assert_eq!(s2.completed_trials, 2);
    assert_eq!(s2.total_trials, 3);
    assert!(s2.has_constraints);
}

#[test]
fn parse_single_study_reads_params_and_excludes_non_complete_trials() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let conn = Connection::open(file.path()).unwrap();
    create_schema(&conn);
    seed_basic(&conn);
    drop(conn);

    let (meta, df) = parse_single_study(file.path(), 1).unwrap();
    assert_eq!(meta.name, "study-a");
    assert_eq!(df.row_count(), 3, "PRUNED trial must be excluded");

    let mut param_names = df.param_col_names().to_vec();
    param_names.sort();
    assert_eq!(
        param_names,
        vec!["material".to_string(), "n".to_string(), "x".to_string()]
    );

    let x_vals = df.get_numeric_column("x").unwrap();
    assert_eq!(x_vals, &[0.5, 1.5, -2.0]);

    // Optuna は param_value に実値（外部表現）を格納するため表示もそのまま
    let n_vals = df.get_numeric_column("n").unwrap();
    assert_eq!(n_vals, &[3.0, 5.0, 7.0]);

    let material_labels = df.get_string_column("material").unwrap();
    assert_eq!(
        material_labels,
        &[
            "steel".to_string(),
            "aluminum".to_string(),
            "wood".to_string()
        ]
    );

    assert_eq!(meta.param_bounds.get("x"), Some(&(-5.0, 5.0)));
    assert_eq!(meta.param_bounds.get("n"), Some(&(1.0, 10.0)));
    assert!(!meta.param_bounds.contains_key("material"));

    let score_vals = df.get_numeric_column("score").unwrap();
    assert_eq!(score_vals[0], 12.5);
}

#[test]
fn parse_single_study_converts_infinite_objective_values() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let conn = Connection::open(file.path()).unwrap();
    create_schema(&conn);
    seed_basic(&conn);
    drop(conn);

    let (meta, df) = parse_single_study(file.path(), 2).unwrap();
    assert_eq!(
        meta.objective_names,
        vec!["Cost".to_string(), "Quality".to_string()]
    );
    assert_eq!(df.row_count(), 2, "RUNNING trial must be excluded");

    let cost = df.get_numeric_column("Cost").unwrap();
    let quality = df.get_numeric_column("Quality").unwrap();
    assert_eq!(cost[0], 1.0);
    assert_eq!(quality[0], 2.0);
    assert_eq!(cost[1], f64::INFINITY);
    assert_eq!(quality[1], f64::NEG_INFINITY);
}

#[test]
fn parse_single_study_extracts_constraints() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let conn = Connection::open(file.path()).unwrap();
    create_schema(&conn);
    seed_basic(&conn);
    drop(conn);

    let (meta, df) = parse_single_study(file.path(), 2).unwrap();
    assert!(meta.has_constraints);
    let c1 = df.get_numeric_column("c1").unwrap();
    let c2 = df.get_numeric_column("c2").unwrap();
    assert_eq!(c1[0], 1.0);
    assert_eq!(c2[0], -2.0);
    // trial 6 has no constraints row -> defaults to 0.0
    assert_eq!(c1[1], 0.0);
    assert_eq!(c2[1], 0.0);
}

#[test]
fn parse_single_study_missing_study_id_errors() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let conn = Connection::open(file.path()).unwrap();
    create_schema(&conn);
    seed_basic(&conn);
    drop(conn);

    let result = parse_single_study(file.path(), 999);
    assert!(result.is_err());
}

#[test]
fn scan_study_list_rejects_non_optuna_database() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let conn = Connection::open(file.path()).unwrap();
    conn.execute_batch("CREATE TABLE not_optuna (id INTEGER);")
        .unwrap();
    drop(conn);

    let result = scan_study_list(file.path());
    assert!(result.is_err());
}

#[test]
fn scan_study_list_empty_database_errors() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let conn = Connection::open(file.path()).unwrap();
    create_schema(&conn);
    drop(conn);

    let result = scan_study_list(file.path());
    assert!(result.is_err());
}
