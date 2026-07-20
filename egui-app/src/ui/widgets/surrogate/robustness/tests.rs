use super::*;

#[test]
fn robustness_chart_default_values() {
    let state = RobustnessChart::default();
    assert_eq!(state.selected_objective, 0);
    assert_eq!(state.center, CenterChoice::BestTrial);
    assert_eq!(state.noise_pct, 2.0);
    assert_eq!(state.noise_dist, NoiseDistKind::Normal);
    assert_eq!(state.weibull_shape, 1.5);
    assert_eq!(state.n_samples, 1024);
    assert!(!state.include_epistemic);
    assert!(!state.use_lower_spec);
    assert_eq!(state.lower_spec_value, 0.0);
    assert!(!state.use_upper_spec);
    assert_eq!(state.upper_spec_value, 0.0);
    assert!(state.trained.is_none());
    assert!(!state.fitting);
    assert!(state.pending_fit.is_none());
    assert!(state.cached_result().is_none());
}

#[test]
fn noise_dist_kind_default_is_normal() {
    assert_eq!(NoiseDistKind::default(), NoiseDistKind::Normal);
}

#[test]
fn noise_dist_labels_cover_all_choices() {
    for kind in [
        NoiseDistKind::Normal,
        NoiseDistKind::Uniform,
        NoiseDistKind::Weibull,
    ] {
        assert!(!noise_dist_label(kind).is_empty());
    }
}

#[test]
fn cache_key_changes_with_distribution_shape_and_specs() {
    // Verify with a fixed fit generation ID (the identity key that replaced `Arc::as_ptr`).
    let gen = 1u64;
    let center = vec![0.5, 0.25];

    let base = cache_key(
        gen,
        &center,
        2.0,
        1024,
        false,
        NoiseDistKind::Normal,
        1.5,
        None,
        None,
    );
    let different_dist = cache_key(
        gen,
        &center,
        2.0,
        1024,
        false,
        NoiseDistKind::Uniform,
        1.5,
        None,
        None,
    );
    let different_shape = cache_key(
        gen,
        &center,
        2.0,
        1024,
        false,
        NoiseDistKind::Weibull,
        1.5,
        None,
        None,
    );
    let different_shape2 = cache_key(
        gen,
        &center,
        2.0,
        1024,
        false,
        NoiseDistKind::Weibull,
        3.0,
        None,
        None,
    );
    let with_lower = cache_key(
        gen,
        &center,
        2.0,
        1024,
        false,
        NoiseDistKind::Normal,
        1.5,
        Some(-1.0),
        None,
    );
    let with_upper = cache_key(
        gen,
        &center,
        2.0,
        1024,
        false,
        NoiseDistKind::Normal,
        1.5,
        None,
        Some(1.0),
    );
    // If the generation ID changes (i.e. a different fit is adopted), the key changes too.
    let different_generation = cache_key(
        gen + 1,
        &center,
        2.0,
        1024,
        false,
        NoiseDistKind::Normal,
        1.5,
        None,
        None,
    );

    assert_ne!(base, different_dist);
    assert_ne!(base, different_shape);
    assert_ne!(different_shape, different_shape2);
    assert_ne!(base, with_lower);
    assert_ne!(base, with_upper);
    assert_ne!(with_lower, with_upper);
    assert_ne!(base, different_generation);

    // Same arguments produce the same key.
    let base_again = cache_key(
        gen,
        &center,
        2.0,
        1024,
        false,
        NoiseDistKind::Normal,
        1.5,
        None,
        None,
    );
    assert_eq!(base, base_again);
}

#[test]
fn adopt_compute_state_propagates_and_keeps_selection() {
    let src = RobustnessChart {
        fitting: false,
        fit_error: Some("err".into()),
        ..Default::default()
    };
    let mut dst = RobustnessChart {
        fitting: true,
        selected_objective: 2,
        noise_pct: 5.0,
        ..Default::default()
    };
    dst.adopt_compute_state(&src);
    assert!(!dst.fitting);
    assert_eq!(dst.fit_error.as_deref(), Some("err"));
    // UI selections are preserved
    assert_eq!(dst.selected_objective, 2);
    assert_eq!(dst.noise_pct, 5.0);
}
