use super::*;
use crate::state::app_state::{StudyContext, StudyMeta, StudyView};
use crate::state::results::ClusterResult;
use crate::theme::colormap::ColorMap;
use crate::ui::widgets::common::cluster_table::ClusterTable;
use egui::{epaint::Shape, Event, PointerButton, Pos2, RawInput, Rect};
use std::sync::Arc;

fn journal() -> Vec<u8> {
    let mut lines = vec![
        serde_json::json!({"op_code":0,"study_name":"A","directions":[1]}),
        serde_json::json!({"op_code":0,"study_name":"B","directions":[1]}),
    ];
    // Both studies start at zero; failed/pruned trials leave gaps in COMPLETE rows.
    for (id, (study, state)) in [(0, 1), (1, 1), (0, 3), (1, 2), (0, 1), (1, 1)]
        .into_iter()
        .enumerate()
    {
        let artifact = serde_json::json!({"artifact_id":format!("file-{id}"),
            "filename":format!("study-{study}-trial-{id}.png"),"mimetype":"image/png"});
        lines.push(
            serde_json::json!({"op_code":4,"study_id":study,"state":state,
            "values":[id as f64],"distributions":{},
            "system_attrs":{format!("artifacts:file-{id}"):artifact.to_string()}}),
        );
    }
    lines
        .into_iter()
        .map(|line| format!("{line}\n"))
        .collect::<String>()
        .into_bytes()
}

fn study(data: &[u8], id: u32) -> StudyContext {
    let (_, df, _) = tunny_core::journal_parser::parse_single_study(data, id).unwrap();
    StudyContext {
        meta: StudyMeta {
            study_id: id,
            name: format!("Study {id}"),
            directions: vec![],
            completed_trials: df.row_count(),
            param_names: vec![],
            objective_names: vec![],
            param_bounds: HashMap::new(),
        },
        view: StudyView::new(Arc::new(df), vec![]),
        pareto_indices: vec![],
    }
}

fn frame(
    ctx: &egui::Context,
    events: Vec<Event>,
    mut draw: impl FnMut(&mut egui::Ui),
) -> Vec<(String, Rect)> {
    let output = ctx.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 800.0))),
            events,
            ..Default::default()
        },
        |ui| draw(ui),
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

fn click(ctx: &egui::Context, pos: Pos2, mut draw: impl FnMut(&mut egui::Ui)) {
    for pressed in [true, false] {
        frame(
            ctx,
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ],
            &mut draw,
        );
    }
}

fn label_rect(labels: &[(String, Rect)], label: &str) -> Rect {
    labels
        .iter()
        .find(|(text, _)| text == label)
        .unwrap_or_else(|| panic!("Missing {label}: {labels:?}"))
        .1
}

#[test]
fn journal_cards_display_local_numbers_and_keep_global_links_after_study_switch() {
    let data = journal();
    let root = tempfile::tempdir().unwrap();
    let journal_path = root.path().join("study.log");
    std::fs::write(&journal_path, &data).unwrap();
    let metadata = crate::io::artifacts::parse_artifact_metadata(&journal_path);
    for entries in metadata.values() {
        for entry in entries {
            std::fs::write(root.path().join(&entry.artifact_id), b"test artifact").unwrap();
        }
    }
    let mut app = AppState {
        artifact_map: crate::io::artifacts::resolve_from_metadata(root.path(), &metadata),
        ..Default::default()
    };
    for (study_id, ids) in [(0, [0, 4]), (1, [1, 5]), (0, [0, 4])] {
        app.current_study = Some(study(&data, study_id));
        let visible = super::super::sections::restrict_to_current_study(
            super::super::sections::artifact_trials_with_index(&app.artifact_map, 0),
            &app,
        );
        assert_eq!(visible, ids);
        let cards: Vec<_> = visible
            .iter()
            .map(|&id| (id, "Cluster 1".to_string(), &app.artifact_map[&id][0]))
            .collect();
        let ctx = egui::Context::default();
        let mut action = (None, None);
        let mut draw = |ui: &mut egui::Ui| {
            action = render_card_grid(ui, &app, 900.0, 140.0, &cards, &HashMap::new());
        };
        frame(&ctx, vec![], &mut draw);
        let labels = frame(&ctx, vec![], &mut draw);
        label_rect(&labels, "Trial 0 · Cluster 1");
        let title = label_rect(&labels, "Trial 2 · Cluster 1");
        label_rect(&labels, &format!("study-{study_id}-trial-{}.png", ids[1]));
        assert_eq!(
            cards[1].2.path,
            root.path().join(format!("file-{}", ids[1]))
        );
        click(&ctx, title.center(), &mut draw);
        assert_eq!(action.0, Some(ids[1]));
        // The image occupies the 140 px immediately above the title.
        click(&ctx, title.center() - egui::vec2(0.0, 70.0), |ui| {
            action = render_card_grid(ui, &app, 900.0, 140.0, &cards, &HashMap::new());
        });
        let target = action.1.unwrap();
        assert_eq!((target.trial_id, target.row_index), (ids[1], 1));
        assert_eq!(target.context, vec![("Group".into(), "Cluster 1".into())]);
        let mut modal = crate::ui::widgets::trial_detail_modal::TrialDetailModal::new();
        modal.open(target);
        let modal_ctx = egui::Context::default();
        let view = &app.current_study.as_ref().unwrap().view;
        frame(&modal_ctx, vec![], |ui| {
            modal.show(ui, view, &[], &[], &app.artifact_map)
        });
        let labels = frame(&modal_ctx, vec![], |ui| {
            modal.show(ui, view, &[], &[], &app.artifact_map)
        });
        label_rect(&labels, "Trial 2");
        let mut table = crate::ui::widgets::common::trial_table::TrialTable::default();
        let table_ctx = egui::Context::default();
        app.selected_indices = vec![ids[1]];
        frame(&table_ctx, vec![], |ui| table.show(ui, &mut app));
        let labels = frame(&table_ctx, vec![], |ui| table.show(ui, &mut app));
        label_rect(&labels, "Trial Number");
        label_rect(&labels, "2");
        app.set_highlight(ids[1]);
        app.toggle_pinned_trial(ids[1]).unwrap();
        assert_eq!(app.highlighted_trial, Some(ids[1]));
        assert!(app.pinned_trials.contains(&ids[1]));
        app.toggle_pinned_trial(ids[1]).unwrap();
    }
}

#[test]
fn cluster_render_keeps_numbers_through_filtering_and_global_click_and_pin_targets() {
    let mut app = AppState {
        current_study: Some(study(&journal(), 1)),
        ..Default::default()
    };
    let mut table = ClusterTable::default();
    app.cluster_cache.insert(
        table.cache_key(),
        ClusterResult {
            labels: vec![-1, 0],
            n_clusters: 1,
        },
    );
    let ctx = egui::Context::default();
    let mut draw = |ui: &mut egui::Ui| table.show(ui, &mut app, &ColorMap::viridis());
    frame(&ctx, vec![], &mut draw);
    let labels = frame(&ctx, vec![], &mut draw);
    label_rect(&labels, "Trial Number");
    let number = labels.iter().rev().find(|(text, _)| text == "2").unwrap().1;
    let pin = label_rect(&labels, "·");
    click(&ctx, number.center(), &mut draw);
    assert_eq!(app.highlighted_trial, Some(5));
    click(&ctx, pin.center(), |ui| {
        table.show(ui, &mut app, &ColorMap::viridis())
    });
    assert_eq!(app.pinned_trials, vec![5]);
}

#[test]
fn trial_table_header_is_trial_number_in_every_mode() {
    use crate::state::results::{McdmResult, TopsisResult};
    use crate::ui::widgets::common::trial_table::{TrialTable, TrialTableMode};

    let mut study = study(&journal(), 1);
    study.meta.objective_names = study.view.objective_names().to_vec();
    let mut app = AppState {
        current_study: Some(study),
        ..Default::default()
    };
    let mut table = TrialTable::default();
    table.mcdm.controls.weights = vec![1.0];
    app.cluster_cache.insert(
        table.cluster.cache_key(),
        ClusterResult {
            labels: vec![0, 0],
            n_clusters: 1,
        },
    );
    app.mcdm_cache.insert(
        table.mcdm.controls.cache_key().unwrap(),
        McdmResult::Topsis(TopsisResult {
            scores: vec![0.25, 0.75],
            ranked_indices: vec![1, 0],
            duration_ms: 0.0,
        }),
    );
    for mode in [
        TrialTableMode::All,
        TrialTableMode::Cluster,
        TrialTableMode::Mcdm,
    ] {
        table.mode = mode;
        let ctx = egui::Context::default();
        frame(&ctx, vec![], |ui| table.show(ui, &mut app));
        let labels = frame(&ctx, vec![], |ui| table.show(ui, &mut app));
        let header = label_rect(&labels, "Trial Number");
        assert!(
            header.width() <= 110.0,
            "Header exceeds initial column width in {mode:?}"
        );
        assert!(!labels
            .iter()
            .any(|(text, _)| text == "Trial" || text == "Trial ID"));
    }
}

#[test]
fn card_and_anchor_missing_number_use_original_row_not_global_id() {
    let mut study = study(&journal(), 1);
    // Simulate an unavailable number without changing the storage API.
    study.view.df = Arc::new(tunny_core::dataframe::DataFrame::empty());
    assert_eq!(
        crate::ui::widgets::surrogate::anchor::center_label(
            crate::ui::widgets::surrogate::anchor::CenterChoice::Pinned(5),
            &study.view
        ),
        "Trial #1"
    );
    let app = AppState {
        current_study: Some(study),
        ..Default::default()
    };
    let entry = ArtifactEntry {
        path: "file-5".into(),
        filename: "result.csv".into(),
        mimetype: "text/csv".into(),
    };
    let ctx = egui::Context::default();
    let labels = frame(&ctx, vec![], |ui| {
        render_card_grid(
            ui,
            &app,
            900.0,
            140.0,
            &[(5, String::new(), &entry)],
            &HashMap::new(),
        );
    });
    label_rect(&labels, "Trial 1");
}

#[test]
fn single_study_card_number_is_unchanged() {
    let data = b"{\"op_code\":0,\"study_name\":\"Only\",\"directions\":[1]}\n{\"op_code\":4,\"study_id\":0,\"state\":1,\"values\":[1],\"distributions\":{}}\n";
    let app = AppState {
        current_study: Some(study(data, 0)),
        ..Default::default()
    };
    let entry = ArtifactEntry {
        path: "file-0".into(),
        filename: "result.csv".into(),
        mimetype: "text/csv".into(),
    };
    let ctx = egui::Context::default();
    let labels = frame(&ctx, vec![], |ui| {
        render_card_grid(
            ui,
            &app,
            900.0,
            140.0,
            &[(0, String::new(), &entry)],
            &HashMap::new(),
        );
    });
    label_rect(&labels, "Trial 0");
}
