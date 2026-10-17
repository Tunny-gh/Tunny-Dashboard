//! D2 headless harness: the same owned-batch/manager/projection path as the GUI.
use std::sync::Arc;
use tdashboard_study::*;
use tunny_core::journal_parser::{self, study::read_batches, study_projection::project};

fn key(id: u32) -> StudyKey {
    StudyKey {
        source: IdentityAllocator::new().unwrap().source().unwrap(),
        stored_id: id,
    }
}
fn acquire(bytes: &[u8], key: StudyKey, size: usize) -> StudyInput {
    let mut manager = StudyManager::new(key);
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let mut header = manager.header(ticket).unwrap();
    read_batches(bytes, key, size, |batch| {
        let receipt = manager.apply(header, batch).map_err(|e| e.to_string())?;
        header = receipt.next_header(ticket);
        Ok(())
    })
    .unwrap();
    manager.finish(ticket, header.sequence).unwrap();
    manager.acquire().unwrap()
}

fn fixture() -> Vec<u8> {
    use serde_json::json;
    [
        json!({"op_code":0,"study_name":"other","directions":[1]}),
        json!({"op_code":0,"study_name":"target","directions":[1]}),
        json!({"op_code":4,"study_id":0}),
        json!({"op_code":4,"study_id":1,"datetime_start":"2026-10-01T00:00:00"}),
        json!({"op_code":4,"study_id":1}),
        json!({"op_code":5,"trial_id":1,"param_name":"x","param_value_internal":2.5,"distribution":{"name":"FloatDistribution","attributes":{"low":0,"high":10,"log":true,"step":0.5}}}),
        json!({"op_code":5,"trial_id":2,"param_name":"cat","param_value_internal":1,"distribution":{"name":"CategoricalDistribution","attributes":{"choices":[1,"1",null]}}}),
        json!({"op_code":6,"trial_id":2,"state":1,"values":[2]}),
        json!({"op_code":8,"trial_id":1,"user_attr":{"null":null,"json":{"a":[1,true]},"numeric":3}}),
        json!({"op_code":9,"trial_id":1,"system_attr":{"constraints":[-1]}}),
        json!({"op_code":7,"trial_id":1,"step":9,"intermediate_value":2}),
        json!({"op_code":7,"trial_id":1,"step":2,"intermediate_value":4}),
        json!({"op_code":6,"trial_id":1,"state":1,"values":[1],"datetime_complete":"2026-10-01T00:01:00"}),
        json!({"op_code":4,"study_id":1}),
        json!({"op_code":5,"trial_id":3,"param_name":"running_only","param_value_internal":7,"distribution":{"name":"IntDistribution","attributes":{"low":0,"high":10,"log":false,"step":2}}}),
        json!({"op_code":8,"trial_id":3,"user_attr":{"running_json":[1,2]}}),
        json!({"op_code":6,"trial_id":3,"state":0,"values":[7]}),
        json!({"op_code":4,"study_id":1,"state":2,"distributions":{},"params":{"p":9},"values":[8],"user_attrs":{"p":true},"system_attrs":{"constraints":[-1,"invalid",0]}}),
        json!({"op_code":4,"study_id":1,"state":3,"distributions":{},"params":{"f":8},"values":[6],"user_attrs":{"f":null}}),
        json!({"op_code":4,"study_id":1,"state":4,"distributions":{},"params":{"w":7},"values":[5],"user_attrs":{"w":{"a":1}}}),
    ].into_iter().map(|v| v.to_string() + "\n").collect::<String>().into_bytes()
}

#[test]
fn journal_slice_retains_all_states_without_exposing_noncomplete_columns() {
    let bytes = fixture();
    let key = key(1);
    let input = acquire(&bytes, key, 1);
    assert_eq!(input.data().trials().len(), 6);
    let trials = input.data().trials();
    assert_eq!(
        trials.iter().map(|t| t.state).collect::<Vec<_>>(),
        vec![
            TrialState::Complete,
            TrialState::Complete,
            TrialState::Running,
            TrialState::Pruned,
            TrialState::Fail,
            TrialState::Waiting
        ]
    );
    assert_eq!(
        trials[2].parameters["running_only"].value,
        ParameterValue::Numeric(Number(7.0))
    );
    assert_eq!(trials[2].objectives[&ObjectiveIndex(0)], Number(7.0));
    assert_eq!(
        trials[2].user_attributes["running_json"].as_ref(),
        &serde_json::json!([1, 2])
    );
    assert!(trials[3].constraints.values[1].0.is_nan());
    assert!(trials[0].user_attributes["null"].is_null());
    assert_eq!(
        trials[0]
            .intermediate
            .iter()
            .map(|(s, _)| *s)
            .collect::<Vec<_>>(),
        vec![2, 9]
    );
    assert!(trials[0].datetime_start.is_some() && trials[0].datetime_complete.is_some());
    assert!(matches!(
        trials[0].parameters["x"].distribution,
        Some(Distribution::Numeric {
            log: true,
            discrete: true,
            step: Some(Number(0.5)),
            ..
        })
    ));
    assert_eq!(
        input.data().constraint_schema(),
        ConstraintSchema {
            present: true,
            width: 3
        }
    );
    let projected = project(&input);
    assert_eq!(
        projected
            .rows
            .iter()
            .map(|k| k.stored_id)
            .collect::<Vec<_>>(),
        vec![2, 1]
    );
    assert_eq!(projected.dataframe.get_trial_number(0), Some(1));
    assert_eq!(projected.dataframe.get_trial_number(1), Some(0));
    assert_eq!(projected.meta.param_names, vec!["cat", "x"]);
    assert_eq!(
        projected.dataframe.category_values("cat").unwrap()[0],
        Some(serde_json::json!("1"))
    );
    assert_eq!(
        projected.dataframe.feasibility().state(1),
        FeasibilityState::Unverified
    );
    assert_eq!(projected.extras.trials.len(), 6);
    assert_eq!(projected.stamp, input.stamp());
    assert!(projected.history.is_some());
    let bulk = acquire(&bytes, key, 1000);
    assert_eq!(input.data(), bulk.data());
    // Full legacy reading has a different established row order, not different IDs.
    let (_, legacy, _) = journal_parser::parse_single_study(&bytes, 1).unwrap();
    assert_eq!(legacy.get_trial_id(0), Some(1));
    assert_eq!(projected.dataframe.get_trial_id(0), Some(2));
}

#[test]
fn full_read_retains_all_state_authority_and_legacy_returned_row_order() {
    let bytes = fixture();
    let key = key(1);
    let mut manager = StudyManager::new(key);
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let receipt = manager
        .apply(
            manager.header(ticket).unwrap(),
            journal_parser::study::read_full(&bytes, key).unwrap(),
        )
        .unwrap();
    manager.finish(ticket, receipt.next_sequence).unwrap();
    let input = manager.acquire().unwrap();
    assert_eq!(input.data().trials().len(), 6);
    assert_eq!(input.data().constraint_schema().width, 3);
    let full = project(&input);
    let (_, legacy, _) = journal_parser::parse_single_study(&bytes, 1).unwrap();
    assert_eq!(
        full.rows
            .iter()
            .map(|key| key.stored_id)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    for row in 0..legacy.row_count() {
        assert_eq!(full.dataframe.get_trial_id(row), legacy.get_trial_id(row));
        assert_eq!(
            full.dataframe.get_trial_number(row),
            legacy.get_trial_number(row)
        );
        assert_eq!(
            full.dataframe.feasibility().state(row),
            legacy.feasibility().state(row)
        );
    }
    assert_eq!(
        full.dataframe.get_numeric_column("obj0"),
        legacy.get_numeric_column("obj0")
    );
}

#[test]
fn streaming_projection_retains_discovery_order_but_resets_layout_on_full_reload() {
    use journal_parser::study_projection::TableProjection;
    let bytes = concat!(
        "{\"op_code\":0,\"study_name\":\"s\",\"directions\":[1]}\n",
        "{\"op_code\":4,\"study_id\":0,\"distributions\":{},\"state\":1,\"params\":{\"z\":1},\"values\":[1],\"user_attrs\":{\"z_num\":1,\"z_text\":\"z\"}}\n",
        "{\"op_code\":4,\"study_id\":0,\"distributions\":{},\"state\":1,\"params\":{\"a\":2},\"values\":[2],\"user_attrs\":{\"a_num\":2,\"a_text\":\"a\"}}\n",
    ).as_bytes();
    let key = key(0);
    let mut manager = StudyManager::new(key);
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let mut table = TableProjection::default();
    let mut observed = Vec::new();
    let mut first_input = None;
    read_batches(bytes, key, 1, |batch| {
        manager
            .apply(manager.header(ticket).unwrap(), batch)
            .unwrap();
        let projection = table.project(&manager.acquire().unwrap());
        if first_input.is_none() {
            first_input = manager.acquire();
        }
        observed.push(projection.dataframe.param_col_names().to_vec());
        Ok(())
    })
    .unwrap();
    assert_eq!(observed, vec![vec!["z"], vec!["z", "a"], vec!["z", "a"]]);
    // Retained inputs cannot inherit columns discovered by a later revision.
    assert_eq!(
        table
            .project(&first_input.unwrap())
            .dataframe
            .param_col_names(),
        ["z"]
    );
    let ticket = manager.begin(LoadKind::Reload).unwrap();
    manager
        .apply(
            manager.header(ticket).unwrap(),
            journal_parser::study::read_full(bytes, key).unwrap(),
        )
        .unwrap();
    let full = table.project(&manager.acquire().unwrap());
    let (_, legacy, _) = journal_parser::parse_single_study(bytes, 0).unwrap();
    assert_eq!(full.dataframe.param_col_names(), ["a", "z"]);
    assert_eq!(full.dataframe.column_names(), legacy.column_names());
}

#[test]
fn metadata_only_final_batch_reclassifies_existing_projection_without_new_rows() {
    let bytes = fixture();
    let key = key(1);
    let mut manager = StudyManager::new(key);
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let mut header = manager.header(ticket).unwrap();
    let mut old = None;
    read_batches(&bytes, key, 1, |batch| {
        header = manager.apply(header, batch).unwrap().next_header(ticket);
        let input = manager.acquire().unwrap();
        if project(&input).dataframe.row_count() == 2 && old.is_none() {
            old = Some(input);
        }
        Ok(())
    })
    .unwrap();
    let old = old.unwrap();
    let updated = manager.acquire().unwrap();
    assert_eq!(
        project(&old).dataframe.row_count(),
        project(&updated).dataframe.row_count()
    );
    assert_eq!(old.data().constraint_schema().width, 1);
    assert_eq!(updated.data().constraint_schema().width, 3);
    assert_ne!(old.stamp().input.revision, updated.stamp().input.revision);
    assert_eq!(
        project(&old).dataframe.feasibility().state(1),
        FeasibilityState::Feasible
    );
    assert_eq!(
        project(&updated).dataframe.feasibility().state(1),
        FeasibilityState::Unverified
    );
    assert_eq!(old.data().trials()[0].key, updated.data().trials()[0].key);
    assert!(Arc::ptr_eq(
        &old.data().trials()[0],
        &updated.data().trials()[0]
    ));
}

#[test]
fn owned_sink_error_stops_delivery_and_preserves_accepted_prefix() {
    let key = key(1);
    let mut manager = StudyManager::new(key);
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let mut calls = 0;
    let result = read_batches(&fixture(), key, 1, |batch| {
        calls += 1;
        if calls == 2 {
            return Err("delivery rejected".into());
        }
        manager
            .apply(manager.header(ticket).unwrap(), batch)
            .unwrap();
        Ok(())
    });
    assert_eq!(result.unwrap_err(), "delivery rejected");
    manager.fail(ticket).unwrap();
    assert_eq!(calls, 2);
    assert_eq!(
        project(&manager.acquire().unwrap()).dataframe.row_count(),
        1
    );
}

#[test]
fn typed_categories_and_supported_inline_display_match_legacy_projection() {
    use serde_json::json;
    for inline in [false, true] {
        let dist = json!({"name":"CategoricalDistribution","attributes":{"choices":["", 7, "7", null, true]}});
        let mut lines = vec![json!({"op_code":0,"study_name":"s","directions":[1]}).to_string()];
        for (i, value) in dist["attributes"]["choices"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            if inline {
                lines.push(json!({"op_code":4,"study_id":0,"state":1,"distributions":{"cat":dist},"params":{"cat":value},"values":[i]}).to_string());
            } else {
                lines.push(json!({"op_code":4,"study_id":0}).to_string());
                lines.push(json!({"op_code":5,"trial_id":i,"param_name":"cat","param_value_internal":i,"distribution":dist.to_string()}).to_string());
                lines.push(json!({"op_code":6,"trial_id":i,"state":1,"values":[i]}).to_string());
            }
        }
        let bytes = lines.join("\n");
        let (_, legacy, _) = journal_parser::parse_single_study(bytes.as_bytes(), 0).unwrap();
        let projection = project(&acquire(bytes.as_bytes(), key(0), 1));
        assert_eq!(
            legacy.category_values("cat"),
            projection.dataframe.category_values("cat")
        );
        assert_eq!(
            legacy.get_string_column("cat"),
            projection.dataframe.get_string_column("cat")
        );
    }
}

#[test]
fn empty_and_constraint_attribute_only_studies_preserve_projection_discovery() {
    for suffix in ["", "\n{\"op_code\":4,\"study_id\":0,\"distributions\":{},\"state\":2,\"system_attrs\":{\"constraints\":[]}}"] {
        let bytes = format!("{{\"op_code\":0,\"study_name\":\"s\",\"directions\":[1]}}{suffix}");
        let input = acquire(bytes.as_bytes(), key(0), 1);
        let projection = project(&input);
        let (meta, legacy, _) = journal_parser::parse_single_study(bytes.as_bytes(), 0).unwrap();
        assert_eq!(projection.meta.objective_names, meta.objective_names);
        assert_eq!(projection.dataframe.row_count(), legacy.row_count());
        assert_eq!(projection.meta.has_constraints, !suffix.is_empty());
        assert_eq!(input.stamp().input.revision, 1);
        assert_eq!(input.load_state().phase, LoadPhase::Complete);
    }
}

#[test]
fn unavailable_categories_retain_typed_absence_and_existing_display_fallback() {
    use serde_json::json;
    let mut lines = vec![json!({"op_code":0,"study_name":"s","directions":[1]}).to_string()];
    for (i, encoded) in [json!(-1), json!(1.4), json!(9), json!(null)]
        .into_iter()
        .enumerate()
    {
        lines.push(json!({"op_code":4,"study_id":0}).to_string());
        lines.push(json!({"op_code":5,"trial_id":i,"param_name":"cat","param_value_internal":encoded,"distribution":{"name":"CategoricalDistribution","attributes":{"choices":["a","b"]}}}).to_string());
        lines.push(json!({"op_code":6,"trial_id":i,"state":1,"values":[i]}).to_string());
    }
    let bytes = lines.join("\n");
    let input = acquire(bytes.as_bytes(), key(0), 1);
    assert!(input
        .data()
        .trials()
        .iter()
        .all(|t| matches!(t.parameters["cat"].value, ParameterValue::Unavailable(_))));
    let projection = project(&input);
    let (_, legacy, _) = journal_parser::parse_single_study(bytes.as_bytes(), 0).unwrap();
    assert_eq!(
        projection.dataframe.category_values("cat"),
        legacy.category_values("cat")
    );
    assert_eq!(
        projection.dataframe.get_string_column("cat"),
        legacy.get_string_column("cat")
    );
}

#[test]
fn shipped_journal_fixtures_preserve_streamed_numeric_table_values() {
    for bytes in [
        include_bytes!("../fixtures/test.log").as_slice(),
        include_bytes!("../fixtures/test_inmem.log").as_slice(),
    ] {
        let input = acquire(bytes, key(0), 1000);
        let projection = project(&input);
        let mut legacy = tunny_core::dataframe::DataFrame::empty();
        journal_parser::parse_single_study_streaming(bytes, 0, 1000, |batch| {
            legacy.append_trials(
                &batch.new_rows,
                &batch.param_names,
                &batch.objective_names,
                &batch.user_attr_numeric_names,
                &batch.user_attr_string_names,
                batch.max_constraints,
            );
            if batch.meta.has_constraints && !legacy.feasibility().has_constraints() {
                legacy.mark_constrained();
            }
        })
        .unwrap();
        assert_eq!(projection.dataframe.row_count(), legacy.row_count());
        assert_eq!(
            projection.dataframe.param_col_names(),
            legacy.param_col_names()
        );
        assert_eq!(
            projection.dataframe.objective_col_names(),
            legacy.objective_col_names()
        );
        for name in legacy
            .param_col_names()
            .iter()
            .chain(legacy.objective_col_names())
        {
            assert_eq!(
                projection.dataframe.get_numeric_column(name).map(|col| col
                    .iter()
                    .copied()
                    .map(Number)
                    .collect::<Vec<_>>()),
                legacy.get_numeric_column(name).map(|col| col
                    .iter()
                    .copied()
                    .map(Number)
                    .collect::<Vec<_>>())
            );
            assert_eq!(
                projection.dataframe.get_string_column(name),
                legacy.get_string_column(name)
            );
        }
        for row in 0..legacy.row_count() {
            assert_eq!(
                projection.dataframe.get_trial_id(row),
                legacy.get_trial_id(row)
            );
            assert_eq!(
                projection.dataframe.get_trial_number(row),
                legacy.get_trial_number(row)
            );
            assert_eq!(
                projection.dataframe.feasibility().state(row),
                legacy.feasibility().state(row)
            );
        }
    }
}
