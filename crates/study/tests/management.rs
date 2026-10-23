use std::collections::BTreeMap;
use std::sync::Arc;
use tdashboard_study::*;

fn key() -> StudyKey {
    StudyKey {
        source: IdentityAllocator::new().unwrap().source().unwrap(),
        stored_id: 4,
    }
}

#[test]
fn numeric_attributes_preserve_nonfinite_identity_revisions_and_retained_inputs() {
    let k = key();
    let mut record = TrialRecord::new(k, 1, StudyTrialNumber(0), TrialState::Running);
    record.user_attributes = Arc::new(BTreeMap::from([
        (
            "positive".into(),
            Arc::new(AttributeValue::from(f64::INFINITY)),
        ),
        (
            "negative".into(),
            Arc::new(AttributeValue::from(f64::NEG_INFINITY)),
        ),
        ("nan".into(), Arc::new(AttributeValue::from(f64::NAN))),
        ("null".into(), Arc::new(serde_json::Value::Null.into())),
        (
            "text".into(),
            Arc::new(serde_json::json!("Infinity").into()),
        ),
    ]));
    let mut manager = StudyManager::new(k);
    let (ticket, receipt) = start(&mut manager, vec![Arc::new(record.clone())]);
    let old = manager.acquire().unwrap();
    let no_change = manager
        .apply(
            receipt.next_header(ticket),
            delta(vec![Arc::new(record.clone())]),
        )
        .unwrap();
    assert_eq!(no_change.stamp.input.revision, old.stamp().input.revision);
    Arc::make_mut(&mut record.user_attributes).insert(
        "positive".into(),
        Arc::new(AttributeValue::from(f64::NEG_INFINITY)),
    );
    manager
        .apply(no_change.next_header(ticket), delta(vec![Arc::new(record)]))
        .unwrap();
    assert_eq!(
        manager.acquire().unwrap().stamp().input.revision,
        old.stamp().input.revision + 1
    );
    let attrs = &old.data().trials()[0].user_attributes;
    assert_eq!(
        attrs["positive"].as_ref(),
        &AttributeValue::from(f64::INFINITY)
    );
    assert_ne!(attrs["positive"], attrs["negative"]);
    assert_ne!(attrs["nan"], attrs["null"]);
    assert_ne!(attrs["positive"], attrs["text"]);
    assert!(!attrs.contains_key("missing"));
}
fn trial(key: StudyKey, id: u32, number: u32, state: TrialState) -> Arc<TrialRecord> {
    let mut t = TrialRecord::new(key, id, StudyTrialNumber(number), state);
    t.parameters = Arc::new(BTreeMap::from([(
        "x".into(),
        ParameterObservation {
            value: ParameterValue::Numeric(Number(f64::NAN)),
            distribution: None,
            external_encoding: false,
            encoded_numeric: None,
        },
    )]));
    t.objectives = Arc::new(BTreeMap::from([(ObjectiveIndex(3), Number(f64::INFINITY))]));
    t.user_attributes = Arc::new(BTreeMap::from([(
        "null".into(),
        Arc::new(serde_json::Value::Null.into()),
    )]));
    t.completed_order = (state == TrialState::Complete).then_some(u64::from(id));
    Arc::new(t)
}
fn replace(trials: Vec<Arc<TrialRecord>>) -> StudyBatch {
    StudyBatch::Replace {
        trials,
        metadata: Default::default(),
    }
}
fn delta(upserts: Vec<Arc<TrialRecord>>) -> StudyBatch {
    StudyBatch::Delta {
        upserts,
        removals: vec![],
        metadata: MetadataEdit::Unchanged,
    }
}
fn start(manager: &mut StudyManager, trials: Vec<Arc<TrialRecord>>) -> (LoadTicket, ApplyReceipt) {
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let receipt = manager
        .apply(manager.header(ticket).unwrap(), replace(trials))
        .unwrap();
    (ticket, receipt)
}

#[test]
fn all_states_and_namespaces_are_not_rows_or_trial_numbers() {
    let k = key();
    let records = [
        TrialState::Complete,
        TrialState::Running,
        TrialState::Pruned,
        TrialState::Fail,
        TrialState::Waiting,
    ]
    .into_iter()
    .enumerate()
    .map(|(i, state)| trial(k, 90 + i as u32, 2 + i as u32 * 7, state))
    .collect();
    let mut m = StudyManager::new(k);
    start(&mut m, records);
    let input = m.acquire().unwrap();
    assert_eq!(input.data().trials().len(), 5);
    for (i, t) in input.data().trials().iter().enumerate() {
        assert_eq!(t.key.stored_id, 90 + i as u32);
        assert_eq!(t.number.0, 2 + i as u32 * 7);
        assert_eq!(t.user_attributes["null"].as_ref(), &serde_json::Value::Null);
        assert!(
            matches!(t.parameters["x"].value, ParameterValue::Numeric(Number(n)) if n.is_nan())
        );
        assert!(t.objectives[&ObjectiveIndex(3)].0.is_infinite());
    }
    let replacement = key();
    assert_ne!(k.source, replacement.source);
    assert!(input
        .data()
        .trial(TrialKey {
            source: replacement.source,
            stored_id: 90
        })
        .is_none());
    let mut other = StudyManager::new(replacement);
    start(
        &mut other,
        vec![trial(replacement, 90, 2, TrialState::Complete)],
    );
    assert!(!other.acquire().unwrap().matches(input.stamp()));
}

#[test]
fn invalid_batches_are_atomic_and_do_not_consume_sequence() {
    let k = key();
    let mut m = StudyManager::new(k);
    let (ticket, receipt) = start(&mut m, vec![trial(k, 10, 7, TrialState::Complete)]);
    let input = m.acquire().unwrap();
    let header = receipt.next_header(ticket);
    let mut other = StudyManager::new(key());
    let other_ticket = other.begin(LoadKind::Initial).unwrap();
    assert_eq!(
        m.apply(
            BatchHeader {
                ticket: other_ticket,
                ..header
            },
            delta(vec![])
        ),
        Err(StudyError::InvalidTicket)
    );
    assert_eq!(
        m.apply(
            BatchHeader {
                sequence: header.sequence + 1,
                ..header
            },
            delta(vec![])
        ),
        Err(StudyError::InvalidSequence)
    );
    let mut bad = (*trial(k, 11, 8, TrialState::Complete)).clone();
    bad.study = key();
    assert_eq!(
        m.apply(
            header,
            delta(vec![trial(k, 12, 9, TrialState::Running), Arc::new(bad)])
        ),
        Err(StudyError::InvalidRecord)
    );
    assert_eq!(m.header(ticket).unwrap(), header);
    assert!(Arc::ptr_eq(m.acquire().unwrap().data(), input.data()));
    assert_eq!(
        m.apply(
            BatchHeader {
                sequence: 0,
                ..header
            },
            delta(vec![])
        ),
        Err(StudyError::InvalidSequence)
    );
    assert_eq!(
        m.apply(
            BatchHeader {
                base_revision: 0,
                ..header
            },
            delta(vec![])
        ),
        Err(StudyError::StaleRevision)
    );
    let duplicate = trial(k, 11, 8, TrialState::Running);
    assert_eq!(
        m.apply(header, delta(vec![duplicate.clone(), duplicate])),
        Err(StudyError::InvalidRecord)
    );
    let mut duplicate_order = (*trial(k, 11, 8, TrialState::Complete)).clone();
    duplicate_order.completed_order = Some(10);
    assert_eq!(
        m.apply(header, delta(vec![Arc::new(duplicate_order)])),
        Err(StudyError::InvalidRecord)
    );
    let mut duplicate_number = (*trial(k, 11, 7, TrialState::Running)).clone();
    duplicate_number.datetime_start = Some(Number(3.0));
    assert_eq!(
        m.apply(header, delta(vec![Arc::new(duplicate_number)])),
        Err(StudyError::InvalidRecord)
    );
    assert_eq!(m.acquire().unwrap().stamp(), input.stamp());
    let mut unsorted = (*trial(k, 15, 11, TrialState::Running)).clone();
    unsorted.intermediate = Arc::new(vec![(4, Number(2.0)), (1, Number(3.0))]);
    assert_eq!(
        m.apply(header, delta(vec![Arc::new(unsorted)])),
        Err(StudyError::InvalidRecord)
    );
    let mut wrong_kind = (*trial(k, 15, 11, TrialState::Running)).clone();
    Arc::make_mut(&mut wrong_kind.parameters)
        .get_mut("x")
        .unwrap()
        .distribution = Some(Distribution::Categorical {
        choices: Default::default(),
    });
    assert_eq!(
        m.apply(header, delta(vec![Arc::new(wrong_kind)])),
        Err(StudyError::InvalidRecord)
    );
    m.fail(ticket).unwrap();
    assert_eq!(
        m.apply(header, delta(vec![])),
        Err(StudyError::InvalidTicket)
    );
    assert_eq!(m.finish(ticket, 1), Err(StudyError::InvalidTicket));
}

#[test]
fn noop_nan_metadata_and_completion_have_distinct_revision_semantics() {
    let k = key();
    let mut m = StudyManager::new(k);
    let row = trial(k, 10, 3, TrialState::Complete);
    let (ticket, first) = start(&mut m, vec![row.clone()]);
    let old = m.acquire().unwrap();
    let noop = m
        .apply(first.next_header(ticket), delta(vec![row]))
        .unwrap();
    assert_eq!(noop.stamp.input.revision, first.stamp.input.revision);
    assert_ne!(noop.stamp.load_epoch, first.stamp.load_epoch);
    assert!(Arc::ptr_eq(old.data(), m.acquire().unwrap().data()));
    let metadata = StudyMetadata {
        name: "renamed".into(),
        constrained: true,
        ..Default::default()
    };
    let changed = m
        .apply(
            noop.next_header(ticket),
            StudyBatch::Delta {
                upserts: vec![],
                removals: vec![],
                metadata: MetadataEdit::Set(metadata),
            },
        )
        .unwrap();
    assert_eq!(changed.stamp.input.revision, first.stamp.input.revision + 1);
    assert!(Arc::ptr_eq(
        old.data().trials(),
        m.acquire().unwrap().data().trials()
    ));
    assert_eq!(
        m.acquire().unwrap().data().constraint_schema(),
        ConstraintSchema {
            present: true,
            width: 0
        }
    );
    let completed = m.finish(ticket, changed.next_sequence).unwrap();
    assert_eq!(completed.input, changed.stamp.input);
    assert_ne!(completed.load_epoch, changed.stamp.load_epoch);
    assert_eq!(old.load_state().phase, LoadPhase::Loading);
    assert_eq!(m.acquire().unwrap().load_state().phase, LoadPhase::Complete);
}

#[test]
fn all_state_constraint_growth_reclassifies_history_and_matches_replacement() {
    let k = key();
    let mut complete = (*trial(k, 10, 1, TrialState::Complete)).clone();
    complete.constraints = ConstraintObservation {
        present: true,
        values: Arc::new(vec![Number(-1.0)]),
    };
    let complete = Arc::new(complete);
    let mut m = StudyManager::new(k);
    let (ticket, receipt) = start(&mut m, vec![complete.clone()]);
    let old = m.acquire().unwrap();
    assert_eq!(
        old.data()
            .constraint_schema()
            .classify(&complete.constraints),
        FeasibilityState::Feasible
    );
    let mut running = (*trial(k, 20, 8, TrialState::Running)).clone();
    running.constraints = ConstraintObservation {
        present: true,
        values: Arc::new(vec![Number(-2.0), Number(f64::NAN), Number(0.0)]),
    };
    let running = Arc::new(running);
    m.apply(receipt.next_header(ticket), delta(vec![running.clone()]))
        .unwrap();
    let grown = m.acquire().unwrap();
    assert_eq!(
        grown.data().constraint_schema(),
        ConstraintSchema {
            present: true,
            width: 3
        }
    );
    assert_eq!(
        grown
            .data()
            .constraint_schema()
            .classify(&complete.constraints),
        FeasibilityState::Unverified
    );
    assert_eq!(old.data().constraint_schema().width, 1);
    let mut bulk = StudyManager::new(k);
    start(&mut bulk, vec![complete, running]);
    assert_eq!(grown.data(), bulk.acquire().unwrap().data());
    // The same non-COMPLETE record changes schema without adding any rows.
    let mut changed_running = (*grown.data().trials()[1]).clone();
    changed_running.constraints.values = Arc::new(vec![Number(-1.0); 4]);
    let next = m.header(ticket).unwrap();
    m.apply(next, delta(vec![Arc::new(changed_running)]))
        .unwrap();
    let wider = m.acquire().unwrap();
    assert_eq!(wider.data().constraint_schema().width, 4);
    assert_eq!(grown.data().trials().len(), wider.data().trials().len());
    assert!(Arc::ptr_eq(
        &grown.data().trials()[0],
        &wider.data().trials()[0]
    ));
    assert_ne!(grown.stamp().input.revision, wider.stamp().input.revision);
}

#[test]
fn reload_failure_handoff_empty_and_superseded_messages() {
    let k = key();
    let mut m = StudyManager::new(k);
    let (initial, receipt) = start(&mut m, vec![trial(k, 10, 1, TrialState::Complete)]);
    m.finish(initial, receipt.next_sequence).unwrap();
    let old = m.acquire().unwrap();
    let failed = m.begin(LoadKind::Reload).unwrap();
    assert_eq!(m.acquire().unwrap().stamp(), old.stamp());
    m.fail(failed).unwrap();
    assert_eq!(m.acquire().unwrap().stamp(), old.stamp());
    let reload = m.begin(LoadKind::Reload).unwrap();
    assert_eq!(m.finish(reload, 0), Err(StudyError::NoInput));
    assert_eq!(
        m.apply(m.header(reload).unwrap(), delta(vec![])),
        Err(StudyError::InvalidRecord)
    );
    let receipt = m.apply(m.header(reload).unwrap(), replace(vec![])).unwrap();
    let empty = m.acquire().unwrap();
    assert!(empty.data().trials().is_empty());
    assert_eq!(empty.stamp().input.revision, 1);
    assert!(!empty.matches(old.stamp()));
    m.fail(reload).unwrap();
    assert_eq!(m.acquire().unwrap().stamp().input, empty.stamp().input);
    assert_eq!(m.acquire().unwrap().load_state().phase, LoadPhase::Failed);
    let latest = m.begin(LoadKind::Reload).unwrap();
    assert_eq!(
        m.apply(receipt.next_header(reload), replace(vec![])),
        Err(StudyError::InvalidTicket)
    );
    assert_eq!(m.fail(reload), Err(StudyError::InvalidTicket));
    let completed = m.apply(m.header(latest).unwrap(), replace(vec![])).unwrap();
    m.finish(latest, completed.next_sequence).unwrap();
    assert!(m.acquire().unwrap().data().trials().is_empty());
    assert_eq!(m.acquire().unwrap().load_state().phase, LoadPhase::Complete);
}

#[test]
fn retained_roots_and_unchanged_payloads_are_shared_then_released() {
    let k = key();
    let mut m = StudyManager::new(k);
    let (ticket, receipt) = start(
        &mut m,
        vec![
            trial(k, 10, 1, TrialState::Complete),
            trial(k, 20, 2, TrialState::Running),
        ],
    );
    let old = m.acquire().unwrap();
    let root = Arc::downgrade(old.data());
    let directory = Arc::downgrade(old.data().trials());
    let original = &old.data().trials()[0];
    let old_record = Arc::downgrade(original);
    let mut updated = (**original).clone();
    Arc::make_mut(&mut updated.user_attributes)
        .insert("new".into(), Arc::new(serde_json::json!([1, 2]).into()));
    m.apply(receipt.next_header(ticket), delta(vec![Arc::new(updated)]))
        .unwrap();
    let new = m.acquire().unwrap();
    let changed = &new.data().trials()[0];
    assert!(Arc::ptr_eq(&original.parameters, &changed.parameters));
    assert!(Arc::ptr_eq(
        &original.user_attributes["null"],
        &changed.user_attributes["null"]
    ));
    assert!(Arc::ptr_eq(
        &old.data().trials()[1],
        &new.data().trials()[1]
    ));
    assert!(!original.user_attributes.contains_key("new"));
    assert!(changed.user_attributes.contains_key("new"));
    drop(old);
    assert!(root.upgrade().is_none());
    assert!(directory.upgrade().is_none());
    assert!(old_record.upgrade().is_none());
}

#[test]
fn independent_managers_refresh_replacement_and_removal() {
    let k = key();
    let other_key = StudyKey { stored_id: 5, ..k };
    let mut a = StudyManager::new(k);
    let mut b = StudyManager::new(other_key);
    let (ta, ra) = start(&mut a, vec![trial(k, 10, 0, TrialState::Complete)]);
    a.finish(ta, ra.next_sequence).unwrap();
    let (tb, rb) = start(&mut b, vec![trial(other_key, 20, 0, TrialState::Complete)]);
    let b_input = b.acquire().unwrap();
    let refresh = a.begin(LoadKind::Refresh).unwrap();
    let receipt = a
        .apply(a.header(refresh).unwrap(), replace(vec![]))
        .unwrap();
    assert_eq!(receipt.stamp.input.generation, ra.stamp.input.generation);
    assert!(a.acquire().unwrap().data().trials().is_empty());
    assert_eq!(b.acquire().unwrap().stamp(), b_input.stamp());
    let header = rb.next_header(tb);
    b.apply(
        header,
        StudyBatch::Delta {
            upserts: vec![],
            removals: vec![TrialKey {
                source: k.source,
                stored_id: 20,
            }],
            metadata: MetadataEdit::Remove,
        },
    )
    .unwrap();
    assert!(b.acquire().unwrap().data().trials().is_empty());
}

#[test]
fn classifier_preserves_invalid_positions_empty_presence_and_violation_priority() {
    use FeasibilityState::*;
    for (values, width, expected) in [
        (vec![], 0, Unverified),
        (vec![-1.0], 2, Unverified),
        (vec![-1.0, f64::INFINITY], 2, Unverified),
        (vec![f64::NAN, 1.0], 3, Infeasible),
        (vec![0.0, -1.0], 2, Feasible),
        (vec![-1.0, 0.1], 2, Infeasible),
        (vec![], 2, Unverified),
        (vec![f64::NAN, -1.0], 2, Unverified),
        (vec![f64::INFINITY, -1.0], 2, Unverified),
        (vec![f64::NEG_INFINITY, -1.0], 2, Unverified),
        (vec![0.1], 2, Infeasible),
        (vec![0.1, f64::NAN], 2, Infeasible),
        (vec![f64::INFINITY, 0.1], 2, Infeasible),
        (vec![f64::NEG_INFINITY, 0.1], 2, Infeasible),
    ] {
        assert_eq!(FeasibilityState::classify(&values, width), expected);
    }
    let schema = ConstraintSchema {
        present: true,
        width: 0,
    };
    assert_eq!(
        schema.classify(&ConstraintObservation::default()),
        Unverified
    );
    assert_eq!(
        ConstraintSchema::default().classify(&ConstraintObservation::default()),
        Feasible
    );
}
