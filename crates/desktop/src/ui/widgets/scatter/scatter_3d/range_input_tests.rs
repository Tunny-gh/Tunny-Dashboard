use crate::state::app_state::AppState;
use crate::state::types::{StudyContext, StudyView};
use crate::ui::widget_states::WidgetStates;
use crate::ui::widgets::cluster_scatter::KSelectionMode;
use std::collections::BTreeMap;
use std::sync::Arc;
use tdashboard_analysis::preparation::{FeatureSpace, Preparation};
use tdashboard_analysis::session::{
    AnalysisKind, AnalysisRequest, AnalysisSession, ReuseOrRerun, WorkOrShared,
};
use tdashboard_study::*;

fn replace(manager: &mut StudyManager, kind: LoadKind, offset: f64) -> StudyInput {
    let ticket = manager.begin(kind).unwrap();
    let key = manager.header(ticket).unwrap().ticket.study();
    let trials = (0..3)
        .map(|i| {
            let mut trial = TrialRecord::new(key, i, StudyTrialNumber(i), TrialState::Complete);
            trial.completed_order = Some(u64::from(i));
            let x = offset + f64::from(i);
            trial.objectives = Arc::new(BTreeMap::from([
                (ObjectiveIndex(0), Number(x)),
                (ObjectiveIndex(1), Number(offset + 2.0 - f64::from(i))),
                (ObjectiveIndex(2), Number(x)),
                (ObjectiveIndex(3), Number(x * 10.0)),
            ]));
            Arc::new(trial)
        })
        .collect();
    let receipt = manager
        .apply(
            manager.header(ticket).unwrap(),
            StudyBatch::Replace {
                trials,
                metadata: StudyMetadata {
                    name: "same-name".into(),
                    objectives: (0..4)
                        .map(|index| ObjectiveDescriptor {
                            index: ObjectiveIndex(index),
                            name: Some(format!("f{index}")),
                            direction: Direction::Minimize,
                        })
                        .collect(),
                    ..Default::default()
                },
            },
        )
        .unwrap();
    manager.finish(ticket, receipt.next_sequence).unwrap();
    manager.acquire().unwrap()
}

fn install(app: &mut AppState, widgets: &WidgetStates, input: StudyInput, unstamped: bool) {
    let mut session = AnalysisSession::default();
    let projection = session.project(input.clone());
    let front = session.pareto(input.clone()).unwrap();
    let prepared = session
        .prepare_for(
            vec![input.clone()],
            AnalysisRequest {
                kind: AnalysisKind::Cluster,
                conditions: vec![("k".into(), 2_usize.into())],
            },
            Preparation::Cluster {
                space: FeatureSpace::Objectives,
                manual_k: Some(2),
            },
            std::slice::from_ref(&front),
        )
        .unwrap();
    let WorkOrShared::Work(work) = session.start(prepared, ReuseOrRerun::Reuse).unwrap() else {
        unreachable!()
    };
    let result = session
        .complete(work.execute(|prepared| {
            assert_eq!(prepared.data().features().rows, vec![0, 1, 2]);
            Ok(crate::state::results::ClusterResult {
                labels: vec![0, 0, 1],
                n_clusters: 2,
            })
        }))
        .unwrap();
    assert_eq!(result.key().analysis.inputs, vec![input.stamp()]);
    app.cluster_cache.clear();
    app.cluster_cache.select(
        widgets.cluster_scatter_3d.cache_key(),
        tdashboard_analysis::shared::SharedResult::new(result).unwrap(),
    );
    let mut view = StudyView::from_projection(projection.clone(), input, Some(front));
    if unstamped {
        let df = tdashboard_analysis::dataframe::DataFrame::from_trials(
            &projection.trial_rows,
            &projection.meta.param_names,
            &projection.meta.objective_names,
            &[],
            &[],
            0,
        );
        assert!(df.provenance().is_none());
        view = StudyView::new(Arc::new(df), vec![0; 3]);
    }
    app.current_study = Some(StudyContext {
        meta: crate::io::journal::convert_projection_meta(projection.meta.clone()),
        view,
        pareto_indices: vec![0, 1, 2],
    });
}

#[derive(Default)]
struct Painted {
    computations: usize,
    text: Vec<String>,
    points: Vec<egui::Pos2>,
}

impl Painted {
    fn collect(&mut self, shape: &egui::epaint::Shape) {
        match shape {
            egui::epaint::Shape::Text(text) => self.text.push(text.galley.text().to_string()),
            egui::epaint::Shape::Circle(circle) if circle.fill != egui::Color32::TRANSPARENT => {
                self.points.push(circle.center);
            }
            egui::epaint::Shape::Vec(shapes) => {
                for shape in shapes {
                    self.collect(shape);
                }
            }
            _ => {}
        }
    }

    fn assert_ranges_and_points(&self, offset: f64, x_axis: usize) {
        let scale = if x_axis == 3 { 10.0 } else { 1.0 };
        for label in [
            format!("{:.3}", offset * scale),
            format!("f{x_axis} ({:.3})", (offset + 2.0) * scale),
            format!("f1 ({:.3})", offset + 2.0),
            format!("f2 ({:.3})", offset + 2.0),
        ] {
            assert!(
                self.text.contains(&label),
                "Missing {label}: {:?}",
                self.text
            );
        }
        assert_eq!(self.points.len(), 3);
        for i in 0..3 {
            for j in i + 1..3 {
                assert!(self.points[i].distance(self.points[j]) > 10.0);
            }
        }
    }
}

fn controls(widgets: &WidgetStates) -> serde_json::Value {
    serde_json::json!([&widgets.cluster_scatter_3d, &widgets.pareto_3d,])
}

pub(crate) fn check(
    mut show: impl FnMut(&mut WidgetStates, &mut egui::Ui, &mut AppState) -> usize,
) {
    let mut widgets = WidgetStates::default();
    widgets.cluster_scatter_3d.k_mode = KSelectionMode::Manual;
    widgets.cluster_scatter_3d.k = 2;
    widgets.cluster_scatter_3d.show_infeasible = false;
    widgets.pareto_3d.show_infeasible = false;
    for camera in [
        &mut widgets.cluster_scatter_3d.camera,
        &mut widgets.pareto_3d.camera,
    ] {
        camera.rotate_by_drag(20.0, 15.0);
        camera.zoom = 4.0;
        camera.pan = [12.0, -8.0];
    }
    let original_controls = controls(&widgets);
    let mut app = AppState::default();
    let ctx = egui::Context::default();
    let mut render = |widgets: &mut WidgetStates, app: &mut AppState| {
        let mut painted = Painted::default();
        for _ in 0..2 {
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1000.0, 800.0),
                    )),
                    ..Default::default()
                },
                |ui| painted.computations = show(widgets, ui, app),
            );
            painted.text.clear();
            painted.points.clear();
            for shape in output.shapes {
                painted.collect(&shape.shape);
            }
        }
        painted
    };
    let mut ids = IdentityAllocator::new().unwrap();
    let source = ids.source().unwrap();
    let mut manager = StudyManager::new(StudyKey {
        source,
        stored_id: 1,
    });
    let first = replace(&mut manager, LoadKind::Initial, 0.0);
    install(&mut app, &widgets, first.clone(), false);
    let painted = render(&mut widgets, &mut app);
    painted.assert_ranges_and_points(0.0, 0);
    assert_eq!(painted.computations, 1);
    install(&mut app, &widgets, first, false);
    assert_eq!(render(&mut widgets, &mut app).computations, 1);

    let revised = replace(&mut manager, LoadKind::Refresh, 100.0);
    widgets.invalidate_analysis_input();
    install(&mut app, &widgets, revised, false);
    let painted = render(&mut widgets, &mut app);
    painted.assert_ranges_and_points(100.0, 0);
    assert_eq!(painted.computations, 2);

    // Requesting a reload retains the acquired display until its first accepted batch.
    manager.begin(LoadKind::Reload).unwrap();
    assert_eq!(render(&mut widgets, &mut app).computations, 2);
    let reloaded = replace(&mut manager, LoadKind::Reload, 200.0);
    install(&mut app, &widgets, reloaded, false);
    let painted = render(&mut widgets, &mut app);
    painted.assert_ranges_and_points(200.0, 0);
    assert_eq!(painted.computations, 3);

    // Equal revision counters must not alias another Study or another source.
    for (source, stored_id, offset, expected) in [
        (source, 2, 300.0, 4),
        (source, 3, 350.0, 5),
        (ids.source().unwrap(), 3, 400.0, 6),
    ] {
        let mut other = StudyManager::new(StudyKey { source, stored_id });
        let input = replace(&mut other, LoadKind::Initial, offset);
        widgets.invalidate_analysis_input();
        install(&mut app, &widgets, input, false);
        let painted = render(&mut widgets, &mut app);
        painted.assert_ranges_and_points(offset, 0);
        assert_eq!(painted.computations, expected);
    }
    assert_eq!(controls(&widgets), original_controls);

    widgets.cluster_scatter_3d.x_objective = 3;
    widgets.pareto_3d.x_objective = 3;
    let painted = render(&mut widgets, &mut app);
    painted.assert_ranges_and_points(400.0, 3);
    assert_eq!(painted.computations, 7);
    assert_eq!(render(&mut widgets, &mut app).computations, 7);

    let axis_controls = controls(&widgets);
    for offset in [500.0, 600.0] {
        let input = replace(&mut manager, LoadKind::Refresh, offset);
        install(&mut app, &widgets, input, true);
        let before = render(&mut widgets, &mut app);
        before.assert_ranges_and_points(offset, 3);
        let after = render(&mut widgets, &mut app);
        assert_eq!(after.computations, before.computations + 2);
    }
    assert_eq!(controls(&widgets), axis_controls);
}
