//! Caller-owned work, result sharing and adoption. Keys never identify a Study by
//! path, row count, mutable selection or pointer identity.
use crate::projection::{Projection, TableProjection};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tdashboard_study::{AcquiredStamp, LoadPhase, StudyInput, StudyKey, TrialKey};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnalysisKind {
    Pareto,
    History,
    Cluster,
    Mcdm,
    Entropy,
    Convergence,
    Importance,
    Heatmap,
    Sobol,
    Pdp1d,
    Pdp2d,
    Fit,
    Optimize,
    Suggest,
    Robustness,
    Surface,
    Compare,
    Report,
    Pca,
    Som,
    Linkage,
    Contour,
    Distribution,
}

/// Canonical calculation settings. Presentation-only settings do not belong here.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Condition {
    Bool(bool),
    Integer(u64),
    Signed(i64),
    Text(String),
    Float(u64),
    /// Invalid requests still need stable keys for their existing error/fallback
    /// results; never normalize an invalid setting into a valid numeric value.
    InvalidFloat(u64),
    Sequence(Vec<Condition>),
}
impl From<bool> for Condition {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}
impl From<usize> for Condition {
    fn from(v: usize) -> Self {
        Self::Integer(v as u64)
    }
}
impl From<u64> for Condition {
    fn from(v: u64) -> Self {
        Self::Integer(v)
    }
}
impl From<u8> for Condition {
    fn from(v: u8) -> Self {
        Self::Integer(v.into())
    }
}
impl From<i64> for Condition {
    fn from(v: i64) -> Self {
        Self::Signed(v)
    }
}
impl From<f64> for Condition {
    fn from(v: f64) -> Self {
        Self::finite(v).unwrap_or(Self::InvalidFloat(v.to_bits()))
    }
}
impl From<String> for Condition {
    fn from(v: String) -> Self {
        Self::Text(v)
    }
}
impl From<&str> for Condition {
    fn from(v: &str) -> Self {
        Self::Text(v.into())
    }
}
impl From<&String> for Condition {
    fn from(v: &String) -> Self {
        Self::Text(v.clone())
    }
}
impl<T: Into<Condition>> From<Vec<T>> for Condition {
    fn from(v: Vec<T>) -> Self {
        Self::Sequence(v.into_iter().map(Into::into).collect())
    }
}
impl<T: Into<Condition>> From<Option<T>> for Condition {
    fn from(v: Option<T>) -> Self {
        Self::Sequence(v.into_iter().map(Into::into).collect())
    }
}
macro_rules! condition_tuple {
    ($($t:ident),+) => {
        impl<$($t: Into<Condition>),+> From<($($t,)+)> for Condition {
            #[allow(non_snake_case)]
            fn from(($($t,)+): ($($t,)+)) -> Self { Self::Sequence(vec![$($t.into(),)+]) }
        }
    };
}
condition_tuple!(A, B);
condition_tuple!(A, B, C);
condition_tuple!(A, B, C, D);
condition_tuple!(A, B, C, D, E);
condition_tuple!(A, B, C, D, E, F);
impl Condition {
    pub fn finite(value: f64) -> Result<Self, Readiness> {
        if !value.is_finite() {
            return Err(Readiness::Unavailable("Nonfinite analysis setting".into()));
        }
        Ok(Self::Float(if value == 0.0 { 0 } else { value.to_bits() }))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AnalysisRequest {
    pub kind: AnalysisKind,
    /// Ordered feature/objective identities and method settings actually used.
    pub conditions: Vec<(String, Condition)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AnalysisKey {
    pub inputs: Vec<AcquiredStamp>,
    pub request: AnalysisRequest,
    pub dependencies: Vec<ResultKey>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResultKey {
    pub analysis: AnalysisKey,
    pub run: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Readiness {
    Waiting,
    Unavailable(String),
}

pub struct PreparedAnalysis {
    pub(crate) key: AnalysisKey,
    pub(crate) tables: Vec<Arc<Projection>>,
    inputs: Vec<StudyInput>,
    dependencies: Vec<ResultHandle>,
    data: crate::preparation::PreparedData,
    row_maps: Vec<Vec<TrialKey>>,
    models: Option<crate::preparation::Models>,
    model_directions: Vec<bool>,
}

#[derive(Clone)]
pub struct ResultHandle(Arc<ResultSlot>);
struct ResultSlot {
    key: ResultKey,
    rows: Vec<Vec<TrialKey>>,
    value: Mutex<Option<Result<Arc<dyn Any + Send + Sync>, String>>>,
}
impl std::fmt::Debug for ResultHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ResultHandle").field(&self.key()).finish()
    }
}
impl ResultHandle {
    pub fn is_pending(&self) -> bool {
        self.0
            .value
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .is_none()
    }
    pub fn key(&self) -> &ResultKey {
        &self.0.key
    }
    pub fn rows(&self) -> &[Vec<TrialKey>] {
        &self.0.rows
    }
    pub fn payload<T: Any + Send + Sync>(&self) -> Option<Result<Arc<T>, String>> {
        self.0
            .value
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_ref()
            .map(|r| match r {
                Ok(value) => value
                    .clone()
                    .downcast::<T>()
                    .map_err(|_| "Analysis payload type mismatch".into()),
                Err(error) => Err(error.clone()),
            })
    }
    pub fn same_result(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

pub struct AnalysisWork {
    prepared: Box<PreparedAnalysis>,
    handle: ResultHandle,
}
pub struct AnalysisCompletion {
    handle: ResultHandle,
    value: Result<Arc<dyn Any + Send + Sync>, String>,
}
impl AnalysisCompletion {
    pub fn key(&self) -> &ResultKey {
        self.handle.key()
    }
}
impl AnalysisWork {
    pub fn prepared(&self) -> &PreparedAnalysis {
        &self.prepared
    }
    pub fn key(&self) -> &ResultKey {
        self.handle.key()
    }
    /// Execute against the retained input only. Success, cancellation and failure
    /// all return the same provenance; adoption is a separate owner operation.
    pub fn execute<T: Any + Send + Sync>(
        self,
        compute: impl FnOnce(&PreparedAnalysis) -> Result<T, String>,
    ) -> AnalysisCompletion {
        let value = compute(&self.prepared).map(|v| Arc::new(v) as Arc<dyn Any + Send + Sync>);
        AnalysisCompletion {
            handle: self.handle,
            value,
        }
    }
    pub fn execute_shared(
        self,
        compute: impl FnOnce(&PreparedAnalysis) -> Result<Arc<dyn Any + Send + Sync>, String>,
    ) -> AnalysisCompletion {
        AnalysisCompletion {
            value: compute(&self.prepared),
            handle: self.handle,
        }
    }
}

pub enum WorkOrShared {
    Work(AnalysisWork),
    Shared(ResultHandle),
}
#[derive(Clone, Copy)]
pub enum ReuseOrRerun {
    Reuse,
    Rerun,
}

#[derive(Default, Debug)]
pub struct AnalysisSession {
    serial: u64,
    projections: HashMap<StudyKey, TableProjection>,
    tables: HashMap<AcquiredStamp, Arc<Projection>>,
    results: HashMap<AnalysisKey, ResultHandle>,
}
impl AnalysisSession {
    pub fn prepare_models(
        &mut self,
        inputs: Vec<StudyInput>,
        request: AnalysisRequest,
        models: crate::preparation::Models,
    ) -> Result<PreparedAnalysis, Readiness> {
        let keys = models.keys()?;
        let mut prepared = self.prepare_inner(
            inputs,
            request,
            crate::preparation::Preparation::Table,
            &[],
            &keys,
        )?;
        prepared.model_directions =
            crate::preparation::flags(&prepared.tables[0], &models.objective_names());
        prepared.models = Some(models);
        Ok(prepared)
    }
    pub fn project(&mut self, input: StudyInput) -> Arc<Projection> {
        self.prepare(
            vec![input],
            AnalysisRequest {
                kind: AnalysisKind::Distribution,
                conditions: vec![],
            },
            &[],
        )
        .expect("A retained input can always be projected")
        .tables
        .remove(0)
    }

    pub fn pareto(&mut self, input: StudyInput) -> Result<ResultHandle, Readiness> {
        let flags: Vec<_> = input
            .data()
            .metadata()
            .objectives
            .iter()
            .map(|o| matches!(o.direction, tdashboard_study::Direction::Minimize))
            .collect();
        let request = AnalysisRequest {
            kind: AnalysisKind::Pareto,
            conditions: vec![(
                "directions".into(),
                Condition::Sequence(flags.iter().copied().map(Condition::Bool).collect()),
            )],
        };
        let prepared = self.prepare(vec![input], request, &[])?;
        match self
            .start(prepared, ReuseOrRerun::Reuse)
            .map_err(Readiness::Unavailable)?
        {
            WorkOrShared::Shared(handle) => Ok(handle),
            WorkOrShared::Work(work) => {
                let completion = work.execute(|p| {
                    Ok(crate::pareto::compute_pareto_ranks(
                        &p.tables[0].dataframe,
                        &flags,
                    ))
                });
                self.complete(completion)
                    .ok_or_else(|| Readiness::Unavailable("Pareto result superseded".into()))
            }
        }
    }

    pub fn history(&mut self, input: StudyInput) -> Result<Vec<(u32, f64)>, Readiness> {
        let prepared = self.prepare(
            vec![input],
            AnalysisRequest {
                kind: AnalysisKind::History,
                conditions: vec![],
            },
            &[],
        )?;
        Ok(prepared.tables()[0].history.clone().unwrap_or_default())
    }

    pub fn prepare(
        &mut self,
        inputs: Vec<StudyInput>,
        request: AnalysisRequest,
        dependencies: &[ResultHandle],
    ) -> Result<PreparedAnalysis, Readiness> {
        let selection = crate::preparation::Preparation::default_for(request.kind)?;
        self.prepare_for(inputs, request, selection, dependencies)
    }

    pub fn prepare_for(
        &mut self,
        inputs: Vec<StudyInput>,
        request: AnalysisRequest,
        selection: crate::preparation::Preparation,
        dependencies: &[ResultHandle],
    ) -> Result<PreparedAnalysis, Readiness> {
        self.prepare_inner(inputs, request, selection, dependencies, &[])
    }

    fn prepare_inner(
        &mut self,
        inputs: Vec<StudyInput>,
        mut request: AnalysisRequest,
        selection: crate::preparation::Preparation,
        dependencies: &[ResultHandle],
        model_keys: &[ResultKey],
    ) -> Result<PreparedAnalysis, Readiness> {
        if !selection.matches(request.kind) {
            return Err(Readiness::Unavailable(
                "Preparation does not match the analysis operation".into(),
            ));
        }
        if inputs.is_empty() {
            return Err(Readiness::Unavailable("No Study input".into()));
        }
        if inputs.len() != 1
            && !matches!(
                request.kind,
                AnalysisKind::Convergence | AnalysisKind::Distribution
            )
        {
            return Err(Readiness::Unavailable(
                "This method requires exactly one Study input".into(),
            ));
        }
        let stamps: Vec<_> = inputs.iter().map(StudyInput::stamp).collect();
        if matches!(request.kind, AnalysisKind::Optimize | AnalysisKind::Suggest)
            && model_keys.is_empty()
        {
            return Err(Readiness::Waiting);
        }
        if model_keys
            .iter()
            .any(|key| key.analysis.inputs.iter().any(|s| !stamps.contains(s)))
        {
            return Err(Readiness::Unavailable(
                "Trained model belongs to another input".into(),
            ));
        }
        for stamp in &stamps {
            self.tables
                .retain(|s, _| s.input.study != stamp.input.study || s == stamp);
            self.results.retain(|key, _| {
                !key.inputs
                    .iter()
                    .any(|s| s.input.study == stamp.input.study && s != stamp)
            });
        }
        if matches!(
            request.kind,
            AnalysisKind::Pareto | AnalysisKind::History | AnalysisKind::Convergence
        ) && inputs
            .iter()
            .any(|i| i.load_state().phase != LoadPhase::Complete)
        {
            return Err(Readiness::Waiting);
        }
        if matches!(request.kind, AnalysisKind::Cluster | AnalysisKind::Mcdm) {
            let front = dependencies.iter().find(|d| {
                d.key().analysis.request.kind == AnalysisKind::Pareto
                    && d.key().analysis.inputs == stamps
            });
            if front.is_none_or(|f| {
                f.payload::<crate::pareto::ParetoResult>()
                    .is_none_or(|p| p.is_err())
            }) || inputs
                .iter()
                .any(|i| i.load_state().phase != LoadPhase::Complete)
            {
                return Err(Readiness::Waiting);
            }
        }
        for dependency in dependencies {
            if dependency
                .key()
                .analysis
                .inputs
                .iter()
                .any(|s| !stamps.contains(s))
            {
                return Err(Readiness::Unavailable(
                    "Analysis dependency belongs to another input".into(),
                ));
            }
            if dependency
                .0
                .value
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .is_none()
            {
                return Err(Readiness::Waiting);
            }
        }
        let mut tables = Vec::new();
        for input in &inputs {
            let stamp = input.stamp();
            let table = self
                .tables
                .entry(stamp)
                .or_insert_with(|| {
                    Arc::new(
                        self.projections
                            .entry(stamp.input.study)
                            .or_default()
                            .project(input),
                    )
                })
                .clone();
            tables.push(table);
        }
        if request.kind == AnalysisKind::History
            && (tables[0].meta.objective_names.len() != 1 || tables[0].meta.directions.len() != 1)
        {
            return Err(Readiness::Unavailable(
                "History requires one objective and direction".into(),
            ));
        }
        let front = dependencies
            .iter()
            .find(|d| d.key().analysis.request.kind == AnalysisKind::Pareto)
            .and_then(|d| d.payload::<crate::pareto::ParetoResult>())
            .and_then(Result::ok);
        let data = crate::preparation::prepare(&tables, &selection, front.as_deref())?;
        let row_maps = data.row_maps(&tables);
        request
            .conditions
            .push(("selection".into(), selection.condition()));
        let key = AnalysisKey {
            inputs: stamps,
            request,
            dependencies: dependencies
                .iter()
                .map(|d| d.key().clone())
                .chain(model_keys.iter().cloned())
                .collect(),
        };
        Ok(PreparedAnalysis {
            key,
            tables,
            inputs,
            dependencies: dependencies.to_vec(),
            data,
            row_maps,
            models: None,
            model_directions: vec![],
        })
    }

    pub fn lookup(&self, key: &AnalysisKey) -> Option<ResultHandle> {
        self.results.get(key).cloned()
    }

    /// Release a completed entry-adapter delivery; retained typed models/results
    /// keep their own provenance after presentation has consumed the delivery.
    pub fn release(&mut self, key: &ResultKey) {
        if self
            .results
            .get(&key.analysis)
            .is_some_and(|r| r.key() == key)
        {
            self.results.remove(&key.analysis);
        }
    }

    pub fn start(
        &mut self,
        prepared: PreparedAnalysis,
        mode: ReuseOrRerun,
    ) -> Result<WorkOrShared, String> {
        if matches!(mode, ReuseOrRerun::Reuse) {
            if let Some(handle) = self.lookup(&prepared.key) {
                return Ok(WorkOrShared::Shared(handle));
            }
        }
        let handle = self.allocate(prepared.key.clone(), prepared.row_maps.clone())?;
        Ok(WorkOrShared::Work(AnalysisWork {
            prepared: Box::new(prepared),
            handle,
        }))
    }

    /// Publish a rejected preparation through the same checked, input-stamped
    /// completion protocol. It is not a Ready analysis and never executes math.
    pub fn reject(
        &mut self,
        inputs: &[StudyInput],
        mut request: AnalysisRequest,
        selection: &crate::preparation::Preparation,
        message: String,
    ) -> Result<AnalysisCompletion, String> {
        request
            .conditions
            .push(("selection".into(), selection.condition()));
        let key = AnalysisKey {
            inputs: inputs.iter().map(StudyInput::stamp).collect(),
            request,
            dependencies: vec![],
        };
        let handle = self.allocate(key, vec![])?;
        Ok(AnalysisCompletion {
            handle,
            value: Err(message),
        })
    }

    fn allocate(
        &mut self,
        key: AnalysisKey,
        rows: Vec<Vec<TrialKey>>,
    ) -> Result<ResultHandle, String> {
        self.serial = self
            .serial
            .checked_add(1)
            .ok_or("Analysis run counter exhausted")?;
        let handle = ResultHandle(Arc::new(ResultSlot {
            key: ResultKey {
                analysis: key.clone(),
                run: self.serial,
            },
            rows,
            value: Mutex::new(None),
        }));
        self.results.insert(key, handle.clone());
        Ok(handle)
    }

    pub fn complete(&mut self, completion: AnalysisCompletion) -> Option<ResultHandle> {
        if self
            .results
            .get(&completion.handle.key().analysis)
            .is_none_or(|current| !current.same_result(&completion.handle))
        {
            return None;
        }
        *completion
            .handle
            .0
            .value
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = Some(completion.value);
        Some(completion.handle)
    }

    pub fn retain_interests(&mut self, interests: &[AnalysisKey]) {
        let selected: HashSet<_> = interests.iter().collect();
        self.results.retain(|key, _| selected.contains(key));
        let inputs: HashSet<_> = interests
            .iter()
            .flat_map(|k| k.inputs.iter().copied())
            .collect();
        self.tables.retain(|stamp, _| inputs.contains(stamp));
        let studies: HashSet<_> = inputs.iter().map(|s| s.input.study).collect();
        self.projections.retain(|key, _| studies.contains(key));
    }
}

impl PreparedAnalysis {
    pub fn model_directions(&self) -> &[bool] {
        &self.model_directions
    }
    pub fn models(&self) -> &crate::preparation::Models {
        self.models
            .as_ref()
            .expect("Model-dependent preparation owns its models")
    }
    pub fn data(&self) -> &crate::preparation::PreparedData {
        &self.data
    }
    pub fn row_maps(&self) -> &[Vec<TrialKey>] {
        &self.row_maps
    }
    pub fn key(&self) -> &AnalysisKey {
        &self.key
    }
    pub fn tables(&self) -> &[Arc<Projection>] {
        &self.tables
    }
    pub fn inputs(&self) -> &[StudyInput] {
        &self.inputs
    }
    pub fn dependencies(&self) -> &[ResultHandle] {
        &self.dependencies
    }
}

pub fn can_adopt(result: &ResultHandle, target: &ResultKey) -> bool {
    result.key() == target
}
