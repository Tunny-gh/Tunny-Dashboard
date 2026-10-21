//! Method boundaries use actual acquired inputs, not GUI eligibility checks.
use std::{collections::BTreeMap, sync::Arc};
use tdashboard_analysis::sensitivity::heatmap::{compute_heatmap, HeatmapMethod};
use tdashboard_analysis::{
    preparation::*,
    sensitivity::{RidgeMetric, SensitivityMetric, SpearmanMetric},
    session::*,
};
use tdashboard_study::*;

fn study(
    n: usize,
    objectives: usize,
    complete: bool,
    edit: impl Fn(usize, &mut TrialRecord),
) -> StudyInput {
    study_with_metadata(n, objectives, complete, edit, |_| {})
}

fn study_with_metadata(
    n: usize,
    objectives: usize,
    complete: bool,
    edit: impl Fn(usize, &mut TrialRecord),
    edit_metadata: impl FnOnce(&mut StudyMetadata),
) -> StudyInput {
    let key = StudyKey {
        source: IdentityAllocator::new().unwrap().source().unwrap(),
        stored_id: 7,
    };
    let mut metadata = StudyMetadata {
        name: "preparation".into(),
        objectives: (0..objectives)
            .map(|i| ObjectiveDescriptor {
                index: ObjectiveIndex(i as u32),
                name: Some(format!("f{i}")),
                direction: Direction::Minimize,
            })
            .collect(),
        ..Default::default()
    };
    edit_metadata(&mut metadata);
    let trials = (0..n)
        .map(|i| {
            let mut t = TrialRecord::new(
                key,
                90 + i as u32 * 3,
                StudyTrialNumber(5 + i as u32 * 2),
                TrialState::Complete,
            );
            t.completed_order = Some(i as u64);
            t.parameters = Arc::new(BTreeMap::from([(
                "x".into(),
                ParameterObservation {
                    value: ParameterValue::Numeric(Number(i as f64)),
                    distribution: None,
                    external_encoding: false,
                    encoded_numeric: None,
                },
            )]));
            t.objectives = Arc::new(
                (0..objectives)
                    .map(|o| (ObjectiveIndex(o as u32), Number(i as f64 * (o + 1) as f64)))
                    .collect(),
            );
            edit(i, &mut t);
            Arc::new(t)
        })
        .collect();
    let mut manager = StudyManager::new(key);
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let receipt = manager
        .apply(
            manager.header(ticket).unwrap(),
            StudyBatch::Replace { trials, metadata },
        )
        .unwrap();
    if complete {
        manager.finish(ticket, receipt.next_sequence).unwrap();
    }
    manager.acquire().unwrap()
}
fn request(kind: AnalysisKind) -> AnalysisRequest {
    AnalysisRequest {
        kind,
        conditions: vec![],
    }
}
fn fit(objectives: FitObjectives, constraints: bool) -> Preparation {
    Preparation::Fit {
        objectives,
        constraints,
    }
}
fn unavailable<T>(result: Result<T, Readiness>, text: &str) {
    assert!(matches!(result, Err(Readiness::Unavailable(message)) if message.contains(text)));
}

#[test]
fn empty_entropy_and_unconfigured_or_featureless_fit_are_not_ready_but_reports_and_heatmaps_are() {
    let input = study(0, 1, false, |_, _| {});
    let mut s = AnalysisSession::default();
    unavailable(
        s.prepare(vec![input.clone()], request(AnalysisKind::Entropy), &[]),
        "At least one",
    );
    unavailable(
        s.prepare(vec![input.clone()], request(AnalysisKind::Fit), &[]),
        "Method-specific",
    );
    unavailable(
        s.prepare_for(
            vec![input.clone()],
            request(AnalysisKind::Fit),
            fit(FitObjectives::Name("f0".into()), false),
            &[],
        ),
        "No numeric",
    );
    assert!(s
        .prepare(vec![input.clone()], request(AnalysisKind::Report), &[])
        .is_ok());
    let heatmap = s
        .prepare_for(
            vec![input],
            request(AnalysisKind::Heatmap),
            Preparation::Sensitivity {
                method: HeatmapMethod::Ridge,
                objective: None,
                feasible_only: false,
                samples: 0,
            },
            &[],
        )
        .unwrap();
    assert!(
        compute_heatmap(&heatmap.data().sensitivity().df, HeatmapMethod::Ridge, 0)
            .values
            .is_empty()
    );
    let input = study(12, 1, true, |_, t| Arc::make_mut(&mut t.parameters).clear());
    unavailable(
        s.prepare_for(
            vec![input],
            request(AnalysisKind::Fit),
            fit(FitObjectives::Index(0), false),
            &[],
        ),
        "No numeric",
    );
    let entropy = study(3, 1, false, |i, t| {
        if i == 0 {
            Arc::make_mut(&mut t.objectives).insert(ObjectiveIndex(0), Number(f64::NAN));
        }
    });
    let p = s
        .prepare(vec![entropy], request(AnalysisKind::Entropy), &[])
        .unwrap();
    assert_eq!(p.data().objectives().rows, [1, 2]);
    assert_eq!(p.row_maps()[0][0].stored_id, 93);
}

#[test]
fn fit_preparation_owns_complete_finite_selection_constraints_and_trial_correspondence() {
    let input = study(14, 1, false, |i, t| {
        if i == 2 {
            Arc::make_mut(&mut t.parameters).clear();
        }
        if i == 13 {
            t.state = TrialState::Pruned;
            t.completed_order = None;
        }
        t.constraints = ConstraintObservation {
            present: true,
            values: vec![Number(if i == 4 { f64::NAN } else { -(i as f64) })].into(),
        };
    });
    let mut s = AnalysisSession::default();
    unavailable(
        s.prepare_for(
            vec![input.clone(), input.clone()],
            request(AnalysisKind::Fit),
            fit(FitObjectives::Name("f0".into()), true),
            &[],
        ),
        "exactly one",
    );
    let p = s
        .prepare_for(
            vec![input.clone()],
            request(AnalysisKind::Fit),
            fit(FitObjectives::Name("f0".into()), true),
            &[],
        )
        .unwrap();
    let data = p.data().fit();
    assert_eq!(data.x.len(), 12);
    assert_eq!(data.x[0], [0.0]);
    assert_eq!(data.rows[..3], [0, 1, 3]);
    assert_eq!(p.row_maps()[0][2].stored_id, 99);
    assert_eq!(
        input.data().trial(p.row_maps()[0][2]).unwrap().number,
        StudyTrialNumber(11)
    );
    assert_eq!(data.constraints[0].values.len(), data.x.len());
    assert!(data.constraints[0].values[3].is_nan());
    assert!(p.row_maps()[0]
        .iter()
        .all(|k| input.data().trial(*k).unwrap().state == TrialState::Complete));
    let short = study(9, 1, true, |_, _| {});
    unavailable(
        s.prepare_for(
            vec![short],
            request(AnalysisKind::Fit),
            fit(FitObjectives::Index(0), false),
            &[],
        ),
        "At least 10",
    );
    let multi = study(14, 2, true, |i, t| {
        if i == 2 {
            Arc::make_mut(&mut t.objectives).insert(ObjectiveIndex(1), Number(f64::NAN));
        }
    });
    let p = s
        .prepare_for(
            vec![multi],
            request(AnalysisKind::Fit),
            fit(FitObjectives::All, false),
            &[],
        )
        .unwrap();
    assert_eq!(p.data().fit().ys.len(), 2);
    assert_eq!(p.data().fit().ys[0].len(), 13);
    assert_eq!(p.data().fit().ys[1].len(), 13);
    assert!(!p.row_maps()[0].iter().any(|k| k.stored_id == 96));
}

#[test]
fn nominal_numeric_looking_categories_remain_ready_unsupported_ridge_data_not_a_numeric_fit() {
    let input = study(12, 1, true, |_, t| {
        Arc::make_mut(&mut t.parameters).clear();
        Arc::make_mut(&mut t.parameters).insert(
            "cat".into(),
            ParameterObservation {
                value: ParameterValue::Category(Arc::new(serde_json::json!(10))),
                distribution: Some(Distribution::Categorical {
                    choices: Arc::new(vec![serde_json::json!(10)]),
                }),
                external_encoding: false,
                encoded_numeric: Some(Number(0.0)),
            },
        );
    });
    let mut s = AnalysisSession::default();
    let p = s
        .prepare_for(
            vec![input.clone()],
            request(AnalysisKind::Importance),
            Preparation::Sensitivity {
                method: HeatmapMethod::Ridge,
                objective: Some(0),
                feasible_only: false,
                samples: 32,
            },
            &[],
        )
        .unwrap();
    let result = RidgeMetric.compute(&p.data().sensitivity().df, 0).unwrap();
    assert_eq!(result.unsupported_categorical, ["cat"]);
    assert!(result.ridge.is_empty());
    unavailable(
        s.prepare_for(
            vec![input],
            request(AnalysisKind::Fit),
            fit(FitObjectives::Index(0), false),
            &[],
        ),
        "No numeric",
    );
}

#[test]
fn model_preparation_owns_the_fitted_dependency_and_rejects_missing_or_retargeted_models() {
    use tdashboard_analysis::surrogate_opt::*;
    let input = study(12, 1, true, |_, _| {});
    let mut s = AnalysisSession::default();
    assert!(matches!(
        s.prepare(vec![input.clone()], request(AnalysisKind::Optimize), &[]),
        Err(Readiness::Waiting)
    ));
    let p = s
        .prepare_for(
            vec![input.clone()],
            request(AnalysisKind::Fit),
            fit(FitObjectives::Index(0), false),
            &[],
        )
        .unwrap();
    let WorkOrShared::Work(work) = s.start(p, ReuseOrRerun::Rerun).unwrap() else {
        panic!()
    };
    let key = work.key().clone();
    let completion = work.execute(|p| {
        let data = p.data().fit();
        let req = SurrogateFitRequest {
            x_matrix: data.x.clone(),
            y: data.ys[0].clone(),
            param_names: data.names.clone(),
            objective_name: data.objectives[0].clone(),
            model: SurrogateModelKind::Ridge,
            auto_select: false,
            constraints: vec![],
            priority_rows: vec![],
            param_bounds: Some(data.bounds.clone()),
        };
        let mut model = fit_surrogate_with_validation(&req)?;
        model.bind_fit_key(key.clone(), &p.row_maps()[0]);
        Ok(model)
    });
    let result = s.complete(completion).unwrap();
    let model = result.payload::<TrainedSurrogate>().unwrap().unwrap();
    let p = s
        .prepare_models(
            vec![input],
            request(AnalysisKind::Optimize),
            Models::Single(model.clone()),
        )
        .unwrap();
    assert_eq!(p.models().single().fit_key(), Some(&key));
    assert_eq!(p.key().dependencies, [key]);
    assert_eq!(p.model_directions(), [true]);
    let other = study(12, 1, true, |_, _| {});
    assert!(s
        .prepare_models(
            vec![other],
            request(AnalysisKind::Suggest),
            Models::Single(model)
        )
        .is_err());
}

#[test]
fn compare_keeps_validation_order_and_rejects_a_best_row_lost_by_feature_filtering() {
    let input = study(12, 1, true, |i, t| {
        if i == 0 {
            Arc::make_mut(&mut t.parameters).clear();
        }
    });
    let mut s = AnalysisSession::default();
    unavailable(
        s.prepare_for(
            vec![input.clone()],
            request(AnalysisKind::Compare),
            Preparation::Compare {
                objective: 0,
                slice: 5,
            },
            &[],
        ),
        "Invalid slice",
    );
    unavailable(
        s.prepare_for(
            vec![input],
            request(AnalysisKind::Compare),
            Preparation::Compare {
                objective: 0,
                slice: 0,
            },
            &[],
        ),
        "anchor",
    );
    let short = study(3, 1, true, |_, _| {});
    let p = s
        .prepare_for(
            vec![short],
            request(AnalysisKind::Compare),
            Preparation::Compare {
                objective: 0,
                slice: 0,
            },
            &[],
        )
        .unwrap();
    assert_eq!(p.data().fit().anchor.as_deref(), Some([0.0].as_slice()));
    assert_eq!(
        p.data().fit().observed,
        [(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]
    );
    // Individual model-fit errors and the all-models-failed presentation remain
    // execution results, not a new blanket Compare preparation rejection.
    assert_eq!(p.data().fit().x.len(), 3);
}

#[test]
fn front_dependent_methods_prepare_distinct_subsets_and_entropy_does_not_wait_for_a_front() {
    let input = study(4, 2, true, |i, t| {
        Arc::make_mut(&mut t.objectives).insert(ObjectiveIndex(1), Number(3.0 - i as f64));
        if i == 1 {
            Arc::make_mut(&mut t.parameters).clear();
        }
    });
    let mut s = AnalysisSession::default();
    let front = s.pareto(input.clone()).unwrap();
    let selection = Preparation::Cluster {
        space: FeatureSpace::Parameters,
        manual_k: Some(3),
    };
    let p = s
        .prepare_for(
            vec![input.clone()],
            request(AnalysisKind::Cluster),
            selection,
            std::slice::from_ref(&front),
        )
        .unwrap();
    assert_eq!(p.data().features().rows, [0, 2, 3]);
    assert_eq!(
        p.row_maps()[0]
            .iter()
            .map(|k| k.stored_id)
            .collect::<Vec<_>>(),
        [90, 96, 99]
    );
    unavailable(
        s.prepare_for(
            vec![input.clone()],
            request(AnalysisKind::Cluster),
            Preparation::Cluster {
                space: FeatureSpace::Parameters,
                manual_k: Some(4),
            },
            std::slice::from_ref(&front),
        ),
        "k must",
    );
    let ranking = s
        .prepare(vec![input], request(AnalysisKind::Mcdm), &[front])
        .unwrap();
    assert_eq!(ranking.data().objectives().rows, [0, 1, 2, 3]);
    assert_eq!(ranking.key().dependencies.len(), 1);
    let partial = study(3, 2, false, |_, _| {});
    assert!(s
        .prepare(vec![partial.clone()], request(AnalysisKind::Entropy), &[])
        .is_ok());
    assert!(matches!(
        s.prepare(vec![partial], request(AnalysisKind::Mcdm), &[]),
        Err(Readiness::Waiting)
    ));
}

#[test]
fn sensitivity_preparation_preserves_pairwise_complete_case_rejection_and_heatmap_fallbacks() {
    let input = study(4, 2, false, |i, t| {
        Arc::make_mut(&mut t.parameters).insert(
            "other".into(),
            ParameterObservation {
                value: ParameterValue::Numeric(Number(if i == 0 || i == 3 {
                    f64::NAN
                } else {
                    i as f64
                })),
                distribution: None,
                external_encoding: false,
                encoded_numeric: None,
            },
        );
        if i == 1 {
            Arc::make_mut(&mut t.parameters).get_mut("x").unwrap().value =
                ParameterValue::Numeric(Number(f64::NAN));
        }
        if i == 2 {
            Arc::make_mut(&mut t.objectives).insert(ObjectiveIndex(0), Number(f64::NAN));
        }
        if i == 3 {
            Arc::make_mut(&mut t.objectives).insert(ObjectiveIndex(1), Number(f64::NAN));
        }
    });
    let mut s = AnalysisSession::default();
    let spec = |method, objective| Preparation::Sensitivity {
        method,
        objective,
        feasible_only: false,
        samples: 32,
    };
    let pairwise = s
        .prepare_for(
            vec![input.clone()],
            request(AnalysisKind::Importance),
            spec(HeatmapMethod::Spearman, Some(0)),
            &[],
        )
        .unwrap();
    let data = pairwise.data().sensitivity();
    let x = data
        .df
        .param_col_names()
        .iter()
        .position(|n| n == "x")
        .unwrap();
    assert_eq!(data.feature_rows[x], [0, 3]);
    assert_eq!(
        SpearmanMetric.compute(&data.df, 0).unwrap().spearman[x][0],
        1.0
    );
    let ridge = s
        .prepare_for(
            vec![input.clone()],
            request(AnalysisKind::Importance),
            spec(HeatmapMethod::Ridge, Some(0)),
            &[],
        )
        .unwrap();
    assert!(ridge.data().sensitivity().feature_rows[0].is_empty());
    assert_eq!(
        RidgeMetric
            .compute(&ridge.data().sensitivity().df, 0)
            .unwrap()
            .ridge[0]
            .r_squared,
        0.0
    );
    assert!(s
        .prepare_for(
            vec![input.clone()],
            request(AnalysisKind::Importance),
            spec(HeatmapMethod::Shap, Some(0)),
            &[]
        )
        .is_err());
    assert!(s
        .prepare_for(
            vec![input.clone()],
            request(AnalysisKind::Importance),
            spec(HeatmapMethod::SobolFirst, Some(0)),
            &[]
        )
        .is_err());
    let heatmap = s
        .prepare_for(
            vec![input],
            request(AnalysisKind::Heatmap),
            spec(HeatmapMethod::Shap, None),
            &[],
        )
        .unwrap();
    assert!(
        compute_heatmap(&heatmap.data().sensitivity().df, HeatmapMethod::Shap, 32)
            .values
            .iter()
            .flatten()
            .all(|v| *v == 0.0)
    );
    let ard = study(12, 1, false, |i, t| {
        if i == 2 {
            Arc::make_mut(&mut t.parameters).get_mut("x").unwrap().value =
                ParameterValue::Numeric(Number(f64::NAN));
        }
    });
    unavailable(
        s.prepare_for(
            vec![ard],
            request(AnalysisKind::Importance),
            spec(HeatmapMethod::Ard, Some(0)),
            &[],
        ),
        "non-finite",
    );
}

#[test]
fn pdp_preparation_maps_finite_rows_without_removing_empty_numerical_fallbacks() {
    let input = study(3, 1, false, |i, t| {
        Arc::make_mut(&mut t.parameters).insert(
            "cat".into(),
            ParameterObservation {
                value: ParameterValue::Category(Arc::new(serde_json::json!("a"))),
                distribution: Some(Distribution::Categorical {
                    choices: Arc::new(vec![serde_json::json!("a")]),
                }),
                external_encoding: false,
                encoded_numeric: Some(Number(0.0)),
            },
        );
        if i == 1 {
            Arc::make_mut(&mut t.parameters).remove("x");
        }
    });
    let mut s = AnalysisSession::default();
    let p = s
        .prepare_for(
            vec![input.clone()],
            request(AnalysisKind::Pdp2d),
            Preparation::Pdp {
                params: vec!["x".into(), "cat".into()],
                objective: "f0".into(),
                feasible_only: false,
                two_dimensional: true,
            },
            &[],
        )
        .unwrap();
    assert_eq!(p.data().pdp().rows, [0, 2]);
    assert_eq!(p.row_maps()[0][1].stored_id, 96);
    let short = study(1, 1, false, |_, _| {});
    let p = s
        .prepare_for(
            vec![short],
            request(AnalysisKind::Pdp1d),
            Preparation::Pdp {
                params: vec!["x".into()],
                objective: "f0".into(),
                feasible_only: false,
                two_dimensional: false,
            },
            &[],
        )
        .unwrap();
    let data = p.data().pdp();
    assert!(tdashboard_analysis::pdp::compute_pdp_from_data(
        data.x.clone(),
        data.y.clone(),
        data.names.clone(),
        &data.objective,
        data.targets[0],
        0,
        "ridge"
    )
    .grid
    .is_empty());
}

#[test]
fn convergence_and_synchronous_feature_preparations_keep_each_inputs_maps_and_method_minimums() {
    let a = study(120, 2, true, |i, t| {
        t.constraints = ConstraintObservation {
            present: true,
            values: vec![Number(if i == 4 { f64::NAN } else { -1.0 })].into(),
        };
    });
    let b = study(3, 2, true, |_, _| {});
    let mut s = AnalysisSession::default();
    let p = s
        .prepare(
            vec![a.clone(), b.clone()],
            request(AnalysisKind::Convergence),
            &[],
        )
        .unwrap();
    assert_eq!(p.data().series()[0].step, 2);
    assert!(p.data().series()[0].objectives[2]
        .iter()
        .all(|v| v.is_nan()));
    assert_eq!(p.row_maps()[0][2].stored_id, 102);
    assert_eq!(p.row_maps()[1][0].source, b.stamp().input.study.source);
    let two = study(2, 1, true, |_, _| {});
    assert!(s
        .prepare_for(
            vec![two.clone()],
            request(AnalysisKind::Pca),
            Preparation::Pca {
                space: tdashboard_analysis::clustering::PcaSpace::Param,
                components: 2
            },
            &[]
        )
        .is_ok());
    assert!(s
        .prepare_for(
            vec![two.clone()],
            request(AnalysisKind::Linkage),
            Preparation::Linkage {
                names: vec!["x".into()]
            },
            &[]
        )
        .is_ok());
    assert!(s
        .prepare_for(
            vec![two],
            request(AnalysisKind::Som),
            Preparation::Som {
                names: vec!["x".into()],
                spec: Default::default()
            },
            &[]
        )
        .is_err());
    let bad_spec = tdashboard_analysis::clustering::SomSpec {
        n_epochs: 0,
        ..Default::default()
    };
    assert!(s
        .prepare_for(
            vec![b],
            request(AnalysisKind::Som),
            Preparation::Som {
                names: vec!["x".into()],
                spec: bad_spec
            },
            &[]
        )
        .is_err());
    let partial = study(12, 1, false, |_, _| {});
    assert!(matches!(
        s.prepare(vec![partial], request(AnalysisKind::History), &[]),
        Err(Readiness::Waiting)
    ));
    assert!(s
        .prepare(vec![a], request(AnalysisKind::History), &[])
        .is_err());
}

#[test]
fn contour_preparation_rejects_degenerate_bounds_but_preserves_collinear_masked_results() {
    let diagonal = study(3, 1, false, |_, _| {});
    let mut session = AnalysisSession::default();
    let spec = |y: &str| Preparation::Contour {
        x: "x".into(),
        y: y.into(),
        value: "f0".into(),
        feasible_only: false,
        n_grid: 5,
    };
    let p = session
        .prepare_for(
            vec![diagonal],
            request(AnalysisKind::Contour),
            spec("f0"),
            &[],
        )
        .unwrap();
    let data = p.data().contour();
    assert_eq!(p.row_maps()[0].len(), 3);
    let surface = tdashboard_analysis::contour::observed_surface_prepared(&data.geometry, 5, 0.0);
    assert_eq!(surface.x_values.len(), 5);
    assert!(surface.z.iter().flatten().all(Option::is_none));
    let flat = study(3, 2, false, |_, t| {
        Arc::make_mut(&mut t.objectives).insert(ObjectiveIndex(1), Number(0.0));
    });
    unavailable(
        session.prepare_for(vec![flat], request(AnalysisKind::Contour), spec("f1"), &[]),
        "degenerate",
    );
}

#[test]
fn full_objective_preparation_preserves_direction_order_with_repeated_labels() {
    let input = study_with_metadata(
        12,
        2,
        true,
        |_, _| {},
        |m| {
            m.objectives[0].name = Some("same".into());
            m.objectives[1].name = Some("same".into());
            m.objectives[1].direction = Direction::Maximize;
        },
    );
    let mut session = AnalysisSession::default();
    let p = session
        .prepare_for(
            vec![input.clone()],
            request(AnalysisKind::Fit),
            fit(FitObjectives::All, false),
            &[],
        )
        .unwrap();
    assert_eq!(p.data().fit().minimize, [true, false]);
    let front = session.pareto(input.clone()).unwrap();
    let p = session
        .prepare(vec![input], request(AnalysisKind::Mcdm), &[front])
        .unwrap();
    assert_eq!(p.data().objectives().minimize, [true, false]);
}
