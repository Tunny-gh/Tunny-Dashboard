use std::collections::BTreeMap;
use std::sync::Arc;

use crate::state::types::StudyView;
use serde_json::Value;
use tunny_core::dataframe::DataFrame;
use tunny_core::statistics::{compute_boxplot, compute_violin, BoxPlotStats, ViolinCurve};

pub const GRID_POINTS: usize = 128;
pub type ViolinData = (Vec<(String, ViolinCurve)>, Vec<(String, &'static str)>);

/// Names alone cannot distinguish an objective from a same-named parameter.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DistributionValue {
    Objective(String),
    Parameter(String),
}

impl DistributionValue {
    pub fn name(&self) -> &str {
        match self {
            Self::Objective(n) | Self::Parameter(n) => n,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Objective(n) => format!("Objective: {n}"),
            Self::Parameter(n) => format!("Parameter: {n}"),
        }
    }

    fn column<'a>(&self, view: &'a StudyView) -> Option<&'a [f64]> {
        match self {
            Self::Objective(n) => view.df.objective_column(n),
            Self::Parameter(n) => view.df.numeric_parameter_column(n),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct DistributionSelection {
    pub value: Option<DistributionValue>,
    pub group_by: Option<String>,
    pub normalize: bool,
}

impl DistributionSelection {
    pub fn candidates(view: &StudyView) -> (Vec<DistributionValue>, Vec<String>) {
        let values = view
            .objective_names()
            .iter()
            .filter(|n| view.df.objective_column(n).is_some())
            .cloned()
            .map(DistributionValue::Objective)
            .chain(
                view.param_names()
                    .iter()
                    .filter(|n| {
                        view.df.numeric_parameter_column(n).is_some()
                            && view.df.category_values(n).is_none()
                    })
                    .cloned()
                    .map(DistributionValue::Parameter),
            )
            .collect();
        let groups = view
            .param_names()
            .iter()
            .filter(|n| {
                view.df.category_values(n).is_some()
                    || (view.df.is_discrete_parameter(n)
                        && view.df.numeric_parameter_column(n).is_some())
            })
            .cloned()
            .collect();
        (values, groups)
    }

    pub fn validate(&mut self, view: &StudyView) {
        let (values, groups) = Self::candidates(view);
        if !self.value.as_ref().is_some_and(|v| values.contains(v)) {
            self.value = values.first().cloned();
        }
        if !self.group_by.as_ref().is_some_and(|g| groups.contains(g)) {
            self.group_by = None;
        }
    }

    pub fn controls(&mut self, ui: &mut egui::Ui, view: &StudyView) {
        self.validate(view);
        let (values, groups) = Self::candidates(view);
        ui.horizontal_wrapped(|ui| {
            ui.label("Value:");
            egui::ComboBox::from_id_salt("distribution_value")
                .selected_text(
                    self.value
                        .as_ref()
                        .map(DistributionValue::label)
                        .unwrap_or_else(|| "None".into()),
                )
                .show_ui(ui, |ui| {
                    for value in values {
                        let label = value.label();
                        ui.selectable_value(&mut self.value, Some(value), label);
                    }
                });
            ui.label("Group by:");
            egui::ComboBox::from_id_salt("distribution_group")
                .selected_text(
                    self.group_by
                        .as_ref()
                        .map(|n| format!("Parameter: {n}"))
                        .unwrap_or_else(|| "None".into()),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.group_by, None, "None");
                    for group in groups {
                        let label = format!("Parameter: {group}");
                        ui.selectable_value(&mut self.group_by, Some(group), label);
                    }
                });
            ui.toggle_value(&mut self.normalize, "Normalize [0,1]")
                .on_hover_text(
                    "Min-max normalize the entire finite Value column before splitting into groups",
                );
        });
    }

    /// Used identically by rendering and CSV, including stale-selection fallback.
    pub fn prepare(&self, view: &StudyView) -> PreparedDistribution {
        let mut selection = self.clone();
        selection.validate(view);
        let mut groups = Vec::new();
        let mut identities = BTreeMap::new();
        if let Some(raw) = selection.value.as_ref().and_then(|v| v.column(view)) {
            let values = if selection.normalize {
                normalize_minmax(raw)
            } else {
                raw.to_vec()
            };
            match selection.group_by.as_deref() {
                None => {
                    let label = selection.value.as_ref().unwrap().name().to_string();
                    identities.insert(
                        label.clone(),
                        format!("Value: {}", selection.value.as_ref().unwrap().label()),
                    );
                    groups.push((
                        label,
                        values.into_iter().filter(|v| v.is_finite()).collect(),
                    ));
                }
                Some(name) => {
                    if let Some(categories) = view.df.category_values(name) {
                        // JSON serialization retains the original scalar type as identity.
                        let mut map: BTreeMap<String, (Value, Vec<f64>)> = BTreeMap::new();
                        for (category, value) in categories.iter().zip(&values) {
                            if let Some(category) = category {
                                let entry = map
                                    .entry(category.to_string())
                                    .or_insert_with(|| (category.clone(), Vec::new()));
                                if value.is_finite() {
                                    entry.1.push(*value);
                                }
                            }
                        }
                        let labels = category_labels(map.values().map(|(v, _)| v));
                        groups = map
                            .into_values()
                            .zip(labels)
                            .map(|((category, v), label)| {
                                identities.insert(label.clone(), format!("Category: {category}"));
                                (label, v)
                            })
                            .collect();
                        groups.sort_by(|a, b| a.0.cmp(&b.0));
                    } else if let Some(keys) = view.df.numeric_parameter_column(name) {
                        let mut pairs: Vec<(f64, f64)> = keys
                            .iter()
                            .copied()
                            .zip(values)
                            .filter(|(k, _)| k.is_finite())
                            .collect();
                        pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
                        let mut numeric: Vec<(f64, Vec<f64>)> = Vec::new();
                        for (key, value) in pairs {
                            if numeric.last().is_none_or(|(k, _)| *k != key) {
                                numeric.push((key, Vec::new()));
                            }
                            if value.is_finite() {
                                numeric.last_mut().unwrap().1.push(value);
                            }
                        }
                        groups = numeric
                            .into_iter()
                            .map(|(k, v)| {
                                let label = k.to_string();
                                identities.insert(label.clone(), format!("Number: {label}"));
                                (label, v)
                            })
                            .collect();
                    }
                }
            }
        }
        PreparedDistribution {
            selection,
            groups,
            identities,
        }
    }
}

fn category_labels<'a>(values: impl Iterator<Item = &'a Value>) -> Vec<String> {
    let values: Vec<&Value> = values.collect();
    let mut labels: Vec<String> = values
        .iter()
        .map(|v| match v {
            Value::String(s) if s.is_empty() => "(empty string)".into(),
            Value::String(s) => s.clone(),
            v => v.to_string(),
        })
        .collect();
    // If a generated label collides with a literal string, qualify both as well.
    loop {
        let mut counts = BTreeMap::new();
        for label in &labels {
            *counts.entry(label.as_str()).or_insert(0usize) += 1;
        }
        let collisions: Vec<usize> = (0..labels.len())
            .filter(|&i| counts[labels[i].as_str()] > 1)
            .collect();
        if collisions.is_empty() {
            return labels;
        }
        for i in collisions {
            let kind = match values[i] {
                Value::String(_) => "string",
                Value::Number(_) => "number",
                Value::Bool(_) => "boolean",
                Value::Null => "null",
                _ => "JSON",
            };
            labels[i] = format!("{kind}: {}", values[i]);
        }
        // Typed JSON is injective: each pass qualifies at least one new label.
    }
}

pub fn normalize_minmax(values: &[f64]) -> Vec<f64> {
    let (min, max) = values
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(v), hi.max(v))
        });
    values
        .iter()
        .map(|&v| {
            if !v.is_finite() {
                v
            } else if min == max {
                0.0
            } else if (max - min).is_finite() {
                (v - min) / (max - min)
            } else {
                (v / 2.0 - min / 2.0) / (max / 2.0 - min / 2.0)
            }
        })
        .collect()
}

pub struct PreparedDistribution {
    pub selection: DistributionSelection,
    pub groups: Vec<(String, Vec<f64>)>,
    identities: BTreeMap<String, String>,
}

impl PreparedDistribution {
    /// Type-preserving export identity, safely prefixed before CSV text sanitization.
    pub fn group_identity(&self, label: &str) -> &str {
        &self.identities[label]
    }
    pub fn boxes(&self) -> Vec<(String, BoxPlotStats)> {
        self.groups
            .iter()
            .filter_map(|(label, values)| compute_boxplot(values).map(|s| (label.clone(), s)))
            .collect()
    }

    pub fn violins(&self) -> ViolinData {
        let mut curves = Vec::new();
        let mut skipped = Vec::new();
        for (label, values) in &self.groups {
            if values.len() < 2 {
                skipped.push((label.clone(), "fewer than 2 finite values"));
                continue;
            }
            if values.iter().all(|v| *v == values[0]) {
                skipped.push((label.clone(), "constant values"));
                continue;
            }
            if let Some(curve) = compute_violin(values, GRID_POINTS) {
                curves.push((label.clone(), curve));
            } else {
                skipped.push((label.clone(), "non-finite or zero KDE bandwidth"));
            }
        }
        (curves, skipped)
    }
}

/// Snapshot identity, not study name or row count, controls cache validity.
#[derive(Default)]
pub struct DistributionCache<T> {
    entry: Option<(Arc<DataFrame>, DistributionSelection, T)>,
}

impl<T> DistributionCache<T> {
    pub fn get(
        &mut self,
        view: &StudyView,
        selection: &DistributionSelection,
        compute: impl FnOnce(&PreparedDistribution) -> T,
    ) -> &T {
        if !self
            .entry
            .as_ref()
            .is_some_and(|(df, s, _)| Arc::ptr_eq(df, &view.df) && s == selection)
        {
            self.entry = Some((
                view.df.clone(),
                selection.clone(),
                compute(&selection.prepare(view)),
            ));
        }
        &self.entry.as_ref().unwrap().2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tunny_core::dataframe::{DistributionMetadata, TrialRow};

    pub(crate) fn view(rows: Vec<TrialRow>, params: &[&str], objectives: &[&str]) -> StudyView {
        let params = params.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        let objectives = objectives.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        StudyView::new(
            Arc::new(DataFrame::from_trials(
                &rows,
                &params,
                &objectives,
                &[],
                &[],
                0,
            )),
            vec![],
        )
    }

    fn row(y: f64, category: Option<Value>, group: f64) -> TrialRow {
        TrialRow {
            param_display: [
                ("discrete".into(), group),
                ("continuous".into(), group),
                ("csv_numeric".into(), group),
            ]
            .into(),
            distribution_metadata: DistributionMetadata {
                numeric_discrete: [("discrete".into(), true), ("continuous".into(), false)].into(),
                categories: category
                    .map(|v| [("cat".into(), v)].into())
                    .unwrap_or_default(),
                ..Default::default()
            },
            objective_values: vec![y, y * 10.0],
            ..Default::default()
        }
    }

    fn selection(group: Option<&str>, normalize: bool) -> DistributionSelection {
        DistributionSelection {
            value: Some(DistributionValue::Objective("obj1".into())),
            group_by: group.map(str::to_string),
            normalize,
        }
    }

    #[test]
    fn candidates_defaults_stale_selections_and_numeric_value() {
        let data = view(
            vec![row(2.0, Some(Value::String("a".into())), 1.0)],
            &["cat", "discrete", "continuous", "csv_numeric"],
            &["obj0", "obj1"],
        );
        let (values, groups) = DistributionSelection::candidates(&data);
        assert_eq!(values[0], DistributionValue::Objective("obj0".into()));
        assert_eq!(values[1], DistributionValue::Objective("obj1".into()));
        assert_eq!(groups, ["cat", "discrete"]);
        let mut state = DistributionSelection::default();
        state.validate(&data);
        assert_eq!(state.value, Some(values[0].clone()));
        state.value = Some(DistributionValue::Parameter("csv_numeric".into()));
        state.group_by = Some("continuous".into());
        state.validate(&data);
        assert_eq!(state.prepare(&data).groups[0].1, [1.0]);
        assert!(state.group_by.is_none());
        let fresh = view(vec![row(7.0, None, 2.0)], &["discrete"], &["fresh"]);
        state.validate(&fresh);
        assert_eq!(
            state.value,
            Some(DistributionValue::Objective("fresh".into()))
        );
        let no_objectives = view(vec![row(7.0, None, 2.0)], &["discrete"], &[]);
        state.validate(&no_objectives);
        assert_eq!(
            state.value,
            Some(DistributionValue::Parameter("discrete".into()))
        );
    }

    #[test]
    fn identity_presence_alignment_sparse_groups_and_normalization() {
        let rows = vec![
            row(0.0, Some(Value::String("".into())), 10.0),
            row(2.0, Some(Value::String("".into())), 10.0),
            row(4.0, Some(serde_json::json!(1)), 2.0),
            row(6.0, Some(Value::String("1".into())), 2.0),
            row(8.0, Some(Value::String("constant".into())), 2.0),
            row(8.0, Some(Value::String("constant".into())), 2.0),
            row(12.0, None, f64::NAN),
            row(f64::NAN, Some(Value::String("a".into())), 1.0),
            row(f64::INFINITY, Some(Value::String("a".into())), 1.0),
        ];
        let data = view(rows, &["cat", "discrete"], &["obj0", "obj1"]);
        let prepared = selection(Some("cat"), true).prepare(&data);
        assert_eq!(
            prepared
                .groups
                .iter()
                .map(|(l, _)| l.as_str())
                .collect::<Vec<_>>(),
            [
                "(empty string)",
                "a",
                "constant",
                "number: 1",
                "string: \"1\""
            ]
        );
        assert_eq!(prepared.groups[0].1, [0.0, 1.0 / 6.0]);
        assert!(prepared.groups[1].1.is_empty());
        assert_eq!(prepared.groups[2].1, [2.0 / 3.0, 2.0 / 3.0]);
        let boxes = prepared.boxes();
        assert_eq!(boxes.len(), 4);
        assert_eq!(boxes[0].1.n, 2);
        let (curves, skipped) = prepared.violins();
        assert_eq!(curves.len(), 1);
        assert_eq!(curves[0].0, "(empty string)");
        assert_eq!(skipped.len(), 4);
        assert!(skipped
            .iter()
            .any(|(_, reason)| *reason == "constant values"));
        assert_eq!(curves[0].1.data_max, 1.0 / 6.0);
        let discrete = selection(Some("discrete"), false).prepare(&data);
        assert_eq!(
            discrete
                .groups
                .iter()
                .map(|(l, _)| l.as_str())
                .collect::<Vec<_>>(),
            ["1", "2", "10"]
        );
        assert!(discrete.groups[0].1.is_empty());
        assert_eq!(discrete.groups[2].1, [0.0, 20.0]);
    }

    #[test]
    fn constant_singleton_all_skipped_and_no_group() {
        for ys in [vec![3.0], vec![3.0, 3.0], vec![f64::NAN, f64::INFINITY]] {
            let data = view(
                ys.iter().map(|&v| row(v, None, 1.0)).collect(),
                &[],
                &["obj0", "obj1"],
            );
            let prepared = selection(None, true).prepare(&data);
            let (curves, skipped) = prepared.violins();
            assert!(curves.is_empty());
            assert_eq!(skipped.len(), 1);
            if ys[0].is_finite() {
                let boxes = prepared.boxes();
                assert_eq!(boxes.len(), 1);
                assert_eq!(boxes[0].1.min, 0.0);
                assert_eq!(boxes[0].1.max, 0.0);
            } else {
                assert!(prepared.boxes().is_empty());
            }
        }
    }

    #[test]
    fn decimal_constant_raw_violin_is_skipped_and_reported_in_empty_ui() {
        let data = view(
            (0..3)
                .map(|_| TrialRow {
                    objective_values: vec![0.1],
                    ..Default::default()
                })
                .collect(),
            &[],
            &["f"],
        );
        let selection = DistributionSelection::default();
        assert!(!selection.normalize);
        let prepared = selection.prepare(&data);
        assert_eq!(prepared.groups[0].1, [0.1, 0.1, 0.1]);
        let (curves, skipped) = prepared.violins();
        assert!(curves.is_empty());
        assert_eq!(skipped, [("f".into(), "constant values")]);
        assert_eq!(prepared.boxes().len(), 1);

        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut chart = super::super::violin_plot::ViolinPlotChart::default();
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 600.0),
                )),
                ..Default::default()
            },
            |ui| chart.show(ui, &data),
        );
        let update = output.platform_output.accesskit_update.unwrap();
        for expected in [
            "1 group(s) skipped.",
            "f: constant values.",
            "Need at least 2 finite, non-identical values to estimate a distribution.",
        ] {
            assert!(
                update
                    .nodes
                    .iter()
                    .any(|(_, n)| n.value() == Some(expected)),
                "missing {expected}"
            );
        }
    }

    #[test]
    fn same_name_same_count_snapshot_refresh_and_widget_independence() {
        let old = view(
            vec![row(1.0, None, 1.0), row(2.0, None, 1.0)],
            &[],
            &["obj0", "obj1"],
        );
        let new = view(
            vec![row(10.0, None, 1.0), row(20.0, None, 1.0)],
            &[],
            &["obj0", "obj1"],
        );
        let mut cache = DistributionCache::default();
        let state = selection(None, false);
        assert_eq!(cache.get(&old, &state, |p| p.boxes())[0].1.max, 20.0);
        assert_eq!(cache.get(&new, &state, |p| p.boxes())[0].1.max, 200.0);
        let mut box_chart = super::super::box_plot::BoxPlotChart::default();
        let violin_chart = super::super::violin_plot::ViolinPlotChart::default();
        box_chart.selection.normalize = true;
        assert!(!violin_chart.selection.normalize);
    }

    #[test]
    fn same_named_objective_and_parameter_remain_distinct() {
        let data = view(
            vec![TrialRow {
                param_display: [("shared".into(), 1.0)].into(),
                objective_values: vec![9.0],
                ..Default::default()
            }],
            &["shared"],
            &["shared"],
        );
        let mut state = DistributionSelection::default();
        assert_eq!(state.prepare(&data).groups[0].1, [9.0]);
        state.value = Some(DistributionValue::Parameter("shared".into()));
        assert_eq!(state.prepare(&data).groups[0].1, [1.0]);
    }

    #[test]
    fn generated_category_labels_cannot_collide_with_literal_labels() {
        let values = [
            serde_json::json!(1),
            serde_json::json!("1"),
            serde_json::json!("number: 1"),
            serde_json::json!(""),
            serde_json::json!("(empty string)"),
        ];
        let labels = category_labels(values.iter());
        let unique: std::collections::BTreeSet<_> = labels.iter().collect();
        assert_eq!(unique.len(), values.len());
        let typed = [
            serde_json::json!(true),
            serde_json::json!("true"),
            Value::Null,
            serde_json::json!("null"),
        ];
        let labels = category_labels(typed.iter());
        assert_eq!(
            labels,
            [
                "boolean: true",
                "string: \"true\"",
                "null: null",
                "string: \"null\""
            ]
        );
    }

    #[test]
    fn csv_numeric_parameters_are_value_only_and_categories_keep_parsed_presence() {
        let parsed =
            tunny_core::flat_csv::parse_flat_csv(b"in:n,in:cat,out:y\n1,,0\n1,a,2\n2,a,4\n", "csv")
                .unwrap();
        let data = StudyView::new(Arc::new(parsed.dataframe), vec![]);
        let (values, groups) = DistributionSelection::candidates(&data);
        assert_eq!(
            values,
            [
                DistributionValue::Objective("y".into()),
                DistributionValue::Parameter("n".into())
            ]
        );
        assert_eq!(groups, ["cat"]);
        let mut state = DistributionSelection {
            group_by: Some("n".into()),
            ..Default::default()
        };
        state.validate(&data);
        assert!(state.group_by.is_none());
        state.group_by = Some("cat".into());
        let groups = state.prepare(&data).groups;
        assert_eq!(
            groups,
            [
                ("(empty string)".into(), vec![0.0]),
                ("a".into(), vec![2.0, 4.0])
            ]
        );
    }

    #[test]
    fn both_widgets_expose_equivalent_controls_and_violin_reports_all_skipped() {
        let data = view(
            vec![
                row(3.0, Some(serde_json::json!("")), 1.0),
                row(3.0, None, 1.0),
            ],
            &["cat", "discrete"],
            &["obj0", "obj1"],
        );
        let mut boxes = super::super::box_plot::BoxPlotChart::default();
        boxes.selection = selection(Some("cat"), false);
        let mut violins = super::super::violin_plot::ViolinPlotChart::default();
        violins.selection = selection(Some("cat"), false);
        let run = |draw: &mut dyn FnMut(&mut egui::Ui)| {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900.0, 600.0),
                    )),
                    ..Default::default()
                },
                |ui| draw(ui),
            )
        };
        let box_output = run(&mut |ui| boxes.show(ui, &data));
        let violin_output = run(&mut |ui| violins.show(ui, &data));
        for output in [&box_output, &violin_output] {
            let update = output.platform_output.accesskit_update.as_ref().unwrap();
            for expected in ["Value:", "Group by:", "Objective: obj1", "Parameter: cat"] {
                assert!(
                    update
                        .nodes
                        .iter()
                        .any(|(_, n)| n.value() == Some(expected) || n.label() == Some(expected)),
                    "missing {expected}"
                );
            }
        }
        let update = violin_output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap();
        for expected in [
            "1 group(s) skipped.",
            "(empty string): fewer than 2 finite values.",
        ] {
            assert!(
                update
                    .nodes
                    .iter()
                    .any(|(_, n)| n.value() == Some(expected)),
                "missing {expected}"
            );
        }
    }
}
