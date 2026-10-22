//! Recorded Journal postprocessing semantics (ADR-0022).
use serde_json::{json, Value};
use tdashboard_analysis::{
    dataframe::DataFrame,
    extras::StudyExtras,
    projection::{self, TableProjection},
};
use tdashboard_io::journal::parser::study;
use tdashboard_study::{IdentityAllocator, LoadKind, StudyInput, StudyKey, StudyManager};
mod common;

const FIXTURE: &[u8] = include_bytes!("fixtures/issue258-terminal-attributes.jsonl");

fn before_finish(trial: u32) -> Vec<u8> {
    let lines: Vec<_> = std::str::from_utf8(FIXTURE).unwrap().lines().collect();
    let is_update = |line: &&str| {
        serde_json::from_str::<Value>(line)
            .is_ok_and(|v| v["trial_id"] == trial && (v["op_code"] == 8 || v["op_code"] == 9))
    };
    let updates: Vec<_> = lines.iter().copied().filter(is_update).collect();
    let mut reordered = Vec::new();
    for line in lines.iter().copied().filter(|line| !is_update(line)) {
        if serde_json::from_str::<Value>(line)
            .is_ok_and(|v| v["trial_id"] == trial && v["op_code"] == 6)
        {
            reordered.extend(updates.iter().copied());
        }
        reordered.push(line);
    }
    (reordered.join("\n") + "\n").into_bytes()
}

fn numbers(values: impl IntoIterator<Item = f64>) -> Vec<String> {
    values.into_iter().map(|v| v.to_string()).collect()
}

fn view(df: &DataFrame, extras: &StudyExtras) -> Value {
    json!({
        "constrained": df.feasibility().has_constraints(),
        "width": df.constraint_col_names().len(),
        "rows": (0..df.row_count()).map(|row| json!({
            "id": df.get_trial_id(row), "number": df.get_trial_number(row),
            "objective": df.get_numeric_column("obj0").map(|c| c[row].to_string()),
            "attrs": df.user_attr_names().filter_map(|name|
                df.user_attr_value(name, row).map(|v| (name.to_string(), v.clone())))
                .collect::<serde_json::Map<_, _>>(),
            "constraints": numbers(df.constraint_col_names().iter()
                .map(|name| df.get_numeric_column(name).unwrap()[row])),
            "feasibility": df.feasibility().state(row).label(),
        })).collect::<Vec<_>>(),
        "extras": extras.trials.iter().map(|t| json!({
            "id": t.trial_id, "number": t.trial_number, "state": t.state.label(),
            "start": t.datetime_start, "complete": t.datetime_complete,
            "intermediate": t.intermediate_values,
        })).collect::<Vec<_>>(),
    })
}

fn bulk(bytes: &[u8], sid: u32) -> Value {
    let p = common::journal(bytes, sid);
    let (meta, df, extras) = (p.meta, p.dataframe, p.extras);
    json!({"meta_constrained": meta.has_constraints, "meta_attrs": meta.user_attr_names,
           "view": view(&df, &extras)})
}

fn streamed(bytes: &[u8], sid: u32, size: usize) -> Value {
    let mut df = DataFrame::empty();
    let mut batches = Vec::new();
    let mut extras = None;
    common::observe_stream(bytes, sid, size, |batch| {
        if batch.is_final {
            extras = Some(batch.extras.clone());
        }
        let mut summary = json!({"new_rows": batch.new_rows.len(), "final": batch.is_final,
            "updated_existing": batch.updated_existing,
            "meta_constrained": batch.meta.has_constraints, "width": batch.max_constraints,
            "meta_attrs": batch.meta.user_attr_names,
        });
        batch.apply_to(&mut df);
        summary["feasibility"] = json!((0..df.row_count())
            .map(|i| df.feasibility().state(i).label())
            .collect::<Vec<_>>());
        batches.push(summary);
    })
    .unwrap();
    let extras = extras.unwrap();
    json!({"view": view(&df, &extras), "batches": batches})
}

fn owned(bytes: &[u8], sid: u32, size: Option<usize>) -> Value {
    let key = StudyKey {
        source: IdentityAllocator::new().unwrap().source().unwrap(),
        stored_id: sid,
    };
    let mut manager = StudyManager::new(key);
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let mut header = manager.header(ticket).unwrap();
    let mut table = TableProjection::default();
    let mut batches = Vec::new();
    let mut first: Option<StudyInput> = None;
    let mut deliver = |batch| {
        header = manager.apply(header, batch).unwrap().next_header(ticket);
        let input = manager.acquire().unwrap();
        first.get_or_insert_with(|| input.clone());
        let projection = table.project(&input);
        batches.push(
            json!({"sequence": header.sequence, "revision": input.stamp().input.revision,
                "width": input.data().constraint_schema().width,
                "feasibility": (0..projection.dataframe.row_count()).map(|i|
                    projection.dataframe.feasibility().state(i).label()).collect::<Vec<_>>(),
            }),
        );
        Ok(())
    };
    if let Some(size) = size {
        study::read_batches(bytes, key, size, &mut deliver).unwrap();
    } else {
        deliver(study::read_full(bytes, key).unwrap()).unwrap();
    }
    manager.finish(ticket, header.sequence).unwrap();
    let input = manager.acquire().unwrap();
    let projection = table.project(&input);
    let first_projection = projection::project(&first.unwrap());
    json!({"view": view(&projection.dataframe, &projection.extras),
        "params": projection.meta.param_names,
        "param_layout": projection.dataframe.param_col_names(),
        "numeric_attrs": projection.dataframe.user_attr_numeric_columns().map(|(name, values)|
            (name.to_string(), json!(numbers(values.iter().copied())))).collect::<serde_json::Map<_,_>>(),
        "string_attrs": projection.dataframe.user_attr_string_columns().map(|(name, values)|
            (name.to_string(), json!(values))).collect::<serde_json::Map<_,_>>(),
        "meta_constrained": projection.meta.has_constraints,
        "meta_attrs": projection.meta.user_attr_names,
        "domain": input.data().trials().iter().map(|t| json!({
            "id": t.key.stored_id, "number": t.number.0, "state": format!("{:?}", t.state),
            "attrs": t.user_attributes.iter().map(|(k,v)| (k.clone(), v.json_view()))
                .collect::<serde_json::Map<_, _>>(),
            "constraint_present": t.constraints.present,
            "constraints": numbers(t.constraints.values.iter().map(|n| n.0)),
            "artifacts": t.artifacts.iter().map(|a| a.reference.clone()).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "batches": batches, "revision": input.stamp().input.revision,
        "phase": format!("{:?}", input.load_state().phase),
        "retained_first": view(&first_projection.dataframe, &first_projection.extras),
    })
}

#[test]
fn supported_before_terminal_and_inline_controls_agree() {
    for sid in [0, 2, 6, 7] {
        let full = bulk(FIXTURE, sid);
        for size in [1, 1000] {
            let stream = streamed(FIXTURE, sid, size);
            let domain = owned(FIXTURE, sid, Some(size));
            assert_eq!(stream["view"], domain["view"], "Study {sid}, size {size}");
            if sid != 7 {
                assert_eq!(full["view"], stream["view"], "Study {sid}");
            }
        }
        assert_eq!(full["view"], owned(FIXTURE, sid, None)["view"]);
    }
    let control = owned(FIXTURE, 0, Some(1));
    assert_eq!(
        control["view"]["rows"][0]["attrs"].get("null"),
        Some(&Value::Null)
    );
    assert_eq!(control["view"]["rows"][0]["attrs"]["text"], "[1,2]");
    assert_eq!(
        control["view"]["rows"][0]["attrs"]["payload"]["array"],
        json!([1, "x", false])
    );
    let growth = owned(FIXTURE, 6, Some(1));
    assert_eq!(growth["batches"][0]["width"], 1);
    assert_eq!(growth["batches"][1]["width"], 5);
    assert_eq!(growth["batches"][1]["revision"], 2);
    assert_eq!(
        growth["retained_first"]["rows"][0]["feasibility"],
        "Feasible"
    );
    assert_eq!(
        growth["view"]["rows"][0]["feasibility"],
        "Feasibility unverified"
    );
    assert_eq!(
        growth["domain"][1]["constraints"],
        json!(["-1", "NaN", "NaN", "inf", "-inf"])
    );
    assert_eq!(owned(FIXTURE, 7, Some(1))["view"]["rows"][0]["id"], 11);
    assert_eq!(owned(FIXTURE, 7, None)["view"]["rows"][0]["id"], 10);
    for (sid, trial, width, feasibility) in [
        (1, 1, 1, "Infeasible"),
        (3, 4, 3, "Feasibility unverified"),
        (4, 6, 4, "Feasibility unverified"),
        (5, 7, 0, "Feasibility unverified"),
    ] {
        let bytes = before_finish(trial);
        let full = bulk(&bytes, sid);
        for size in [1, 1000] {
            assert_eq!(full["view"], streamed(&bytes, sid, size)["view"]);
            assert_eq!(full["view"], owned(&bytes, sid, Some(size))["view"]);
        }
        assert_eq!(full["view"], owned(&bytes, sid, None)["view"]);
        assert_eq!(full["view"]["width"], width);
        assert_eq!(full["view"]["rows"][0]["feasibility"], feasibility);
        println!(
            "before_finish trial={trial} width={width} first_row={feasibility}: all readers agree"
        );
    }
}

#[test]
fn recorded_terminal_attributes_agree_across_readers() {
    for sid in 0..9 {
        let full = bulk(FIXTURE, sid);
        let stream1 = streamed(FIXTURE, sid, 1);
        let stream1000 = streamed(FIXTURE, sid, 1000);
        let owned1 = owned(FIXTURE, sid, Some(1));
        let owned1000 = owned(FIXTURE, sid, Some(1000));
        let owned_full = owned(FIXTURE, sid, None);
        assert_eq!(stream1["view"], stream1000["view"]);
        assert_eq!(stream1["view"], owned1["view"]);
        assert_eq!(owned1["view"], owned1000["view"]);
        for (name, result) in [
            ("bulk", &full),
            ("stream1", &stream1),
            ("stream1000", &stream1000),
            ("owned1", &owned1),
            ("owned1000", &owned1000),
            ("owned_full", &owned_full),
        ] {
            println!("study={sid} path={name} {result}");
        }
        if [1, 3, 4, 5].contains(&sid) {
            assert_eq!(full["view"], stream1["view"]);
            assert_eq!(stream1["view"], owned_full["view"]);
        }
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/issue258-terminal-attributes.jsonl");
    let metadata = tdashboard_io::artifacts::parse_artifact_metadata(&path);
    assert_eq!(metadata[&12][0].artifact_id, "image");
    assert_eq!(owned(FIXTURE, 8, None)["domain"][0]["artifacts"], json!([]));
    println!("artifact scanner: trial=12 ids=[image]; owned reader: ids=[]");
    assert_eq!(
        tdashboard_io::journal::parser::scan_study_list(FIXTURE)
            .unwrap()
            .len(),
        9
    );
    let p = common::journal(FIXTURE, 1);
    let installed = p.dataframe;
    assert_eq!(
        installed.user_attr_value("payload", 0),
        Some(&json!({"new":[1,"x",false]}))
    );
    assert_eq!(installed.feasibility().state(0).label(), "Infeasible");
}

#[test]
fn replacements_shrink_constraint_evidence_without_losing_layout_or_prior_inputs() {
    let bytes = concat!(
        "{\"op_code\":0,\"study_name\":\"replacements\",\"directions\":[1]}\n",
        "{\"op_code\":4,\"study_id\":0,\"distributions\":{},\"state\":1,\"params\":{\"z\":1},\"values\":[1],\"user_attrs\":{\"change\":2,\"merge\":true},\"system_attrs\":{\"constraints\":[-1]}}\n",
        "{\"op_code\":4,\"study_id\":0,\"distributions\":{},\"state\":2,\"system_attrs\":{\"constraints\":[-1]}}\n",
        "{\"op_code\":9,\"trial_id\":1,\"system_attr\":{\"constraints\":[-1,null,Infinity]}}\n",
        "{\"op_code\":4,\"study_id\":0,\"distributions\":{},\"state\":1,\"params\":{\"a\":2},\"values\":[2]}\n",
        "{\"op_code\":9,\"trial_id\":1,\"system_attr\":{\"constraints\":[-1]}}\n",
        "{\"op_code\":8,\"trial_id\":0,\"user_attr\":{\"change\":\"text\",\"empty\":[]}}\n",
        "{\"op_code\":4,\"study_id\":0,\"distributions\":{},\"state\":1,\"values\":[3]}\n",
        "{\"op_code\":8,\"trial_id\":0,\"user_attr\":{\"change\":null,\"object\":{}}}\n",
        "{\"op_code\":9,\"trial_id\":0,\"system_attr\":{\"constraints\":[]}}\n",
        "{\"op_code\":9,\"trial_id\":1,\"system_attr\":{\"constraints\":[]}}\n",
    ).as_bytes();
    let full = bulk(bytes, 0);
    for size in [1, 1000] {
        let domain = owned(bytes, 0, Some(size));
        assert_eq!(domain["view"], full["view"]);
        let stream = streamed(bytes, 0, size);
        assert_eq!(stream["view"], full["view"]);
        assert_eq!(domain["view"]["width"], 0);
        assert_eq!(domain["domain"][1]["constraints"], json!([]));
        let attrs = &domain["view"]["rows"][0]["attrs"];
        assert_eq!(attrs.get("change"), Some(&Value::Null));
        assert_eq!(attrs["merge"], true);
        assert_eq!(attrs["empty"], json!([]));
        assert_eq!(attrs["object"], json!({}));
        assert_eq!(domain["view"]["rows"].as_array().unwrap().len(), 3);
    }
    assert_eq!(full["view"], owned(bytes, 0, None)["view"]);
    let incremental = owned(bytes, 0, Some(1));
    assert_eq!(
        incremental["batches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["width"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        [1, 3, 1, 0]
    );
    assert_eq!(
        incremental["retained_first"]["rows"][0]["attrs"]["change"],
        2
    );
    assert_eq!(incremental["retained_first"]["width"], 1);
    assert_eq!(incremental["param_layout"], json!(["z", "a"]));
    assert_eq!(
        incremental["numeric_attrs"]["change"],
        json!(["NaN", "NaN", "NaN"])
    );
    assert_eq!(incremental["string_attrs"]["change"], json!(["", "", ""]));
    let legacy = streamed(bytes, 0, 1);
    assert_eq!(legacy["batches"][3]["new_rows"], 0);
    assert_eq!(legacy["batches"][3]["updated_existing"], true);
}

#[test]
fn late_ordinary_attributes_do_not_enable_other_terminal_mutations() {
    let bytes = concat!(
        "{\"op_code\":0,\"study_name\":\"limits\",\"directions\":[1]}\n",
        "{\"op_code\":4,\"study_id\":0,\"distributions\":{},\"state\":1,\"params\":{\"x\":1},\"values\":[2]}\n",
        "{\"op_code\":5,\"trial_id\":0,\"param_name\":\"new\",\"param_value_internal\":99}\n",
        "{\"op_code\":6,\"trial_id\":0,\"state\":3,\"values\":[99],\"datetime_complete\":\"2026-10-17T00:01:00\"}\n",
        "{\"op_code\":7,\"trial_id\":0,\"step\":1,\"intermediate_value\":99}\n",
        "{\"op_code\":9,\"trial_id\":0,\"system_attr\":{\"unrelated\":true}}\n",
        "{\"op_code\":8,\"trial_id\":999,\"user_attr\":{\"unknown\":true}}\n",
        "{\"op_code\":8,\"trial_id\":0,\"user_attr\":{\"kept\":true}}\n",
    ).as_bytes();
    for size in [Some(1), Some(1000), None] {
        let result = owned(bytes, 0, size);
        assert_eq!(result["view"]["rows"][0]["objective"], "2");
        assert_eq!(result["domain"][0]["state"], "Complete");
        assert_eq!(result["view"]["extras"][0]["complete"], Value::Null);
        assert_eq!(result["view"]["extras"][0]["intermediate"], json!([]));
        assert_eq!(result["domain"].as_array().unwrap().len(), 1);
        assert_eq!(result["params"], json!(["x"]));
        assert_eq!(result["view"]["rows"][0]["attrs"], json!({"kept":true}));
    }
}

#[test]
fn final_correction_delivery_is_fallible_and_does_not_mutate_an_accepted_prefix() {
    let key = StudyKey {
        source: IdentityAllocator::new().unwrap().source().unwrap(),
        stored_id: 1,
    };
    let mut manager = StudyManager::new(key);
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let mut first = None;
    let mut calls = 0;
    let error = study::read_batches(FIXTURE, key, 1, |batch| {
        calls += 1;
        if calls == 2 {
            return Err("correction rejected".into());
        }
        manager
            .apply(manager.header(ticket).unwrap(), batch)
            .unwrap();
        first = manager.acquire();
        Ok(())
    })
    .unwrap_err();
    assert_eq!(error, "correction rejected");
    manager.fail(ticket).unwrap();
    assert_eq!(
        first.unwrap().data().trials()[0].user_attributes["payload"].as_ref(),
        &json!({"old":true})
    );
    assert_eq!(
        manager.acquire().unwrap().data().trials()[0]
            .constraints
            .values[0]
            .0,
        -1.0
    );
}

#[test]
#[ignore = "controlled processing-cost probe; run explicitly with --ignored --nocapture"]
fn journal_postprocessing_cost_probe() {
    use std::time::Instant;
    let mut completion_bytes =
        String::from("{\"op_code\":0,\"study_name\":\"cost\",\"directions\":[1]}\n");
    for id in 0..10_000 {
        completion_bytes.push_str(&format!("{{\"op_code\":4,\"study_id\":0,\"distributions\":{{}},\"state\":1,\"params\":{{\"x\":{id}}},\"values\":[{id}]}}\n"));
    }
    let mut tail_bytes = completion_bytes.clone();
    for i in 0..100_000 {
        tail_bytes.push_str(&format!(
            "{{\"op_code\":8,\"trial_id\":{},\"user_attr\":{{\"tail\":{i}}}}}\n",
            i % 1000
        ));
    }
    for run in 0..4 {
        for (name, bytes) in [
            ("completions", &completion_bytes),
            ("attribute-tail", &tail_bytes),
        ] {
            let key = StudyKey {
                source: IdentityAllocator::new().unwrap().source().unwrap(),
                stored_id: 0,
            };
            let mut manager = StudyManager::new(key);
            let ticket = manager.begin(LoadKind::Initial).unwrap();
            let mut table = TableProjection::default();
            let mut batches = 0;
            let mut upserts = 0;
            let mut first_visible = None;
            let start = Instant::now();
            study::read_batches(bytes.as_bytes(), key, 1000, |batch| {
                upserts += match &batch {
                    tdashboard_study::StudyBatch::Replace { trials, .. } => trials.len(),
                    tdashboard_study::StudyBatch::Delta { upserts, .. } => upserts.len(),
                };
                manager
                    .apply(manager.header(ticket).unwrap(), batch)
                    .unwrap();
                let projection = table.project(&manager.acquire().unwrap());
                std::hint::black_box(projection);
                first_visible.get_or_insert(start.elapsed());
                batches += 1;
                Ok(())
            })
            .unwrap();
            manager
                .finish(ticket, manager.header(ticket).unwrap().sequence)
                .unwrap();
            assert_eq!(manager.acquire().unwrap().data().trials().len(), 10_000);
            assert_eq!(batches, 11);
            assert_eq!(
                upserts,
                if name == "completions" {
                    10_000
                } else {
                    11_000
                }
            );
            println!("run={run} case={name} bytes={} batch_size=1000 first_ms={:.3} final_ms={:.3} batches={batches} upserts={upserts} projections={batches}", bytes.len(), first_visible.unwrap().as_secs_f64()*1000.0, start.elapsed().as_secs_f64()*1000.0);
        }
    }
}
