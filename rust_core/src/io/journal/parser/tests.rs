use super::*;
use crate::dataframe::DataFrame;
use serde_json::Value;
use std::collections::HashMap;

#[test]
fn distribution_metadata_survives_inline_events_streaming_and_filtering() {
    use serde_json::json;
    for inline in [true, false] {
        let categorical =
            json!({"name":"CategoricalDistribution","attributes":{"choices":["",1,"1",null]}});
        let distributions = json!({
            "cat": categorical,
            "integer": {"name":"IntDistribution","attributes":{"low":0,"high":10,"step":1}},
            "stepped": {"name":"FloatDistribution","attributes":{"low":0,"high":10,"step":0.5}},
            "continuous": {"name":"FloatDistribution","attributes":{"low":0,"high":10,"step":null}}
        });
        let mut lines = vec![json!({"op_code":0,"study_name":"s","directions":[1]}).to_string()];
        for i in 0..5 {
            if inline {
                let mut params = json!({"integer":2,"stepped":2.5,"continuous":2});
                if i < 4 {
                    params["cat"] = distributions["cat"]["attributes"]["choices"][i].clone();
                }
                lines.push(json!({"op_code":4,"study_id":0,"state":1,"values":[i],"distributions":distributions,"params":params}).to_string());
            } else {
                lines.push(json!({"op_code":4,"study_id":0}).to_string());
                for (name, dist) in distributions.as_object().unwrap() {
                    if name == "cat" && i == 4 {
                        continue;
                    }
                    let value = if name == "cat" { i as f64 } else { 2.0 };
                    lines.push(json!({"op_code":5,"trial_id":i,"param_name":name,"param_value_internal":value,"distribution":dist.to_string()}).to_string());
                }
                lines.push(json!({"op_code":6,"trial_id":i,"state":1,"values":[i]}).to_string());
            }
        }
        let data = lines.join("\n") + "\n";
        let (_, initial, _) = parse_single_study(data.as_bytes(), 0).unwrap();
        let mut streamed = DataFrame::empty();
        parse_single_study_streaming(data.as_bytes(), 0, 1, |batch| {
            streamed.append_trials(
                &batch.new_rows,
                &batch.param_names,
                &batch.objective_names,
                &batch.user_attr_numeric_names,
                &batch.user_attr_string_names,
                batch.max_constraints,
            );
        })
        .unwrap();
        for df in [&initial, &streamed] {
            assert!(df.is_discrete_parameter("integer"));
            assert!(df.is_discrete_parameter("stepped"));
            assert!(!df.is_discrete_parameter("continuous"));
            assert_eq!(
                df.category_values("cat").unwrap(),
                &[
                    Some(json!("")),
                    Some(json!(1)),
                    Some(json!("1")),
                    Some(Value::Null),
                    None
                ]
            );
            let filtered = df.filter_feasible();
            assert_eq!(filtered.category_values("cat"), df.category_values("cat"));
            assert!(filtered.is_discrete_parameter("stepped"));
        }
    }
}

#[test]
fn missing_param_event_values_do_not_fabricate_numeric_or_category_groups() {
    let data = concat!(
        "{\"op_code\":0,\"study_name\":\"s\",\"directions\":[1]}\n",
        "{\"op_code\":4,\"study_id\":0}\n",
        "{\"op_code\":5,\"trial_id\":0,\"param_name\":\"n\",\"distribution\":{\"name\":\"IntDistribution\",\"attributes\":{\"low\":0,\"high\":5}}}\n",
        "{\"op_code\":5,\"trial_id\":0,\"param_name\":\"cat\",\"distribution\":{\"name\":\"CategoricalDistribution\",\"attributes\":{\"choices\":[\"\",\"a\"]}}}\n",
        "{\"op_code\":6,\"trial_id\":0,\"state\":1,\"values\":[2]}\n"
    );
    let (_, df, _) = parse_single_study(data.as_bytes(), 0).unwrap();
    assert!(df.numeric_parameter_column("n").unwrap()[0].is_nan());
    assert_eq!(df.category_values("cat").unwrap(), &[None]);
}

#[test]
fn objective_nonfinite_positions_survive_op4_and_op6_and_are_excluded_from_reports() {
    use crate::report::{
        build_study_report, render_html, render_markdown, Outcome, ReportLang, ReportOptions,
        ReportSource,
    };
    for inline in [true, false] {
        for invalid in ["Infinity", "NaN", "-Infinity", "null", "\"invalid\""] {
            let trial = if inline {
                format!("{{\"op_code\":4,\"study_id\":0,\"distributions\":{{}},\"state\":1,\"values\":[{invalid},5]}}\n")
            } else {
                format!("{{\"op_code\":4,\"study_id\":0}}\n{{\"op_code\":6,\"trial_id\":0,\"state\":1,\"values\":[{invalid},5]}}\n")
            };
            let data = format!("{{\"op_code\":0,\"study_name\":\"s\",\"directions\":[1,1]}}\n{trial}{{\"op_code\":4,\"study_id\":0,\"distributions\":{{}},\"state\":1,\"values\":[7,8]}}\n");
            let (meta, initial, extras) = parse_single_study(data.as_bytes(), 0).unwrap();
            assert_eq!(
                meta.objective_names,
                vec!["obj0", "obj1"],
                "inline={inline}, {invalid}"
            );
            assert_eq!(initial.row_count(), 2);
            let mut streamed = DataFrame::empty();
            parse_single_study_streaming(data.as_bytes(), 0, 1, |batch| {
                for row in &batch.new_rows {
                    assert_eq!(row.objective_values.len(), 2);
                }
                streamed.append_trials(
                    &batch.new_rows,
                    &batch.param_names,
                    &batch.objective_names,
                    &batch.user_attr_numeric_names,
                    &batch.user_attr_string_names,
                    batch.max_constraints,
                );
            })
            .unwrap();
            for df in [&initial, &streamed] {
                assert_eq!(df.objective_col_names(), &["obj0", "obj1"]);
                let first = df.get_numeric_column("obj0").unwrap();
                assert!(first[0].is_nan());
                assert_eq!(first[1], 7.0);
                assert_eq!(df.get_numeric_column("obj1").unwrap(), &[5.0, 8.0]);
                let report = build_study_report(
                    &meta,
                    df,
                    Some(&extras),
                    &ReportSource {
                        storage_display: "journal".into(),
                        generated_at_unix: None,
                    },
                    &ReportOptions::default(),
                );
                let Outcome::MultiObj {
                    pareto_size,
                    complete_count,
                    pareto_table,
                    scatter,
                    ..
                } = &report.outcome
                else {
                    panic!("objective dimension must remain two");
                };
                assert_eq!(*pareto_size, 1);
                assert_eq!(*complete_count, 1);
                assert_eq!(pareto_table.len(), 1);
                assert_eq!(pareto_table[0].trial_number, 1);
                assert_eq!(pareto_table[0].objectives, vec![7.0, 8.0]);
                assert_eq!(scatter.len(), 1);
                assert_eq!(scatter[0].trial_number, 1);
                let json = serde_json::to_value(&report).unwrap();
                let outcome = &json["outcome"]["MultiObj"];
                assert_eq!(outcome["objective_count"], 2);
                assert_eq!(outcome["complete_count"], 1);
                assert_eq!(
                    outcome["pareto_table"][0]["objectives"],
                    serde_json::json!([7.0, 8.0])
                );
                for output in [
                    render_html(&report, ReportLang::En),
                    render_markdown(&report, ReportLang::En),
                ] {
                    assert!(output.contains("1 trial(s) with non-finite objective values"));
                }
            }
        }
    }
}

#[test]
fn python_nonfinite_constraints_and_all_trial_schema_match_metadata_only_batches() {
    use crate::dataframe::FeasibilityState::*;
    let data = concat!(
        "{\"op_code\":0,\"study_name\":\"s\",\"directions\":[1]}\n",
        "{\"op_code\":4,\"study_id\":0,\"distributions\":{},\"state\":1,\"values\":[1],\"system_attrs\":{\"constraints\":[-1]}}\n",
        "{\"op_code\":4,\"study_id\":0}\n",
        "{\"op_code\":9,\"trial_id\":1,\"system_attr\":{\"constraints\":[NaN,0.5,Infinity,-Infinity]}}\n",
        "{\"op_code\":8,\"trial_id\":1,\"user_attr\":{\"text\":\"NaN Infinity -Infinity \\\"Infinity\\\"\",\"number\":7}}\n",
        "{\"op_code\":6,\"trial_id\":1,\"state\":1,\"values\":[2]}\n",
        "{\"op_code\":4,\"study_id\":0}\n",
        "{\"op_code\":9,\"trial_id\":2,\"system_attr\":{\"constraints\":[-1,-2,NaN,Infinity,-Infinity]}}\n",
        "{\"op_code\":6,\"trial_id\":2,\"state\":2,\"values\":[3]}\n",
        "{\"op_code\":4,\"study_id\":0,\"distributions\":{},\"state\":0,\"system_attrs\":{\"constraints\":[-1,-2,-3,-4,-5,-6]}}\n"
    );
    let (meta, initial, _) = parse_single_study(data.as_bytes(), 0).unwrap();
    assert!(meta.has_constraints);
    assert_eq!(initial.row_count(), 2);
    assert_eq!(initial.constraint_col_names().len(), 6);
    assert_eq!(
        initial.feasibility().partition_indices(2),
        (vec![], vec![1], vec![0])
    );
    assert!(initial.get_numeric_column("c1").unwrap()[1].is_nan());
    assert_eq!(initial.get_numeric_column("c2").unwrap()[1], 0.5);
    assert_eq!(
        initial.user_attr_value("text", 1),
        Some(&serde_json::json!("NaN Infinity -Infinity \"Infinity\""))
    );
    let mut batches = Vec::new();
    parse_single_study_streaming(data.as_bytes(), 0, 1, |batch| batches.push(batch)).unwrap();
    let last = batches.last().unwrap();
    assert!(last.new_rows.is_empty());
    assert!(last.is_final && last.meta.has_constraints);
    assert_eq!(last.max_constraints, 6);
    assert_eq!(last.meta.completed_trials, meta.completed_trials);
    assert_eq!(last.meta.total_trials, meta.total_trials);
    let mut streamed = DataFrame::empty();
    for batch in batches {
        streamed.append_trials(
            &batch.new_rows,
            &batch.param_names,
            &batch.objective_names,
            &batch.user_attr_numeric_names,
            &batch.user_attr_string_names,
            batch.max_constraints,
        );
        if batch.meta.has_constraints && !streamed.feasibility().has_constraints() {
            streamed.mark_constrained();
        }
    }
    assert_eq!(
        streamed.constraint_col_names(),
        initial.constraint_col_names()
    );
    for row in 0..2 {
        assert_eq!(
            streamed.feasibility().state(row),
            initial.feasibility().state(row)
        );
    }
    assert_eq!(streamed.feasibility().state(1), Infeasible);
}

#[test]
fn noncomplete_attribute_only_evidence_is_not_unconstrained() {
    for state in [0, 2, 3] {
        let data = format!("{{\"op_code\":0,\"study_name\":\"s\",\"directions\":[1]}}\n{{\"op_code\":4,\"study_id\":0,\"distributions\":{{}},\"state\":1,\"values\":[1]}}\n{{\"op_code\":4,\"study_id\":0,\"distributions\":{{}},\"state\":{state},\"system_attrs\":{{\"constraints\":[]}}}}\n");
        let (meta, initial, _) = parse_single_study(data.as_bytes(), 0).unwrap();
        assert!(meta.has_constraints);
        assert_eq!(initial.row_count(), 1);
        assert_eq!(
            initial.feasibility().state(0),
            crate::dataframe::FeasibilityState::Unverified
        );
        let mut batches = Vec::new();
        parse_single_study_streaming(data.as_bytes(), 0, 1, |batch| batches.push(batch)).unwrap();
        let last = batches.last().unwrap();
        assert!(last.new_rows.is_empty());
        assert!(last.meta.has_constraints);
        assert_eq!(last.max_constraints, 0);
        let mut streamed = DataFrame::empty();
        for batch in batches {
            streamed.append_trials(
                &batch.new_rows,
                &batch.param_names,
                &batch.objective_names,
                &batch.user_attr_numeric_names,
                &batch.user_attr_string_names,
                batch.max_constraints,
            );
            if batch.meta.has_constraints && !streamed.feasibility().has_constraints() {
                streamed.mark_constrained();
            }
        }
        assert_eq!(streamed.row_count(), initial.row_count());
        assert_eq!(
            streamed.constraint_col_names(),
            initial.constraint_col_names()
        );
        assert_eq!(
            streamed.feasibility().state(0),
            initial.feasibility().state(0)
        );
    }
}

#[test]
fn studies_without_complete_rows_still_infer_noncomplete_constraint_schema() {
    for state in [0, 2, 3] {
        let data = format!("{{\"op_code\":0,\"study_name\":\"s\",\"directions\":[1]}}\n{{\"op_code\":4,\"study_id\":0,\"distributions\":{{}},\"state\":{state},\"system_attrs\":{{\"constraints\":[NaN,Infinity,-Infinity]}}}}\n");
        let (meta, initial, _) = parse_single_study(data.as_bytes(), 0).unwrap();
        assert!(meta.has_constraints);
        assert_eq!(meta.completed_trials, 0);
        assert_eq!(initial.row_count(), 0);
        assert_eq!(initial.constraint_col_names().len(), 3);
        let mut batches = Vec::new();
        parse_single_study_streaming(data.as_bytes(), 0, 1, |batch| batches.push(batch)).unwrap();
        assert_eq!(batches.len(), 1);
        let batch = &batches[0];
        assert!(batch.new_rows.is_empty());
        assert!(batch.is_first && batch.is_final && batch.meta.has_constraints);
        assert_eq!(batch.max_constraints, 3);
        let mut streamed = DataFrame::empty();
        streamed.append_trials(
            &batch.new_rows,
            &batch.param_names,
            &batch.objective_names,
            &batch.user_attr_numeric_names,
            &batch.user_attr_string_names,
            batch.max_constraints,
        );
        assert_eq!(
            streamed.constraint_col_names(),
            initial.constraint_col_names()
        );
        assert!(streamed.feasibility().has_constraints());
    }
}

#[test]
fn python_nonfinite_inline_array_is_unverified_not_a_lost_trial() {
    let data = concat!(
        "{\"op_code\":0,\"study_name\":\"s\",\"directions\":[1]}\n",
        "{\"op_code\":4,\"study_id\":0,\"distributions\":{},\"state\":1,\"values\":[2],\"user_attrs\":{\"text\":\"Infinity NaN\",\"value\":17},\"system_attrs\":{\"constraints\":[NaN,Infinity,-Infinity]}}\n"
    );
    let (meta, initial, _) = parse_single_study(data.as_bytes(), 0).unwrap();
    assert!(meta.has_constraints);
    assert_eq!(initial.row_count(), 1);
    assert_eq!(initial.constraint_col_names().len(), 3);
    assert_eq!(
        initial.feasibility().state(0),
        crate::dataframe::FeasibilityState::Unverified
    );
    assert_eq!(
        initial.user_attr_value("text", 0),
        Some(&serde_json::json!("Infinity NaN"))
    );
    assert_eq!(
        initial.user_attr_value("value", 0),
        Some(&serde_json::json!(17))
    );
    let mut streamed = DataFrame::empty();
    parse_single_study_streaming(data.as_bytes(), 0, 1, |batch| {
        streamed.append_trials(
            &batch.new_rows,
            &batch.param_names,
            &batch.objective_names,
            &batch.user_attr_numeric_names,
            &batch.user_attr_string_names,
            batch.max_constraints,
        );
    })
    .unwrap();
    assert_eq!(streamed.row_count(), initial.row_count());
    assert_eq!(
        streamed.feasibility().state(0),
        initial.feasibility().state(0)
    );
    assert_eq!(
        parse_journal(data.as_bytes()).unwrap().studies[0].completed_trials,
        1
    );
}

#[test]
fn invalid_constraint_positions_and_schema_growth_match_streaming() {
    use crate::dataframe::FeasibilityState::*;
    let data = concat!(
        "{\"op_code\":0,\"study_name\":\"s\",\"directions\":[1]}\n",
        "{\"op_code\":4,\"study_id\":0}\n",
        "{\"op_code\":6,\"trial_id\":0,\"state\":1,\"values\":[0]}\n",
        "{\"op_code\":4,\"study_id\":0}\n",
        "{\"op_code\":9,\"trial_id\":1,\"system_attr\":{\"constraints\":[-1]}}\n",
        "{\"op_code\":6,\"trial_id\":1,\"state\":1,\"values\":[1]}\n",
        "{\"op_code\":4,\"study_id\":0}\n",
        "{\"op_code\":9,\"trial_id\":2,\"system_attr\":{\"constraints\":[null,-2,\"invalid\"]}}\n",
        "{\"op_code\":6,\"trial_id\":2,\"state\":1,\"values\":[2]}\n",
        "{\"op_code\":4,\"study_id\":0,\"distributions\":{},\"state\":1,\"values\":[3],\"system_attrs\":{\"constraints\":[null,1,null]}}\n"
    );
    let (meta, initial, _) = parse_single_study(data.as_bytes(), 0).unwrap();
    assert!(meta.has_constraints);
    assert_eq!(initial.constraint_col_names().len(), 3);
    assert!(initial.get_numeric_column("c1").unwrap()[2].is_nan());
    assert_eq!(initial.get_numeric_column("c2").unwrap()[2], -2.0);
    assert!(initial.get_numeric_column("c3").unwrap()[2].is_nan());
    let mut streamed = DataFrame::empty();
    parse_single_study_streaming(data.as_bytes(), 0, 1, |batch| {
        streamed.append_trials(
            &batch.new_rows,
            &batch.param_names,
            &batch.objective_names,
            &batch.user_attr_numeric_names,
            &batch.user_attr_string_names,
            batch.max_constraints,
        );
        if batch.meta.has_constraints && !streamed.feasibility().has_constraints() {
            streamed.mark_constrained();
        }
    })
    .unwrap();
    let expected = vec![Unverified, Unverified, Unverified, Infeasible];
    for df in [&initial, &streamed] {
        assert_eq!(
            (0..4)
                .map(|r| df.feasibility().state(r))
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(df.constraint_col_names().len(), 3);
    }
}

#[test]
fn constraints_attribute_with_empty_or_invalid_value_establishes_study() {
    for attr in ["[]", "null", "\"invalid\""] {
        let data = format!("{{\"op_code\":0,\"study_name\":\"s\",\"directions\":[1]}}\n{{\"op_code\":4,\"study_id\":0,\"distributions\":{{}},\"state\":1,\"values\":[1],\"system_attrs\":{{\"constraints\":{attr}}}}}\n");
        let (meta, df, _) = parse_single_study(data.as_bytes(), 0).unwrap();
        assert!(meta.has_constraints);
        assert!(df.feasibility().has_constraints());
        assert!(df.constraint_col_names().is_empty());
        assert_eq!(
            df.feasibility().state(0),
            crate::dataframe::FeasibilityState::Unverified
        );
    }
}

// ── parse_single_study / scan_study_list ──────────────────────────────────

fn two_study_log() -> Vec<u8> {
    // Study 0: "alpha", 2 trials, param "x"
    // Study 1: "beta",  3 trials, param "y"
    to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"alpha\",\"directions\":[1]}\n",
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"beta\",\"directions\":[2]}\n",
        // --- alpha trials (study_id=0) ---
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"x\",\"param_value_internal\":0.1,\"distribution\":{\"name\":\"FloatDistribution\",\"low\":0.0,\"high\":1.0}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[1.0],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:02.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":1,\"param_name\":\"x\",\"param_value_internal\":0.2,\"distribution\":{\"name\":\"FloatDistribution\",\"low\":0.0,\"high\":1.0}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":1,\"state\":1,\"values\":[2.0],\"datetime_complete\":\"2024-01-01T00:00:03.000000\"}\n",
        // --- beta trials (study_id=1) ---
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":1,\"datetime_start\":\"2024-01-01T00:00:04.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":2,\"param_name\":\"y\",\"param_value_internal\":10.0,\"distribution\":{\"name\":\"FloatDistribution\",\"low\":0.0,\"high\":100.0}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":2,\"state\":1,\"values\":[3.0],\"datetime_complete\":\"2024-01-01T00:00:05.000000\"}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":1,\"datetime_start\":\"2024-01-01T00:00:06.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":3,\"param_name\":\"y\",\"param_value_internal\":20.0,\"distribution\":{\"name\":\"FloatDistribution\",\"low\":0.0,\"high\":100.0}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":3,\"state\":1,\"values\":[4.0],\"datetime_complete\":\"2024-01-01T00:00:07.000000\"}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":1,\"datetime_start\":\"2024-01-01T00:00:08.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":4,\"param_name\":\"y\",\"param_value_internal\":30.0,\"distribution\":{\"name\":\"FloatDistribution\",\"low\":0.0,\"high\":100.0}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":4,\"state\":1,\"values\":[5.0],\"datetime_complete\":\"2024-01-01T00:00:09.000000\"}\n"
    ))
}

#[test]
fn scan_study_list_returns_names_only() {
    let data = two_study_log();
    let studies = scan_study_list(&data).unwrap();
    assert_eq!(studies.len(), 2);
    assert_eq!(studies[0].name, "alpha");
    assert_eq!(studies[1].name, "beta");
    // completed_trials is not yet determined in Phase 1
    assert_eq!(studies[0].completed_trials, 0);
    assert_eq!(studies[1].completed_trials, 0);
}

#[test]
fn parse_single_study_alpha_returns_correct_trials() {
    let data = two_study_log();
    let (meta, df, _extras) = parse_single_study(&data, 0).unwrap();
    assert_eq!(meta.name, "alpha");
    assert_eq!(meta.completed_trials, 2);
    assert_eq!(meta.total_trials, 2);
    assert!(meta.param_names.contains(&"x".to_string()));
    assert_eq!(df.row_count(), 2);
}

#[test]
fn parse_single_study_beta_skips_alpha_trials() {
    let data = two_study_log();
    let (meta, df, _extras) = parse_single_study(&data, 1).unwrap();
    assert_eq!(meta.name, "beta");
    assert_eq!(meta.completed_trials, 3);
    assert_eq!(meta.total_trials, 3);
    // beta's only param is "y" (alpha's "x" is not included)
    assert!(meta.param_names.contains(&"y".to_string()));
    assert!(!meta.param_names.contains(&"x".to_string()));
    assert_eq!(df.row_count(), 3);
}

#[test]
fn parse_single_study_beta_trial_number_is_in_study_index() {
    // beta's (study_id=1) global trial_id values are 2,3,4, but the trial.number within the
    // study must be 0,1,2 (Optuna's trial.number = creation order within the study).
    let data = two_study_log();
    let (_meta, df, _extras) = parse_single_study(&data, 1).unwrap();
    let ids: Vec<u32> = (0..df.row_count())
        .map(|r| df.get_trial_id(r).unwrap())
        .collect();
    let numbers: Vec<u32> = (0..df.row_count())
        .map(|r| df.get_trial_number(r).unwrap())
        .collect();
    assert_eq!(ids, vec![2, 3, 4]);
    assert_eq!(numbers, vec![0, 1, 2]);
}

#[test]
fn streaming_beta_trial_number_is_in_study_index() {
    // trial.number must also be sequential within the study on the streaming path (the route the UI uses).
    let data = two_study_log();
    let mut batches: Vec<StudyStreamBatch> = Vec::new();
    parse_single_study_streaming(&data, 1, 2, |b| batches.push(b)).unwrap();
    let ids: Vec<u32> = batches
        .iter()
        .flat_map(|b| b.new_rows.iter().map(|r| r.trial_id))
        .collect();
    let numbers: Vec<u32> = batches
        .iter()
        .flat_map(|b| b.new_rows.iter().map(|r| r.trial_number))
        .collect();
    assert_eq!(ids, vec![2, 3, 4]);
    assert_eq!(numbers, vec![0, 1, 2]);
}

#[test]
fn parse_single_study_objective_values_correct() {
    let data = two_study_log();
    let (_meta, df, _extras) = parse_single_study(&data, 0).unwrap();
    let obj: Vec<f64> = df
        .get_numeric_column("obj0")
        .map(|c| c.to_vec())
        .unwrap_or_default();
    assert_eq!(obj, vec![1.0, 2.0]);
}

#[test]
fn parse_single_study_nonexistent_id_returns_error() {
    let data = two_study_log();
    let result = parse_single_study(&data, 99);
    assert!(result.is_err());
}

#[test]
fn streaming_beta_matches_single_study_and_batches() {
    let data = two_study_log();
    // batch_size=2 -> beta has 3 trials, so 2 batches: [2 rows] + [1 row (final)]
    let mut batches: Vec<StudyStreamBatch> = Vec::new();
    parse_single_study_streaming(&data, 1, 2, |b| batches.push(b)).unwrap();

    assert_eq!(batches.len(), 2);
    assert!(batches[0].is_first && !batches[0].is_final);
    assert!(!batches[1].is_first && batches[1].is_final);
    assert_eq!(batches[0].new_rows.len(), 2);
    assert_eq!(batches[1].new_rows.len(), 1);

    // Concatenating all rows should match parse_single_study
    let all_ids: Vec<u32> = batches
        .iter()
        .flat_map(|b| b.new_rows.iter().map(|r| r.trial_id))
        .collect();
    assert_eq!(all_ids, vec![2, 3, 4]);

    // The final meta should be beta, 3 trials, param "y" only (no mixing in of alpha's "x")
    let final_meta = &batches.last().unwrap().meta;
    assert_eq!(final_meta.name, "beta");
    assert_eq!(final_meta.completed_trials, 3);
    assert!(final_meta.param_names.contains(&"y".to_string()));
    assert!(!final_meta.param_names.contains(&"x".to_string()));

    // Objective values should line up in the correct order
    let objs: Vec<f64> = batches
        .iter()
        .flat_map(|b| b.new_rows.iter().map(|r| r.objective_values[0]))
        .collect();
    assert_eq!(objs, vec![3.0, 4.0, 5.0]);
}

#[test]
fn streaming_emits_inline_completed_trials_inmem() {
    // In-memory storage format: op_code=4 carries state/values/params inline, with no
    // subsequent op_code=5/6. Verifies that completed trials can still be emitted on the
    // streaming path (the route the UI uses) (regression guard: this used to yield 0 rows and
    // the trial count wasn't displayed).
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"dtlz\",\"directions\":[1,1]}\n",
        "{\"op_code\":3,\"worker_id\":\"w\",\"study_id\":0,\"system_attr\":{\"study:metric_names\":[\"Obj1\",\"Obj2\"]}}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"state\":1,\"value\":null,\"values\":[1.0,2.0],\"distributions\":{\"x\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": 0.0, \\\"high\\\": 1.0, \\\"log\\\": false}}\"},\"params\":{\"x\":0.5},\"user_attrs\":{},\"system_attrs\":{}}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"state\":3,\"value\":null,\"values\":null,\"distributions\":{\"x\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": 0.0, \\\"high\\\": 1.0, \\\"log\\\": false}}\"},\"params\":{\"x\":0.2},\"user_attrs\":{},\"system_attrs\":{}}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"state\":1,\"value\":null,\"values\":[3.0,0.5],\"distributions\":{\"x\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": 0.0, \\\"high\\\": 1.0, \\\"log\\\": false}}\"},\"params\":{\"x\":0.7},\"user_attrs\":{},\"system_attrs\":{}}\n"
    ));
    let mut batches: Vec<StudyStreamBatch> = Vec::new();
    parse_single_study_streaming(&data, 0, 1000, |b| batches.push(b)).unwrap();

    // There are 2 completed trials (state==1). state==3 (fail) is excluded.
    let total: usize = batches.iter().map(|b| b.new_rows.len()).sum();
    assert_eq!(total, 2, "inline-completed trials should be emitted");

    let last = batches.last().expect("at least one batch");
    assert!(last.is_final);
    assert_eq!(last.meta.completed_trials, 2);
    assert_eq!(last.objective_names, vec!["Obj1", "Obj2"]);
    assert!(last.param_names.contains(&"x".to_string()));

    // Objective values should line up in the correct order.
    let objs: Vec<f64> = batches
        .iter()
        .flat_map(|b| b.new_rows.iter().map(|r| r.objective_values[0]))
        .collect();
    assert_eq!(objs, vec![1.0, 3.0]);
}

#[test]
fn streaming_single_batch_when_batch_size_large() {
    let data = two_study_log();
    let mut batches: Vec<StudyStreamBatch> = Vec::new();
    parse_single_study_streaming(&data, 0, 1000, |b| batches.push(b)).unwrap();
    // alpha has 2 trials -> 1 batch (is_first and is_final)
    assert_eq!(batches.len(), 1);
    assert!(batches[0].is_first && batches[0].is_final);
    assert_eq!(batches[0].new_rows.len(), 2);
    assert_eq!(batches[0].objective_names, vec!["obj0".to_string()]);
}

#[test]
fn streaming_nonexistent_id_returns_error() {
    let data = two_study_log();
    assert!(parse_single_study_streaming(&data, 99, 100, |_| {}).is_err());
}

/// Regression test confirming that when study:metric_names is set via op_code=3, the streaming
/// batch's objective_names matches the meta names (e.g. "Obj") rather than the derived ones.
/// This used to prioritize derived_objective_names ("obj0"), leaving the chart empty.
#[test]
fn streaming_objective_names_prefer_metric_names_over_derived() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":3,\"worker_id\":\"w\",\"study_id\":0,\"system_attr\":{\"study:metric_names\":[\"MyObj\"]}}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[1.5],\"datetime_complete\":\"2024-01-01T00:00:01\"}\n",
    ));
    let mut batches: Vec<StudyStreamBatch> = Vec::new();
    parse_single_study_streaming(&data, 0, 1000, |b| batches.push(b)).unwrap();
    assert_eq!(batches.len(), 1);
    // The batch's objective_names should use metric_names ("MyObj"), not derived ("obj0")
    assert_eq!(batches[0].objective_names, vec!["MyObj".to_string()]);
    // Should also match meta's objective_names
    assert_eq!(batches[0].meta.objective_names, vec!["MyObj".to_string()]);
}

#[test]
fn line_u32_field_basic() {
    assert_eq!(
        line_u32_field(r#"{"op_code":4,"study_id":2,"x":0}"#, "study_id"),
        Some(2)
    );
    assert_eq!(
        line_u32_field(r#"{"trial_id":  42,"state":1}"#, "trial_id"),
        Some(42)
    );
    assert_eq!(line_u32_field(r#"{"no_field":1}"#, "study_id"), None);
}

fn to_bytes(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

#[test]
fn tc_101_01_create_study_basic() {
    let data =
        to_bytes(r#"{"op_code":0,"worker_id":"w1","study_name":"my_study","directions":[1,2]}"#);
    let result = parse_journal(&data).expect("journal should parse successfully");
    assert_eq!(result.studies.len(), 1);
    assert_eq!(result.studies[0].name, "my_study");
    assert_eq!(
        result.studies[0].directions,
        vec![
            OptimizationDirection::Minimize,
            OptimizationDirection::Maximize,
        ]
    );
}

// ── extras: op7 intermediate values / datetimes / all states ────────────────────────────────

/// study 0: trial 0 is COMPLETE (2 intermediate values with datetimes, submitted in reverse step order),
/// trial 1 is PRUNED (no op7). extras contains both (all states), while
/// the DataFrame contains COMPLETE only.
fn extras_log() -> Vec<u8> {
    to_bytes(concat!(
        "{\"op_code\":0,\"study_name\":\"alpha\",\"directions\":[1]}\n",
        "{\"op_code\":4,\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
        // Submitted with steps not in ascending order, to verify the sort in finalize.
        "{\"op_code\":7,\"trial_id\":0,\"step\":1,\"intermediate_value\":0.2}\n",
        "{\"op_code\":7,\"trial_id\":0,\"step\":0,\"intermediate_value\":0.1}\n",
        "{\"op_code\":6,\"trial_id\":0,\"state\":1,\"values\":[1.0],\"datetime_complete\":\"2024-01-01T00:00:10\"}\n",
        "{\"op_code\":4,\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:11\"}\n",
        "{\"op_code\":6,\"trial_id\":1,\"state\":2}\n"
    ))
}

#[test]
fn parse_single_study_collects_extras_for_all_states() {
    use crate::extras::TrialState;

    let data = extras_log();
    let (_meta, df, extras) = parse_single_study(&data, 0).unwrap();

    // DataFrame contains COMPLETE only.
    assert_eq!(
        df.row_count(),
        1,
        "PRUNED trial must be excluded from DataFrame"
    );

    // extras keeps all states (trial 0 and 1) ordered by ascending trial_id.
    assert_eq!(extras.trials.len(), 2);
    assert!(extras.has_intermediate());
    assert!(extras.has_datetimes());
    let ids: Vec<u32> = extras.trials.iter().map(|t| t.trial_id).collect();
    assert_eq!(ids, vec![0, 1]);

    let t0 = &extras.trials[0];
    assert_eq!(t0.state, TrialState::Complete);
    // Sorted by ascending step regardless of insertion order.
    assert_eq!(t0.intermediate_values, vec![(0, 0.1), (1, 0.2)]);
    assert_eq!(t0.datetime_start, Some(1_704_067_200.0));
    assert_eq!(t0.datetime_complete, Some(1_704_067_210.0));

    let t1 = &extras.trials[1];
    assert_eq!(t1.state, TrialState::Pruned);
    assert!(t1.intermediate_values.is_empty());
    assert_eq!(t1.datetime_start, Some(1_704_067_211.0));
    assert_eq!(t1.datetime_complete, None);
}

#[test]
fn parse_journal_stores_extras_in_shared_store() {
    let data = extras_log();
    parse_journal(&data).unwrap();
    // Full parsing stores extras keyed by enumerate (study_id = index).
    let extras = crate::dataframe::extras_snapshot(0).expect("extras must be stored");
    assert_eq!(extras.trials.len(), 2);
    assert!(extras.has_intermediate());
}

/// Verifies that Phase 2 streaming (the single-study load path the UI actually uses) also
/// reflects every trial in extras — completed, pruned, and still Running at EOF — and stores
/// them into the shared store via `store_extras_for` (keyed by the real study_id).
#[test]
fn parse_single_study_streaming_stores_extras_for_all_states() {
    use crate::extras::TrialState;

    let data = to_bytes(concat!(
        "{\"op_code\":0,\"study_name\":\"alpha\",\"directions\":[1]}\n",
        "{\"op_code\":4,\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00\"}\n",
        // Submitted with steps not in ascending order, to verify sorting also occurs on the streaming path.
        "{\"op_code\":7,\"trial_id\":0,\"step\":1,\"intermediate_value\":0.2}\n",
        "{\"op_code\":7,\"trial_id\":0,\"step\":0,\"intermediate_value\":0.1}\n",
        "{\"op_code\":6,\"trial_id\":0,\"state\":1,\"values\":[1.0],\"datetime_complete\":\"2024-01-01T00:00:10\"}\n",
        // trial 1: PRUNED (op6 state=2)
        "{\"op_code\":4,\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:11\"}\n",
        "{\"op_code\":6,\"trial_id\":1,\"state\":2}\n",
        // trial 2: never completes (stays Running in extras at EOF)
        "{\"op_code\":4,\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:12\"}\n"
    ));

    // Compare before/after the run to confirm storage is keyed by the target study's own study_id.
    let study_id = 0u32;
    let mut batches: Vec<StudyStreamBatch> = Vec::new();
    parse_single_study_streaming(&data, study_id, 1000, |b| batches.push(b)).unwrap();

    let extras = crate::dataframe::extras_snapshot(study_id)
        .expect("streaming path must store_extras_for the target study_id");
    assert_eq!(extras.trials.len(), 3, "COMPLETE + PRUNED + still-Running");
    assert!(extras.has_intermediate());
    assert!(extras.has_datetimes());

    let ids: Vec<u32> = extras.trials.iter().map(|t| t.trial_id).collect();
    assert_eq!(ids, vec![0, 1, 2], "trial_id 昇順");

    let t0 = &extras.trials[0];
    assert_eq!(t0.state, TrialState::Complete);
    assert_eq!(
        t0.intermediate_values,
        vec![(0, 0.1), (1, 0.2)],
        "step 昇順"
    );
    assert_eq!(t0.datetime_start, Some(1_704_067_200.0));
    assert_eq!(t0.datetime_complete, Some(1_704_067_210.0));

    let t1 = &extras.trials[1];
    assert_eq!(t1.state, TrialState::Pruned);
    assert!(t1.intermediate_values.is_empty());

    let t2 = &extras.trials[2];
    assert_eq!(
        t2.state,
        TrialState::Running,
        "EOF 時点で未完了の trial は Running のまま extras に含まれる"
    );
    assert!(t2.datetime_complete.is_none());

    // The DataFrame (COMPLETE only) contains only trial 0.
    let total_rows: usize = batches.iter().map(|b| b.new_rows.len()).sum();
    assert_eq!(
        total_rows, 1,
        "PRUNED/Running trial must be excluded from rows"
    );
}

#[test]
fn tc_101_02_create_trial_complete() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("journal should parse successfully");
    assert_eq!(result.studies[0].completed_trials, 1);
    assert_eq!(result.studies[0].total_trials, 1);
}

#[test]
fn tc_101_03_float_distribution_no_log() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"x\",\"param_value_internal\":0.5,\"distribution\":{\"name\":\"FloatDistribution\",\"low\":0.0,\"high\":1.0,\"log\":false}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("journal should parse successfully");
    assert!(result.studies[0].param_names.contains(&"x".to_string()));
}

#[test]
fn tc_101_04_float_distribution_log_true() {
    let ln2: f64 = std::f64::consts::LN_2;
    let line = format!(
        "{{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"lr\",\"param_value_internal\":{ln2},\"distribution\":{{\"name\":\"FloatDistribution\",\"low\":1e-5,\"high\":1.0,\"log\":true}}}}"
    );
    let data = to_bytes(&format!(
        "{}\n{}\n{}\n{}\n",
        r#"{"op_code":0,"worker_id":"w","study_name":"s","directions":[0]}"#,
        r#"{"op_code":4,"worker_id":"w","study_id":0,"datetime_start":"2024-01-01T00:00:00.000000"}"#,
        line,
        r#"{"op_code":6,"worker_id":"w","trial_id":0,"state":1,"values":[0.5],"datetime_complete":"2024-01-01T00:00:01.000000"}"#,
    ));
    let result = parse_journal(&data).expect("journal should parse successfully");
    assert!(result.studies[0].param_names.contains(&"lr".to_string()));
}

#[test]
fn tc_101_05_int_distribution_basic() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"n\",\"param_value_internal\":3.0,\"distribution\":{\"name\":\"IntDistribution\",\"low\":0,\"high\":10,\"step\":1,\"log\":false}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("journal should parse successfully");
    assert!(result.studies[0].param_names.contains(&"n".to_string()));
}

#[test]
fn tc_101_07_categorical_distribution_string() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"cat\",\"param_value_internal\":1.0,\"distribution\":{\"name\":\"CategoricalDistribution\",\"choices\":[\"a\",\"b\",\"c\"]}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("journal should parse successfully");
    assert!(result.studies[0].param_names.contains(&"cat".to_string()));
}

#[test]
fn tc_101_10_multiple_studies() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"A\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.1],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:01.000000\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":1,\"state\":1,\"values\":[0.2],\"datetime_complete\":\"2024-01-01T00:00:02.000000\"}\n",
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"B\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":1,\"datetime_start\":\"2024-01-01T00:00:02.000000\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":2,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:03.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("journal should parse successfully");
    assert_eq!(result.studies.len(), 2);
    let sa = result
        .studies
        .iter()
        .find(|study| study.name == "A")
        .unwrap();
    let sb = result
        .studies
        .iter()
        .find(|study| study.name == "B")
        .unwrap();
    assert_eq!(sa.completed_trials, 2);
    assert_eq!(sb.completed_trials, 1);
}

#[test]
fn tc_101_11_trial_id_sequential() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:01.000000\"}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:02.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("journal should parse successfully");
    assert_eq!(result.studies[0].total_trials, 3);
}

#[test]
fn tc_101_12_user_attr_numeric() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":8,\"worker_id\":\"w\",\"trial_id\":0,\"user_attr\":{\"loss\":0.123}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("journal should parse successfully");
    assert!(result.studies[0]
        .user_attr_names
        .contains(&"loss".to_string()));
}

#[test]
fn tc_101_13_user_attr_string() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":8,\"worker_id\":\"w\",\"trial_id\":0,\"user_attr\":{\"tag\":\"run_a\"}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("journal should parse successfully");
    assert!(result.studies[0]
        .user_attr_names
        .contains(&"tag".to_string()));
}

#[test]
fn json_user_attrs_survive_inline_and_update_events_in_streaming() {
    use serde_json::json;

    let events = [
        json!({"op_code":0,"worker_id":"w","study_name":"s","directions":[1]}),
        json!({"op_code":4,"worker_id":"w","study_id":0,"state":1,"values":[1.0],"distributions":{},
            "user_attrs":{"flag":true,"nothing":null,"list":[1,{"ok":false}],
                "details":{"b":2,"a":1},"mixed":1.5,"literal":"{\"a\":1}"}}),
        json!({"op_code":4,"worker_id":"w","study_id":0}),
        json!({"op_code":8,"worker_id":"w","trial_id":1,"user_attr":{"mixed":2}}),
        json!({"op_code":8,"worker_id":"w","trial_id":1,"user_attr":{"mixed":"two"}}),
        json!({"op_code":8,"worker_id":"w","trial_id":1,"user_attr":{"mixed":null}}),
        json!({"op_code":6,"worker_id":"w","trial_id":1,"state":1,"values":[2.0]}),
    ];
    let data = events
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    let (_, full, _) = parse_single_study(data.as_bytes(), 0).unwrap();
    assert_eq!(full.row_count(), 2);
    assert_eq!(full.user_attr_value("flag", 0), Some(&json!(true)));
    assert_eq!(full.user_attr_value("nothing", 0), Some(&Value::Null));
    assert_eq!(full.user_attr_value("nothing", 1), None);
    assert_eq!(
        full.user_attr_value("list", 0),
        Some(&json!([1,{"ok":false}]))
    );
    assert_eq!(
        full.user_attr_value("details", 0),
        Some(&json!({"a":1,"b":2}))
    );
    assert_eq!(
        full.user_attr_value("literal", 0),
        Some(&json!("{\"a\":1}"))
    );
    assert_eq!(full.user_attr_value("mixed", 0), Some(&json!(1.5)));
    assert_eq!(full.user_attr_value("mixed", 1), Some(&Value::Null));
    assert!(full.get_numeric_column("mixed").unwrap()[1].is_nan());

    let mut batches = Vec::new();
    parse_single_study_streaming(data.as_bytes(), 0, 1, |batch| batches.push(batch)).unwrap();
    let mut streamed = DataFrame::from_trials(
        &batches[0].new_rows,
        &batches[0].param_names,
        &batches[0].objective_names,
        &batches[0].user_attr_numeric_names,
        &batches[0].user_attr_string_names,
        batches[0].max_constraints,
    );
    for batch in batches.iter().skip(1) {
        streamed.append_trials(
            &batch.new_rows,
            &batch.param_names,
            &batch.objective_names,
            &batch.user_attr_numeric_names,
            &batch.user_attr_string_names,
            batch.max_constraints,
        );
    }
    for name in full.user_attr_names() {
        for row in 0..full.row_count() {
            assert_eq!(
                streamed.user_attr_value(name, row),
                full.user_attr_value(name, row)
            );
        }
    }
}

#[test]
fn tc_101_14_constraints_expansion() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":9,\"worker_id\":\"w\",\"trial_id\":0,\"system_attr\":{\"constraints\":[-0.5,0.3]}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("journal should parse successfully");
    assert!(result.studies[0].has_constraints);
}

#[test]
fn tc_101_15_constraints_all_feasible() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":9,\"worker_id\":\"w\",\"trial_id\":0,\"system_attr\":{\"constraints\":[-1.0,-0.5,0.0]}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("journal should parse successfully");
    assert!(result.studies[0].has_constraints);
    assert_eq!(result.studies[0].completed_trials, 1);
}

#[test]
fn tc_101_16_multi_objective_values() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[1,2]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.1,0.9],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("journal should parse successfully");
    assert_eq!(result.studies[0].objective_names.len(), 2);
}

#[test]
fn tc_101_17_duration_ms_returned() {
    let data = to_bytes(r#"{"op_code":0,"worker_id":"w","study_name":"s","directions":[0]}"#);
    let result = parse_journal(&data).expect("journal should parse successfully");
    assert!(result.duration_ms >= 0.0);
}

#[test]
fn tc_101_e01_incomplete_json_line_skipped() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().studies[0].completed_trials, 1);
}

#[test]
fn tc_101_e02_non_json_line_skipped() {
    let mut data = Vec::new();
    data.extend_from_slice(
        b"{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
    );
    data.extend_from_slice(b"not-json-at-all\n");
    data.extend_from_slice(b"\xff\xfe\x00\n");
    data.extend_from_slice(b"{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n");
    data.extend_from_slice(b"{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n");
    let result = parse_journal(&data);
    assert!(result.is_ok());
}

#[test]
fn tc_101_e03_unknown_opcode_ignored() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":99,\"worker_id\":\"w\"}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data);
    assert!(result.is_ok());
}

#[test]
fn tc_101_e04_all_lines_invalid_returns_error() {
    let data: Vec<u8> = vec![0xff, 0xfe, 0x00, 0x01, 0x02];
    let result = parse_journal(&data);
    assert!(result.is_err());
}

#[test]
fn tc_101_e06_all_trials_not_complete() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("journal should parse successfully");
    assert_eq!(result.studies[0].completed_trials, 0);
    assert_eq!(result.studies[0].total_trials, 2);
}

#[test]
fn tc_101_e07_distributed_optimization_overwrite() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w1\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w2\",\"trial_id\":0,\"state\":1,\"values\":[0.3],\"datetime_complete\":\"2024-01-01T00:00:02.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("journal should parse successfully");
    assert_eq!(result.studies[0].completed_trials, 1);
}

#[test]
fn tc_101_b01_empty_file() {
    let data: Vec<u8> = Vec::new();
    let result = parse_journal(&data);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().studies.len(), 0);
}

#[test]
fn tc_101_b02_study_only_no_trials() {
    let data = to_bytes(r#"{"op_code":0,"worker_id":"w","study_name":"s","directions":[0]}"#);
    let result = parse_journal(&data).expect("journal should parse successfully");
    assert_eq!(result.studies.len(), 1);
    assert_eq!(result.studies[0].completed_trials, 0);
}

#[test]
fn tc_101_b03_categorical_boundary_indices() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"cat\",\"param_value_internal\":0.0,\"distribution\":{\"name\":\"CategoricalDistribution\",\"choices\":[\"a\",\"b\",\"c\"]}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[0.5],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("journal should parse successfully");
    assert!(result.studies[0].param_names.contains(&"cat".to_string()));
}

#[test]
fn tc_101_b07_minimal_journal() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[0]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[1.0],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n"
    ));
    let result = parse_journal(&data).expect("journal should parse successfully");
    assert_eq!(result.studies[0].completed_trials, 1);
}

#[test]
fn tc_101_p01_performance_50000_lines() {
    let mut lines = Vec::with_capacity(150_002);
    lines.push(r#"{"op_code":0,"worker_id":"w","study_name":"perf","directions":[0]}"#.to_string());
    for i in 0u32..50_000 {
        let val = f64::from(i) / 50_000.0;
        lines.push(r#"{"op_code":4,"worker_id":"w","study_id":0,"datetime_start":"2024-01-01T00:00:00.000000"}"#.to_string());
        lines.push(format!(r#"{{"op_code":5,"worker_id":"w","trial_id":{i},"param_name":"x","param_value_internal":{val},"distribution":{{"name":"FloatDistribution","low":0.0,"high":1.0,"log":false}}}}"#));
        lines.push(format!(r#"{{"op_code":6,"worker_id":"w","trial_id":{i},"state":1,"values":[{val}],"datetime_complete":"2024-01-01T00:00:01.000000"}}"#));
    }
    let data = lines.join("\n").into_bytes();

    let result = parse_journal(&data).expect("50,000-line journal should parse successfully");

    assert_eq!(result.studies[0].completed_trials, 50_000);
}

#[test]
fn distribution_float_display_is_identity() {
    let dist = Distribution::Float {
        step: None,
        low: 0.0,
        high: 1.0,
    };
    assert!((dist.to_display_f64(0.5) - 0.5).abs() < 1e-10);
}

#[test]
fn distribution_int_display_is_stored_value() {
    // Optuna stores the real value regardless of low/step (no offset even when low=1)
    let dist = Distribution::Int { low: 1, high: 10 };
    assert!((dist.to_display_f64(3.0) - 3.0).abs() < 1e-10);
}

#[test]
fn distribution_int_display_rounds_float_noise() {
    let dist = Distribution::Int { low: 0, high: 10 };
    assert!((dist.to_display_f64(4.000000001) - 4.0).abs() < 1e-10);
}

#[test]
fn distribution_categorical_label() {
    let dist = Distribution::Categorical {
        choices: vec![
            Value::String("a".to_string()),
            Value::String("b".to_string()),
            Value::String("c".to_string()),
        ],
    };
    assert_eq!(dist.categorical_label(1.0), Some("b".to_string()));
    assert_eq!(dist.categorical_label(0.0), Some("a".to_string()));
    assert_eq!(dist.categorical_label(2.0), Some("c".to_string()));
}

#[test]
fn trial_builder_constraint_values_stored() {
    let trial = TrialBuilder {
        study_id: 0,
        trial_number: 0,
        state: 1,
        values: None,
        param_display: HashMap::new(),
        distribution_metadata: Default::default(),
        param_category_label: HashMap::new(),
        user_attrs_numeric: HashMap::new(),
        user_attrs_string: HashMap::new(),
        user_attrs_json: HashMap::new(),
        constraint_values: vec![-1.0, -0.5, 0.0],
        has_constraints: true,
        datetime_start: None,
        datetime_complete: None,
        intermediate_values: Vec::new(),
    };
    assert_eq!(trial.constraint_values.len(), 3);
    assert!(trial.constraint_values.iter().all(|&value| value <= 0.0));
    let sum: f64 = trial.constraint_values.iter().sum();
    assert!((sum - (-1.5)).abs() < 1e-10);
}

#[test]
fn distribution_from_json_string_with_attributes() {
    let json_str = r#""{\"name\": \"FloatDistribution\", \"attributes\": {\"step\": 0.01, \"low\": -32.77, \"high\": 32.77, \"log\": false}}""#;
    let val: Value = serde_json::from_str(json_str).unwrap();
    let dist = Distribution::from_json(&val);
    assert!(matches!(dist, Distribution::Float { .. }));
    assert!((dist.to_display_f64(7.4) - 7.4).abs() < 1e-10);
}

#[test]
fn distribution_from_json_string_log_true() {
    // Even for a log distribution, the stored value is the external representation (real value), so the display conversion is the identity
    let json_str = r#""{\"name\": \"FloatDistribution\", \"attributes\": {\"step\": 0.0, \"low\": 1e-5, \"high\": 1.0, \"log\": true}}""#;
    let val: Value = serde_json::from_str(json_str).unwrap();
    let dist = Distribution::from_json(&val);
    assert!(matches!(dist, Distribution::Float { .. }));
    let x = 0.125;
    assert!((dist.to_display_f64(x) - x).abs() < 1e-10);
}

#[test]
fn distribution_from_json_object_with_attributes() {
    let val: Value = serde_json::from_str(
        r#"{"name": "IntDistribution", "attributes": {"low": 0, "high": 10, "step": 2, "log": false}}"#,
    )
    .unwrap();
    let dist = Distribution::from_json(&val);
    assert!(matches!(dist, Distribution::Int { low: 0, high: 10 }));
    // Even with a step set, the stored value is the real value (external representation) itself
    assert!((dist.to_display_f64(6.0) - 6.0).abs() < 1e-10);
}

#[test]
fn parse_real_log_format_param_values() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[1]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2026-03-28T11:58:48.485367\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"x0\",\"param_value_internal\":7.4,\"distribution\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": -32.77, \\\"high\\\": 32.77, \\\"log\\\": false}}\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"x1\",\"param_value_internal\":17.43,\"distribution\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": -32.77, \\\"high\\\": 32.77, \\\"log\\\": false}}\"}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[21.64],\"datetime_complete\":\"2026-03-28T11:58:48.612043\"}\n"
    ));
    let result = parse_journal(&data).expect("journal should parse successfully");
    assert_eq!(result.studies[0].completed_trials, 1);
    assert!(result.studies[0].param_names.contains(&"x0".to_string()));
    assert!(result.studies[0].param_names.contains(&"x1".to_string()));
}

#[test]
fn tc_inmem_01_basic() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"dtlz\",\"directions\":[1,1]}\n",
        "{\"op_code\":3,\"worker_id\":\"w\",\"study_id\":0,\"system_attr\":{\"study:metric_names\":[\"Obj1\",\"Obj2\"]}}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2026-01-01T00:00:00.000000\",\"state\":1,\"value\":null,\"values\":[1.0,2.0],\"datetime_complete\":\"2026-01-01T00:00:01.000000\",\"distributions\":{\"x\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": 0.0, \\\"high\\\": 1.0, \\\"log\\\": false}}\"},\"params\":{\"x\":0.5},\"user_attrs\":{},\"system_attrs\":{},\"intermediate_values\":{}}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2026-01-01T00:00:01.000000\",\"state\":1,\"value\":null,\"values\":[3.0,0.5],\"datetime_complete\":\"2026-01-01T00:00:02.000000\",\"distributions\":{\"x\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": 0.0, \\\"high\\\": 1.0, \\\"log\\\": false}}\"},\"params\":{\"x\":0.7},\"user_attrs\":{},\"system_attrs\":{},\"intermediate_values\":{}}\n"
    ));
    let result = parse_journal(&data).expect("inmem parse");
    assert_eq!(result.studies.len(), 1);
    assert_eq!(result.studies[0].name, "dtlz");
    assert_eq!(result.studies[0].completed_trials, 2);
    assert_eq!(result.studies[0].total_trials, 2);
    assert!(result.studies[0].param_names.contains(&"x".to_string()));
    assert_eq!(result.studies[0].objective_names, vec!["Obj1", "Obj2"]);
}

#[test]
fn tc_inmem_02_incomplete_state_not_counted() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[1]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2026-01-01T00:00:00.000000\",\"state\":3,\"value\":null,\"values\":null,\"datetime_complete\":null,\"distributions\":{\"x\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": 0.0, \\\"high\\\": 1.0, \\\"log\\\": false}}\"},\"params\":{\"x\":0.5},\"user_attrs\":{},\"system_attrs\":{},\"intermediate_values\":{}}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2026-01-01T00:00:01.000000\",\"state\":1,\"value\":null,\"values\":[0.8],\"datetime_complete\":\"2026-01-01T00:00:02.000000\",\"distributions\":{\"x\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": 0.0, \\\"high\\\": 1.0, \\\"log\\\": false}}\"},\"params\":{\"x\":0.3},\"user_attrs\":{},\"system_attrs\":{},\"intermediate_values\":{}}\n"
    ));
    let result = parse_journal(&data).expect("inmem incomplete state parse");
    assert_eq!(result.studies[0].total_trials, 2);
    assert_eq!(result.studies[0].completed_trials, 1);
}

#[test]
fn tc_inmem_03_user_attrs_inline() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"s\",\"directions\":[1]}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2026-01-01T00:00:00.000000\",\"state\":1,\"value\":null,\"values\":[0.5],\"datetime_complete\":\"2026-01-01T00:00:01.000000\",\"distributions\":{\"x\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": 0.01, \\\"low\\\": 0.0, \\\"high\\\": 1.0, \\\"log\\\": false}}\"},\"params\":{\"x\":0.5},\"user_attrs\":{\"loss\":0.123,\"tag\":\"run_a\"},\"system_attrs\":{},\"intermediate_values\":{}}\n"
    ));
    let result = parse_journal(&data).expect("inmem user_attrs parse");
    assert!(result.studies[0]
        .user_attr_names
        .contains(&"loss".to_string()));
    assert!(result.studies[0]
        .user_attr_names
        .contains(&"tag".to_string()));
}

#[test]
fn tc_inmem_04_single_objective_value_singular() {
    let data = to_bytes(concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"ackley\",\"directions\":[1]}\n",
        "{\"op_code\":3,\"worker_id\":\"w\",\"study_id\":0,\"system_attr\":{\"study:metric_names\":[\"Obj\"]}}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2026-01-01T00:00:00.000000\",\"state\":1,\"value\":21.13,\"values\":null,\"datetime_complete\":\"2026-01-01T00:00:01.000000\",\"distributions\":{\"x\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": null, \\\"low\\\": -32.77, \\\"high\\\": 32.77, \\\"log\\\": false}}\"},\"params\":{\"x\":1.5},\"user_attrs\":{},\"system_attrs\":{},\"intermediate_values\":{}}\n",
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2026-01-01T00:00:01.000000\",\"state\":1,\"value\":5.42,\"values\":null,\"datetime_complete\":\"2026-01-01T00:00:02.000000\",\"distributions\":{\"x\":\"{\\\"name\\\": \\\"FloatDistribution\\\", \\\"attributes\\\": {\\\"step\\\": null, \\\"low\\\": -32.77, \\\"high\\\": 32.77, \\\"log\\\": false}}\"},\"params\":{\"x\":0.1},\"user_attrs\":{},\"system_attrs\":{},\"intermediate_values\":{}}\n"
    ));
    let result = parse_journal(&data).expect("single-objective inmem parse");
    assert_eq!(result.studies.len(), 1);
    assert_eq!(result.studies[0].completed_trials, 2);
    assert_eq!(result.studies[0].objective_names, vec!["Obj"]);
    assert!(result.studies[0].param_names.contains(&"x".to_string()));
}

#[test]
fn parse_real_inmem_log_file() {
    let log_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("test_inmem.log");
    if !log_path.exists() {
        eprintln!("test_inmem.log not found at {:?}, skipping", log_path);
        return;
    }
    let data = std::fs::read(&log_path).expect("test_inmem.log read");
    let result = parse_journal(&data).expect("test_inmem.log parse");

    assert!(!result.studies.is_empty(), "at least 1 study");
    let study = &result.studies[0];
    assert!(study.completed_trials > 0, "should have completed trials");
    assert_eq!(study.param_names.len(), 10, "DTLZ1 has 10 params");
    for i in 0..10 {
        let name = format!("DTLZ1_Variable{i}");
        assert!(study.param_names.contains(&name), "missing param {name}");
    }
    assert_eq!(
        study.objective_names,
        vec!["Obj1", "Obj2"],
        "objective names from study:metric_names"
    );
}

#[test]
fn parse_real_log_file() {
    let log_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("test.log");
    if !log_path.exists() {
        eprintln!("test.log not found at {:?}, skipping", log_path);
        return;
    }
    let data = std::fs::read(&log_path).expect("test.log should be readable");
    let result = parse_journal(&data).expect("test.log should parse successfully");

    assert!(
        result.studies.len() >= 2,
        "test.log should contain at least 2 studies"
    );

    let ackley = &result.studies[0];
    assert!(
        ackley.completed_trials > 0,
        "Ackley study should have completed trials"
    );
    assert_eq!(
        ackley.param_names.len(),
        10,
        "Ackley study should have 10 params"
    );
    for i in 0..10 {
        let name = format!("Ackley_Variable{i}");
        assert!(
            ackley.param_names.contains(&name),
            "Ackley study should contain param {name}"
        );
    }

    let dtlz = &result.studies[1];
    assert!(
        dtlz.completed_trials > 0,
        "DTLZ1 study should have completed trials"
    );
    assert_eq!(
        dtlz.param_names.len(),
        10,
        "DTLZ1 study should have 10 params"
    );
    for i in 0..10 {
        let name = format!("DTLZ1_Variable{i}");
        assert!(
            dtlz.param_names.contains(&name),
            "DTLZ1 study should contain param {name}"
        );
    }

    use crate::dataframe::with_df;
    let df_check = with_df(0, |df| {
        let param_cols = df.param_col_names();
        assert_eq!(
            param_cols.len(),
            10,
            "Ackley DataFrame should have 10 param columns"
        );
        let col = df
            .get_numeric_column("Ackley_Variable0")
            .expect("Ackley_Variable0 column should exist");
        assert!(
            col[0].abs() > 1e-10,
            "Ackley_Variable0 should be non-zero: {}",
            col[0]
        );
    });
    assert!(
        df_check.is_some(),
        "DataFrame for study 0 should be accessible"
    );
}
