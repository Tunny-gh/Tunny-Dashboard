use std::collections::BTreeMap;
use std::sync::Arc;
use tdashboard_analysis::session::*;
use tdashboard_study::*;

fn fixture() -> (StudyManager, LoadTicket, ApplyReceipt) {
    let key = StudyKey {
        source: IdentityAllocator::new().unwrap().source().unwrap(),
        stored_id: 7,
    };
    let mut manager = StudyManager::new(key);
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let metadata = StudyMetadata {
        name: "same name".into(),
        objectives: vec![ObjectiveDescriptor {
            index: ObjectiveIndex(0),
            name: Some("y".into()),
            direction: Direction::Minimize,
        }],
        ..Default::default()
    };
    let rows = (0..3)
        .map(|i| {
            let mut t = TrialRecord::new(
                key,
                90 + i,
                StudyTrialNumber(4 + 3 * i),
                TrialState::Complete,
            );
            t.completed_order = Some(u64::from(2 - i));
            t.objectives = Arc::new(BTreeMap::from([(ObjectiveIndex(0), Number(f64::from(i)))]));
            t.parameters = Arc::new(BTreeMap::from([(
                "x".into(),
                ParameterObservation {
                    value: ParameterValue::Numeric(Number(f64::from(i))),
                    distribution: None,
                    external_encoding: false,
                    encoded_numeric: None,
                },
            )]));
            t.constraints = ConstraintObservation {
                present: true,
                values: vec![Number(-1.0)].into(),
            };
            Arc::new(t)
        })
        .collect();
    let receipt = manager
        .apply(
            manager.header(ticket).unwrap(),
            StudyBatch::Replace {
                metadata,
                trials: rows,
            },
        )
        .unwrap();
    (manager, ticket, receipt)
}

fn request(kind: AnalysisKind, condition: u64) -> AnalysisRequest {
    AnalysisRequest {
        kind,
        conditions: vec![("condition".into(), Condition::Integer(condition))],
    }
}

fn work(session: &mut AnalysisSession, input: StudyInput, value: u64) -> AnalysisWork {
    let prepared = session
        .prepare(vec![input], request(AnalysisKind::Distribution, value), &[])
        .unwrap();
    match session.start(prepared, ReuseOrRerun::Rerun).unwrap() {
        WorkOrShared::Work(work) => work,
        _ => panic!("Explicit rerun must create work"),
    }
}

#[test]
fn explicit_input_shares_tables_and_results_but_not_conditions_or_sources() {
    let (manager, _, _) = fixture();
    let input = manager.acquire().unwrap();
    let mut session = AnalysisSession::default();
    let p1 = session
        .prepare(
            vec![input.clone()],
            request(AnalysisKind::Distribution, 1),
            &[],
        )
        .unwrap();
    let p2 = session
        .prepare(
            vec![input.clone()],
            request(AnalysisKind::Distribution, 2),
            &[],
        )
        .unwrap();
    assert!(Arc::ptr_eq(&p1.tables()[0], &p2.tables()[0]));
    assert_ne!(p1.key(), p2.key());
    assert_eq!(
        p1.tables()[0]
            .rows
            .iter()
            .map(|k| k.stored_id)
            .collect::<Vec<_>>(),
        [92, 91, 90]
    );
    let key = p1.key().clone();
    let WorkOrShared::Work(first) = session.start(p1, ReuseOrRerun::Reuse).unwrap() else {
        panic!()
    };
    let same = session
        .prepare(
            vec![input.clone()],
            request(AnalysisKind::Distribution, 1),
            &[],
        )
        .unwrap();
    let WorkOrShared::Shared(pending) = session.start(same, ReuseOrRerun::Reuse).unwrap() else {
        panic!()
    };
    let result = session
        .complete(first.execute(|p| Ok(p.tables()[0].rows.clone())))
        .unwrap();
    assert!(pending.same_result(&result));
    assert!(can_adopt(&result, result.key()));
    assert_eq!(
        result
            .payload::<Vec<TrialKey>>()
            .unwrap()
            .unwrap()
            .as_slice(),
        result.rows()[0]
    );
    let (other, _, _) = fixture();
    let other = session
        .prepare(
            vec![other.acquire().unwrap()],
            request(AnalysisKind::Distribution, 1),
            &[],
        )
        .unwrap();
    assert_ne!(&key, other.key());
    session.retain_interests(&[]);
    assert!(session.lookup(&key).is_none());
    assert!(result.payload::<Vec<TrialKey>>().unwrap().is_ok());
}

#[test]
fn completion_cannot_publish_into_another_sessions_equal_numbered_slot() {
    let (manager, _, _) = fixture();
    let input = manager.acquire().unwrap();
    let mut a = AnalysisSession::default();
    let mut b = AnalysisSession::default();
    let first = work(&mut a, input.clone(), 0);
    let second = work(&mut b, input, 0);
    assert_eq!(first.key(), second.key());
    assert!(b.complete(first.execute(|_| Ok(1u32))).is_none());
    assert_eq!(
        *b.complete(second.execute(|_| Ok(2u32)))
            .unwrap()
            .payload::<u32>()
            .unwrap()
            .unwrap(),
        2
    );
}

#[test]
fn same_count_updates_schema_growth_and_completion_invalidate_without_mixing_inputs() {
    let (mut manager, ticket, receipt) = fixture();
    let old = manager.acquire().unwrap();
    let mut session = AnalysisSession::default();
    assert!(matches!(
        session.pareto(old.clone()),
        Err(Readiness::Waiting)
    ));
    let old_work = work(&mut session, old.clone(), 0);
    let original = old_work.prepared().tables()[0]
        .dataframe
        .get_numeric_column("y")
        .unwrap()
        .to_vec();
    let mut changed = (**old.data().trials().first().unwrap()).clone();
    Arc::make_mut(&mut changed.objectives).insert(ObjectiveIndex(0), Number(99.0));
    let mut evidence = TrialRecord::new(
        old.stamp().input.study,
        500,
        StudyTrialNumber(99),
        TrialState::Pruned,
    );
    evidence.constraints = ConstraintObservation {
        present: true,
        values: vec![Number(-1.0), Number(-1.0)].into(),
    };
    let receipt = manager
        .apply(
            receipt.next_header(ticket),
            StudyBatch::Delta {
                upserts: vec![Arc::new(changed), Arc::new(evidence)],
                removals: vec![],
                metadata: MetadataEdit::Unchanged,
            },
        )
        .unwrap();
    let current = manager.acquire().unwrap();
    let new_work = work(&mut session, current.clone(), 0);
    assert_eq!(
        old_work.prepared().tables()[0].dataframe.row_count(),
        new_work.prepared().tables()[0].dataframe.row_count()
    );
    assert_eq!(
        old_work.prepared().tables()[0]
            .dataframe
            .get_numeric_column("y")
            .unwrap(),
        original
    );
    assert!(
        new_work.prepared().tables()[0]
            .dataframe
            .feasibility()
            .state(0)
            == FeasibilityState::Unverified
    );
    assert!(session.complete(old_work.execute(|_| Ok(1u32))).is_none());
    assert!(session.complete(new_work.execute(|_| Ok(2u32))).is_some());
    manager.finish(ticket, receipt.next_sequence).unwrap();
    let completed = manager.acquire().unwrap();
    assert_eq!(current.stamp().input, completed.stamp().input);
    assert_ne!(current.stamp(), completed.stamp());
    let front = session.pareto(completed.clone()).unwrap();
    assert!(front
        .payload::<tdashboard_analysis::pareto::ParetoResult>()
        .unwrap()
        .unwrap()
        .pareto_indices
        .is_empty());
    let prepared = session.prepare_for(
        vec![completed.clone()],
        request(AnalysisKind::Cluster, 0),
        tdashboard_analysis::preparation::Preparation::Cluster {
            space: tdashboard_analysis::preparation::FeatureSpace::Parameters,
            manual_k: None,
        },
        std::slice::from_ref(&front),
    );
    assert!(
        matches!(prepared, Err(Readiness::Unavailable(message)) if message.contains("At least 2"))
    );
    assert!(matches!(
        session.prepare_for(
            vec![current],
            request(AnalysisKind::Cluster, 0),
            tdashboard_analysis::preparation::Preparation::Cluster {
                space: tdashboard_analysis::preparation::FeatureSpace::Parameters,
                manual_k: None
            },
            &[front]
        ),
        Err(Readiness::Waiting)
    ));
}

#[test]
fn rerun_failure_reload_and_dependency_keys_cannot_relabel_previous_work() {
    let (mut manager, ticket, receipt) = fixture();
    manager.finish(ticket, receipt.next_sequence).unwrap();
    let input = manager.acquire().unwrap();
    let mut session = AnalysisSession::default();
    let first = work(&mut session, input.clone(), 0);
    let second = work(&mut session, input.clone(), 0);
    assert_ne!(first.key(), second.key());
    assert!(session
        .complete(first.execute::<u32>(|_| Err("old failure".into())))
        .is_none());
    let fit = session.complete(second.execute(|_| Ok(7u32))).unwrap();
    let opt = session
        .prepare(
            vec![input.clone()],
            request(AnalysisKind::Distribution, 0),
            std::slice::from_ref(&fit),
        )
        .unwrap();
    assert_eq!(opt.key().dependencies, [fit.key().clone()]);
    let reload = manager.begin(LoadKind::Reload).unwrap();
    manager
        .apply(
            manager.header(reload).unwrap(),
            StudyBatch::Replace {
                metadata: (**input.data().metadata()).clone(),
                trials: input.data().trials().as_ref().clone(),
            },
        )
        .unwrap();
    let reloaded = manager.acquire().unwrap();
    assert_ne!(
        input.stamp().input.generation,
        reloaded.stamp().input.generation
    );
    assert!(session
        .prepare(
            vec![reloaded],
            request(AnalysisKind::Distribution, 0),
            &[fit]
        )
        .is_err());
    assert_eq!(opt.inputs()[0].stamp(), input.stamp());
}

#[test]
fn comparison_keys_track_every_input_without_invalidating_independent_study() {
    let (mut a, at, ar) = fixture();
    a.finish(at, ar.next_sequence).unwrap();
    let (mut b, ticket, receipt) = fixture();
    b.finish(ticket, receipt.next_sequence).unwrap();
    let a = a.acquire().unwrap();
    let old_b = b.acquire().unwrap();
    let mut session = AnalysisSession::default();
    let a_result = work(&mut session, a.clone(), 0);
    let a_key = a_result.prepared().key().clone();
    session.complete(a_result.execute(|_| Ok(1u32))).unwrap();
    let first = session
        .prepare(
            vec![a.clone(), old_b.clone()],
            request(AnalysisKind::Convergence, 0),
            &[],
        )
        .unwrap();
    let ticket = b.begin(LoadKind::Refresh).unwrap();
    let receipt = b
        .apply(
            b.header(ticket).unwrap(),
            StudyBatch::Replace {
                metadata: (**old_b.data().metadata()).clone(),
                trials: old_b.data().trials().as_ref().clone(),
            },
        )
        .unwrap();
    b.finish(ticket, receipt.next_sequence).unwrap();
    let second = session
        .prepare(
            vec![a, b.acquire().unwrap()],
            request(AnalysisKind::Convergence, 0),
            &[],
        )
        .unwrap();
    assert_ne!(first.key(), second.key());
    assert!(session.lookup(&a_key).is_some());
}

#[test]
fn subset_tables_keep_trial_keys_and_release_source_roots_after_work_finishes() {
    let (mut manager, ticket, receipt) = fixture();
    manager.finish(ticket, receipt.next_sequence).unwrap();
    let input = manager.acquire().unwrap();
    let weak = Arc::downgrade(input.data());
    let mut session = AnalysisSession::default();
    let job = work(&mut session, input.clone(), 0);
    let df = job.prepared().tables()[0].dataframe.filter_feasible();
    assert_eq!(df.trial_keys().unwrap(), job.prepared().tables()[0].rows);
    let key = job.key().clone();
    drop(input);
    drop(manager);
    assert!(weak.upgrade().is_some());
    let result = session.complete(job.execute(|_| Ok(123u32))).unwrap();
    assert!(weak.upgrade().is_none());
    session.release(&key);
    assert_eq!(*result.payload::<u32>().unwrap().unwrap(), 123);
    assert!(Condition::finite(f64::NAN).is_err());
    assert_eq!(
        Condition::finite(-0.0).unwrap(),
        Condition::finite(0.0).unwrap()
    );
}

#[test]
fn journal_nonfinite_objectives_do_not_shift_into_finite_analysis() {
    for inline in [true, false] {
        for token in ["Infinity", "NaN", "-Infinity"] {
            let trial = if inline {
                format!("{{\"op_code\":4,\"study_id\":0,\"distributions\":{{}},\"state\":1,\"values\":[{token},5]}}\n")
            } else {
                format!("{{\"op_code\":4,\"study_id\":0}}\n{{\"op_code\":6,\"trial_id\":0,\"state\":1,\"values\":[{token},5]}}\n")
            };
            let data = format!("{{\"op_code\":0,\"study_name\":\"s\",\"directions\":[1,1]}}\n{trial}{{\"op_code\":4,\"study_id\":0,\"distributions\":{{}},\"state\":1,\"values\":[7,8]}}\n");
            let (_, df, _) = {
                let p = common::journal(data.as_bytes(), 0);
                (p.meta, p.dataframe, p.extras)
            };
            assert_eq!(
                tdashboard_analysis::pdp::extract_xy(&df, &[], "obj0", false).1,
                [7.0]
            );
            assert_eq!(
                tdashboard_analysis::pdp::extract_xy(&df, &[], "obj1", false).1,
                [5.0, 8.0]
            );
        }
    }
}

#[test]
fn optuna_numeric_categorical_choices_stay_nominal_while_int_and_float_are_supported() {
    use serde_json::json;
    use tdashboard_analysis::sensitivity::{RidgeMetric, SensitivityMetric, SpearmanMetric};
    let mut lines = vec![
        json!({"op_code":0,"worker_id":"w","study_name":"nominal","directions":[1]}).to_string(),
    ];
    for i in 0..12 {
        lines.push(json!({"op_code":4,"worker_id":"w","study_id":0}).to_string());
        for (name, value, distribution) in [
            (
                "cat",
                (i % 3) as f64,
                json!({"name":"CategoricalDistribution","choices":[10,2,99]}),
            ),
            (
                "integer",
                i as f64,
                json!({"name":"IntDistribution","low":0,"high":20,"step":1,"log":false}),
            ),
            (
                "float",
                i as f64 / 2.0,
                json!({"name":"FloatDistribution","low":0.0,"high":20.0,"log":false}),
            ),
        ] {
            lines.push(json!({"op_code":5,"worker_id":"w","trial_id":i,"param_name":name,"param_value_internal":value,"distribution":distribution}).to_string());
        }
        lines.push(
            json!({"op_code":6,"worker_id":"w","trial_id":i,"state":1,"values":[i as f64]})
                .to_string(),
        );
    }
    let (_, df, _) = {
        let p = common::journal(lines.join("\n").as_bytes(), 0);
        (p.meta, p.dataframe, p.extras)
    };
    assert!(df.get_string_column("cat").is_some());
    assert!(df.get_numeric_column("cat").is_none());
    for metric in [&SpearmanMetric as &dyn SensitivityMetric, &RidgeMetric] {
        let r = metric.compute(&df, 0).unwrap();
        assert_eq!(r.unsupported_categorical, vec!["cat"]);
        assert_eq!(r.param_names.len(), 2);
        assert!(r.param_names.iter().any(|n| n == "integer"));
        assert!(r.param_names.iter().any(|n| n == "float"));
    }
}

mod common;
