//! Cross-boundary contracts live above I/O and analysis, never as a reverse edge.
use std::sync::Arc;
use tdashboard_io::{journal::parser::study, storage::ReadSource};
use tdashboard_study::*;
use tunny_core::journal_parser::study_projection;

fn key() -> StudyKey {
    StudyKey {
        source: IdentityAllocator::new().unwrap().source().unwrap(),
        stored_id: 0,
    }
}

#[test]
fn recorded_nonfinite_numeric_parameters_remain_observations_not_missing_values() {
    let bytes = br#"{"op_code":0,"study_name":"numbers","directions":[1]}
{"op_code":4,"study_id":0,"distributions":{"inline":{"name":"FloatDistribution","attributes":{"low":0,"high":10}}},"params":{"inline":Infinity},"state":1,"values":[1]}
{"op_code":4,"study_id":0}
{"op_code":5,"trial_id":1,"param_name":"update","param_value_internal":-Infinity,"distribution":{"name":"FloatDistribution","attributes":{"low":0,"high":10}}}
{"op_code":6,"trial_id":1,"state":1,"values":[2]}
{"op_code":4,"study_id":0,"distributions":{"nan":{"name":"FloatDistribution","attributes":{"low":0,"high":10}}},"params":{"nan":NaN},"state":2}
"#;
    let key = key();
    let source = ReadSource::Journal(Arc::new(bytes.to_vec()));
    let input = acquire(key, &source, false);
    assert_eq!(
        input.data().trials()[0].parameters["inline"].value,
        ParameterValue::Numeric(Number(f64::INFINITY))
    );
    assert_eq!(
        input.data().trials()[1].parameters["update"].value,
        ParameterValue::Numeric(Number(f64::NEG_INFINITY))
    );
    assert!(
        matches!(input.data().trials()[2].parameters["nan"].value,ParameterValue::Numeric(Number(v)) if v.is_nan())
    );
    assert!(!input.data().trials()[0].parameters.contains_key("missing"));
    let projection = study_projection::project(&input);
    assert_eq!(
        projection
            .dataframe
            .numeric_parameter_column("inline")
            .unwrap()[0],
        f64::INFINITY
    );
    assert_eq!(
        projection
            .dataframe
            .numeric_parameter_column("update")
            .unwrap()[1],
        f64::NEG_INFINITY
    );
}

#[test]
fn metadata_only_metric_names_replace_labels_without_losing_streaming_layout() {
    let bytes = br#"{"op_code":0,"study_name":"names","directions":[1]}
{"op_code":3,"study_id":0,"system_attr":{"study:metric_names":["old"]}}
{"op_code":4,"study_id":0,"distributions":{},"state":1,"values":[2],"params":{"z":1}}
{"op_code":4,"study_id":0,"distributions":{},"state":1,"values":[4],"params":{"a":2}}
{"op_code":3,"study_id":0,"system_attr":{"study:metric_names":["new"]}}
"#;
    let key = key();
    let mut manager = StudyManager::new(key);
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let mut header = manager.header(ticket).unwrap();
    let mut table = study_projection::TableProjection::default();
    let mut projection = None;
    study::read_batches(bytes, key, 1, |batch| {
        header = manager.apply(header, batch).unwrap().next_header(ticket);
        projection = Some(table.project(&manager.acquire().unwrap()));
        Ok(())
    })
    .unwrap();
    let projection = projection.unwrap();
    assert_eq!(projection.meta.param_names, vec!["a", "z"]);
    assert_eq!(projection.dataframe.param_col_names(), &["z", "a"]);
    assert_eq!(projection.dataframe.objective_col_names(), &["new"]);
    assert_eq!(
        projection.dataframe.get_numeric_column("new"),
        Some([2.0, 4.0].as_slice())
    );
    assert!(projection.dataframe.get_numeric_column("old").is_none());
}

#[test]
fn repeated_objective_labels_keep_indexed_values_separate_from_parameters() {
    let bytes = br#"{"op_code":0,"study_name":"roles","directions":[1,2]}
{"op_code":3,"study_id":0,"system_attr":{"study:metric_names":["same","same"]}}
{"op_code":4,"study_id":0,"distributions":{},"state":1,"values":[2,7],"params":{"same":5}}
{"op_code":4,"study_id":0,"distributions":{},"state":1,"values":[3,8],"params":{"same":6}}
"#;
    let key = key();
    let mut manager = StudyManager::new(key);
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let mut header = manager.header(ticket).unwrap();
    let mut table = study_projection::TableProjection::default();
    study::read_batches(bytes, key, 1, |batch| {
        header = manager.apply(header, batch).unwrap().next_header(ticket);
        let projection = table.project(&manager.acquire().unwrap());
        assert_eq!(
            projection.dataframe.objective_col_names(),
            &["same", "same"]
        );
        assert_eq!(
            projection
                .dataframe
                .numeric_parameter_column("same")
                .unwrap()[0],
            5.0
        );
        assert_eq!(projection.dataframe.objective_column_at(0).unwrap()[0], 2.0);
        assert_eq!(projection.dataframe.objective_column_at(1).unwrap()[0], 7.0);
        Ok(())
    })
    .unwrap();
}

#[test]
fn actual_optuna5_journal_retains_nonfinite_observations_without_conflating_null() {
    let bytes = include_bytes!("../../crates/io/fixtures/optuna5-nonfinite.jsonl");
    let key = key();
    let source = ReadSource::Journal(Arc::new(bytes.to_vec()));
    let full = acquire(key, &source, false);
    let streaming = acquire(key, &source, true);
    assert_eq!(full.data(), streaming.data());
    let records = full.data().trials();
    assert_eq!(records.len(), 3);
    assert_eq!(records[0].objectives[&ObjectiveIndex(0)].0, f64::INFINITY);
    assert_eq!(
        records[1].objectives[&ObjectiveIndex(0)].0,
        f64::NEG_INFINITY
    );
    assert_eq!(records[2].objectives[&ObjectiveIndex(0)].0, 2.0);
    for trial in records.iter() {
        assert_eq!(trial.state, TrialState::Complete);
        assert!(
            matches!(trial.user_attributes["positive"].as_ref(),AttributeValue::Number(Number(n)) if *n == f64::INFINITY)
        );
        assert!(
            matches!(trial.user_attributes["negative"].as_ref(),AttributeValue::Number(Number(n)) if *n == f64::NEG_INFINITY)
        );
        assert!(
            matches!(trial.user_attributes["nan"].as_ref(),AttributeValue::Number(Number(n)) if n.is_nan())
        );
        assert!(trial.user_attributes["null"].is_null());
        assert!(!trial.user_attributes.contains_key("missing"));
        assert_eq!(
            trial.user_attributes["text"].as_ref(),
            &serde_json::json!("Infinity")
        );
        assert_eq!(trial.intermediate[0], (0, Number(f64::INFINITY)));
        assert!(trial.intermediate[1].1 .0.is_nan());
    }
    let projection = study_projection::project(&full);
    assert_eq!(
        projection.dataframe.get_numeric_column("positive").unwrap(),
        &[f64::INFINITY; 3]
    );
    assert!(projection
        .dataframe
        .get_numeric_column("nan")
        .unwrap()
        .iter()
        .all(|v| v.is_nan()));
}
fn acquire(key: StudyKey, source: &ReadSource, streaming: bool) -> StudyInput {
    let mut manager = StudyManager::new(key);
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let mut header = manager.header(ticket).unwrap();
    source
        .read(key, streaming, |batch| {
            header = manager.apply(header, batch).unwrap().next_header(ticket);
            Ok(())
        })
        .unwrap();
    manager.finish(ticket, header.sequence).unwrap();
    manager.acquire().unwrap()
}
const JOURNAL: &[u8] = br#"{"op_code":0,"study_name":"a","directions":[1]}
{"op_code":0,"study_name":"b","directions":[2]}
{"op_code":4,"study_id":0}
{"op_code":5,"trial_id":0,"param_name":"kind","param_value_internal":1,"distribution":{"name":"CategoricalDistribution","attributes":{"choices":[1,"1",null,""]}}}
{"op_code":4,"study_id":1,"distributions":{},"state":3,"values":[99]}
{"op_code":4,"study_id":0,"distributions":{},"state":1,"values":[3],"params":{"x":0},"user_attrs":{"empty":{},"null":null},"system_attrs":{"constraints":[-1]}}
{"op_code":6,"trial_id":0,"state":1,"values":[Infinity]}
{"op_code":8,"trial_id":0,"user_attr":{"p":Infinity,"n":-Infinity,"nan":NaN,"null":null,"text":"Infinity","nested":[Infinity,null,{"x":NaN}],"literal":{"\u005f_tunny_nonfinite":"Infinity"}}}
{"op_code":4,"study_id":0,"distributions":{},"state":2,"values":[8],"params":{"unseen":7},"user_attrs":{"pruned":[]},"system_attrs":{"constraints":[-1,null,"bad",NaN,Infinity,-Infinity]}}
{"op_code":4,"study_id":0,"distributions":{},"state":3,"values":[9],"user_attrs":{"failed":{"why":"x"}}}
{"op_code":4,"study_id":0,"distributions":{},"state":4,"params":{"waiting":2}}
{"op_code":4,"study_id":0,"distributions":{},"state":0,"params":{"running":4},"user_attrs":{"running_json":{"v":true}},"system_attrs":{"constraints":[]}}
"#;

#[test]
fn journal_bulk_streaming_reload_preserve_domain_and_path_order() {
    let key = key();
    let source = ReadSource::Journal(Arc::new(JOURNAL.to_vec()));
    let full = acquire(key, &source, false);
    assert_eq!(full.data().trials().len(), 6);
    assert_eq!(full.data().constraint_schema().width, 6);
    for size in [1, 2, 1000] {
        let mut manager = StudyManager::new(key);
        for kind in [LoadKind::Initial, LoadKind::Reload] {
            let ticket = manager.begin(kind).unwrap();
            let mut header = manager.header(ticket).unwrap();
            study::read_batches(JOURNAL, key, size, |batch| {
                header = manager.apply(header, batch).unwrap().next_header(ticket);
                Ok(())
            })
            .unwrap();
            manager.finish(ticket, header.sequence).unwrap();
            let input = manager.acquire().unwrap();
            assert_eq!(input.data().metadata(), full.data().metadata());
            for (streamed, bulk) in input
                .data()
                .trials()
                .iter()
                .zip(full.data().trials().iter())
            {
                let mut streamed = (**streamed).clone();
                let mut bulk = (**bulk).clone();
                streamed.completed_order = None;
                bulk.completed_order = None;
                assert_eq!(streamed, bulk);
            }
            let projection = study_projection::project(&input);
            assert!(!projection.meta.param_names.contains(&"unseen".into()));
            assert!(!projection.meta.param_names.contains(&"running".into()));
            assert!(!projection.meta.user_attr_names.contains(&"pruned".into()));
            if size == 1 {
                assert_eq!(
                    projection
                        .rows
                        .iter()
                        .map(|k| k.stored_id)
                        .collect::<Vec<_>>(),
                    vec![2, 0]
                );
            }
        }
    }
    let projection = study_projection::project(&full);
    assert_eq!(
        projection
            .rows
            .iter()
            .map(|k| k.stored_id)
            .collect::<Vec<_>>(),
        vec![0, 2]
    );
    assert!(projection.dataframe.numeric_parameter_column("x").unwrap()[0].is_nan());
    let record = &full.data().trials()[0];
    assert_eq!(record.objectives[&ObjectiveIndex(0)].0, f64::INFINITY);
    assert!(
        matches!(record.user_attributes["p"].as_ref(),AttributeValue::Number(Number(v)) if *v == f64::INFINITY)
    );
    assert!(
        matches!(record.user_attributes["n"].as_ref(),AttributeValue::Number(Number(v)) if *v == f64::NEG_INFINITY)
    );
    assert!(
        matches!(record.user_attributes["nan"].as_ref(),AttributeValue::Number(Number(v)) if v.is_nan())
    );
    assert!(record.user_attributes["null"].is_null());
    assert!(!record.user_attributes.contains_key("missing"));
    assert_eq!(
        record.user_attributes["text"].as_ref(),
        &serde_json::json!("Infinity")
    );
    assert_eq!(
        record.user_attributes["literal"].as_ref(),
        &serde_json::json!({"__tunny_nonfinite":"Infinity"})
    );
    assert!(matches!(
        record.user_attributes["nested"].as_ref(),
        AttributeValue::Array(_)
    ));
    assert_eq!(
        record.parameters["kind"].value,
        ParameterValue::Category(Arc::new(serde_json::json!("1")))
    );
}

#[test]
fn journal_sink_failure_retains_only_accepted_prefix_and_empty_reload_replaces() {
    let key = key();
    let mut manager = StudyManager::new(key);
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let mut header = manager.header(ticket).unwrap();
    let mut deliveries = 0;
    let error = study::read_batches(JOURNAL, key, 1, |batch| {
        deliveries += 1;
        if deliveries == 2 {
            return Err("fixture sink rejected".into());
        }
        header = manager.apply(header, batch).unwrap().next_header(ticket);
        Ok(())
    })
    .unwrap_err();
    assert_eq!(error, "fixture sink rejected");
    manager.fail(ticket).unwrap();
    let retained = manager.acquire().unwrap();
    assert_eq!(
        retained
            .data()
            .trials()
            .iter()
            .filter(|t| t.state == TrialState::Complete)
            .count(),
        1
    );
    let next = manager.begin(LoadKind::Reload).unwrap();
    assert_eq!(manager.acquire().unwrap().stamp(), retained.stamp());
    let empty = study::read_full(
        br#"{"op_code":0,"study_name":"empty","directions":[1]}"#,
        key,
    )
    .unwrap();
    let receipt = manager.apply(manager.header(next).unwrap(), empty).unwrap();
    manager.finish(next, receipt.next_sequence).unwrap();
    assert!(manager.acquire().unwrap().data().trials().is_empty());
    assert_eq!(retained.data().trials().len(), 2);
}

#[test]
fn csv_owned_load_matches_legacy_projection_including_nonfinite_attributes() {
    let bytes = b"in:z,in:a,out:f,score,note,img\n1,,Infinity,inf,one,a.png\n2,1,-Infinity,-inf,two,\n3,x,NaN,NaN,,missing.png\n4,a,7,3.5,three,\n";
    let legacy = tunny_core::flat_csv::parse_flat_csv(bytes, "csv").unwrap();
    let source = ReadSource::Csv {
        bytes: Arc::new(bytes.to_vec()),
        name: "csv".into(),
    };
    let key = key();
    let input = acquire(key, &source, false);
    let next = acquire(key, &source, true);
    assert_eq!(input.data(), next.data());
    let projection = study_projection::project(&input);
    assert_eq!(projection.meta.param_names, legacy.meta.param_names);
    assert_eq!(
        projection.dataframe.column_names(),
        legacy.dataframe.column_names()
    );
    for name in legacy.dataframe.column_names() {
        if let Some(values) = legacy.dataframe.get_numeric_column(&name) {
            let actual = projection.dataframe.get_numeric_column(&name).unwrap();
            assert_eq!(
                values.iter().map(|v| Number(*v)).collect::<Vec<_>>(),
                actual.iter().map(|v| Number(*v)).collect::<Vec<_>>(),
                "{name}"
            );
        }
        assert_eq!(
            projection.dataframe.get_string_column(&name),
            legacy.dataframe.get_string_column(&name)
        );
    }
    assert_eq!(input.data().trials()[0].artifacts[0].reference, "a.png");
    assert_eq!(
        input.data().trials()[2].artifacts[0].reference,
        "missing.png"
    );
    assert!(input
        .data()
        .trials()
        .iter()
        .all(|t| t.parameters.values().all(|p| p.distribution.is_none())));
}

#[test]
fn sqlite_all_state_batches_reload_and_metadata_growth_keep_visible_schema() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("study.db");
    let writer = rusqlite::Connection::open(&path).unwrap();
    writer.execute_batch(r#"
        CREATE TABLE studies(study_id INTEGER PRIMARY KEY,study_name TEXT);
        CREATE TABLE study_directions(study_id INTEGER,objective INTEGER,direction TEXT);
        CREATE TABLE study_system_attributes(study_id INTEGER,key TEXT,value_json TEXT);
        CREATE TABLE trials(trial_id INTEGER PRIMARY KEY,number INTEGER,study_id INTEGER,state TEXT,datetime_start TEXT,datetime_complete TEXT);
        CREATE TABLE trial_values(trial_id INTEGER,objective INTEGER,value REAL,value_type TEXT);
        CREATE TABLE trial_params(trial_id INTEGER,param_name TEXT,param_value REAL,distribution_json TEXT);
        CREATE TABLE trial_user_attributes(trial_id INTEGER,key TEXT,value_json TEXT);
        CREATE TABLE trial_system_attributes(trial_id INTEGER,key TEXT,value_json TEXT);
        CREATE TABLE trial_intermediate_values(trial_id INTEGER,step INTEGER,intermediate_value REAL,intermediate_value_type TEXT);
        INSERT INTO studies VALUES(0,'all-state');
        INSERT INTO study_directions VALUES(0,0,'MINIMIZE');
        INSERT INTO trials VALUES(10,0,0,'COMPLETE','2024-01-01 00:00:00','2024-01-01 00:00:01'),(15,2,0,'COMPLETE',NULL,NULL),
            (20,4,0,'RUNNING',NULL,NULL),(30,9,0,'PRUNED',NULL,NULL),(40,12,0,'FAIL',NULL,NULL),(50,16,0,'WAITING',NULL,NULL);
        INSERT INTO trial_values VALUES(10,0,NULL,'INF_POS'),(15,0,2,'FINITE'),(20,0,3,'FINITE'),(40,0,9,'FINITE');
        INSERT INTO trial_params VALUES(10,'x',1,'{"name":"FloatDistribution","attributes":{"low":0,"high":10,"step":null,"log":false}}'),
            (10,'c',1,'{"name":"CategoricalDistribution","attributes":{"choices":[1,"1",null]}}'),
            (20,'x',2,'{"name":"FloatDistribution","attributes":{"low":0,"high":10,"step":null,"log":false}}'),
            (30,'hidden',7,'{"name":"IntDistribution","attributes":{"low":0,"high":10,"step":1,"log":false}}');
        INSERT INTO trial_user_attributes VALUES(10,'positive','Infinity'),(10,'negative','-Infinity'),(10,'nan','NaN'),
            (10,'null','null'),(10,'empty','{}'),(30,'only_pruned','null'),(50,'pending','[]');
        INSERT INTO trial_system_attributes VALUES(10,'constraints','[-1]'),(30,'constraints','[-1,null,"bad",Infinity]'),(50,'constraints','[]');
        INSERT INTO trial_intermediate_values VALUES(20,0,NULL,'INF_NEG'),(40,1,NULL,'NAN');
    "#).unwrap();
    writer.execute("INSERT INTO trial_params VALUES(10,'edge',?1,'{\"name\":\"FloatDistribution\",\"attributes\":{\"low\":0,\"high\":10}}')",[f64::INFINITY]).unwrap();
    let key = key();
    let source = ReadSource::Sqlite(path);
    let mut manager = StudyManager::new(key);
    let mut retained = None;
    for (kind, width) in [(LoadKind::Initial, 4), (LoadKind::Reload, 5)] {
        let ticket = manager.begin(kind).unwrap();
        let mut header = manager.header(ticket).unwrap();
        source
            .read(key, false, |batch| {
                header = manager.apply(header, batch).unwrap().next_header(ticket);
                Ok(())
            })
            .unwrap();
        manager.finish(ticket, header.sequence).unwrap();
        let input = manager.acquire().unwrap();
        assert_eq!(input.data().trials().len(), 6);
        assert_eq!(input.data().constraint_schema().width, width);
        assert_eq!(
            input.data().trials()[2].parameters["x"].value,
            ParameterValue::Numeric(Number(2.0))
        );
        assert_eq!(
            input.data().trials()[2].objectives[&ObjectiveIndex(0)],
            Number(3.0)
        );
        assert!(input.data().trials()[3].user_attributes["only_pruned"].is_null());
        assert_eq!(
            input.data().trials()[4].objectives[&ObjectiveIndex(0)],
            Number(9.0)
        );
        assert_eq!(input.data().trials()[5].state, TrialState::Waiting);
        assert!(input.data().trials()[5]
            .user_attributes
            .contains_key("pending"));
        assert!(input.data().trials()[5].constraints.present);
        assert!(input.data().trials()[5].constraints.values.is_empty());
        assert_eq!(
            input.data().trials()[2].intermediate[0].1,
            Number(f64::NEG_INFINITY)
        );
        assert!(input.data().trials()[4].intermediate[0].1 .0.is_nan());
        let first = &input.data().trials()[0];
        assert!(
            matches!(first.user_attributes["positive"].as_ref(),AttributeValue::Number(Number(n)) if *n == f64::INFINITY)
        );
        assert!(first.user_attributes["null"].is_null());
        assert_eq!(
            first.parameters["c"].value,
            ParameterValue::Category(Arc::new(serde_json::json!("1")))
        );
        let projection = study_projection::project(&input);
        assert_eq!(
            projection
                .rows
                .iter()
                .map(|k| k.stored_id)
                .collect::<Vec<_>>(),
            vec![10, 15]
        );
        assert_eq!(projection.meta.param_names, vec!["c", "edge", "x"]);
        assert_eq!(
            first.parameters["edge"].value,
            ParameterValue::Numeric(Number(f64::INFINITY))
        );
        assert_eq!(
            projection
                .dataframe
                .numeric_parameter_column("edge")
                .unwrap()[0],
            f64::INFINITY
        );
        assert!(!projection
            .meta
            .user_attr_names
            .contains(&"only_pruned".into()));
        assert!(projection.dataframe.numeric_parameter_column("x").unwrap()[1].is_nan());
        assert_eq!(projection.dataframe.get_trial_number(1), Some(2));
        if kind == LoadKind::Initial {
            retained = Some(input);
            writer.execute("UPDATE trial_system_attributes SET value_json='[-1,-2,-3,-4,-5]' WHERE trial_id=30",[]).unwrap();
        } else {
            assert_ne!(
                input.stamp().input.generation,
                retained.as_ref().unwrap().stamp().input.generation
            );
            assert_eq!(
                retained.as_ref().unwrap().data().constraint_schema().width,
                4
            );
            assert_eq!(
                input
                    .data()
                    .trials()
                    .iter()
                    .filter(|t| t.state == TrialState::Complete)
                    .count(),
                2
            );
        }
    }
}
