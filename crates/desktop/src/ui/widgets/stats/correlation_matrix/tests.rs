use super::*;
use crate::ui::widget_states::WidgetStates;
use std::collections::BTreeMap;
use std::sync::Arc;
use tdashboard_study::*;

fn manager() -> StudyManager {
    StudyManager::new(StudyKey {
        source: IdentityAllocator::new().unwrap().source().unwrap(),
        stored_id: 1,
    })
}

fn replace(manager: &mut StudyManager, ticket: LoadTicket, label: &str, y: [f64; 3]) -> StudyView {
    let header = manager.header(ticket).unwrap();
    let key = header.ticket.study();
    let trials = y
        .into_iter()
        .enumerate()
        .map(|(i, y)| {
            let mut trial = TrialRecord::new(
                key,
                i as u32,
                StudyTrialNumber(i as u32),
                TrialState::Complete,
            );
            trial.completed_order = Some(i as u64);
            trial.parameters = Arc::new(BTreeMap::from([
                ("x".into(), observation(i as f64)),
                (label.into(), observation(y)),
            ]));
            trial.objectives = Arc::new(BTreeMap::from([(
                ObjectiveIndex(0),
                Number((i * i) as f64),
            )]));
            Arc::new(trial)
        })
        .collect();
    manager
        .apply(
            header,
            StudyBatch::Replace {
                trials,
                metadata: StudyMetadata {
                    name: "same-name".into(),
                    objectives: vec![ObjectiveDescriptor {
                        index: ObjectiveIndex(0),
                        name: Some("objective".into()),
                        direction: Direction::Minimize,
                    }],
                    ..Default::default()
                },
            },
        )
        .unwrap();
    let input = manager.acquire().unwrap();
    let projection = Arc::new(tdashboard_analysis::projection::project(&input));
    StudyView::from_projection(projection, input, None)
}

fn observation(value: f64) -> ParameterObservation {
    ParameterObservation {
        value: ParameterValue::Numeric(Number(value)),
        distribution: None,
        external_encoding: false,
        encoded_numeric: None,
    }
}

fn render(chart: &mut CorrelationMatrixChart, view: &StudyView, params: &[&str]) -> Vec<String> {
    fn collect(shape: &egui::epaint::Shape, text: &mut Vec<String>) {
        match shape {
            egui::epaint::Shape::Text(shape) => text.push(shape.galley.text().to_string()),
            egui::epaint::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, text);
                }
            }
            _ => {}
        }
    }
    let ctx = egui::Context::default();
    let params: Vec<_> = params.iter().map(|name| (*name).to_string()).collect();
    let mut text = vec![];
    for _ in 0..2 {
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 800.0),
                )),
                ..Default::default()
            },
            |ui| chart.show(ui, view, &params, view.objective_names()),
        );
        text.clear();
        for shape in output.shapes {
            collect(&shape.shape, &mut text);
        }
    }
    text
}

fn assert_matrix(
    chart: &CorrelationMatrixChart,
    text: &[String],
    labels: &[&str],
    coefficient: f64,
) {
    let matrix = &chart.cache.as_ref().unwrap().1;
    assert_eq!(matrix.labels, labels);
    assert!((matrix.values[0][1] - coefficient).abs() < 1e-10);
    assert!((matrix.values[1][0] - coefficient).abs() < 1e-10);
    for label in labels {
        assert_eq!(text.iter().filter(|s| s.as_str() == *label).count(), 2);
    }
    assert!(text.contains(&format!("{coefficient:.2}")));
}

#[test]
fn correlation_matrix_chart_default_values() {
    let chart = CorrelationMatrixChart::default();
    assert_eq!(chart.method, CorrelationMethod::Pearson);
    assert!(chart.include_params);
    assert!(chart.include_objectives);
    assert!(chart.cache.is_none());
}

#[test]
fn displayed_revisions_schema_sources_and_reload_refresh_the_painted_matrix() {
    let mut manager = manager();
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let first = replace(&mut manager, ticket, "y", [0.0, 1.0, 2.0]);
    let mut widgets = WidgetStates::default();
    widgets.correlation_matrix.method = CorrelationMethod::Spearman;
    widgets.correlation_matrix.include_objectives = false;
    let text = render(&mut widgets.correlation_matrix, &first, &["x", "y"]);
    assert_matrix(&widgets.correlation_matrix, &text, &["x", "y"], 1.0);
    render(&mut widgets.correlation_matrix, &first.clone(), &["x", "y"]);
    assert_eq!(widgets.correlation_matrix.computations, 1);

    let revised = replace(&mut manager, ticket, "y", [2.0, 1.0, 0.0]);
    assert_eq!(first.row_count(), revised.row_count());
    widgets.invalidate_analysis_input();
    let text = render(&mut widgets.correlation_matrix, &revised, &["x", "y"]);
    assert_matrix(&widgets.correlation_matrix, &text, &["x", "y"], -1.0);
    assert_eq!(widgets.correlation_matrix.computations, 2);

    let renamed = replace(&mut manager, ticket, "renamed", [0.0, 1.0, 2.0]);
    widgets.invalidate_analysis_input();
    let text = render(&mut widgets.correlation_matrix, &renamed, &["x", "renamed"]);
    assert_matrix(&widgets.correlation_matrix, &text, &["x", "renamed"], 1.0);
    assert!(!text.contains(&"y".into()));
    assert_eq!(widgets.correlation_matrix.computations, 3);

    let reload = manager.begin(LoadKind::Reload).unwrap();
    // A requested reload must not relabel or invalidate the still-displayed input.
    render(&mut widgets.correlation_matrix, &renamed, &["x", "renamed"]);
    assert_eq!(widgets.correlation_matrix.computations, 3);
    let reloaded = replace(&mut manager, reload, "renamed", [2.0, 1.0, 0.0]);
    let text = render(
        &mut widgets.correlation_matrix,
        &reloaded,
        &["x", "renamed"],
    );
    assert_matrix(&widgets.correlation_matrix, &text, &["x", "renamed"], -1.0);
    assert_eq!(widgets.correlation_matrix.computations, 4);

    let mut other = self::manager();
    let ticket = other.begin(LoadKind::Initial).unwrap();
    let replacement = replace(&mut other, ticket, "renamed", [0.0, 1.0, 2.0]);
    widgets.invalidate_analysis_input();
    let text = render(
        &mut widgets.correlation_matrix,
        &replacement,
        &["x", "renamed"],
    );
    assert_matrix(&widgets.correlation_matrix, &text, &["x", "renamed"], 1.0);
    assert_eq!(widgets.correlation_matrix.computations, 5);
    assert_eq!(
        widgets.correlation_matrix.method,
        CorrelationMethod::Spearman
    );
    assert!(widgets.correlation_matrix.include_params);
    assert!(!widgets.correlation_matrix.include_objectives);
}

#[test]
fn ordered_columns_method_and_groups_select_the_actual_rendered_result() {
    let mut manager = manager();
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let view = replace(&mut manager, ticket, "y", [2.0, 1.0, 0.0]);
    let mut chart = CorrelationMatrixChart {
        include_objectives: false,
        ..Default::default()
    };
    let text = render(&mut chart, &view, &["x", "y"]);
    assert_matrix(&chart, &text, &["x", "y"], -1.0);
    let text = render(&mut chart, &view, &["y", "x"]);
    assert_matrix(&chart, &text, &["y", "x"], -1.0);
    assert_eq!(chart.computations, 2);

    chart.include_objectives = true;
    let text = render(&mut chart, &view, &["x"]);
    assert_matrix(&chart, &text, &["x", "objective"], (12.0_f64 / 13.0).sqrt());
    chart.method = CorrelationMethod::Spearman;
    let text = render(&mut chart, &view, &["x"]);
    assert_matrix(&chart, &text, &["x", "objective"], 1.0);
    assert_eq!(chart.computations, 4);

    chart.include_params = false;
    chart.include_objectives = false;
    let text = render(&mut chart, &view, &["x"]);
    assert!(text.contains(&"Select at least one column group.".into()));
    assert!(!text.contains(&"objective".into()));
    chart.include_params = true;
    let text = render(&mut chart, &view, &["missing"]);
    assert!(text.contains(&"No data.".into()));
    assert!(!text.contains(&"objective".into()));
    let text = render(&mut chart, &view, &["x", "y"]);
    assert_matrix(&chart, &text, &["x", "y"], -1.0);
    assert_eq!(chart.computations, 5);
}

#[test]
fn equal_revision_counters_do_not_alias_sources_or_reload_generations() {
    let mut first_manager = manager();
    let ticket = first_manager.begin(LoadKind::Initial).unwrap();
    let first = replace(&mut first_manager, ticket, "y", [0.0, 1.0, 2.0]);
    let mut other_manager = manager();
    let ticket = other_manager.begin(LoadKind::Initial).unwrap();
    let other = replace(&mut other_manager, ticket, "y", [2.0, 1.0, 0.0]);
    let mut chart = CorrelationMatrixChart {
        include_objectives: false,
        ..Default::default()
    };
    let text = render(&mut chart, &first, &["x", "y"]);
    assert_matrix(&chart, &text, &["x", "y"], 1.0);
    let text = render(&mut chart, &other, &["x", "y"]);
    assert_matrix(&chart, &text, &["x", "y"], -1.0);
    assert_eq!(chart.computations, 2);

    let ticket = other_manager.begin(LoadKind::Reload).unwrap();
    let reloaded = replace(&mut other_manager, ticket, "y", [0.0, 1.0, 2.0]);
    let text = render(&mut chart, &reloaded, &["x", "y"]);
    assert_matrix(&chart, &text, &["x", "y"], 1.0);
    assert_eq!(chart.computations, 3);
}

#[test]
fn unstamped_equal_size_inputs_never_reuse_stale_coefficients_or_labels() {
    let mut manager = manager();
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let first = replace(&mut manager, ticket, "y", [0.0, 1.0, 2.0]);
    let revised = replace(&mut manager, ticket, "y", [2.0, 1.0, 0.0]);
    let renamed = replace(&mut manager, ticket, "new", [0.0, 1.0, 2.0]);
    let unstamped = |view: &StudyView| {
        let projection = tdashboard_analysis::projection::project(&view.input().unwrap());
        let df = tdashboard_analysis::dataframe::DataFrame::from_trials(
            &projection.trial_rows,
            view.param_names(),
            view.objective_names(),
            &[],
            &[],
            0,
        );
        assert!(df.provenance().is_none());
        StudyView::new(Arc::new(df), vec![])
    };
    let mut chart = CorrelationMatrixChart {
        include_objectives: false,
        ..Default::default()
    };
    let text = render(&mut chart, &unstamped(&first), &["x", "y"]);
    assert_matrix(&chart, &text, &["x", "y"], 1.0);
    let text = render(&mut chart, &unstamped(&revised), &["x", "y"]);
    assert_matrix(&chart, &text, &["x", "y"], -1.0);
    let text = render(&mut chart, &unstamped(&renamed), &["x", "new"]);
    assert_matrix(&chart, &text, &["x", "new"], 1.0);
    assert!(!text.contains(&"y".into()));
    assert_eq!(chart.computations, 6);
}
