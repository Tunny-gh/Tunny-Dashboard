use super::*;
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn colormap_name_all_has_nine_variants() {
    assert_eq!(ColormapName::all().len(), 9);
}

#[test]
fn colormap_name_labels_not_empty() {
    for cmap in ColormapName::all() {
        assert!(!cmap.label().is_empty(), "{:?} has empty label", cmap);
    }
}

// ── Default derivation for GhOptDialogState::new ──────────────────
fn make_gh_problem(n_objectives: usize) -> tunny_core::gh::GhProblem {
    tunny_core::gh::GhProblem {
        variables: vec![],
        objectives: (0..n_objectives)
            .map(|i| tunny_core::gh::GhObjective {
                source_guid: format!("guid-{i}"),
                name: format!("f{i}"),
            })
            .collect(),
        constraints: vec![],
        attributes: vec![],
        tunny_component: "Tunny".to_string(),
        tunny_instance_guid: "tunny-guid".to_string(),
        warnings: vec![],
    }
}

/// The persisted prefs round-trip: capture from a dialog, apply onto a
/// fresh one, and the connection/sampler settings carry over while
/// per-file values (study name, journal path) stay derived from the path.
#[test]
fn gh_compute_prefs_capture_and_apply_roundtrip() {
    let path = PathBuf::from("/tmp/a/model.ghx");
    let mut first = GhOptDialogState::new(path.clone(), "<xml/>".to_string(), make_gh_problem(1));
    first.compute_use_exe = false;
    first.compute_url = "http://build-server:9900".to_string();
    first.compute_exe_path = r"C:\compute\rhino.compute.exe".to_string();
    first.compute_port = 9900;
    first.api_key = "secret".to_string();
    first.max_parallel = 8;
    first.sampler = GhSamplerChoice::Adaptive;
    first.adaptive_initial = 20;
    first.adaptive_batch = 6;
    first.adaptive_iterations = 3;
    first.n_trials = 123;
    first.population_size = 32;
    first.generations = 5;
    first.seed = 7;

    let prefs = GhComputePrefs::capture(&first);
    let other = PathBuf::from("/tmp/b/other.ghx");
    let mut second = GhOptDialogState::new(other.clone(), "<xml/>".to_string(), make_gh_problem(1));
    prefs.apply_to(&mut second);

    assert!(!second.compute_use_exe);
    assert_eq!(second.compute_url, "http://build-server:9900");
    assert_eq!(second.compute_exe_path, r"C:\compute\rhino.compute.exe");
    assert_eq!(second.compute_port, 9900);
    assert_eq!(second.api_key, "secret");
    assert_eq!(second.max_parallel, 8);
    assert_eq!(second.sampler, GhSamplerChoice::Adaptive);
    assert_eq!(second.adaptive_initial, 20);
    assert_eq!(second.adaptive_batch, 6);
    assert_eq!(second.adaptive_iterations, 3);
    assert_eq!(second.n_trials, 123);
    assert_eq!(second.population_size, 32);
    assert_eq!(second.generations, 5);
    assert_eq!(second.seed, 7);
    // Per-file values stay derived from the new path.
    assert!(second.study_name.starts_with("other-"));
    assert!(second.journal_path.contains("other_optuna"));
}

/// Out-of-range persisted values (hand-edited or from an older version)
/// are clamped on apply instead of propagating into the run config.
#[test]
fn gh_compute_prefs_apply_clamps_invalid_values() {
    let prefs = GhComputePrefs {
        max_parallel: 0,
        n_trials: 0,
        population_size: 0,
        generations: 0,
        ..GhComputePrefs::default()
    };
    let mut dialog = GhOptDialogState::new(
        PathBuf::from("/tmp/m.ghx"),
        "<xml/>".to_string(),
        make_gh_problem(1),
    );
    prefs.apply_to(&mut dialog);
    assert_eq!(dialog.max_parallel, 1);
    assert_eq!(dialog.n_trials, 1);
    assert_eq!(dialog.population_size, 1);
    assert_eq!(dialog.generations, 1);
}

/// Serde round-trip with `#[serde(default)]`: an older stored blob with
/// missing fields deserializes with defaults instead of failing.
#[test]
fn gh_compute_prefs_serde_tolerates_missing_fields() {
    let prefs: GhComputePrefs =
        serde_json::from_str(r#"{"compute_exe_path": "C:/x/rhino.compute.exe"}"#).unwrap();
    assert_eq!(prefs.compute_exe_path, "C:/x/rhino.compute.exe");
    assert_eq!(prefs.compute_port, 6500);
    assert!(prefs.compute_use_exe);

    let json = serde_json::to_string(&GhComputePrefs::default()).unwrap();
    let back: GhComputePrefs = serde_json::from_str(&json).unwrap();
    assert_eq!(back.compute_url, "http://localhost:6500");
}

#[test]
fn gh_opt_dialog_state_derives_defaults_from_path() {
    let path = PathBuf::from("/tmp/some_dir/model.ghx");
    let state = GhOptDialogState::new(path.clone(), "<xml/>".to_string(), make_gh_problem(2));

    assert_eq!(state.ghx_path, path);
    assert_eq!(state.ghx_text, "<xml/>");
    assert_eq!(state.maximize, vec![false, false]);
    // study_name: "<stem>-<last 6 digits of unix seconds>" (zero-padded to 6 digits)
    assert!(
        state.study_name.starts_with("model-"),
        "study_name: {}",
        state.study_name
    );
    assert_eq!(state.study_name.len(), "model-".len() + 6);
    // journal_path: "<stem>_optuna.log" in the same directory as the ghx
    // (built via PathBuf::join so the separator matches the platform)
    let expected_journal = PathBuf::from("/tmp/some_dir")
        .join("model_optuna.log")
        .display()
        .to_string();
    assert_eq!(state.journal_path, expected_journal);
    assert!(state.compute_use_exe);
    assert_eq!(state.compute_url, "http://localhost:6500");
    assert_eq!(state.compute_exe_path, "");
    assert_eq!(state.compute_port, 6500);
    assert_eq!(state.api_key, "");
    assert_eq!(state.max_parallel, 4);
    assert_eq!(state.sampler, GhSamplerChoice::Nsga2);
    assert_eq!(state.n_trials, 50);
    assert_eq!(state.population_size, 16);
    assert_eq!(state.generations, 10);
    assert_eq!(state.seed, 42);
    assert!(state.error.is_none());
}

#[test]
fn gh_opt_dialog_state_maximize_matches_objective_count() {
    let path = PathBuf::from("model.ghx");
    let state = GhOptDialogState::new(path, String::new(), make_gh_problem(3));
    assert_eq!(state.maximize.len(), 3);
    assert!(state.maximize.iter().all(|&m| !m));
}

// ── TASK-2331: StudyView tests ──────────────────────────────
fn make_study_view(n: usize) -> StudyView {
    use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};
    let core_rows: Vec<CoreRow> = (0..n)
        .map(|i| CoreRow {
            trial_id: i as u32,
            trial_number: i as u32,
            param_display: HashMap::from([("x".to_string(), i as f64 * 0.1)]),
            param_category_label: HashMap::new(),
            objective_values: vec![i as f64, i as f64 * 2.0],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        })
        .collect();
    let df = DataFrame::from_trials(
        &core_rows,
        &["x".to_string()],
        &["obj0".to_string(), "obj1".to_string()],
        &[],
        &[],
        0,
    );
    StudyView::new(std::sync::Arc::new(df), vec![0; n])
}

#[test]
fn study_view_row_count_and_columns() {
    let view = make_study_view(3);
    assert_eq!(view.row_count(), 3);
    assert_eq!(view.numeric_column("x").map(|c| c.len()), Some(3));
    assert!(view.numeric_column("missing").is_none());
    assert_eq!(view.param_names(), &["x".to_string()]);
}

#[test]
fn study_view_row_at_matches_columnar_values() {
    let view = make_study_view(3);
    let row = view.row_at(2);
    assert_eq!(row.trial_id, 2);
    assert_eq!(row.trial_number, 2);
    assert!((row.params["x"] - 0.2).abs() < 1e-9);
    assert_eq!(row.objectives, vec![2.0, 4.0]);
    assert_eq!(row.pareto_rank, 0);
    assert_eq!(row.cluster_id, None);
    assert!(row.user_attrs.is_empty());
}

#[test]
fn study_view_to_trial_rows_roundtrip() {
    let view = make_study_view(4);
    let rows = view.to_trial_rows();
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0].trial_id, 0);
    assert_eq!(rows[3].objectives, vec![3.0, 6.0]);
}

#[test]
fn study_view_new_pads_mismatched_pareto_rank() {
    use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};
    let core_rows: Vec<CoreRow> = (0..2)
        .map(|i| CoreRow {
            trial_id: i,
            trial_number: i,
            param_display: HashMap::new(),
            param_category_label: HashMap::new(),
            objective_values: vec![i as f64],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        })
        .collect();
    let df = DataFrame::from_trials(&core_rows, &[], &["obj0".to_string()], &[], &[], 0);
    // pareto_rank length (1) != row_count (2) -> pad with 0
    let view = StudyView::new(std::sync::Arc::new(df), vec![5]);
    assert_eq!(view.pareto_rank, vec![0, 0]);
}
