//! Explicit input acquisition and revision-safe shared result delivery.
use super::app_state::AppState;
use super::messages::AppMessage;
use std::sync::{mpsc::SyncSender, Mutex};
use tdashboard_analysis::preparation::Preparation;
use tdashboard_analysis::session::*;
use tdashboard_analysis::shared::KeyedResult;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DeliverySlot {
    Convergence,
    Importance,
    Heatmap,
    Pdp1d,
    Pdp2d,
    Contour,
    Cluster(super::messages::ClusterChartSource),
    Mcdm(super::messages::McdmChartSource),
    Entropy(super::messages::McdmChartSource),
    Fit,
    MultiFit,
    Optimize,
    MultiOptimize,
    Suggest,
    MultiSuggest,
    Robustness,
    Surface,
    Compare,
}
impl DeliverySlot {
    fn kind(self) -> AnalysisKind {
        match self {
            Self::Convergence => AnalysisKind::Convergence,
            Self::Importance => AnalysisKind::Importance,
            Self::Heatmap => AnalysisKind::Heatmap,
            Self::Pdp1d => AnalysisKind::Pdp1d,
            Self::Pdp2d => AnalysisKind::Pdp2d,
            Self::Contour => AnalysisKind::Contour,
            Self::Cluster(_) => AnalysisKind::Cluster,
            Self::Mcdm(_) => AnalysisKind::Mcdm,
            Self::Entropy(_) => AnalysisKind::Entropy,
            Self::Fit | Self::MultiFit | Self::Robustness | Self::Surface => AnalysisKind::Fit,
            Self::Optimize | Self::MultiOptimize => AnalysisKind::Optimize,
            Self::Suggest | Self::MultiSuggest => AnalysisKind::Suggest,
            Self::Compare => AnalysisKind::Compare,
        }
    }

    pub fn clear_busy(self, w: &mut crate::ui::widget_states::WidgetStates) {
        use super::messages::{ClusterChartSource as C, McdmChartSource as M};
        match self {
            Self::Convergence => w.convergence.computing = false,
            Self::Importance => w.importance.computing = false,
            Self::Heatmap => w.sensitivity_heatmap.computing = false,
            Self::Pdp1d => w.pdp_chart.computing = false,
            Self::Pdp2d => w.pdp_2d.computing = false,
            Self::Contour => w.observed_contour.computing = false,
            Self::Cluster(source) => match source {
                C::Scatter2D => w.cluster_scatter.computing = false,
                C::Scatter3D => w.cluster_scatter_3d.computing = false,
                C::Table => w.trial_table.cluster.computing = false,
                C::ArtifactGallery => w.artifact_gallery.cluster_computing = false,
            },
            Self::Mcdm(source) | Self::Entropy(source) => {
                let controls = match source {
                    M::Rank => &mut w.mcdm_chart.controls,
                    M::Scatter2D => &mut w.scatter_chart.controls,
                    M::Scatter3D => &mut w.mcdm_scatter_3d.controls,
                    M::Table => &mut w.trial_table.mcdm.controls,
                    M::ArtifactGallery => &mut w.artifact_gallery.mcdm,
                };
                controls.computing = false;
                if matches!(self, Self::Entropy(_)) {
                    controls.pending_entropy = false;
                }
            }
            Self::Fit | Self::MultiFit => {
                w.surrogate_opt.fitting = false;
                w.surrogate_opt.fit_progress = None;
            }
            Self::Optimize | Self::MultiOptimize => w.surrogate_opt.optimizing = false,
            Self::Suggest => w.surrogate_opt.suggesting = false,
            Self::MultiSuggest => w.surrogate_opt.multi_suggesting = false,
            Self::Robustness => w.robustness.fitting = false,
            Self::Surface => w.response_surface.fitting = false,
            Self::Compare => w.surrogate_compare.computing = false,
        }
    }
}

pub struct Delivery(pub Mutex<Option<AppMessage>>);

#[cfg(test)]
pub(crate) fn fixture_result<T: std::any::Any + Send + Sync>(
    value: T,
) -> tdashboard_analysis::shared::SharedResult<T> {
    let (state, _, _) = tests::state(true);
    let journal = state.journal_studies.as_ref().unwrap();
    let input = journal.managers[&0].acquire().unwrap();
    let mut session = journal.analysis.borrow_mut();
    let prepared = session
        .prepare(
            vec![input],
            AnalysisRequest {
                kind: AnalysisKind::Distribution,
                conditions: vec![],
            },
            &[],
        )
        .unwrap();
    let WorkOrShared::Work(work) = session.start(prepared, ReuseOrRerun::Rerun).unwrap() else {
        unreachable!()
    };
    let result = session.complete(work.execute(|_| Ok(value))).unwrap();
    tdashboard_analysis::shared::SharedResult::new(result).unwrap()
}

fn select_job(
    journal: &super::journal_studies::JournalStudies,
    session: &mut AnalysisSession,
    slot: DeliverySlot,
    consumer: u64,
    key: Option<&ResultKey>,
) {
    let mut jobs = journal.analysis_jobs.borrow_mut();
    // Rejected preparations supersede a calculation just like successful work:
    // every still-interested presentation follows its latest checked run key.
    if let Some(key) = key {
        for selected in jobs.values_mut() {
            if selected.analysis == key.analysis {
                *selected = key.clone();
            }
        }
    }
    let previous = if let Some(key) = key {
        jobs.insert((consumer, slot), key.clone())
    } else {
        jobs.remove(&(consumer, slot))
    };
    let release = previous.filter(|previous| {
        Some(previous) != key && !jobs.values().any(|selected| selected == previous)
    });
    drop(jobs);
    if let Some(previous) = release {
        session.release(&previous);
    }
}

/// A newly consumed request may adopt a cached result or fail locally without
/// starting work. It still replaces this slot's old adoption interest; it does
/// not cancel the retained calculation or detach another interested consumer.
pub fn supersede(state: &AppState, slot: DeliverySlot) {
    if let Some(journal) = state.journal_studies.as_ref() {
        select_job(
            journal,
            &mut journal.analysis.borrow_mut(),
            slot,
            state.analysis_consumer.get(),
            None,
        );
    }
}

/// Cache-key validation already delegates to the analysis method's validation.
/// Route its rejection through the same provenance/busy-slot protocol, before
/// front readiness can obscure the original method error.
pub fn reject_settings(
    state: &AppState,
    tx: SyncSender<AppMessage>,
    slot: DeliverySlot,
    message: String,
) {
    let Some(input) = state.current_study.as_ref().and_then(|s| s.view.input()) else {
        #[cfg(test)]
        crate::app::spawn_task(tx, move || {
            preparation_error(slot, &Preparation::Table, Readiness::Unavailable(message))
        });
        return;
    };
    let Some(journal) = state.journal_studies.as_ref() else {
        return;
    };
    let request = AnalysisRequest {
        kind: slot.kind(),
        conditions: vec![("invalid settings".into(), Condition::Text(message.clone()))],
    };
    let mut session = journal.analysis.borrow_mut();
    let Ok(completion) = session.reject(&[input], request, &Preparation::Table, message) else {
        return;
    };
    select_job(
        journal,
        &mut session,
        slot,
        state.analysis_consumer.get(),
        Some(completion.key()),
    );
    drop(session);
    crate::app::spawn_task(tx, move || AppMessage::AnalysisCompleted {
        slot,
        completion,
    });
}

fn preparation_error(slot: DeliverySlot, _selection: &Preparation, error: Readiness) -> AppMessage {
    let message = match error {
        Readiness::Unavailable(message) => message,
        Readiness::Waiting => "Analysis is waiting for its input".into(),
    };
    match slot {
        DeliverySlot::Pdp1d => AppMessage::AnalysisUnavailable(slot),
        DeliverySlot::Entropy(_) if message.starts_with("At least one") => {
            AppMessage::AnalysisUnavailable(slot)
        }
        DeliverySlot::Entropy(source) => AppMessage::McdmFailed {
            source,
            message: format!("Entropy computation failed: {message}"),
        },
        DeliverySlot::Mcdm(source) => AppMessage::McdmFailed { source, message },
        DeliverySlot::Cluster(source) => AppMessage::ClusterFailed {
            source,
            err: super::messages::cluster_ui_error(&message, None, message.starts_with("k ")),
        },
        DeliverySlot::Fit => AppMessage::SurrogateFitFailed(message),
        DeliverySlot::Optimize => AppMessage::SurrogateFitFailed(message),
        DeliverySlot::MultiOptimize => AppMessage::SurrogateMultiOptFailed(message),
        DeliverySlot::Suggest => AppMessage::SurrogateSuggestFailed(message),
        DeliverySlot::MultiSuggest => AppMessage::SurrogateMultiSuggestFailed(message),
        DeliverySlot::MultiFit => AppMessage::SurrogateMultiFitFailed(message),
        DeliverySlot::Robustness => AppMessage::RobustnessFitFailed(message),
        DeliverySlot::Surface => AppMessage::ResponseSurfaceFitFailed(message),
        DeliverySlot::Compare => AppMessage::SurrogateCompareFailed(message),
        DeliverySlot::Contour => AppMessage::ObservedContourFailed(message),
        DeliverySlot::Importance => AppMessage::SensitivityError(message),
        DeliverySlot::Pdp2d => AppMessage::Error("PDP 2D computation failed".into()),
        _ => AppMessage::Error(message),
    }
}

#[cfg(test)]
fn fixture_inputs(state: &AppState, kind: AnalysisKind) -> Vec<tdashboard_study::StudyInput> {
    use std::{collections::BTreeMap, sync::Arc};
    use tdashboard_study::*;
    let views = state.current_study.iter().chain(
        state
            .comparison_studies
            .iter()
            .filter(|_| kind == AnalysisKind::Convergence),
    );
    views
        .map(|study| {
            if let Some(input) = study.view.input() {
                return input;
            }
            let key = StudyKey {
                source: IdentityAllocator::new().unwrap().source().unwrap(),
                stored_id: study.meta.study_id,
            };
            let metadata = StudyMetadata {
                name: study.meta.name.clone(),
                objectives: study
                    .meta
                    .objective_names
                    .iter()
                    .enumerate()
                    .map(|(i, name)| ObjectiveDescriptor {
                        index: ObjectiveIndex(i as u32),
                        name: Some(name.clone()),
                        direction: if matches!(
                            study.meta.directions.get(i),
                            Some(super::app_state::Direction::Maximize)
                        ) {
                            Direction::Maximize
                        } else {
                            Direction::Minimize
                        },
                    })
                    .collect(),
                ..Default::default()
            };
            let trials = (0..study.view.row_count())
                .map(|r| {
                    let df = &study.view.df;
                    let mut trial = TrialRecord::new(
                        key,
                        df.get_trial_id(r).unwrap_or(r as u32),
                        StudyTrialNumber(df.get_trial_number(r).unwrap_or(r as u32)),
                        TrialState::Complete,
                    );
                    trial.completed_order = Some(r as u64);
                    trial.objectives = Arc::new(
                        study
                            .meta
                            .objective_names
                            .iter()
                            .enumerate()
                            .map(|(i, n)| {
                                (
                                    ObjectiveIndex(i as u32),
                                    Number(
                                        df.get_numeric_column(n)
                                            .and_then(|c| c.get(r))
                                            .copied()
                                            .unwrap_or(f64::NAN),
                                    ),
                                )
                            })
                            .collect(),
                    );
                    trial.parameters = Arc::new(
                        study
                            .meta
                            .param_names
                            .iter()
                            .map(|n| {
                                let value = if let Some(v) =
                                    df.get_numeric_column(n).and_then(|c| c.get(r))
                                {
                                    ParameterValue::Numeric(Number(*v))
                                } else if let Some(v) =
                                    df.get_string_column(n).and_then(|c| c.get(r))
                                {
                                    ParameterValue::Category(Arc::new(serde_json::Value::String(
                                        v.clone(),
                                    )))
                                } else {
                                    ParameterValue::Numeric(Number(f64::NAN))
                                };
                                (
                                    n.clone(),
                                    ParameterObservation {
                                        value,
                                        distribution: None,
                                        external_encoding: true,
                                        encoded_numeric: None,
                                    },
                                )
                            })
                            .collect::<BTreeMap<_, _>>(),
                    );
                    trial.constraints = ConstraintObservation {
                        present: df.feasibility().has_constraints(),
                        values: df
                            .constraint_col_names()
                            .iter()
                            .map(|n| Number(df.get_numeric_column(n).unwrap()[r]))
                            .collect::<Vec<_>>()
                            .into(),
                    };
                    Arc::new(trial)
                })
                .collect();
            let mut manager = StudyManager::new(key);
            let ticket = manager.begin(LoadKind::Initial).unwrap();
            let receipt = manager
                .apply(
                    manager.header(ticket).unwrap(),
                    StudyBatch::Replace { trials, metadata },
                )
                .unwrap();
            manager.finish(ticket, receipt.next_sequence).unwrap();
            manager.acquire().unwrap()
        })
        .collect()
}

pub fn spawn(
    state: &AppState,
    tx: SyncSender<AppMessage>,
    slot: DeliverySlot,
    settings: impl Into<Condition>,
    selection: Preparation,
    compute: impl FnOnce(&PreparedAnalysis) -> AppMessage + Send + 'static,
) {
    spawn_inner(state, tx, slot, settings, selection, None, compute);
}

pub fn spawn_with_models(
    state: &AppState,
    tx: SyncSender<AppMessage>,
    slot: DeliverySlot,
    settings: impl Into<Condition>,
    models: tdashboard_analysis::preparation::Models,
    compute: impl FnOnce(&PreparedAnalysis) -> AppMessage + Send + 'static,
) {
    spawn_inner(
        state,
        tx,
        slot,
        settings,
        Preparation::Table,
        Some(models),
        compute,
    );
}

#[allow(clippy::too_many_arguments)]
fn spawn_inner(
    state: &AppState,
    tx: SyncSender<AppMessage>,
    slot: DeliverySlot,
    settings: impl Into<Condition>,
    selection: Preparation,
    models: Option<tdashboard_analysis::preparation::Models>,
    compute: impl FnOnce(&PreparedAnalysis) -> AppMessage + Send + 'static,
) {
    let kind = slot.kind();
    let request = AnalysisRequest {
        kind,
        conditions: vec![("request".into(), settings.into())],
    };
    let Some(input) = state.current_study.as_ref().and_then(|s| s.view.input()) else {
        #[cfg(test)]
        {
            let mut session = AnalysisSession::default();
            let inputs = fixture_inputs(state, kind);
            let front = if matches!(kind, AnalysisKind::Cluster | AnalysisKind::Mcdm) {
                inputs
                    .first()
                    .and_then(|i| session.pareto(i.clone()).ok())
                    .into_iter()
                    .collect()
            } else {
                vec![]
            };
            let prepared = session.prepare_for(inputs, request, selection.clone(), &front);
            crate::app::spawn_task(tx, move || match prepared {
                Ok(prepared) => compute(&prepared),
                Err(error) => preparation_error(slot, &selection, error),
            });
        }
        return;
    };
    let Some(journal) = state.journal_studies.as_ref() else {
        return;
    };
    let mut inputs = vec![input];
    if kind == AnalysisKind::Convergence {
        for study in &state.comparison_studies {
            let Some(input) = study.view.input() else {
                return;
            };
            inputs.push(input);
        }
    }
    let mut session = journal.analysis.borrow_mut();
    let dependencies = if matches!(kind, AnalysisKind::Cluster | AnalysisKind::Mcdm) {
        match session.pareto(inputs[0].clone()) {
            Ok(front) => vec![front],
            Err(_) => return,
        }
    } else {
        vec![]
    };
    let prepared = if let Some(models) = models {
        session.prepare_models(inputs.clone(), request.clone(), models)
    } else {
        session.prepare_for(
            inputs.clone(),
            request.clone(),
            selection.clone(),
            &dependencies,
        )
    };
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(Readiness::Waiting) => return,
        Err(Readiness::Unavailable(message)) => {
            let Ok(completion) = session.reject(&inputs, request, &selection, message) else {
                return;
            };
            select_job(
                journal,
                &mut session,
                slot,
                state.analysis_consumer.get(),
                Some(completion.key()),
            );
            drop(session);
            crate::app::spawn_task(tx, move || AppMessage::AnalysisCompleted {
                slot,
                completion,
            });
            return;
        }
    };
    let sharing = matches!(
        slot,
        DeliverySlot::Cluster(_) | DeliverySlot::Mcdm(_) | DeliverySlot::Entropy(_)
    ) && !journal
        .analysis_jobs
        .borrow()
        .contains_key(&(state.analysis_consumer.get(), slot))
        && session
            .lookup(prepared.key())
            .is_some_and(|r| r.is_pending());
    let started = session.start(
        prepared,
        if sharing {
            ReuseOrRerun::Reuse
        } else {
            ReuseOrRerun::Rerun
        },
    );
    let work = match started {
        Ok(WorkOrShared::Shared(handle)) => {
            journal
                .analysis_jobs
                .borrow_mut()
                .insert((state.analysis_consumer.get(), slot), handle.key().clone());
            return;
        }
        Ok(WorkOrShared::Work(work)) => work,
        Err(_) => return,
    };
    let key = work.key().clone();
    select_job(
        journal,
        &mut session,
        slot,
        state.analysis_consumer.get(),
        Some(&key),
    );
    drop(session);
    crate::app::spawn_task(tx, move || {
        let completion = work.execute_shared(|prepared| {
            let message = crate::app::task_result(|| {
                let mut message = compute(prepared);
                bind_trained(&mut message, &key, prepared);
                message
            });
            Ok(match message {
                AppMessage::ClusteringDone { key, result, .. } => std::sync::Arc::new(KeyedResult {
                    selection: key,
                    value: std::sync::Arc::new(result),
                    rows: prepared.tables()[0].rows.clone(),
                })
                    as std::sync::Arc<dyn std::any::Any + Send + Sync>,
                AppMessage::McdmDone { key, result, .. } => std::sync::Arc::new(KeyedResult {
                    selection: key,
                    value: std::sync::Arc::new(result),
                    rows: prepared.tables()[0].rows.clone(),
                }),
                AppMessage::SensitivityDone { key, result } => std::sync::Arc::new(KeyedResult {
                    selection: key,
                    value: std::sync::Arc::new(result),
                    rows: prepared.tables()[0].rows.clone(),
                }),
                AppMessage::SobolDone { key, result } => std::sync::Arc::new(KeyedResult {
                    selection: key,
                    value: std::sync::Arc::new(result),
                    rows: prepared.tables()[0].rows.clone(),
                }),
                AppMessage::SensitivityHeatmapDone {
                    metric,
                    feasible_only,
                    result,
                } => std::sync::Arc::new(KeyedResult {
                    selection: (metric.cache_id(), feasible_only),
                    value: std::sync::Arc::new(result),
                    rows: prepared.tables()[0].rows.clone(),
                }),
                AppMessage::SurrogateFitDone(model)
                | AppMessage::RobustnessFitDone(model)
                | AppMessage::ResponseSurfaceFitDone(model) => model,
                AppMessage::SurrogateMultiFitDone(models) => models,
                message => std::sync::Arc::new(Delivery(Mutex::new(Some(message)))),
            })
        });
        AppMessage::AnalysisCompleted { slot, completion }
    });
}

fn bind_trained(message: &mut AppMessage, key: &ResultKey, prepared: &PreparedAnalysis) {
    match message {
        AppMessage::SurrogateFitDone(model)
        | AppMessage::RobustnessFitDone(model)
        | AppMessage::ResponseSurfaceFitDone(model) => {
            if let Some(model) = std::sync::Arc::get_mut(model) {
                model.bind_fit_key(key.clone(), &prepared.row_maps()[0]);
            }
        }
        AppMessage::SurrogateMultiFitDone(models) => {
            if let Some(models) = std::sync::Arc::get_mut(models) {
                for model in models {
                    model.bind_fit_key(key.clone(), &prepared.row_maps()[0]);
                }
            }
        }
        _ => {}
    }
}

pub fn adopt_all(
    slot: DeliverySlot,
    completion: AnalysisCompletion,
    state: &AppState,
    widgets: &mut crate::ui::widget_states::WidgetStates,
) -> Vec<AppMessage> {
    let key = completion.key().clone();
    let Some(current) = state
        .current_study
        .as_ref()
        .and_then(|s| s.view.input())
        .map(|i| i.stamp())
    else {
        return vec![];
    };
    let expected: Vec<_> = if key.analysis.request.kind == AnalysisKind::Convergence {
        std::iter::once(current)
            .chain(
                state
                    .comparison_studies
                    .iter()
                    .filter_map(|s| s.view.input().map(|i| i.stamp())),
            )
            .collect()
    } else {
        vec![current]
    };
    let Some(journal) = state.journal_studies.as_ref() else {
        return vec![];
    };
    let selected: Vec<_> = journal
        .analysis_jobs
        .borrow()
        .iter()
        .filter(|(_, value)| **value == key)
        .map(|(slot, _)| *slot)
        .collect();
    if selected.is_empty() {
        journal.analysis.borrow_mut().release(&key);
        return vec![];
    }
    for selected in &selected {
        journal.analysis_jobs.borrow_mut().remove(selected);
        if selected.0 == 0 {
            selected.1.clear_busy(widgets);
        }
    }
    let mut session = journal.analysis.borrow_mut();
    if key.analysis.inputs != expected {
        session.release(&key);
        return vec![];
    }
    let Some(result) = session.complete(completion) else {
        return vec![];
    };
    if let Some(Ok(model)) =
        result.payload::<tdashboard_analysis::surrogate_opt::TrainedSurrogate>()
    {
        return selected
            .into_iter()
            .map(|(consumer, slot)| {
                targeted(
                    consumer,
                    slot,
                    match slot {
                        DeliverySlot::Fit => AppMessage::SurrogateFitDone(model.clone()),
                        DeliverySlot::Robustness => AppMessage::RobustnessFitDone(model.clone()),
                        DeliverySlot::Surface => AppMessage::ResponseSurfaceFitDone(model.clone()),
                        _ => unreachable!("Fit payload delivered only to fit consumers"),
                    },
                )
            })
            .collect();
    }
    if let Some(Ok(models)) =
        result.payload::<Vec<tdashboard_analysis::surrogate_opt::TrainedSurrogate>>()
    {
        return selected
            .into_iter()
            .map(|(consumer, slot)| {
                targeted(
                    consumer,
                    slot,
                    AppMessage::SurrogateMultiFitDone(models.clone()),
                )
            })
            .collect();
    }
    let typed =
        match slot {
            DeliverySlot::Cluster(_) => result
                .payload::<KeyedResult<
                    crate::ui::widgets::cluster_scatter::ClusterCacheKey,
                    super::results::ClusterResult,
                >>()
                .is_some_and(|r| r.is_ok()),
            DeliverySlot::Mcdm(_) => result
                .payload::<KeyedResult<
                    crate::ui::widgets::mcdm_chart::McdmCacheKey,
                    super::results::McdmResult,
                >>()
                .is_some_and(|r| r.is_ok()),
            DeliverySlot::Importance => {
                result
                    .payload::<KeyedResult<(u8, usize, bool), super::results::SensitivityResult>>()
                    .is_some_and(|r| r.is_ok())
                    || result
                        .payload::<KeyedResult<(usize, bool), super::results::SobolResult>>()
                        .is_some_and(|r| r.is_ok())
            }
            DeliverySlot::Heatmap => result
                .payload::<KeyedResult<(u8, bool), super::results::HeatmapMatrix>>()
                .is_some_and(|r| r.is_ok()),
            _ => false,
        };
    if typed {
        return selected
            .into_iter()
            .map(|(consumer, slot)| {
                targeted(
                    consumer,
                    slot,
                    AppMessage::SharedAnalysis {
                        slot,
                        result: result.clone(),
                    },
                )
            })
            .collect();
    }
    let Some(payload) = result.payload::<Delivery>() else {
        return vec![];
    };
    let payload = match payload {
        Ok(payload) => payload,
        Err(message) => {
            session.release(&key);
            return selected
                .into_iter()
                .map(|(consumer, slot)| {
                    targeted(
                        consumer,
                        slot,
                        preparation_error(
                            slot,
                            &Preparation::Table,
                            Readiness::Unavailable(message.clone()),
                        ),
                    )
                })
                .collect();
        }
    };
    let message = payload.0.lock().unwrap_or_else(|p| p.into_inner()).take();
    session.release(&key);
    let Some(message) = message else {
        return vec![];
    };
    let original = selected.iter().position(|(_, selected)| *selected == slot);
    let mut messages: Vec<_> = selected
        .iter()
        .enumerate()
        .filter(|(i, _)| Some(*i) != original)
        .map(|(_, (consumer, slot))| targeted(*consumer, *slot, share_delivery(&message, *slot)))
        .collect();
    if let Some(original) = original {
        messages.push(targeted(
            selected[original].0,
            selected[original].1,
            message,
        ));
    }
    messages
}

fn targeted(consumer: u64, slot: DeliverySlot, message: AppMessage) -> AppMessage {
    if consumer == 0 {
        message
    } else {
        AppMessage::ConsumerDelivery {
            consumer,
            slot,
            message: Box::new(message),
        }
    }
}

fn share_delivery(message: &AppMessage, slot: DeliverySlot) -> AppMessage {
    match (message, slot) {
        (AppMessage::ClusteringDone { key, result, .. }, DeliverySlot::Cluster(source)) => {
            AppMessage::ClusteringDone {
                source,
                key: key.clone(),
                result: result.clone(),
            }
        }
        (AppMessage::ClusterFailed { err, .. }, DeliverySlot::Cluster(source)) => {
            AppMessage::ClusterFailed {
                source,
                err: err.clone(),
            }
        }
        (AppMessage::McdmDone { key, result, .. }, DeliverySlot::Mcdm(source)) => {
            AppMessage::McdmDone {
                source,
                key: key.clone(),
                result: result.clone(),
            }
        }
        (
            AppMessage::McdmFailed { message, .. },
            DeliverySlot::Mcdm(source) | DeliverySlot::Entropy(source),
        ) => AppMessage::McdmFailed {
            source,
            message: message.clone(),
        },
        (AppMessage::EntropyDone { result, .. }, DeliverySlot::Entropy(source)) => {
            AppMessage::EntropyDone {
                source,
                result: result.clone(),
            }
        }
        (
            AppMessage::PdpDone {
                param,
                objective,
                model_type,
                feasible_only,
                result,
            },
            _,
        ) => AppMessage::PdpDone {
            param: param.clone(),
            objective: objective.clone(),
            model_type: model_type.clone(),
            feasible_only: *feasible_only,
            result: result.clone(),
        },
        (AppMessage::Pdp2dDone(result), _) => AppMessage::Pdp2dDone(result.clone()),
        (
            AppMessage::IndicatorHistoryDone {
                #[cfg(test)]
                source_df,
                indicator,
                base,
                comparisons,
            },
            _,
        ) => AppMessage::IndicatorHistoryDone {
            #[cfg(test)]
            source_df: source_df.clone(),
            indicator: *indicator,
            base: base.clone(),
            comparisons: comparisons.clone(),
        },
        (AppMessage::ObservedContourDone(result), _) => {
            AppMessage::ObservedContourDone(result.clone())
        }
        (AppMessage::ObservedContourFailed(message), _) => {
            AppMessage::ObservedContourFailed(message.clone())
        }
        (AppMessage::SurrogateOptDone(result), _) => AppMessage::SurrogateOptDone(result.clone()),
        (AppMessage::SurrogateMultiOptDone(result), _) => {
            AppMessage::SurrogateMultiOptDone(result.clone())
        }
        (AppMessage::SurrogateMultiOptFailed(message), _) => {
            AppMessage::SurrogateMultiOptFailed(message.clone())
        }
        (AppMessage::SurrogateSuggestDone(result), _) => {
            AppMessage::SurrogateSuggestDone(result.clone())
        }
        (AppMessage::SurrogateSuggestFailed(message), _) => {
            AppMessage::SurrogateSuggestFailed(message.clone())
        }
        (AppMessage::SurrogateMultiSuggestDone(result), _) => {
            AppMessage::SurrogateMultiSuggestDone(result.clone())
        }
        (AppMessage::SurrogateMultiSuggestFailed(message), _) => {
            AppMessage::SurrogateMultiSuggestFailed(message.clone())
        }
        (AppMessage::SurrogateCompareDone(result), _) => {
            AppMessage::SurrogateCompareDone(result.clone())
        }
        (AppMessage::SurrogateCompareFailed(message), _) => {
            AppMessage::SurrogateCompareFailed(message.clone())
        }
        (AppMessage::AnalysisUnavailable(_), _) => AppMessage::AnalysisUnavailable(slot),
        (AppMessage::TaskPanicked(message), _) => AppMessage::TaskPanicked(message.clone()),
        (AppMessage::Error(message), _) => AppMessage::Error(message.clone()),
        (AppMessage::SensitivityError(message), _) => AppMessage::SensitivityError(message.clone()),
        (AppMessage::SurrogateFitFailed(message), _) => {
            AppMessage::SurrogateFitFailed(message.clone())
        }
        (AppMessage::SurrogateMultiFitFailed(message), _) => {
            AppMessage::SurrogateMultiFitFailed(message.clone())
        }
        (AppMessage::SurrogateFitCancelled, _) => AppMessage::SurrogateFitCancelled,
        (AppMessage::SurrogateMultiFitCancelled, _) => AppMessage::SurrogateMultiFitCancelled,
        (AppMessage::RobustnessFitFailed(message), _) => {
            AppMessage::RobustnessFitFailed(message.clone())
        }
        (AppMessage::ResponseSurfaceFitFailed(message), _) => {
            AppMessage::ResponseSurfaceFitFailed(message.clone())
        }
        _ => unreachable!("Every rebound analysis consumer requires a supported notification"),
    }
}

#[cfg(test)]
fn adopt(
    slot: DeliverySlot,
    completion: AnalysisCompletion,
    state: &AppState,
    widgets: &mut crate::ui::widget_states::WidgetStates,
) -> Option<AppMessage> {
    adopt_all(slot, completion, state, widgets)
        .into_iter()
        .next()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::state::app_state::{StudyContext, StudyView};
    use crate::state::journal_studies::JournalStudies;
    use crate::ui::widget_states::WidgetStates;
    use std::collections::BTreeMap;
    use std::sync::{mpsc, Arc};
    use std::time::Duration;
    use tdashboard_study::*;

    #[test]
    fn d5_notification_success_fanout_preserves_each_operations_payload() {
        use super::super::messages::*;
        let (mut state, _, _) = state(true);
        let cases = vec![
            (
                DeliverySlot::Convergence,
                AppMessage::IndicatorHistoryDone {
                    source_df: state.current_study.as_ref().unwrap().view.df.clone(),
                    indicator: tdashboard_analysis::indicators::MoIndicator::Hypervolume,
                    base: super::super::results::ConvergenceHistory {
                        trial_ids: vec![90],
                        values: vec![13.0],
                        sample_step: 1,
                        ref_point: vec![],
                    },
                    comparisons: vec![],
                },
            ),
            (
                DeliverySlot::Contour,
                AppMessage::ObservedContourDone(ObservedContourResult {
                    x_name: "x".into(),
                    y_name: "z".into(),
                    value_name: "y".into(),
                    surface: tdashboard_analysis::contour::ObservedSurface {
                        x_values: vec![1.0],
                        y_values: vec![2.0],
                        z: vec![vec![Some(13.0)]],
                    },
                    points: vec![[1.0, 2.0, 13.0]],
                    point_trial_ids: vec![90],
                }),
            ),
            (
                DeliverySlot::Optimize,
                AppMessage::SurrogateOptDone(SurrogateOptUiResult {
                    best_params: vec![("x".into(), 1.0)],
                    best_value: 13.0,
                    predicted_std: None,
                    r_squared: 1.0,
                    objective_name: "y".into(),
                    minimize: true,
                    best_observed_value: 14.0,
                    predicted_constraints: vec![],
                    feasibility_probability: None,
                }),
            ),
            (
                DeliverySlot::MultiOptimize,
                AppMessage::SurrogateMultiOptDone(SurrogateMultiOptUiResult {
                    param_names: vec!["x".into()],
                    objective_names: vec!["y".into()],
                    front: vec![],
                    r_squared: vec![13.0],
                }),
            ),
            (
                DeliverySlot::Suggest,
                AppMessage::SurrogateSuggestDone(SurrogateSuggestUiResult {
                    candidates: vec![],
                    param_names: vec!["x".into()],
                    objective_name: "y".into(),
                }),
            ),
            (
                DeliverySlot::MultiSuggest,
                AppMessage::SurrogateMultiSuggestDone(SurrogateMultiSuggestUiResult {
                    candidates: vec![],
                    param_names: vec!["x".into()],
                    objective_names: vec!["y".into()],
                }),
            ),
            (
                DeliverySlot::Compare,
                AppMessage::SurrogateCompareDone(Arc::new(SurrogateCompareUiResult {
                    rows: vec![],
                    slices: vec![],
                    observed: vec![(1.0, 13.0)],
                    param_name: "x".into(),
                    objective_name: "y".into(),
                    anchor: vec![1.0],
                })),
            ),
        ];
        for (slot, message) in cases {
            for _ in 0..2 {
                let delivery = share_delivery(&message, slot);
                assert_eq!(
                    std::mem::discriminant(&delivery),
                    std::mem::discriminant(&message)
                );
                let mut widgets = WidgetStates::default();
                widgets.convergence.computing = true;
                widgets.observed_contour.computing = true;
                widgets.surrogate_opt.optimizing = true;
                widgets.surrogate_opt.suggesting = true;
                widgets.surrogate_opt.multi_suggesting = true;
                widgets.surrogate_compare.computing = true;
                crate::state::message_handler::MessageHandler::handle(
                    delivery,
                    &mut state,
                    &mut widgets,
                    &mut false,
                    &mut None,
                );
                match slot {
                    DeliverySlot::Convergence => {
                        assert!(!widgets.convergence.computing);
                        assert_eq!(state.convergence_history.as_ref().unwrap().values, [13.0]);
                    }
                    DeliverySlot::Contour => {
                        assert!(!widgets.observed_contour.computing);
                        assert_eq!(
                            widgets
                                .observed_contour
                                .result
                                .as_ref()
                                .unwrap()
                                .point_trial_ids,
                            [90]
                        );
                    }
                    DeliverySlot::Optimize => {
                        assert!(!widgets.surrogate_opt.optimizing);
                        assert_eq!(
                            widgets.surrogate_opt.result.as_ref().unwrap().best_value,
                            13.0
                        );
                    }
                    DeliverySlot::MultiOptimize => {
                        assert!(!widgets.surrogate_opt.optimizing);
                        assert_eq!(
                            widgets
                                .surrogate_opt
                                .multi_result
                                .as_ref()
                                .unwrap()
                                .r_squared,
                            [13.0]
                        );
                    }
                    DeliverySlot::Suggest => {
                        assert!(!widgets.surrogate_opt.suggesting);
                        assert_eq!(
                            widgets
                                .surrogate_opt
                                .suggest_result
                                .as_ref()
                                .unwrap()
                                .objective_name,
                            "y"
                        );
                    }
                    DeliverySlot::MultiSuggest => {
                        assert!(!widgets.surrogate_opt.multi_suggesting);
                        assert_eq!(
                            widgets
                                .surrogate_opt
                                .multi_suggest_result
                                .as_ref()
                                .unwrap()
                                .objective_names,
                            ["y"]
                        );
                    }
                    DeliverySlot::Compare => {
                        assert!(!widgets.surrogate_compare.computing);
                        assert_eq!(
                            widgets.surrogate_compare.result.as_ref().unwrap().observed,
                            [(1.0, 13.0)]
                        );
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    #[test]
    fn d5_notification_failure_fanout_preserves_diagnostics_and_unavailable_slots() {
        use crate::state::messages::McdmChartSource;
        let cases = [
            (
                DeliverySlot::Contour,
                AppMessage::ObservedContourFailed("contour".into()),
            ),
            (
                DeliverySlot::MultiOptimize,
                AppMessage::SurrogateMultiOptFailed("optimization".into()),
            ),
            (
                DeliverySlot::Suggest,
                AppMessage::SurrogateSuggestFailed("suggestion".into()),
            ),
            (
                DeliverySlot::MultiSuggest,
                AppMessage::SurrogateMultiSuggestFailed("multi suggestion".into()),
            ),
            (
                DeliverySlot::Compare,
                AppMessage::SurrogateCompareFailed("comparison".into()),
            ),
        ];
        for (slot, message) in cases {
            let shared = share_delivery(&message, slot);
            let text = |message| match message {
                AppMessage::ObservedContourFailed(s)
                | AppMessage::SurrogateMultiOptFailed(s)
                | AppMessage::SurrogateSuggestFailed(s)
                | AppMessage::SurrogateMultiSuggestFailed(s)
                | AppMessage::SurrogateCompareFailed(s) => s,
                _ => panic!("The notification must retain its failure variant"),
            };
            assert_eq!(text(shared), text(message));
        }
        let slot = DeliverySlot::Entropy(McdmChartSource::Table);
        assert!(
            matches!(share_delivery(&AppMessage::AnalysisUnavailable(DeliverySlot::Entropy(McdmChartSource::Rank)), slot), AppMessage::AnalysisUnavailable(selected) if selected == slot)
        );
    }

    pub(crate) fn state(complete: bool) -> (AppState, LoadTicket, ApplyReceipt) {
        let mut journal = JournalStudies::new(1).unwrap();
        journal.selected = Some(0);
        let key = StudyKey {
            source: journal.source,
            stored_id: 0,
        };
        let mut manager = StudyManager::new(key);
        let ticket = manager.begin(LoadKind::Initial).unwrap();
        let metadata = StudyMetadata {
            name: "same name".into(),
            objectives: vec![ObjectiveDescriptor {
                index: ObjectiveIndex(0),
                name: Some("y".into()),
                direction: Direction::Minimize,
            }],
            ..Default::default()
        };
        let trials = (0..20)
            .map(|i| {
                let mut t = TrialRecord::new(
                    key,
                    90 + i,
                    StudyTrialNumber(5 + i * 3),
                    TrialState::Complete,
                );
                t.completed_order = Some(u64::from(i));
                t.objectives = Arc::new(BTreeMap::from([(
                    ObjectiveIndex(0),
                    Number(f64::from(i) * 2.0),
                )]));
                t.parameters = Arc::new(BTreeMap::from([(
                    "x".into(),
                    ParameterObservation {
                        value: ParameterValue::Numeric(Number(f64::from(i))),
                        distribution: None,
                        external_encoding: false,
                        encoded_numeric: None,
                    },
                )]));
                Arc::new(t)
            })
            .collect();
        let receipt = manager
            .apply(
                manager.header(ticket).unwrap(),
                StudyBatch::Replace { trials, metadata },
            )
            .unwrap();
        if complete {
            manager.finish(ticket, receipt.next_sequence).unwrap();
        }
        journal.managers.insert(0, manager);
        let mut state = AppState {
            journal_studies: Some(journal),
            ..Default::default()
        };
        install(&mut state);
        (state, ticket, receipt)
    }

    fn install(state: &mut AppState) {
        let journal = state.journal_studies.as_ref().unwrap();
        let input = journal.managers[&journal.selected.unwrap_or(0)]
            .acquire()
            .unwrap();
        let p = journal.analysis.borrow_mut().project(input.clone());
        state.journal_display = Some(input.stamp());
        state.current_study = Some(StudyContext {
            meta: crate::io::journal::convert_projection_meta(p.meta.clone()),
            view: StudyView::from_projection(p, input, None),
            pareto_indices: vec![],
        });
    }

    pub(crate) fn d5_front_state() -> AppState {
        let (mut state, _, _) = state(true);
        let manager = state
            .journal_studies
            .as_mut()
            .unwrap()
            .managers
            .get_mut(&0)
            .unwrap();
        let input = manager.acquire().unwrap();
        let trials = input
            .data()
            .trials()
            .iter()
            .map(|t| {
                let mut t = (**t).clone();
                Arc::make_mut(&mut t.objectives).insert(ObjectiveIndex(0), Number(0.0));
                if t.key.stored_id == 92 {
                    Arc::make_mut(&mut t.parameters).clear();
                }
                Arc::new(t)
            })
            .collect();
        let ticket = manager.begin(LoadKind::Refresh).unwrap();
        let receipt = manager
            .apply(
                manager.header(ticket).unwrap(),
                StudyBatch::Replace {
                    trials,
                    metadata: (**input.data().metadata()).clone(),
                },
            )
            .unwrap();
        manager.finish(ticket, receipt.next_sequence).unwrap();
        install(&mut state);
        state
    }

    fn d5_work(state: &AppState, cluster: bool, condition: usize, consumer: u64) -> AnalysisWork {
        use crate::state::messages::{ClusterChartSource as C, McdmChartSource as M};
        let journal = state.journal_studies.as_ref().unwrap();
        let input = state.current_study.as_ref().unwrap().view.input().unwrap();
        let mut session = journal.analysis.borrow_mut();
        let front = session.pareto(input.clone()).unwrap();
        let slot = if cluster {
            DeliverySlot::Cluster(C::Table)
        } else {
            DeliverySlot::Mcdm(M::Table)
        };
        let selection = if cluster {
            Preparation::Cluster {
                space: tdashboard_analysis::preparation::FeatureSpace::Parameters,
                manual_k: Some(2),
            }
        } else {
            Preparation::Mcdm
        };
        let prepared = session
            .prepare_for(
                vec![input],
                AnalysisRequest {
                    kind: slot.kind(),
                    conditions: vec![("condition".into(), condition.into())],
                },
                selection,
                &[front],
            )
            .unwrap();
        let WorkOrShared::Work(work) = session.start(prepared, ReuseOrRerun::Rerun).unwrap() else {
            panic!()
        };
        select_job(journal, &mut session, slot, consumer, Some(work.key()));
        work
    }

    fn d5_success(work: AnalysisWork, cluster: bool, value: f64) -> AnalysisCompletion {
        work.execute_shared(|prepared| {
            let rows = prepared.tables()[0].rows.clone();
            if cluster {
                let mut labels = vec![-1; rows.len()];
                for &row in &prepared.data().features().rows {
                    labels[row] = 0;
                }
                Ok(Arc::new(KeyedResult {
                    selection: crate::ui::widgets::cluster_scatter::ClusterCacheKey::new(
                        crate::ui::widgets::cluster_scatter::ClusterSpace::Variable,
                        crate::ui::widgets::cluster_scatter::KSelectionMode::Manual,
                        2,
                        crate::ui::widgets::cluster_scatter::KMeansInitStrategy::Deterministic,
                        10,
                    ),
                    value: Arc::new(super::super::results::ClusterResult {
                        labels,
                        n_clusters: 2,
                    }),
                    rows,
                }) as Arc<dyn std::any::Any + Send + Sync>)
            } else {
                let weight_mode = if value == 1.0 {
                    super::super::results::WeightMode::Manual
                } else {
                    super::super::results::WeightMode::Entropy
                };
                Ok(Arc::new(KeyedResult {
                    selection: crate::ui::widgets::mcdm_chart::McdmCacheKey::from_settings(
                        super::super::results::McdmMethod::Topsis,
                        weight_mode,
                        &[1.0],
                        0.5,
                    )
                    .unwrap(),
                    value: Arc::new(super::super::results::McdmResult::Topsis(
                        super::super::results::TopsisResult {
                            scores: vec![value; rows.len()],
                            ranked_indices: (0..rows.len() as u32).rev().collect(),
                            duration_ms: 0.0,
                        },
                    )),
                    rows,
                }))
            }
        })
    }

    fn d5_deliver(
        state: &mut AppState,
        cluster: bool,
        completion: AnalysisCompletion,
    ) -> Vec<ResultHandle> {
        use crate::state::messages::{ClusterChartSource as C, McdmChartSource as M};
        let slot = if cluster {
            DeliverySlot::Cluster(C::Table)
        } else {
            DeliverySlot::Mcdm(M::Table)
        };
        let mut widgets = WidgetStates::default();
        let messages = adopt_all(slot, completion, state, &mut widgets);
        let mut handles = vec![];
        for message in messages {
            let message = match message {
                AppMessage::ConsumerDelivery { message, .. } => *message,
                m => m,
            };
            if let AppMessage::SharedAnalysis { result, .. } = &message {
                handles.push(result.clone());
            }
            crate::state::message_handler::MessageHandler::handle(
                message,
                state,
                &mut widgets,
                &mut false,
                &mut None,
            );
        }
        handles
    }

    #[test]
    fn d5_typed_completed_and_inflight_results_share_payload_and_trial_maps() {
        for cluster in [true, false] {
            let mut state = d5_front_state();
            let work = d5_work(&state, cluster, 1, 11);
            let key = work.key().clone();
            let journal = state.journal_studies.as_ref().unwrap();
            let input = state.current_study.as_ref().unwrap().view.input().unwrap();
            let mut session = journal.analysis.borrow_mut();
            let front = session.pareto(input.clone()).unwrap();
            let selection = if cluster {
                Preparation::Cluster {
                    space: tdashboard_analysis::preparation::FeatureSpace::Parameters,
                    manual_k: Some(2),
                }
            } else {
                Preparation::Mcdm
            };
            // Reconstruct the request without the preparation-owned condition.
            let mut request = key.analysis.request.clone();
            request.conditions.pop();
            let prepared = session
                .prepare_for(vec![input], request, selection, &[front])
                .unwrap();
            let WorkOrShared::Shared(pending) =
                session.start(prepared, ReuseOrRerun::Reuse).unwrap()
            else {
                panic!("Matching conditions must share pending work")
            };
            let slot = if cluster {
                DeliverySlot::Cluster(crate::state::messages::ClusterChartSource::Table)
            } else {
                DeliverySlot::Mcdm(crate::state::messages::McdmChartSource::Table)
            };
            select_job(journal, &mut session, slot, 22, Some(&key));
            drop(session);
            let handles = d5_deliver(&mut state, cluster, d5_success(work, cluster, 1.0));
            assert_eq!(handles.len(), 2);
            assert!(handles[0].same_result(&handles[1]));
            assert!(pending.same_result(&handles[0]));
            assert_eq!(handles[0].rows()[0].len(), if cluster { 19 } else { 20 });
            if cluster {
                assert!(!handles[0].rows()[0].iter().any(|key| key.stored_id == 92));
            }
            let cached = state
                .journal_studies
                .as_ref()
                .unwrap()
                .analysis
                .borrow()
                .lookup(&key.analysis)
                .unwrap();
            assert!(cached.same_result(&handles[0]));
            assert_eq!(
                cached.key().analysis.inputs[0],
                state
                    .current_study
                    .as_ref()
                    .unwrap()
                    .view
                    .input()
                    .unwrap()
                    .stamp()
            );
            if cluster {
                let selected = state
                    .cluster_cache
                    .selected(&crate::ui::widgets::cluster_scatter::ClusterCacheKey::new(
                        crate::ui::widgets::cluster_scatter::ClusterSpace::Variable,
                        crate::ui::widgets::cluster_scatter::KSelectionMode::Manual,
                        2,
                        crate::ui::widgets::cluster_scatter::KMeansInitStrategy::Deterministic,
                        10,
                    ))
                    .unwrap();
                assert_eq!(selected.labels[2], -1);
                assert_eq!(selected.rows()[2].stored_id, 92);
                assert_eq!(
                    state
                        .current_study
                        .as_ref()
                        .unwrap()
                        .view
                        .df
                        .get_trial_number(2),
                    Some(11)
                );
            }
        }
    }

    #[test]
    fn d5_cluster_and_mcdm_reject_stale_success_error_and_cancellation() {
        for cluster in [true, false] {
            for change in ["revision", "reload", "source", "study", "rerun"] {
                for outcome in ["success", "error", "cancelled"] {
                    let mut state = d5_front_state();
                    let work = d5_work(&state, cluster, 1, 0);
                    let completion = if outcome == "success" {
                        d5_success(work, cluster, 1.0)
                    } else {
                        work.execute::<()>(|_| Err(outcome.into()))
                    };
                    if change == "source" {
                        state = d5_front_state();
                    } else if change == "rerun" {
                        let _new_work = d5_work(&state, cluster, 1, 0);
                    } else {
                        let journal = state.journal_studies.as_mut().unwrap();
                        let old = journal.managers[&0].acquire().unwrap();
                        if change == "study" {
                            let key = StudyKey {
                                source: journal.source,
                                stored_id: 1,
                            };
                            let mut manager = StudyManager::new(key);
                            let ticket = manager.begin(LoadKind::Initial).unwrap();
                            let trials = old
                                .data()
                                .trials()
                                .iter()
                                .map(|t| {
                                    let mut t = (**t).clone();
                                    t.study = key;
                                    t.key.stored_id += 100;
                                    Arc::new(t)
                                })
                                .collect();
                            let receipt = manager
                                .apply(
                                    manager.header(ticket).unwrap(),
                                    StudyBatch::Replace {
                                        trials,
                                        metadata: (**old.data().metadata()).clone(),
                                    },
                                )
                                .unwrap();
                            manager.finish(ticket, receipt.next_sequence).unwrap();
                            journal.managers.insert(1, manager);
                            journal.selected = Some(1);
                        } else {
                            let manager = journal.managers.get_mut(&0).unwrap();
                            let ticket = manager
                                .begin(if change == "reload" {
                                    LoadKind::Reload
                                } else {
                                    LoadKind::Refresh
                                })
                                .unwrap();
                            let mut metadata = (**old.data().metadata()).clone();
                            metadata.name.push('!');
                            let receipt = manager
                                .apply(
                                    manager.header(ticket).unwrap(),
                                    StudyBatch::Replace {
                                        trials: old.data().trials().as_ref().clone(),
                                        metadata,
                                    },
                                )
                                .unwrap();
                            manager.finish(ticket, receipt.next_sequence).unwrap();
                        }
                        install(&mut state);
                    }
                    assert_eq!(state.current_study.as_ref().unwrap().view.row_count(), 20);
                    let slot = if cluster {
                        DeliverySlot::Cluster(crate::state::messages::ClusterChartSource::Table)
                    } else {
                        DeliverySlot::Mcdm(crate::state::messages::McdmChartSource::Table)
                    };
                    assert!(
                        adopt_all(slot, completion, &state, &mut WidgetStates::default())
                            .is_empty(),
                        "{cluster} {change} {outcome}"
                    );
                    assert!(state.cluster_cache.is_empty());
                    assert!(state.mcdm_cache.is_empty());
                }
            }
        }
    }

    #[test]
    fn d5_coloring_keeps_the_last_computation_not_the_table_selection() {
        let mut state = d5_front_state();
        let a = d5_work(&state, false, 1, 0);
        let a = d5_deliver(&mut state, false, d5_success(a, false, 1.0))
            .pop()
            .unwrap();
        let b = d5_work(&state, false, 2, 0);
        let b = d5_deliver(&mut state, false, d5_success(b, false, 2.0))
            .pop()
            .unwrap();
        assert_ne!(a.key().analysis, b.key().analysis);
        assert!(state.mcdm_result.as_ref().unwrap().handle().same_result(&b));
        let table = crate::ui::widgets::mcdm_chart::McdmCacheKey::from_settings(
            super::super::results::McdmMethod::Topsis,
            super::super::results::WeightMode::Manual,
            &[1.0],
            0.5,
        )
        .unwrap();
        assert!(state
            .mcdm_cache
            .selected(&table)
            .unwrap()
            .handle()
            .same_result(&a));
        assert_eq!(
            state.mcdm_cache.get(&table).unwrap().primary_scores()[0],
            1.0
        );
        assert_eq!(state.mcdm_result.as_ref().unwrap().primary_scores()[0], 2.0);
        let weak = Arc::downgrade(
            &a.payload::<KeyedResult<
                crate::ui::widgets::mcdm_chart::McdmCacheKey,
                super::super::results::McdmResult,
            >>()
            .unwrap()
            .unwrap(),
        );
        state.mcdm_cache.retain(|key| key != &table);
        state
            .journal_studies
            .as_ref()
            .unwrap()
            .analysis
            .borrow_mut()
            .retain_interests(&[b.key().analysis.clone()]);
        assert!(
            weak.upgrade().is_some(),
            "An external retained result stays valid"
        );
        drop(a);
        assert!(
            weak.upgrade().is_none(),
            "Uninterested completed payloads are released"
        );
    }

    fn completion(state: &AppState, text: &'static str) -> AnalysisCompletion {
        let (tx, rx) = mpsc::sync_channel(1);
        spawn(
            state,
            tx,
            DeliverySlot::Importance,
            "same request",
            Preparation::Sensitivity {
                method: tdashboard_analysis::sensitivity::heatmap::HeatmapMethod::Spearman,
                objective: Some(0),
                feasible_only: false,
                samples: 32,
            },
            move |_| AppMessage::SensitivityError(text.into()),
        );
        match rx.recv_timeout(Duration::from_secs(30)).unwrap() {
            AppMessage::AnalysisCompleted { completion, .. } => completion,
            _ => panic!("Production acquisition must use the stamped handoff"),
        }
    }

    #[test]
    fn d4_entry_rejects_same_count_revision_load_epoch_and_source_results() {
        let (mut state, ticket, receipt) = state(false);
        let mut widgets = WidgetStates::default();
        let old = completion(&state, "old loading failure");
        state
            .journal_studies
            .as_mut()
            .unwrap()
            .managers
            .get_mut(&0)
            .unwrap()
            .finish(ticket, receipt.next_sequence)
            .unwrap();
        install(&mut state);
        widgets.importance.computing = true;
        assert!(adopt(DeliverySlot::Importance, old, &state, &mut widgets).is_none());
        assert!(!widgets.importance.computing);

        let old = completion(&state, "old revision failure");
        let manager = state
            .journal_studies
            .as_mut()
            .unwrap()
            .managers
            .get_mut(&0)
            .unwrap();
        let input = manager.acquire().unwrap();
        let mut trial = (**input.data().trials().first().unwrap()).clone();
        Arc::make_mut(&mut trial.objectives).insert(ObjectiveIndex(0), Number(123.0));
        let ticket = manager.begin(LoadKind::Refresh).unwrap();
        manager
            .apply(
                manager.header(ticket).unwrap(),
                StudyBatch::Delta {
                    upserts: vec![Arc::new(trial)],
                    removals: vec![],
                    metadata: MetadataEdit::Unchanged,
                },
            )
            .unwrap();
        install(&mut state);
        assert_eq!(state.current_study.as_ref().unwrap().view.row_count(), 20);
        assert!(adopt(DeliverySlot::Importance, old, &state, &mut widgets).is_none());

        let old = completion(&state, "old source failure");
        let (replacement, _, _) = self::state(true);
        assert!(adopt(DeliverySlot::Importance, old, &replacement, &mut widgets).is_none());
    }

    #[test]
    fn d4_entry_rerun_rejects_predecessor_without_clearing_the_new_busy_slot() {
        let (state, _, _) = state(true);
        let old = completion(&state, "old");
        let current = completion(&state, "current");
        assert_ne!(old.key(), current.key());
        let mut widgets = WidgetStates::default();
        widgets.importance.computing = true;
        assert!(adopt(DeliverySlot::Importance, old, &state, &mut widgets).is_none());
        assert!(widgets.importance.computing);
        assert!(
            matches!(adopt(DeliverySlot::Importance, current, &state, &mut widgets), Some(AppMessage::SensitivityError(text)) if text == "current")
        );
        assert!(!widgets.importance.computing);
    }

    #[test]
    fn d4_entry_fit_retains_actual_input_and_condition_provenance() {
        let (mut state, _, _) = state(true);
        let manager = state
            .journal_studies
            .as_mut()
            .unwrap()
            .managers
            .get_mut(&0)
            .unwrap();
        let mut missing = (**manager.acquire().unwrap().data().trials().get(2).unwrap()).clone();
        Arc::make_mut(&mut missing.parameters).clear();
        let ticket = manager.begin(LoadKind::Refresh).unwrap();
        let receipt = manager
            .apply(
                manager.header(ticket).unwrap(),
                StudyBatch::Delta {
                    upserts: vec![Arc::new(missing)],
                    removals: vec![],
                    metadata: MetadataEdit::Unchanged,
                },
            )
            .unwrap();
        manager.finish(ticket, receipt.next_sequence).unwrap();
        install(&mut state);
        let input = state.current_study.as_ref().unwrap().view.input().unwrap();
        let (tx, rx) = mpsc::sync_channel(1);
        spawn(
            &state,
            tx,
            DeliverySlot::Fit,
            ("y", "Ridge", false),
            Preparation::Fit {
                objectives: tdashboard_analysis::preparation::FitObjectives::Name("y".into()),
                constraints: false,
            },
            move |prepared| {
                use tdashboard_analysis::surrogate_opt::*;
                let data = prepared.data().fit();
                let req = SurrogateFitRequest {
                    x_matrix: data.x.clone(),
                    y: data.ys[0].clone(),
                    param_names: data.names.clone(),
                    objective_name: "y".into(),
                    model: SurrogateModelKind::Ridge,
                    auto_select: false,
                    constraints: vec![],
                    priority_rows: vec![],
                    param_bounds: Some(data.bounds.clone()),
                };
                AppMessage::SurrogateFitDone(Arc::new(fit_surrogate_with_validation(&req).unwrap()))
            },
        );
        let AppMessage::AnalysisCompleted { completion, slot } =
            rx.recv_timeout(Duration::from_secs(30)).unwrap()
        else {
            panic!()
        };
        let key = completion.key().clone();
        let mut widgets = WidgetStates::default();
        let Some(AppMessage::SurrogateFitDone(trained)) =
            adopt(slot, completion, &state, &mut widgets)
        else {
            panic!()
        };
        assert_eq!(trained.fit_key(), Some(&key));
        assert_eq!(key.analysis.inputs, [input.stamp()]);
        assert_eq!(trained.fit_rows().len(), 19);
        assert_eq!(trained.fit_rows()[0].stored_id, 90);
        assert!(!trained.fit_rows().iter().any(|key| key.stored_id == 92));
        assert!(!key.analysis.request.conditions.is_empty());
        let journal = state.journal_studies.as_ref().unwrap();
        let mut session = journal.analysis.borrow_mut();
        assert!(session
            .lookup(&key.analysis)
            .unwrap()
            .payload::<tdashboard_analysis::surrogate_opt::TrainedSurrogate>()
            .unwrap()
            .is_ok());
        let prepared = session
            .prepare_models(
                vec![input.clone()],
                AnalysisRequest {
                    kind: AnalysisKind::Optimize,
                    conditions: vec![],
                },
                tdashboard_analysis::preparation::Models::Single(trained.clone()),
            )
            .unwrap();
        assert_eq!(prepared.key().dependencies, [key]);
        drop(prepared);
        drop(session);
        let manager = state
            .journal_studies
            .as_mut()
            .unwrap()
            .managers
            .get_mut(&0)
            .unwrap();
        let ticket = manager.begin(LoadKind::Refresh).unwrap();
        let mut metadata = (**input.data().metadata()).clone();
        metadata.name.push('!');
        let receipt = manager
            .apply(
                manager.header(ticket).unwrap(),
                StudyBatch::Replace {
                    trials: input.data().trials().as_ref().clone(),
                    metadata,
                },
            )
            .unwrap();
        manager.finish(ticket, receipt.next_sequence).unwrap();
        install(&mut state);
        let (tx, rx) = mpsc::sync_channel(1);
        spawn_with_models(
            &state,
            tx,
            DeliverySlot::Optimize,
            "same fit",
            tdashboard_analysis::preparation::Models::Single(trained),
            |_| panic!("A stale fit must not reach optimization"),
        );
        let AppMessage::AnalysisCompleted { slot, completion } =
            rx.recv_timeout(Duration::from_secs(30)).unwrap()
        else {
            panic!()
        };
        assert!(
            matches!(adopt(slot, completion, &state, &mut widgets), Some(AppMessage::SurrogateFitFailed(message)) if message == "Trained model belongs to another input")
        );
    }

    #[test]
    fn d4_unavailable_preparation_is_keyed_and_never_executes_the_calculation() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let (mut state, _, _) = state(true);
        let manager = state
            .journal_studies
            .as_mut()
            .unwrap()
            .managers
            .get_mut(&0)
            .unwrap();
        let input = manager.acquire().unwrap();
        let trials = input
            .data()
            .trials()
            .iter()
            .map(|t| {
                let mut t = (**t).clone();
                Arc::make_mut(&mut t.parameters).clear();
                Arc::new(t)
            })
            .collect();
        let ticket = manager.begin(LoadKind::Refresh).unwrap();
        let receipt = manager
            .apply(
                manager.header(ticket).unwrap(),
                StudyBatch::Replace {
                    trials,
                    metadata: (**input.data().metadata()).clone(),
                },
            )
            .unwrap();
        manager.finish(ticket, receipt.next_sequence).unwrap();
        install(&mut state);
        let stamp = state
            .current_study
            .as_ref()
            .unwrap()
            .view
            .input()
            .unwrap()
            .stamp();
        let calls = Arc::new(AtomicUsize::new(0));
        let run = calls.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        spawn(
            &state,
            tx,
            DeliverySlot::Fit,
            "Ridge",
            Preparation::Fit {
                objectives: tdashboard_analysis::preparation::FitObjectives::Name("y".into()),
                constraints: false,
            },
            move |_| {
                run.fetch_add(1, Ordering::SeqCst);
                panic!("An unavailable preparation must not execute")
            },
        );
        let AppMessage::AnalysisCompleted { slot, completion } =
            rx.recv_timeout(Duration::from_secs(30)).unwrap()
        else {
            panic!()
        };
        assert_eq!(completion.key().analysis.inputs, [stamp]);
        let mut widgets = WidgetStates::default();
        widgets.surrogate_opt.fitting = true;
        assert!(
            matches!(adopt(slot, completion, &state, &mut widgets), Some(AppMessage::SurrogateFitFailed(message)) if message == "No numeric parameters available")
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(!widgets.surrogate_opt.fitting);
    }

    #[test]
    fn d4_publication_invalidates_same_count_schema_results_without_resetting_controls() {
        let (mut state, _, _) = state(true);
        state.selected_indices = vec![90];
        state.hv_ref_point_override = Some(vec![100.0]);
        state.sobol_cache.select(
            (0, false),
            fixture_result(crate::state::results::SobolResult {
                param_names: vec!["x".into()],
                first_order: vec![vec![1.0]],
                total_effect: vec![vec![1.0]],
                r_squared: vec![1.0],
            }),
        );
        let mut widgets = WidgetStates::default();
        widgets.surrogate_opt.auto_select = true;
        widgets.surrogate_opt.fitting = true;
        let manager = state
            .journal_studies
            .as_mut()
            .unwrap()
            .managers
            .get_mut(&0)
            .unwrap();
        let input = manager.acquire().unwrap();
        let key = input.stamp().input.study;
        let mut evidence = TrialRecord::new(key, 500, StudyTrialNumber(99), TrialState::Pruned);
        evidence.constraints = ConstraintObservation {
            present: true,
            values: vec![Number(-1.0), Number(-1.0)].into(),
        };
        let ticket = manager.begin(LoadKind::Refresh).unwrap();
        let receipt = manager
            .apply(
                manager.header(ticket).unwrap(),
                StudyBatch::Delta {
                    upserts: vec![Arc::new(evidence)],
                    removals: vec![],
                    metadata: MetadataEdit::Unchanged,
                },
            )
            .unwrap();
        manager.finish(ticket, receipt.next_sequence).unwrap();
        crate::state::message_handler::MessageHandler::handle(
            AppMessage::JournalActivate { key },
            &mut state,
            &mut widgets,
            &mut false,
            &mut None,
        );
        assert!(state.sobol_cache.is_empty());
        assert_eq!(state.selected_indices, [90]);
        assert_eq!(state.hv_ref_point_override, Some(vec![100.0]));
        assert!(widgets.surrogate_opt.auto_select);
        assert!(!widgets.surrogate_opt.fitting);
        let view = &state.current_study.as_ref().unwrap().view;
        assert_eq!(view.row_count(), 20);
        assert_eq!(view.feasibility().state(0), FeasibilityState::Unverified);
    }

    #[test]
    fn d4_matching_widget_conditions_share_one_inflight_calculation() {
        use crate::state::messages::McdmChartSource;
        use std::sync::atomic::{AtomicUsize, Ordering};
        let (state, _, _) = state(true);
        let (tx, rx) = mpsc::sync_channel(2);
        let (release, wait) = mpsc::channel();
        let count = Arc::new(AtomicUsize::new(0));
        let first = count.clone();
        spawn(
            &state,
            tx.clone(),
            DeliverySlot::Mcdm(McdmChartSource::Scatter2D),
            ("objective", 2usize),
            Preparation::Mcdm,
            move |_| {
                first.fetch_add(1, Ordering::SeqCst);
                wait.recv_timeout(Duration::from_secs(30)).unwrap();
                AppMessage::McdmFailed {
                    source: McdmChartSource::Scatter2D,
                    message: "fixture".into(),
                }
            },
        );
        let second = count.clone();
        spawn(
            &state,
            tx,
            DeliverySlot::Mcdm(McdmChartSource::Table),
            ("objective", 2usize),
            Preparation::Mcdm,
            move |_| {
                second.fetch_add(1, Ordering::SeqCst);
                panic!("Matching settings must not start a second calculation")
            },
        );
        release.send(()).unwrap();
        let AppMessage::AnalysisCompleted { slot, completion } =
            rx.recv_timeout(Duration::from_secs(30)).unwrap()
        else {
            panic!()
        };
        let mut widgets = WidgetStates::default();
        widgets.scatter_chart.controls.computing = true;
        widgets.trial_table.mcdm.controls.computing = true;
        let delivered = adopt_all(slot, completion, &state, &mut widgets);
        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert_eq!(delivered.len(), 2);
        assert!(delivered.iter().any(|m| matches!(
            m,
            AppMessage::McdmFailed {
                source: McdmChartSource::Table,
                ..
            }
        )));
        assert!(!widgets.scatter_chart.controls.computing);
        assert!(!widgets.trial_table.mcdm.controls.computing);
    }

    fn empty_front_state() -> AppState {
        let (mut state, _, _) = state(true);
        let manager = state
            .journal_studies
            .as_mut()
            .unwrap()
            .managers
            .get_mut(&0)
            .unwrap();
        let input = manager.acquire().unwrap();
        let trials = input
            .data()
            .trials()
            .iter()
            .map(|t| {
                let mut t = (**t).clone();
                t.constraints = ConstraintObservation {
                    present: true,
                    values: vec![Number(1.0)].into(),
                };
                Arc::new(t)
            })
            .collect();
        let ticket = manager.begin(LoadKind::Refresh).unwrap();
        let receipt = manager
            .apply(
                manager.header(ticket).unwrap(),
                StudyBatch::Replace {
                    trials,
                    metadata: (**input.data().metadata()).clone(),
                },
            )
            .unwrap();
        manager.finish(ticket, receipt.next_sequence).unwrap();
        install(&mut state);
        state
    }

    fn request_empty_front(
        state: &AppState,
        tx: &mpsc::SyncSender<AppMessage>,
        source: super::super::messages::McdmChartSource,
        settings: &'static str,
    ) {
        spawn(
            state,
            tx.clone(),
            DeliverySlot::Mcdm(source),
            settings,
            Preparation::Mcdm,
            |_| panic!("An empty-front request must not execute math"),
        );
    }

    fn receive_rejection(rx: &mpsc::Receiver<AppMessage>) -> (DeliverySlot, AnalysisCompletion) {
        match rx.recv_timeout(Duration::from_secs(30)).unwrap() {
            AppMessage::AnalysisCompleted { slot, completion } => (slot, completion),
            _ => panic!("A rejected preparation must use keyed completion delivery"),
        }
    }

    #[test]
    fn d4_rejections_reach_both_current_widgets_in_either_completion_order() {
        use crate::state::messages::McdmChartSource as M;
        for settings_rejection in [false, true] {
            for newest_first in [false, true] {
                let mut state = empty_front_state();
                let (tx, rx) = mpsc::sync_channel(2);
                for source in [M::Rank, M::Table] {
                    if settings_rejection {
                        reject_settings(
                            &state,
                            tx.clone(),
                            DeliverySlot::Mcdm(source),
                            "invalid weights".into(),
                        );
                    } else {
                        request_empty_front(&state, &tx, source, "same settings");
                    }
                }
                let mut completions = vec![receive_rejection(&rx), receive_rejection(&rx)];
                completions.sort_by_key(|(_, c)| c.key().run);
                assert_ne!(completions[0].1.key(), completions[1].1.key());
                if newest_first {
                    completions.reverse();
                }
                let mut widgets = WidgetStates::default();
                widgets.mcdm_chart.controls.computing = true;
                widgets.trial_table.mcdm.controls.computing = true;
                let expected = if settings_rejection {
                    "invalid weights"
                } else {
                    "MCDM: Pareto front is empty. Run the optimizer first."
                };
                let mut targets = vec![];
                let mut load_error = None;
                for (slot, completion) in completions {
                    for message in adopt_all(slot, completion, &state, &mut widgets) {
                        let AppMessage::McdmFailed {
                            source,
                            message: text,
                        } = &message
                        else {
                            panic!("Each current requester must receive its failure")
                        };
                        assert_eq!(text, expected);
                        targets.push(*source);
                        crate::state::message_handler::MessageHandler::handle(
                            message,
                            &mut state,
                            &mut widgets,
                            &mut false,
                            &mut load_error,
                        );
                    }
                }
                assert_eq!(targets.len(), 2, "Both current widgets need error delivery");
                assert!(targets.contains(&M::Rank));
                assert!(targets.contains(&M::Table));
                assert_eq!(load_error.as_deref(), Some(expected));
                assert!(!widgets.mcdm_chart.controls.computing);
                assert!(!widgets.trial_table.mcdm.controls.computing);
            }
        }
    }

    #[test]
    fn d4_rejection_supersession_preserves_other_current_interests() {
        use crate::state::messages::McdmChartSource as M;
        for same_settings in [false, true] {
            let state = empty_front_state();
            let (tx, rx) = mpsc::sync_channel(3);
            request_empty_front(&state, &tx, M::Rank, "same settings");
            request_empty_front(&state, &tx, M::Table, "same settings");
            request_empty_front(
                &state,
                &tx,
                M::Rank,
                if same_settings {
                    "same settings"
                } else {
                    "new settings"
                },
            );
            let mut completions: Vec<_> = (0..3).map(|_| receive_rejection(&rx)).collect();
            completions.sort_by_key(|(_, c)| c.key().run);
            let mut completions = completions.into_iter();
            let mut widgets = WidgetStates::default();
            widgets.mcdm_chart.controls.computing = true;
            widgets.trial_table.mcdm.controls.computing = true;
            let (slot, obsolete) = completions.next().unwrap();
            assert!(adopt_all(slot, obsolete, &state, &mut widgets).is_empty());
            assert!(
                widgets.mcdm_chart.controls.computing,
                "A stale error must not clear a newer request"
            );
            assert!(widgets.trial_table.mcdm.controls.computing);
            let (slot, previous) = completions.next().unwrap();
            let messages = adopt_all(slot, previous, &state, &mut widgets);
            if same_settings {
                assert!(messages.is_empty());
                assert!(widgets.trial_table.mcdm.controls.computing);
            } else {
                assert!(matches!(
                    messages.as_slice(),
                    [AppMessage::McdmFailed {
                        source: M::Table,
                        ..
                    }]
                ));
                assert!(!widgets.trial_table.mcdm.controls.computing);
            }
            assert!(widgets.mcdm_chart.controls.computing);
            let (slot, current) = completions.next().unwrap();
            let messages = adopt_all(slot, current, &state, &mut widgets);
            assert_eq!(messages.len(), if same_settings { 2 } else { 1 });
            assert!(messages.iter().any(|m| matches!(
                m,
                AppMessage::McdmFailed {
                    source: M::Rank,
                    ..
                }
            )));
            assert!(!widgets.mcdm_chart.controls.computing);
            assert!(!widgets.trial_table.mcdm.controls.computing);
        }
    }

    fn sensitivity_fixture(value: f64) -> crate::state::results::SensitivityResult {
        crate::state::results::SensitivityResult {
            param_names: vec!["x".into()],
            unsupported_categorical: vec![],
            spearman: vec![vec![value]],
            ridge: vec![crate::state::results::RidgeResult {
                beta: vec![value],
                r_squared: value,
            }],
            rf_anova: None,
            mdi: None,
            shap: None,
            permutation: None,
            ard: None,
        }
    }

    fn heatmap_fixture(value: f64) -> crate::state::results::HeatmapMatrix {
        crate::state::results::HeatmapMatrix {
            param_names: vec!["x".into()],
            unsupported_categorical: vec![],
            objective_names: vec!["y".into()],
            values: vec![vec![value]],
            signed: true,
        }
    }

    fn pdp_fixture(value: f64) -> crate::state::messages::PdpResult1d {
        crate::state::messages::PdpResult1d {
            x_values: vec![0.0],
            y_values: vec![value],
            y_upper: None,
            y_lower: None,
            ice_lines: vec![],
            r2: Some(value),
            param_name: "x".into(),
        }
    }

    #[test]
    fn d4_cached_selection_rejects_queued_success_error_and_panic_from_previous_request() {
        use crate::state::layout_state::ChartId;
        use crate::ui::widgets::importance_chart::ImportanceMetric as M;
        use tdashboard_analysis::sensitivity::heatmap::HeatmapMethod;
        // Regular importance, its Sobol cache, heatmap, and widget-owned PDP cache.
        for path in 0..4 {
            for outcome in 0..3 {
                let (mut state, _, _) = state(true);
                let mut widgets = WidgetStates::default();
                let (tx, rx) = mpsc::sync_channel(2);
                let slot = match path {
                    0 | 1 => DeliverySlot::Importance,
                    2 => DeliverySlot::Heatmap,
                    _ => DeliverySlot::Pdp1d,
                };
                let selection = if path == 3 {
                    Preparation::Pdp {
                        params: vec!["x".into()],
                        objective: "y".into(),
                        feasible_only: false,
                        two_dimensional: false,
                    }
                } else {
                    Preparation::Sensitivity {
                        method: HeatmapMethod::Spearman,
                        objective: (path != 2).then_some(0),
                        feasible_only: false,
                        samples: 32,
                    }
                };
                spawn(
                    &state,
                    tx.clone(),
                    slot,
                    "request A",
                    selection,
                    move |_| {
                        match outcome {
                            1 => return AppMessage::Error("stale A error".into()),
                            2 => panic!("stale A panic"),
                            _ => {}
                        }
                        match path {
                            0 | 1 => AppMessage::SensitivityDone {
                                key: (M::Spearman.cache_id(), 0, false),
                                result: sensitivity_fixture(0.2),
                            },
                            2 => AppMessage::SensitivityHeatmapDone {
                                metric: M::Spearman,
                                feasible_only: false,
                                result: heatmap_fixture(0.2),
                            },
                            _ => AppMessage::PdpDone {
                                param: "x".into(),
                                objective: "y".into(),
                                model_type: "gp_fitc".into(),
                                feasible_only: false,
                                result: pdp_fixture(0.2),
                            },
                        }
                    },
                );
                // A has finished, but its queued completion has not been adopted.
                let (origin, queued) = receive_rejection(&rx);
                let old_key = queued.key().clone();
                let chart = match path {
                    0 => {
                        state.importance_cache.select(
                            (M::Ridge.cache_id(), 0, false),
                            fixture_result(sensitivity_fixture(0.8)),
                        );
                        widgets.importance.pending_compute = Some((M::Ridge, 0, false));
                        widgets.importance.computing = true;
                        ChartId::ImportanceChart
                    }
                    1 => {
                        state.sobol_cache.select(
                            (0, false),
                            fixture_result(crate::state::results::SobolResult {
                                param_names: vec!["x".into()],
                                first_order: vec![vec![0.8]],
                                total_effect: vec![vec![0.8]],
                                r_squared: vec![0.8],
                            }),
                        );
                        widgets.importance.pending_compute = Some((M::SobolFirst, 0, false));
                        widgets.importance.computing = true;
                        ChartId::ImportanceChart
                    }
                    2 => {
                        state.sensitivity_heatmap_cache.select(
                            (M::Ridge.cache_id(), false),
                            fixture_result(heatmap_fixture(0.8)),
                        );
                        widgets.sensitivity_heatmap.pending_compute = Some((M::Ridge, false));
                        widgets.sensitivity_heatmap.computing = true;
                        ChartId::SensitivityHeatmap
                    }
                    _ => {
                        widgets
                            .pdp_chart
                            .insert_cache("x", "y", "ridge", false, pdp_fixture(0.8));
                        widgets.pdp_chart.pending_compute =
                            Some(crate::ui::widgets::pdp_chart::PdpComputeRequest {
                                param: "x".into(),
                                objective: "y".into(),
                                n_grid: 50,
                                model_type: "ridge".into(),
                                feasible_only: false,
                            });
                        widgets.pdp_chart.computing = true;
                        ChartId::PdpChart
                    }
                };
                crate::ui::poll_chart::poll_chart_work(&mut state, &mut widgets, &chart, &tx);
                let journal = state.journal_studies.as_ref().unwrap();
                assert!(
                    !journal.analysis_jobs.borrow().contains_key(&(0, slot)),
                    "A cache hit must replace the old pending adoption interest"
                );
                assert!(journal
                    .analysis
                    .borrow()
                    .lookup(&old_key.analysis)
                    .is_none());
                let mut load_error = None;
                crate::state::message_handler::MessageHandler::handle(
                    AppMessage::AnalysisCompleted {
                        slot: origin,
                        completion: queued,
                    },
                    &mut state,
                    &mut widgets,
                    &mut false,
                    &mut load_error,
                );
                assert!(
                    load_error.is_none(),
                    "Neither stale errors nor caught panics may overwrite cached B"
                );
                assert!(!state
                    .importance_cache
                    .contains_key(&(M::Spearman.cache_id(), 0, false)));
                assert!(!state
                    .sensitivity_heatmap_cache
                    .contains_key(&(M::Spearman.cache_id(), false)));
                match path {
                    0 => assert_eq!(
                        state
                            .importance_cache
                            .get(&(M::Ridge.cache_id(), 0, false))
                            .unwrap()
                            .ridge[0]
                            .beta,
                        [0.8]
                    ),
                    1 => assert_eq!(
                        state.sobol_cache.get(&(0, false)).unwrap().first_order,
                        [vec![0.8]]
                    ),
                    2 => assert_eq!(
                        state
                            .sensitivity_heatmap_cache
                            .get(&(M::Ridge.cache_id(), false))
                            .unwrap()
                            .values,
                        [vec![0.8]]
                    ),
                    _ => {
                        assert_eq!(widgets.pdp_chart.result.as_ref().unwrap().y_values, [0.8]);
                        assert!(!widgets.pdp_chart.cache.contains_key(
                            &crate::ui::widgets::pdp_chart::cache_key("x", "y", "gp_fitc", false)
                        ));
                    }
                }
                assert!(!widgets.importance.computing);
                assert!(!widgets.sensitivity_heatmap.computing);
                assert!(!widgets.pdp_chart.computing);
            }
        }
    }

    #[test]
    fn d4_cached_selection_detaches_only_its_consumer_of_shared_work() {
        use crate::state::messages::McdmChartSource as M;
        let (state, _, _) = state(true);
        let (tx, rx) = mpsc::sync_channel(1);
        spawn(
            &state,
            tx.clone(),
            DeliverySlot::Mcdm(M::Rank),
            "shared A",
            Preparation::Mcdm,
            |_| AppMessage::McdmFailed {
                source: M::Rank,
                message: "still current for the table".into(),
            },
        );
        spawn(
            &state,
            tx,
            DeliverySlot::Mcdm(M::Table),
            "shared A",
            Preparation::Mcdm,
            |_| panic!("The second consumer must share the existing job"),
        );
        let (origin, queued) = receive_rejection(&rx);
        let key = queued.key().clone();
        supersede(&state, DeliverySlot::Mcdm(M::Rank));
        let journal = state.journal_studies.as_ref().unwrap();
        assert!(!journal
            .analysis_jobs
            .borrow()
            .contains_key(&(0, DeliverySlot::Mcdm(M::Rank))));
        assert_eq!(
            journal
                .analysis_jobs
                .borrow()
                .get(&(0, DeliverySlot::Mcdm(M::Table))),
            Some(&key)
        );
        assert!(journal.analysis.borrow().lookup(&key.analysis).is_some());
        let mut widgets = WidgetStates::default();
        widgets.trial_table.mcdm.controls.computing = true;
        let messages = adopt_all(origin, queued, &state, &mut widgets);
        assert!(
            matches!(messages.as_slice(), [AppMessage::McdmFailed { source: M::Table, message }] if message == "still current for the table")
        );
        assert!(!widgets.mcdm_chart.controls.computing);
        assert!(!widgets.trial_table.mcdm.controls.computing);
    }

    #[test]
    fn d4_consumed_missing_model_requests_supersede_old_failures() {
        use crate::state::layout_state::ChartId;
        use crate::ui::widget_states::{
            SurrogateMultiOptimizeComputeRequest, SurrogateMultiSuggestComputeRequest,
            SurrogateOptimizeComputeRequest, SurrogateSuggestComputeRequest,
        };
        use tdashboard_analysis::surrogate_opt::{AcquisitionKind, OptimizerKind};
        for slot in [
            DeliverySlot::Optimize,
            DeliverySlot::MultiOptimize,
            DeliverySlot::Suggest,
            DeliverySlot::MultiSuggest,
        ] {
            let (mut state, _, _) = state(true);
            let mut widgets = WidgetStates::default();
            let (tx, rx) = mpsc::sync_channel(1);
            reject_settings(&state, tx.clone(), slot, "stale A failure".into());
            let (origin, queued) = receive_rejection(&rx);
            match slot {
                DeliverySlot::Optimize => {
                    widgets.surrogate_opt.pending_optimize =
                        Some(SurrogateOptimizeComputeRequest {
                            optimizer: OptimizerKind::RandomSearch,
                        });
                    widgets.surrogate_opt.optimizing = true;
                }
                DeliverySlot::MultiOptimize => {
                    widgets.surrogate_opt.pending_multi_optimize =
                        Some(SurrogateMultiOptimizeComputeRequest);
                    widgets.surrogate_opt.optimizing = true;
                }
                DeliverySlot::Suggest => {
                    widgets.surrogate_opt.pending_suggest = Some(SurrogateSuggestComputeRequest {
                        acquisition: AcquisitionKind::ExpectedImprovement,
                        n_candidates: 1,
                        minimize: true,
                    });
                    widgets.surrogate_opt.suggesting = true;
                }
                DeliverySlot::MultiSuggest => {
                    widgets.surrogate_opt.pending_multi_suggest =
                        Some(SurrogateMultiSuggestComputeRequest { n_candidates: 1 });
                    widgets.surrogate_opt.multi_suggesting = true;
                }
                _ => unreachable!(),
            }
            crate::ui::poll_chart::poll_chart_work(
                &mut state,
                &mut widgets,
                &ChartId::SurrogateOpt,
                &tx,
            );
            let expected = widgets.surrogate_opt.error_message.clone();
            assert!(expected
                .as_ref()
                .is_some_and(|m| m.starts_with("No trained")));
            assert!(adopt_all(origin, queued, &state, &mut widgets).is_empty());
            assert_eq!(widgets.surrogate_opt.error_message, expected);
            assert!(!widgets.surrogate_opt.optimizing);
            assert!(!widgets.surrogate_opt.suggesting);
            assert!(!widgets.surrogate_opt.multi_suggesting);
        }
    }

    #[test]
    fn d4_convergence_control_requests_supersede_work_before_the_busy_early_return() {
        use crate::state::layout_state::ChartId;
        use crate::ui::widgets::convergence::RefPointChange;
        use tdashboard_analysis::indicators::MoIndicator;
        for change_reference in [false, true] {
            let (mut state, _, _) = state(true);
            let manager = state
                .journal_studies
                .as_mut()
                .unwrap()
                .managers
                .get_mut(&0)
                .unwrap();
            let input = manager.acquire().unwrap();
            let mut metadata = (**input.data().metadata()).clone();
            metadata.objectives.push(ObjectiveDescriptor {
                index: ObjectiveIndex(1),
                name: Some("z".into()),
                direction: Direction::Minimize,
            });
            let trials = input
                .data()
                .trials()
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let mut t = (**t).clone();
                    Arc::make_mut(&mut t.objectives).insert(ObjectiveIndex(1), Number(-(i as f64)));
                    Arc::new(t)
                })
                .collect();
            let ticket = manager.begin(LoadKind::Refresh).unwrap();
            let receipt = manager
                .apply(
                    manager.header(ticket).unwrap(),
                    StudyBatch::Replace { trials, metadata },
                )
                .unwrap();
            manager.finish(ticket, receipt.next_sequence).unwrap();
            install(&mut state);
            let mut widgets = WidgetStates::default();
            let (tx, rx) = mpsc::sync_channel(1);
            spawn(
                &state,
                tx,
                DeliverySlot::Convergence,
                "A",
                Preparation::Convergence { target_points: 50 },
                |_| AppMessage::TaskPanicked("stale convergence panic".into()),
            );
            let (origin, queued) = receive_rejection(&rx);
            widgets.convergence.computing = true;
            if change_reference {
                widgets.convergence.pending_ref_point =
                    Some(RefPointChange::Manual(vec![100.0, 100.0]));
            } else {
                widgets.convergence.pending_indicator = Some(MoIndicator::IgdPlus);
            }
            let ctx = egui::Context::default();
            let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
                crate::ui::render_chart::render_chart(
                    ui,
                    &mut state,
                    &mut widgets,
                    &ChartId::ConvergenceIndicators,
                )
            });
            assert!(!widgets.convergence.computing);
            assert!(!state
                .journal_studies
                .as_ref()
                .unwrap()
                .analysis_jobs
                .borrow()
                .contains_key(&(0, DeliverySlot::Convergence)));
            assert!(adopt_all(origin, queued, &state, &mut widgets).is_empty());
            if change_reference {
                assert_eq!(state.hv_ref_point_override, Some(vec![100.0, 100.0]));
            } else {
                assert_eq!(state.convergence_indicator, MoIndicator::IgdPlus);
            }
        }
    }
}
