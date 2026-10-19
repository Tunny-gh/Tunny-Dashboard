//! The authoritative attribute boundary preserves non-JSON numeric CSV cells.
use std::sync::Arc;
use tdashboard_study::*;
use tunny_core::journal_parser::study_projection;

#[test]
fn csv_nonfinite_numeric_attributes_survive_the_authoritative_handoff() {
    let csv = tunny_core::flat_csv::parse_flat_csv(
        b"out:f,score\n10,inf\n20,-inf\n30,NaN\n40,3.5\n",
        "nonfinite-attributes",
    )
    .unwrap();
    let original = csv.dataframe.get_numeric_column("score").unwrap();
    assert_eq!(original[0], f64::INFINITY);
    assert_eq!(original[1], f64::NEG_INFINITY);
    assert!(original[2].is_nan());
    assert_eq!(original[3], 3.5);

    let source = IdentityAllocator::new().unwrap().source().unwrap();
    let key = StudyKey {
        source,
        stored_id: 0,
    };
    let mut metadata = StudyMetadata {
        name: csv.meta.name,
        ..Default::default()
    };
    metadata.objectives.push(ObjectiveDescriptor {
        index: ObjectiveIndex(0),
        name: Some("f".into()),
        direction: Direction::Minimize,
    });
    metadata.user_attribute_names.insert("score".into());
    let trials = original
        .iter()
        .enumerate()
        .map(|(index, &value)| {
            let mut record = TrialRecord::new(
                key,
                index as u32,
                StudyTrialNumber(index as u32),
                TrialState::Complete,
            );
            record.completed_order = Some(index as u64);
            Arc::make_mut(&mut record.objectives)
                .insert(ObjectiveIndex(0), Number((index + 1) as f64 * 10.0));
            Arc::make_mut(&mut record.user_attributes)
                .insert("score".into(), Arc::new(AttributeValue::from(value)));
            Arc::new(record)
        })
        .collect();
    let mut manager = StudyManager::new(key);
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let receipt = manager
        .apply(
            manager.header(ticket).unwrap(),
            StudyBatch::Replace { metadata, trials },
        )
        .unwrap();
    manager
        .finish(ticket, receipt.next_header(ticket).sequence)
        .unwrap();
    let projection = study_projection::project(&manager.acquire().unwrap());
    let projected = projection.dataframe.get_numeric_column("score").unwrap();
    assert_eq!(projected[0], f64::INFINITY);
    assert_eq!(projected[1], f64::NEG_INFINITY);
    assert!(projected[2].is_nan());
    assert_eq!(projected[3], 3.5);
}
