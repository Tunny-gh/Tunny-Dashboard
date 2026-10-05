use std::{collections::HashMap, sync::Arc};

use super::rank_plot::{
    cmap_fingerprint, compute_rank_percentiles, draw_rank_legend, rank_colormap_input,
};
use super::scatter_3d::{
    draw_3d_axes, draw_3d_grid, draw_depth_sorted_points, project_value_3d, setup_3d_canvas,
    show_hover_and_click_detail, show_objective_combo, val_range, ArcballCamera, DepthPoint,
};
use crate::io::artifacts::ArtifactEntry;
use crate::state::types::{Direction, StudyView};
use crate::theme::colormap::ColorMap;
use crate::ui::widgets::scatter_matrix::{downsample_indices_to_cap, MAX_SCATTER_POINTS};
use crate::ui::widgets::trial_detail_modal::{axis_row, fmt_opt, TrialDetailModal};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RankPlot3D {
    pub x_param_idx: usize,
    pub y_param_idx: usize,
    pub z_param_idx: usize,
    pub obj_idx: usize,
    pub camera: ArcballCamera,
    #[serde(skip)]
    cache: Option<Rank3DCache>,
    #[serde(skip)]
    detail_modal: TrialDetailModal,
}

impl Default for RankPlot3D {
    fn default() -> Self {
        Self {
            x_param_idx: 0,
            y_param_idx: 1,
            z_param_idx: 2,
            obj_idx: 0,
            camera: ArcballCamera::isometric_default(),
            cache: None,
            detail_modal: TrialDetailModal::new(),
        }
    }
}

struct Rank3DPoint {
    trial_id: u32,
    row: usize,
    xyz: [f64; 3],
    color: egui::Color32,
}

struct Rank3DCache {
    // Retain the source so its address cannot be reused by another study.
    source: Arc<tunny_core::dataframe::DataFrame>,
    objective: (usize, bool),
    ranks: Vec<f64>,
    point_key: ([usize; 3], u64),
    ranges: [(f64, f64); 3],
    points: Vec<Rank3DPoint>,
}

/// Repair persisted selections after a study change. Earlier axes take precedence.
fn distinct_axes(mut axes: [usize; 3], count: usize) -> [usize; 3] {
    if count < 3 {
        return [0, 1, 2];
    }
    for i in 0..3 {
        axes[i] = axes[i].min(count - 1);
        if axes[..i].contains(&axes[i]) {
            axes[i] = (0..count).find(|v| !axes[..i].contains(v)).unwrap();
        }
    }
    axes
}

fn collect_points(
    view: &StudyView,
    names: [&str; 3],
    ranks: &[f64],
    cmap: &ColorMap,
) -> Vec<Rank3DPoint> {
    let columns = names.map(|name| view.numeric_column(name));
    let usable: Vec<u32> = (0..view.row_count())
        .filter(|&row| {
            columns
                .iter()
                .all(|col| col.and_then(|c| c.get(row)).is_some_and(|v| v.is_finite()))
        })
        .map(|row| row as u32)
        .collect();
    downsample_indices_to_cap(&usable, MAX_SCATTER_POINTS)
        .into_iter()
        .map(|idx| {
            let row = idx as usize;
            Rank3DPoint {
                trial_id: view.trial_ids.get(row).copied().unwrap_or(idx),
                row,
                xyz: columns.map(|col| col.unwrap()[row]),
                color: cmap
                    .interpolate(rank_colormap_input(ranks.get(row).copied().unwrap_or(1.0))),
            }
        })
        .collect()
}

impl RankPlot3D {
    fn axes(&self) -> [usize; 3] {
        [self.x_param_idx, self.y_param_idx, self.z_param_idx]
    }

    fn clamp(&mut self, params: usize, objectives: usize) {
        [self.x_param_idx, self.y_param_idx, self.z_param_idx] = distinct_axes(self.axes(), params);
        self.obj_idx = self.obj_idx.min(objectives.saturating_sub(1));
    }

    fn update_cache(
        &mut self,
        view: &StudyView,
        params: &[String],
        objectives: &[String],
        minimize: bool,
        cmap: &ColorMap,
    ) {
        let objective = (self.obj_idx, minimize);
        let point_key = (self.axes(), cmap_fingerprint(cmap));
        let ranks_valid = self
            .cache
            .as_ref()
            .is_some_and(|c| Arc::ptr_eq(&c.source, &view.df) && c.objective == objective);
        if !ranks_valid {
            let ranks = compute_rank_percentiles(
                view.numeric_column(&objectives[self.obj_idx])
                    .unwrap_or_default(),
                minimize,
            );
            self.cache = Some(Rank3DCache {
                source: Arc::clone(&view.df),
                objective,
                ranks,
                point_key,
                ranges: [(-1.0, 1.0); 3],
                points: Vec::new(),
            });
        }
        let names = self.axes().map(|idx| params[idx].as_str());
        let cache = self.cache.as_mut().unwrap();
        if !ranks_valid || cache.point_key != point_key {
            cache.points = collect_points(view, names, &cache.ranks, cmap);
            cache.ranges =
                names.map(|name| val_range(view.numeric_column(name).unwrap_or_default()));
            cache.point_key = point_key;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        directions: &[Direction],
        cmap: &ColorMap,
        artifact_map: &HashMap<u32, Vec<ArtifactEntry>>,
    ) {
        self.clamp(param_names.len(), obj_names.len());
        if param_names.len() < 3 {
            ui.label("Rank Plot 3D requires at least 3 parameters.");
            return;
        }
        if obj_names.is_empty() {
            ui.label("No objectives.");
            return;
        }
        if view.row_count() == 0 {
            ui.label("No trial data.");
            return;
        }
        ui.horizontal(|ui| {
            let mut axes = self.axes();
            for axis in 0..3 {
                ui.label(["X:", "Y:", "Z:"][axis]);
                egui::ComboBox::from_id_salt(("rank3d_param", axis))
                    .selected_text(&param_names[axes[axis]])
                    .show_ui(ui, |ui| {
                        for (idx, name) in param_names.iter().enumerate() {
                            if !axes
                                .iter()
                                .enumerate()
                                .any(|(other, &selected)| other != axis && selected == idx)
                            {
                                ui.selectable_value(&mut axes[axis], idx, name);
                            }
                        }
                    });
            }
            [self.x_param_idx, self.y_param_idx, self.z_param_idx] = axes;
            if obj_names.len() > 1 {
                show_objective_combo(ui, "Objective:", "rank3d_obj", &mut self.obj_idx, obj_names);
            }
        });
        let minimize = matches!(directions.get(self.obj_idx), Some(Direction::Minimize));
        self.update_cache(view, param_names, obj_names, minimize, cmap);
        let cache = self.cache.as_ref().unwrap();
        let names = self.axes().map(|idx| param_names[idx].as_str());
        let objective = &obj_names[self.obj_idx];
        if cache.points.is_empty() {
            ui.label("No finite points to plot.");
            return;
        }
        let (painter, canvas_rect, project, click, hover) = setup_3d_canvas(ui, &mut self.camera);
        draw_3d_grid(&painter, &project);
        draw_3d_axes(&painter, &project, names, cache.ranges);
        // Only the capped sample is projected/sorted each frame as the camera moves.
        let mut points = Vec::with_capacity(cache.points.len());
        let mut candidates = Vec::with_capacity(cache.points.len());
        for point in &cache.points {
            let (pos, depth) = project_value_3d(&project, point.xyz, cache.ranges);
            points.push(DepthPoint {
                pos,
                depth,
                color: point.color,
                radius: 3.0,
            });
            candidates.push((point.trial_id, point.row, pos));
        }
        draw_depth_sorted_points(&painter, &mut points, None);
        let rank_row = |row| {
            (
                "Rank Percentile".to_string(),
                fmt_opt(cache.ranks.get(row).copied()),
            )
        };
        show_hover_and_click_detail(
            ui,
            view,
            &candidates,
            hover,
            click,
            "rank3d_hover",
            &mut self.detail_modal,
            |row| {
                let mut rows: Vec<_> = names
                    .iter()
                    .map(|name| axis_row(name, view.numeric_column(name), row))
                    .collect();
                rows.push(axis_row(objective, view.numeric_column(objective), row));
                rows.push(rank_row(row));
                rows
            },
            |row| vec![rank_row(row)],
        );
        // Paint the legend inside the canvas without reserving layout space.
        let legend_rect = egui::Rect::from_min_size(
            egui::pos2(canvas_rect.right() - 24.0, canvas_rect.top() + 16.0),
            egui::vec2(14.0, (canvas_rect.height() - 32.0).clamp(0.0, 160.0)),
        );
        draw_rank_legend(ui, legend_rect, cmap);
        self.detail_modal
            .show(ui, view, param_names, obj_names, artifact_map);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::types::{StudyContext, StudyMeta, TrialRow};

    fn study() -> StudyContext {
        let values = [
            1.0,
            1.0,
            2.0,
            3.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ];
        let rows = values
            .iter()
            .enumerate()
            .map(|(i, &value)| TrialRow {
                trial_id: 100 + i as u32,
                trial_number: i as u32,
                params: HashMap::from([
                    ("x".into(), if i == 0 { f64::NAN } else { i as f64 }),
                    ("y".into(), if i == 2 { f64::INFINITY } else { i as f64 }),
                    (
                        "z".into(),
                        if i == 3 { f64::NEG_INFINITY } else { i as f64 },
                    ),
                    ("w".into(), i as f64),
                ]),
                objectives: vec![value, -value],
                ..Default::default()
            })
            .collect();
        StudyContext::from_rows_for_test(
            StudyMeta {
                study_id: 1,
                name: "rank".into(),
                directions: vec![Direction::Minimize, Direction::Maximize],
                completed_trials: 7,
                param_names: ["x", "y", "z", "w"].map(String::from).to_vec(),
                objective_names: vec!["f".into(), "g".into()],
                param_bounds: Default::default(),
            },
            rows,
        )
    }

    #[test]
    fn shared_ranks_filtering_colors_and_cache_invalidation() {
        let study = study();
        let mut chart = RankPlot3D::default();
        let cmap = ColorMap::viridis();
        let update = |chart: &mut RankPlot3D, view: &StudyView, minimize, cmap: &ColorMap| {
            chart.update_cache(
                view,
                &study.meta.param_names,
                &study.meta.objective_names,
                minimize,
                cmap,
            );
        };
        for (minimize, expected) in [
            (
                true,
                vec![1.0 / 6.0, 1.0 / 6.0, 2.0 / 3.0, 1.0, 1.0, 1.0, 1.0],
            ),
            (
                false,
                vec![5.0 / 6.0, 5.0 / 6.0, 1.0 / 3.0, 0.0, 1.0, 1.0, 1.0],
            ),
        ] {
            update(&mut chart, &study.view, minimize, &cmap);
            let cache = chart.cache.as_ref().unwrap();
            assert_eq!(cache.ranks, expected);
            assert_eq!(
                cache.ranks,
                compute_rank_percentiles(study.view.numeric_column("f").unwrap(), minimize)
            );
            assert_eq!(
                cache.points.iter().map(|p| p.row).collect::<Vec<_>>(),
                [1, 4, 5, 6]
            );
            assert_eq!(cache.ranges, [(1.0, 6.0), (0.0, 6.0), (0.0, 6.0)]);
            for p in &cache.points {
                assert_eq!(p.trial_id, 100 + p.row as u32);
                assert_eq!(p.xyz, [p.row as f64; 3]);
                assert_eq!(
                    p.color,
                    cmap.interpolate(rank_colormap_input(expected[p.row]))
                );
            }
        }
        let ranks_ptr = chart.cache.as_ref().unwrap().ranks.as_ptr();
        let points_ptr = chart.cache.as_ref().unwrap().points.as_ptr();
        update(&mut chart, &study.view, false, &cmap);
        assert_eq!(chart.cache.as_ref().unwrap().points.as_ptr(), points_ptr);
        chart.z_param_idx = 3;
        update(&mut chart, &study.view, false, &cmap);
        assert_eq!(chart.cache.as_ref().unwrap().ranks.as_ptr(), ranks_ptr);
        assert_eq!(
            chart
                .cache
                .as_ref()
                .unwrap()
                .points
                .iter()
                .map(|p| p.row)
                .collect::<Vec<_>>(),
            [1, 3, 4, 5, 6]
        );
        let plasma = ColorMap::plasma();
        update(&mut chart, &study.view, false, &plasma);
        assert_eq!(chart.cache.as_ref().unwrap().ranks.as_ptr(), ranks_ptr);
        assert_eq!(
            chart.cache.as_ref().unwrap().points[1].color,
            plasma.interpolate(1.0)
        );
        chart.obj_idx = 1;
        update(&mut chart, &study.view, false, &plasma);
        assert_eq!(chart.cache.as_ref().unwrap().objective, (1, false));
        assert_eq!(chart.cache.as_ref().unwrap().ranks[3], 1.0);
        let mut replacement = study.view.clone();
        replacement.df = Arc::new((*replacement.df).clone());
        update(&mut chart, &replacement, false, &plasma);
        assert!(Arc::ptr_eq(
            &chart.cache.as_ref().unwrap().source,
            &replacement.df
        ));
        assert!(!Arc::ptr_eq(
            &chart.cache.as_ref().unwrap().source,
            &study.view.df
        ));
    }

    #[test]
    fn canvas_fills_remaining_area_and_legend_stays_inside_on_resize() {
        let study = study();
        let ctx = egui::Context::default();
        let mut chart = RankPlot3D::default();
        let cmap = ColorMap::viridis();
        let artifacts = HashMap::new();
        // Reuse the context and chart across both growing and shrinking frames.
        for size in [
            egui::vec2(900.0, 650.0),
            egui::vec2(1200.0, 900.0),
            egui::vec2(600.0, 300.0),
            egui::vec2(900.0, 650.0),
        ] {
            let mut available = egui::Rect::NOTHING;
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    ..Default::default()
                },
                |ui| {
                    available = ui.available_rect_before_wrap();
                    chart.show(
                        ui,
                        &study.view,
                        &study.meta.param_names,
                        &study.meta.objective_names,
                        &study.meta.directions,
                        &cmap,
                        &artifacts,
                    );
                },
            );
            let canvas = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Rect(r) if r.fill == crate::theme::chart_colors::COLOR_3D_BG() => {
                        Some(r.rect)
                    }
                    _ => None,
                })
                .expect("3D canvas background rendered");
            assert!(
                canvas.height() > available.height() - 60.0,
                "full-height canvas at {size:?}"
            );
            assert!((canvas.left() - available.left()).abs() < 1.0);
            assert!((canvas.right() - available.right()).abs() < 1.0);
            assert!((canvas.bottom() - available.bottom()).abs() < 1.0);
            assert!(
                canvas.top() > available.top(),
                "selectors precede the canvas"
            );
            let bar = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Rect(r) if r.rect.width() == 14.0 && r.stroke.width == 0.5 => {
                        Some(r.rect)
                    }
                    _ => None,
                })
                .expect("vertical rank legend rendered");
            assert!(canvas.contains_rect(bar));
            assert_eq!(bar.height(), 160.0);
            assert!((canvas.right() - bar.right() - 10.0).abs() < 1.0);
            for label in ["Best", "Worst"] {
                let (bounds, clip) = output
                    .shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::Text(t) if t.galley.text() == label => Some((
                            egui::Rect::from_min_size(t.pos, t.galley.size()),
                            s.clip_rect,
                        )),
                        _ => None,
                    })
                    .expect("rank label rendered");
                assert!(canvas.contains_rect(bounds), "{label} stays inside canvas");
                assert!(clip.contains_rect(bounds), "{label} is not clipped");
            }
        }
    }

    #[test]
    fn rendered_points_click_opens_detail_and_study_changes_clamp() {
        let study = study();
        let ctx = egui::Context::default();
        let mut state = crate::state::app_state::AppState {
            current_study: Some(study),
            ..Default::default()
        };
        let mut widgets = crate::ui::widget_states::WidgetStates::default();
        let render = |events,
                      widgets: &mut crate::ui::widget_states::WidgetStates,
                      state: &mut crate::state::app_state::AppState| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900.0, 650.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    crate::ui::chart::render_chart::render_chart(
                        ui,
                        state,
                        widgets,
                        &crate::state::layout_state::ChartId::RankPlot3D,
                    )
                },
            )
        };
        let output = render(vec![], &mut widgets, &mut state);
        let pos = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Circle(c) if c.radius == 3.0 => Some(c.center),
                _ => None,
            })
            .expect("3D points rendered through chart dispatch");
        render(
            vec![egui::Event::PointerMoved(pos)],
            &mut widgets,
            &mut state,
        );
        let hover = render(vec![], &mut widgets, &mut state);
        for label in ["x", "y", "z", "f", "Rank Percentile"] {
            assert!(
                hover
                    .shapes
                    .iter()
                    .any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == label)),
                "hover must show {label}"
            );
        }
        render(
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
            &mut widgets,
            &mut state,
        );
        render(
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
            &mut widgets,
            &mut state,
        );
        assert!(widgets.rank_plot_3d.detail_modal.is_open());
        let detail = render(vec![], &mut widgets, &mut state);
        assert!(
            detail.shapes.iter().any(
                |s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "Rank Percentile")
            ),
            "Trial Detail must display the chart-specific rank context"
        );
        widgets.rank_plot_3d.x_param_idx = 8;
        widgets.rank_plot_3d.y_param_idx = 8;
        widgets.rank_plot_3d.z_param_idx = 8;
        state.current_study.as_mut().unwrap().meta.param_names.pop();
        render(vec![], &mut widgets, &mut state);
        assert_eq!(widgets.rank_plot_3d.axes(), [2, 0, 1]);
    }

    #[test]
    fn selectors_only_offer_distinct_axes_and_rebuild_points_immediately() {
        let study = study();
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut chart = RankPlot3D::default();
        let cmap = ColorMap::viridis();
        let artifacts = HashMap::new();
        let run = |chart: &mut RankPlot3D, events| {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900.0, 650.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    chart.show(
                        ui,
                        &study.view,
                        &study.meta.param_names,
                        &study.meta.objective_names,
                        &study.meta.directions,
                        &cmap,
                        &artifacts,
                    )
                },
            )
        };
        let node = |output: &egui::FullOutput, role, text: &str| {
            output
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .find_map(|(id, node)| {
                    (node.role() == role
                        && (node.label() == Some(text) || node.value() == Some(text)))
                    .then_some(*id)
                })
        };
        let click = |id| {
            vec![egui::Event::AccessKitActionRequest(
                egui::accesskit::ActionRequest {
                    action: egui::accesskit::Action::Click,
                    target_tree: egui::accesskit::TreeId::ROOT,
                    target_node: id,
                    data: None,
                },
            )]
        };
        let output = run(&mut chart, vec![]);
        let combo = node(&output, egui::accesskit::Role::ComboBox, "x").unwrap();
        let menu = run(&mut chart, click(combo));
        assert!(node(&menu, egui::accesskit::Role::Button, "y").is_none());
        assert!(node(&menu, egui::accesskit::Role::Button, "z").is_none());
        let item = node(&menu, egui::accesskit::Role::Button, "w").unwrap();
        run(&mut chart, click(item));
        assert_eq!(chart.axes(), [3, 1, 2]);
        assert_eq!(chart.cache.as_ref().unwrap().point_key.0, [3, 1, 2]);
        assert_eq!(chart.cache.as_ref().unwrap().points[0].row, 0);
    }

    #[test]
    fn sampling_is_capped_and_does_not_change_full_population_ranks() {
        let mut study = study();
        let count = MAX_SCATTER_POINTS * 2;
        study.set_rows_for_test(
            (0..count)
                .map(|i| TrialRow {
                    trial_id: i as u32,
                    trial_number: i as u32,
                    params: ["x", "y", "z", "w"]
                        .map(|name| (name.into(), i as f64))
                        .into_iter()
                        .collect(),
                    objectives: vec![i as f64, -(i as f64)],
                    ..Default::default()
                })
                .collect(),
        );
        let mut chart = RankPlot3D::default();
        chart.update_cache(
            &study.view,
            &study.meta.param_names,
            &study.meta.objective_names,
            true,
            &ColorMap::viridis(),
        );
        let cache = chart.cache.as_ref().unwrap();
        assert_eq!(cache.ranks.len(), count);
        assert_eq!(cache.points.len(), MAX_SCATTER_POINTS);
        assert_eq!(cache.ranks[0], 0.0);
        assert_eq!(cache.ranks[count - 1], 1.0);
        for point in &cache.points {
            assert_eq!(
                cache.ranks[point.row],
                point.row as f64 / (count - 1) as f64
            );
        }
        let state = crate::state::app_state::AppState {
            current_study: Some(study),
            ..Default::default()
        };
        let csv = crate::io::csv_export::build_chart_csv(
            &crate::state::layout_state::ChartId::RankPlot3D,
            &state,
            &crate::ui::widget_states::WidgetStates::default(),
        )
        .unwrap();
        assert_eq!(csv.lines().count(), count + 1);
        assert!(collect_points(
            &state.current_study.as_ref().unwrap().view,
            ["absent", "y", "z"],
            &cache.ranks,
            &ColorMap::viridis()
        )
        .is_empty());
    }

    #[test]
    fn defaults_clamping_and_serialization() {
        let mut chart = RankPlot3D::default();
        assert_eq!(chart.axes(), [0, 1, 2]);
        chart.x_param_idx = 9;
        chart.y_param_idx = 9;
        chart.z_param_idx = 9;
        chart.obj_idx = 9;
        chart.clamp(3, 1);
        assert_eq!(chart.axes(), [2, 0, 1]);
        assert_eq!(chart.obj_idx, 0);
        chart.clamp(4, 2);
        assert_eq!(chart.axes(), [2, 0, 1]);
        let restored: RankPlot3D =
            serde_json::from_str(&serde_json::to_string(&chart).unwrap()).unwrap();
        assert_eq!(restored.axes(), chart.axes());
        assert!(restored.cache.is_none());
        for x in 0..6 {
            for y in 0..6 {
                for z in 0..6 {
                    let axes = distinct_axes([x, y, z], 3);
                    assert!(axes.iter().all(|&i| i < 3));
                    assert!(axes[0] != axes[1] && axes[0] != axes[2] && axes[1] != axes[2]);
                }
            }
        }
    }
}
