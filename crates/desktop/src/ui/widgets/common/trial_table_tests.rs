use super::*;
use crate::state::app_state::{StudyContext, StudyMeta, StudyView};
use crate::ui::widget_states::WidgetStates;
use egui::{epaint::Shape, Event, PointerButton, Pos2, RawInput, Rect};
use std::sync::Arc;
use tdashboard_analysis::dataframe::{DataFrame, TrialRow};

fn study(ids: &[u32]) -> StudyContext {
    let rows = ids
        .iter()
        .map(|&id| TrialRow {
            trial_id: id,
            trial_number: 100 + id,
            user_attrs_json: [("list".into(), serde_json::json!([id]))].into(),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    StudyContext {
        meta: StudyMeta {
            study_id: 0,
            name: "Table fixture".into(),
            directions: vec![],
            completed_trials: rows.len(),
            param_names: vec![],
            objective_names: vec![],
            param_bounds: Default::default(),
        },
        view: StudyView::new(
            Arc::new(DataFrame::from_trials(&rows, &[], &[], &[], &[], 0)),
            vec![],
        ),
        pareto_indices: vec![],
    }
}

fn frame(
    ctx: &egui::Context,
    table: &mut TrialTable,
    app: &mut AppState,
    events: Vec<Event>,
) -> Vec<(String, Rect)> {
    let output = ctx.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 800.0))),
            events,
            ..Default::default()
        },
        |ui| table.show(ui, app),
    );
    fn text(shape: &Shape, labels: &mut Vec<(String, Rect)>) {
        match shape {
            Shape::Text(t) => labels.push((
                t.galley.text().to_string(),
                t.galley.rect.translate(t.pos.to_vec2()),
            )),
            Shape::Vec(shapes) => {
                for shape in shapes {
                    text(shape, labels);
                }
            }
            _ => {}
        }
    }
    let mut labels = vec![];
    for shape in output.shapes {
        text(&shape.shape, &mut labels);
    }
    labels
}

fn render(
    ctx: &egui::Context,
    table: &mut TrialTable,
    app: &mut AppState,
    expected_numbers: &[u32],
) -> Vec<(String, Rect)> {
    frame(ctx, table, app, vec![]);
    let labels = frame(ctx, table, app, vec![]);
    let numbers = labels
        .iter()
        .filter_map(|(text, _)| text.parse::<u32>().ok().filter(|n| *n >= 100))
        .collect::<Vec<_>>();
    assert_eq!(numbers, expected_numbers);
    labels
}

fn click(ctx: &egui::Context, table: &mut TrialTable, app: &mut AppState, pos: Pos2) {
    for pressed in [true, false] {
        frame(
            ctx,
            table,
            app,
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ],
        );
    }
}

fn check_targets(
    ctx: &egui::Context,
    table: &mut TrialTable,
    app: &mut AppState,
    labels: &[(String, Rect)],
    id: u32,
) {
    let number = labels
        .iter()
        .find(|(s, _)| *s == (100 + id).to_string())
        .unwrap()
        .1;
    let pin = labels
        .iter()
        .find(|(s, r)| (s == "·" || s == "📌") && (r.center().y - number.center().y).abs() < 5.0)
        .unwrap()
        .1;
    click(ctx, table, app, number.center());
    assert_eq!(app.highlighted_trial, Some(id));
    let mut expected = app.pinned_trials.clone();
    if expected.contains(&id) {
        expected.retain(|&p| p != id);
    } else {
        expected.push(id);
    }
    click(ctx, table, app, pin.center());
    assert_eq!(app.pinned_trials, expected);
}

#[test]
fn all_rows_follow_identity_after_equal_sized_reorder_and_replacement() {
    // Both the normal invalidation path and a mapping change without notification
    // must be safe. Selection and pins stay unchanged across each replacement.
    for invalidate in [false, true] {
        for (before, after, pins, expected, target) in [
            (vec![10, 20], vec![20, 10], vec![], vec![110], 10),
            (
                vec![10, 20, 30],
                vec![30, 10, 20],
                vec![30],
                vec![130, 110],
                30,
            ),
            (vec![10, 20], vec![40, 10], vec![], vec![110], 10),
            (vec![10, 20], vec![40, 20], vec![], vec![], 0),
        ] {
            let ctx = egui::Context::default();
            let mut widgets = WidgetStates::default();
            let mut app = AppState {
                current_study: Some(study(&before)),
                selected_indices: vec![10],
                pinned_trials: pins,
                ..Default::default()
            };
            frame(&ctx, &mut widgets.trial_table, &mut app, vec![]);
            app.current_study = Some(study(&after));
            if invalidate {
                widgets.invalidate_analysis_input();
            }
            let labels = render(&ctx, &mut widgets.trial_table, &mut app, &expected);
            if !expected.is_empty() {
                check_targets(&ctx, &mut widgets.trial_table, &mut app, &labels, target);
            }
        }
    }
}

#[test]
fn empty_selection_keeps_input_order_and_instances_keep_local_controls() {
    let ctx = egui::Context::default();
    let mut first = WidgetStates::default();
    let mut second = WidgetStates::default();
    first.trial_table.show_user_attrs = false;
    first.trial_table.expand_user_attr_lists = true;
    second.trial_table.expand_user_attr_lists = true;
    let mut app = AppState {
        current_study: Some(study(&[10, 20])),
        selected_indices: vec![10],
        pinned_trials: vec![20],
        ..Default::default()
    };
    render(&ctx, &mut first.trial_table, &mut app, &[110, 120]);
    render(&ctx, &mut second.trial_table, &mut app, &[110, 120]);
    app.current_study = Some(study(&[20, 10]));
    first.invalidate_analysis_input();
    second.invalidate_analysis_input();
    let labels = render(&ctx, &mut first.trial_table, &mut app, &[120, 110]);
    assert!(!labels.iter().any(|(s, _)| s.starts_with("User attr:")));
    let labels = render(&ctx, &mut second.trial_table, &mut app, &[120, 110]);
    assert!(labels.iter().any(|(s, _)| s == "User attr: list[0]"));
    app.selected_indices.clear();
    app.current_study = Some(study(&[40, 30]));
    render(&ctx, &mut first.trial_table, &mut app, &[140, 130]);
    render(&ctx, &mut second.trial_table, &mut app, &[140, 130]);
    assert!(!first.trial_table.show_user_attrs);
    assert!(first.trial_table.expand_user_attr_lists);
    assert!(second.trial_table.show_user_attrs);
    assert!(second.trial_table.expand_user_attr_lists);
}
