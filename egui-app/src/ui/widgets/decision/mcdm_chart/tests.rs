use super::compute::normalize_weights;
use super::controls::McdmTopN;
use super::ranking::build_ranking_rows;
use super::*;
use crate::state::results::TopsisResult;
use std::collections::HashMap;
use std::sync::Arc;
use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};

#[test]
fn adopt_compute_state_syncs_runtime_and_preserves_ui_settings() {
    let mut item = McdmRankChart {
        controls: McdmControls {
            computing: true,
            method: McdmMethod::Vikor,
            top_n: McdmTopN::Top20,
            v_param: 0.7,
            ..Default::default()
        },
    };
    let global = McdmRankChart {
        controls: McdmControls {
            computing: false,
            weights: vec![0.25, 0.75],
            ..Default::default()
        },
    };

    item.adopt_compute_state(&global);

    // Execution state and shared output are adopted.
    assert!(!item.controls.computing);
    assert_eq!(item.controls.weights, vec![0.25, 0.75]);
    // UI settings remain item-specific.
    assert_eq!(item.controls.method, McdmMethod::Vikor);
    assert_eq!(item.controls.top_n, McdmTopN::Top20);
    assert_eq!(item.controls.v_param, 0.7);
}

fn make_simple_view(n: usize) -> StudyView {
    if n == 0 {
        let df = DataFrame::from_trials(&[], &[], &[], &[], &[], 0);
        return StudyView::new(Arc::new(df), vec![]);
    }
    let core_rows: Vec<CoreRow> = (0..n)
        .map(|i| CoreRow {
            trial_id: i as u32,
            trial_number: i as u32,
            param_display: HashMap::new(),
            distribution_metadata: Default::default(),
            param_category_label: HashMap::new(),
            objective_values: vec![],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            user_attrs_json: HashMap::new(),
            constraint_values: vec![],
        })
        .collect();
    let df = DataFrame::from_trials(&core_rows, &[], &[], &[], &[], 0);
    StudyView::new(Arc::new(df), vec![0; n])
}

fn make_view_with_objectives(objective_rows: &[Vec<f64>]) -> (StudyView, Vec<String>) {
    let n = objective_rows.len();
    if n == 0 {
        return (make_simple_view(0), vec![]);
    }
    let n_obj = objective_rows[0].len();
    let obj_names: Vec<String> = (0..n_obj).map(|i| format!("obj{i}")).collect();
    let core_rows: Vec<CoreRow> = (0..n)
        .map(|i| CoreRow {
            trial_id: i as u32,
            trial_number: i as u32,
            param_display: HashMap::new(),
            distribution_metadata: Default::default(),
            param_category_label: HashMap::new(),
            objective_values: objective_rows[i].clone(),
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            user_attrs_json: HashMap::new(),
            constraint_values: vec![],
        })
        .collect();
    let df = DataFrame::from_trials(&core_rows, &[], &obj_names, &[], &[], 0);
    (StudyView::new(Arc::new(df), vec![0; n]), obj_names)
}

fn make_topsis_result(scores: Vec<f64>, ranked_indices: Vec<u32>) -> McdmResult {
    McdmResult::Topsis(TopsisResult {
        scores,
        ranked_indices,
        duration_ms: 10.0,
    })
}

// Exercise the actual painted labels, not a separate label-formatting helper.
fn rank_chart_text(
    chart: &mut McdmRankChart,
    view: &StudyView,
    result: &McdmResult,
) -> Vec<String> {
    fn collect(shape: &egui::epaint::Shape, text: &mut Vec<String>) {
        match shape {
            egui::epaint::Shape::Text(t) => text.push(t.galley.text().to_string()),
            egui::epaint::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, text);
                }
            }
            _ => {}
        }
    }
    let ctx = egui::Context::default();
    let mut text = vec![];
    for _ in 0..2 {
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 1000.0),
                )),
                ..Default::default()
            },
            |ui| chart.show(ui, view, &["Objective".into()], Some(result)),
        );
        text.clear();
        for shape in output.shapes {
            collect(&shape.shape, &mut text);
        }
    }
    text
}

fn rank_chart_results() -> Vec<McdmResult> {
    use crate::state::results::{PrometheeResult, VikorResult};
    let scores = vec![0.4, 0.2, 0.6, 0.1, 0.3, 0.7, 0.5];
    let ranked = vec![5, 2, 6, 0, 4, 1, 3];
    let promethee = PrometheeResult {
        phi_plus: scores.clone(),
        phi_minus: vec![0.3, 0.5, 0.1, 0.6, 0.4, 0.0, 0.2],
        phi_net: vec![0.1, -0.3, 0.5, -0.5, -0.1, 0.7, 0.3],
        ranked_indices_i: ranked.clone(),
        // Deliberately different to catch use of the wrong PROMETHEE order.
        ranked_indices_ii: vec![6, 0, 5, 2, 1, 3, 4],
        incomparable_counts: vec![0, 1, 0, 1, 0, 0, 0],
        duration_ms: 1.0,
    };
    vec![
        make_topsis_result(scores.clone(), ranked.clone()),
        McdmResult::Vikor(VikorResult {
            s_values: scores.clone(),
            r_values: scores.clone(),
            q_values: scores.iter().map(|s| 1.0 - s).collect(),
            display_scores: scores,
            ranked_indices: ranked,
            compromise_indices: vec![5],
            duration_ms: 1.0,
        }),
        McdmResult::PrometheeI(promethee.clone()),
        McdmResult::PrometheeII(promethee),
    ]
}

fn assert_rank_chart_labels(view: &StudyView, numbers: &[u32]) {
    for result in rank_chart_results() {
        let before = format!("{result:?}");
        let mut chart = McdmRankChart::default();
        chart.controls.method = result.method();
        // Change Top N on the same widget: truncation must not renumber rows.
        for top_n in [McdmTopN::Top5, McdmTopN::Top10, McdmTopN::Top5] {
            chart.controls.top_n = top_n;
            let text = rank_chart_text(&mut chart, view, &result);
            let expected: Vec<_> = result
                .ranked_indices()
                .iter()
                .take(top_n.value())
                .map(|&idx| format!("Trial {}", numbers[idx as usize]))
                .collect();
            let labels: Vec<_> = text
                .iter()
                .filter(|text| text.starts_with("Trial "))
                .cloned()
                .collect();
            assert_eq!(labels, expected, "{} {top_n:?}", result.method_label());
            let rows = build_ranking_rows(&result, view, &[], &[], top_n.value());
            assert_eq!(
                labels,
                rows.iter()
                    .map(|row| format!("Trial {}", row.trial_number))
                    .collect::<Vec<_>>()
            );
            for (rank, row) in rows.iter().enumerate() {
                let idx = result.ranked_indices()[rank] as usize;
                assert_eq!(row.trial_id, view.trial_ids[idx]);
                assert_eq!(row.score, result.primary_scores()[idx]);
                let score_text = match &result {
                    McdmResult::PrometheeI(r) => {
                        let suffix = if r.incomparable_counts[idx] > 0 {
                            format!(" ⇹{}", r.incomparable_counts[idx])
                        } else {
                            String::new()
                        };
                        format!("Φ+{:.3} Φ-{:.3}{suffix}", r.phi_plus[idx], r.phi_minus[idx])
                    }
                    _ => format!("{:.4}", row.score),
                };
                let label_pos = text.iter().position(|t| t == &labels[rank]).unwrap();
                assert_eq!(text[label_pos + 1], score_text);
            }
        }
        assert_eq!(
            format!("{result:?}"),
            before,
            "Rendering must not change calculations"
        );
    }
}

#[test]
fn rank_chart_all_paths_keep_journal_gaps_and_study_local_numbers() {
    let mut lines = vec![
        serde_json::json!({"op_code":0,"study_name":"A","directions":[1]}),
        serde_json::json!({"op_code":0,"study_name":"B","directions":[1]}),
    ];
    for number in 0..9 {
        for study in 0..2 {
            // Failed and pruned trials are excluded from COMPLETE rows, not numbering.
            let state = match number {
                1 => 3,
                3 => 2,
                _ => 1,
            };
            lines.push(
                serde_json::json!({"op_code":4,"study_id":study,"state":state,
                "values":[number as f64],"distributions":{}}),
            );
        }
    }
    let data = lines
        .iter()
        .map(|line| format!("{line}\n"))
        .collect::<String>();
    for study in [0, 1, 0] {
        let (_, df, _) =
            tunny_core::journal_parser::parse_single_study(data.as_bytes(), study).unwrap();
        let view = StudyView::new(Arc::new(df), vec![]);
        assert_eq!(
            view.trial_ids,
            vec![
                study,
                4 + study,
                8 + study,
                10 + study,
                12 + study,
                14 + study,
                16 + study
            ]
        );
        assert_rank_chart_labels(&view, &[0, 2, 4, 5, 6, 7, 8]);
    }
}

#[test]
fn rank_chart_all_paths_missing_number_falls_back_to_original_row() {
    let mut view = make_simple_view(7);
    view.trial_ids = vec![10, 20, 30, 40, 50, 60, 70];
    // Simulate unavailable numbers without changing the storage API.
    view.df = Arc::new(DataFrame::empty());
    assert_rank_chart_labels(&view, &[0, 1, 2, 3, 4, 5, 6]);
}

#[test]
fn rank_chart_all_paths_ordinary_single_study_is_unchanged() {
    assert_rank_chart_labels(&make_simple_view(7), &[0, 1, 2, 3, 4, 5, 6]);
}

#[test]
fn mcdm_top_n_values() {
    assert_eq!(McdmTopN::Top5.value(), 5);
    assert_eq!(McdmTopN::Top10.value(), 10);
    assert_eq!(McdmTopN::Top20.value(), 20);
}

#[test]
fn normalize_weights_equal() {
    let result = normalize_weights(&[0.5, 0.5]).unwrap();
    assert!((result[0] - 0.5).abs() < 1e-9);
    assert!((result[1] - 0.5).abs() < 1e-9);
}

#[test]
fn normalize_weights_unequal() {
    let result = normalize_weights(&[1.0, 3.0]).unwrap();
    assert!((result[0] - 0.25).abs() < 1e-9);
    assert!((result[1] - 0.75).abs() < 1e-9);
}

#[test]
fn normalize_weights_three_equal() {
    let result = normalize_weights(&[2.0, 2.0, 2.0]).unwrap();
    for w in &result {
        assert!((w - 1.0 / 3.0).abs() < 1e-9);
    }
}

#[test]
fn normalize_weights_zero_fallback() {
    let result = normalize_weights(&[0.0, 0.0]).unwrap();
    assert!((result[0] - 0.5).abs() < 1e-9);
    assert!((result[1] - 0.5).abs() < 1e-9);
}

#[test]
fn normalize_weights_empty() {
    let result = normalize_weights(&[]).unwrap();
    assert!(result.is_empty());
}

#[test]
fn controls_run_preserves_negative_weights_for_error_propagation() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
    let mut controls = McdmControls {
        weights: vec![2.0, -1.0],
        ..Default::default()
    };
    let names = vec!["cost".to_string(), "performance".to_string()];
    let run = |controls: &mut McdmControls, input: egui::RawInput| {
        ctx.run_ui(input, |ui| {
            controls.show_controls(ui, &names, "negative_weights");
        })
    };
    let output = run(
        &mut controls,
        egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        },
    );
    let button = output
        .platform_output
        .accesskit_update
        .unwrap()
        .nodes
        .into_iter()
        .find(|(_, node)| {
            node.role() == egui::accesskit::Role::Button && node.label() == Some("Run")
        })
        .expect("Run button must be available")
        .0;
    let _ = run(
        &mut controls,
        egui::RawInput {
            screen_rect: Some(screen),
            events: vec![egui::Event::AccessKitActionRequest(
                egui::accesskit::ActionRequest {
                    action: egui::accesskit::Action::Click,
                    target_tree: egui::accesskit::TreeId::ROOT,
                    target_node: button,
                    data: None,
                },
            )],
            ..Default::default()
        },
    );
    assert!(controls.computing);
    let request = controls
        .pending_compute
        .take()
        .expect("Run must queue a request");
    assert_eq!(request.weights, vec![2.0, -1.0]);
    assert!(McdmCacheKey::from_request(&request, controls.weight_mode).is_err());
}

#[test]
fn negative_weights_cannot_produce_a_cache_key() {
    for &method in McdmMethod::all() {
        for weights in [
            [2.0, -1.0],
            [-1.0, -1.0],
            [f64::NAN, -1.0],
            [f64::NAN, f64::NEG_INFINITY],
            [1.0, -1e-12],
        ] {
            let expected = normalize_weights(&weights).unwrap_err();
            let controls = McdmControls {
                method,
                weights: weights.to_vec(),
                ..Default::default()
            };
            assert_eq!(controls.cache_key().unwrap_err(), expected);
            let request = McdmComputeRequest {
                method,
                weights: weights.to_vec(),
                v: 0.5,
            };
            assert_eq!(
                McdmCacheKey::from_request(&request, WeightMode::Manual).unwrap_err(),
                expected
            );
        }
    }
}

#[test]
fn request_and_settings_cache_keys_use_the_same_normalization() {
    for &method in McdmMethod::all() {
        for weights in [
            [0.0, 0.0],
            [2.0, 6.0],
            [0.0, 2.0],
            [f64::NAN, 1.0],
            [f64::INFINITY, 1.0],
            [f64::MAX, f64::MAX],
        ] {
            let controls = McdmControls {
                method,
                weights: weights.to_vec(),
                ..Default::default()
            };
            let request = McdmComputeRequest {
                method,
                weights: weights.to_vec(),
                v: 0.5,
            };
            let normalized_request = McdmComputeRequest {
                method,
                weights: normalize_weights(&weights).unwrap(),
                v: 0.5,
            };
            assert_eq!(
                controls.cache_key().unwrap(),
                McdmCacheKey::from_request(&request, WeightMode::Manual).unwrap()
            );
            assert_eq!(
                controls.cache_key().unwrap(),
                McdmCacheKey::from_request(&normalized_request, WeightMode::Manual).unwrap()
            );
        }
    }
}

#[test]
fn mcdm_rank_chart_default() {
    let chart = McdmRankChart::default();
    let c = &chart.controls;
    assert_eq!(c.method, McdmMethod::Topsis);
    assert_eq!(c.weight_mode, WeightMode::Manual);
    assert!(!c.computing);
    assert!(c.pending_compute.is_none());
    assert!(!c.pending_entropy);
    assert!(c.entropy_result.is_none());
    assert_eq!(c.top_n, McdmTopN::Top10);
    assert!(c.weights.is_empty());
    assert!((c.v_param - 0.5).abs() < f64::EPSILON);
}

#[test]
fn mcdm_table_default() {
    let table = McdmTable::default();
    assert_eq!(table.controls.top_n, McdmTopN::Top10);
}

#[test]
fn enumerate_ranked_top5_with_5_results() {
    let result = make_topsis_result(vec![0.9, 0.7, 0.5, 0.3, 0.1], vec![0, 1, 2, 3, 4]);
    let view = make_simple_view(5);
    let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
    assert_eq!(ranking.len(), 5);
    assert!((ranking[0].score - 0.9).abs() < 1e-9);
    assert!((ranking[4].score - 0.1).abs() < 1e-9);
}

#[test]
fn enumerate_ranked_top10_with_20_results() {
    let scores: Vec<f64> = (0..20).map(|i| 1.0 - i as f64 / 20.0).collect();
    let ranked: Vec<u32> = (0..20).collect();
    let result = make_topsis_result(scores, ranked);
    let view = make_simple_view(20);
    let ranking = build_ranking_rows(&result, &view, &[], &[], 10);
    assert_eq!(ranking.len(), 10);
}

#[test]
fn enumerate_ranked_top5_with_3_results_min_applied() {
    let result = make_topsis_result(vec![0.9, 0.5, 0.1], vec![0, 1, 2]);
    let view = make_simple_view(3);
    let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
    assert_eq!(ranking.len(), 3);
}

#[test]
fn enumerate_ranked_scores_match_ranked_order() {
    let result = make_topsis_result(vec![0.1, 0.9, 0.5], vec![1, 2, 0]);
    let view = make_simple_view(3);
    let ranking = build_ranking_rows(&result, &view, &[], &[], 10);
    assert_eq!(ranking.len(), 3);
    assert!((ranking[0].score - 0.9).abs() < 1e-9);
    assert!((ranking[1].score - 0.5).abs() < 1e-9);
    assert!((ranking[2].score - 0.1).abs() < 1e-9);
}

#[test]
fn enumerate_ranked_empty_result() {
    let result = make_topsis_result(vec![], vec![]);
    let view = make_simple_view(0);
    let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
    assert!(ranking.is_empty());
}

#[test]
fn top_n_toggle_cycle() {
    let mut chart = McdmRankChart::default();
    assert_eq!(chart.controls.top_n, McdmTopN::Top10);
    chart.controls.top_n = McdmTopN::Top5;
    assert_eq!(chart.controls.top_n.value(), 5);
    chart.controls.top_n = McdmTopN::Top20;
    assert_eq!(chart.controls.top_n.value(), 20);
}

#[test]
fn build_ranking_rows_basic() {
    let result = make_topsis_result(vec![0.9, 0.5, 0.1], vec![0, 1, 2]);
    let view = make_simple_view(3);
    let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
    assert_eq!(ranking.len(), 3);
    assert_eq!(ranking[0].rank, 1);
    assert_eq!(ranking[0].trial_number, 0);
    assert!((ranking[0].score - 0.9).abs() < 1e-9);
}

#[test]
fn build_ranking_rows_top_n_limit() {
    let scores: Vec<f64> = (0..20).map(|i| 1.0 - i as f64 / 20.0).collect();
    let ranked: Vec<u32> = (0..20).collect();
    let result = make_topsis_result(scores, ranked);
    let view = make_simple_view(20);
    let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
    assert_eq!(ranking.len(), 5);
}

#[test]
fn build_ranking_rows_rank_starts_at_1() {
    let result = make_topsis_result(vec![0.8], vec![0]);
    let view = make_simple_view(1);
    let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
    assert_eq!(ranking[0].rank, 1);
}

#[test]
fn build_ranking_rows_distinguishes_trial_id_and_number() {
    // Verify both are resolved correctly for a Study where trial_id (global, used
    // for pinning) and trial.number (for display) diverge (e.g. when it includes
    // pruned/failed trials).
    let core_rows: Vec<CoreRow> = (0..3)
        .map(|i| CoreRow {
            trial_id: i as u32 + 10,
            trial_number: i as u32 + 100,
            param_display: HashMap::new(),
            distribution_metadata: Default::default(),
            param_category_label: HashMap::new(),
            objective_values: vec![],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            user_attrs_json: HashMap::new(),
            constraint_values: vec![],
        })
        .collect();
    let df = DataFrame::from_trials(&core_rows, &[], &[], &[], &[], 0);
    let view = StudyView::new(Arc::new(df), vec![0; 3]);

    let result = make_topsis_result(vec![0.9, 0.5, 0.1], vec![2, 0, 1]);
    let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
    // rank 1 is trial_idx 2 -> trial_id 12 / number 102
    assert_eq!(ranking[0].trial_id, 12);
    assert_eq!(ranking[0].trial_number, 102);
}

#[test]
fn build_ranking_rows_empty() {
    let result = make_topsis_result(vec![], vec![]);
    let view = make_simple_view(0);
    let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
    assert!(ranking.is_empty());
}

#[test]
fn build_ranking_rows_objectives_included() {
    let result = make_topsis_result(vec![0.9, 0.5], vec![0, 1]);
    let (view, obj_names) = make_view_with_objectives(&[vec![1.0, 2.0], vec![3.0, 4.0]]);
    let ranking = build_ranking_rows(&result, &view, &[], &obj_names, 10);
    assert_eq!(ranking[0].objectives, vec![1.0, 2.0]);
    assert_eq!(ranking[1].objectives, vec![3.0, 4.0]);
}

// ── E2E / integration tests ──

fn multi_obj_data() -> Vec<Vec<f64>> {
    vec![
        vec![0.1, 0.9],
        vec![0.5, 0.5],
        vec![0.9, 0.1],
        vec![0.3, 0.7],
        vec![0.7, 0.3],
    ]
}

#[test]
fn topsis_full_pipeline_equal_weights() {
    let data = multi_obj_data();
    let objectives: Vec<f64> = data.iter().flat_map(|r| r.iter().copied()).collect();
    let weights = normalize_weights(&[1.0, 1.0]).unwrap();
    let is_minimize = vec![true, true];

    let core_result =
        tunny_core::topsis::compute_topsis(&objectives, 5, 2, &weights, &is_minimize).unwrap();

    let mcdm_result = McdmResult::Topsis(TopsisResult {
        scores: core_result.scores.clone(),
        ranked_indices: core_result.ranked_indices.clone(),
        duration_ms: core_result.duration_ms,
    });

    assert_eq!(mcdm_result.primary_scores().len(), 5);
    assert!(!mcdm_result.primary_scores().iter().any(|s| s.is_nan()));

    let (view, obj_names) = make_view_with_objectives(&data);
    let ranking = build_ranking_rows(&mcdm_result, &view, &[], &obj_names, 5);
    assert_eq!(ranking.len(), 5);
    assert_eq!(ranking[0].rank, 1);
    for i in 1..ranking.len() {
        assert!(ranking[i - 1].score >= ranking[i].score);
    }
}

#[test]
fn topsis_weight_bias_changes_ranking() {
    let data = multi_obj_data();
    let objectives: Vec<f64> = data.iter().flat_map(|r| r.iter().copied()).collect();
    let is_minimize = vec![true, true];

    let weights_obj0 = normalize_weights(&[1.0, 0.0]).unwrap();
    let r0 =
        tunny_core::topsis::compute_topsis(&objectives, 5, 2, &weights_obj0, &is_minimize).unwrap();

    let weights_obj1 = normalize_weights(&[0.0, 1.0]).unwrap();
    let r1 =
        tunny_core::topsis::compute_topsis(&objectives, 5, 2, &weights_obj1, &is_minimize).unwrap();

    assert_ne!(
        r0.ranked_indices, r1.ranked_indices,
        "different weights should produce different rankings"
    );
}

#[test]
fn topsis_single_objective_works() {
    let objectives: Vec<f64> = (0..5).map(|i| i as f64 * 0.2).collect();
    let weights = normalize_weights(&[1.0]).unwrap();
    let is_minimize = vec![true];

    let result = tunny_core::topsis::compute_topsis(&objectives, 5, 1, &weights, &is_minimize);
    assert!(result.is_ok());
    let r = result.unwrap();
    assert_eq!(r.scores.len(), 5);
}

#[test]
fn mcdm_chart_run_button_sets_pending_compute() {
    let mut chart = McdmRankChart::default();
    assert!(chart.controls.pending_compute.is_none());
    assert!(!chart.controls.computing);

    let normalized = normalize_weights(&[1.0, 1.0]).unwrap();
    chart.controls.pending_compute = Some(McdmComputeRequest {
        method: McdmMethod::Topsis,
        weights: normalized,
        v: 0.5,
    });
    chart.controls.computing = true;

    assert!(chart.controls.pending_compute.is_some());
    assert!(chart.controls.computing);

    let payload = chart.controls.pending_compute.take();
    assert!(payload.is_some());
    assert!(chart.controls.pending_compute.is_none());
    assert!(chart.controls.computing);
}

#[test]
fn mcdm_compute_request_vikor_includes_v() {
    let req = McdmComputeRequest {
        method: McdmMethod::Vikor,
        weights: vec![0.5, 0.5],
        v: 0.3,
    };
    assert_eq!(req.method, McdmMethod::Vikor);
    assert!((req.v - 0.3).abs() < f64::EPSILON);
}

#[test]
fn top_n_toggle_updates_display() {
    let data = multi_obj_data();
    let objectives: Vec<f64> = data.iter().flat_map(|r| r.iter().copied()).collect();
    let weights = normalize_weights(&[1.0, 1.0]).unwrap();
    let is_minimize = vec![true, true];

    let core_result =
        tunny_core::topsis::compute_topsis(&objectives, 5, 2, &weights, &is_minimize).unwrap();
    let mcdm = McdmResult::Topsis(TopsisResult {
        scores: core_result.scores,
        ranked_indices: core_result.ranked_indices,
        duration_ms: core_result.duration_ms,
    });

    let (view, obj_names) = make_view_with_objectives(&data);

    let rows5 = build_ranking_rows(&mcdm, &view, &[], &obj_names, 5);
    assert_eq!(rows5.len(), 5);

    let rows3 = build_ranking_rows(&mcdm, &view, &[], &obj_names, 3);
    assert_eq!(rows3.len(), 3);

    let rows10 = build_ranking_rows(&mcdm, &view, &[], &obj_names, 10);
    assert_eq!(rows10.len(), 5);
}
