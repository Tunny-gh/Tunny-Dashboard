use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Representation equality is for revision detection, not numerical ordering.
#[derive(Clone, Copy, Debug, Default)]
pub struct Number(pub f64);
impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceInstanceId {
    namespace: u64,
    serial: u64,
}

/// Source attributes have one authority, including non-JSON numeric CSV cells.
#[derive(Clone, Debug, PartialEq)]
pub enum AttributeValue {
    Json(Value),
    Number(Number),
    Array(Vec<AttributeValue>),
    Object(BTreeMap<String, AttributeValue>),
}
impl From<Value> for AttributeValue {
    fn from(value: Value) -> Self {
        Self::Json(value)
    }
}
impl From<f64> for AttributeValue {
    fn from(value: f64) -> Self {
        Self::Number(Number(value))
    }
}
impl PartialEq<Value> for AttributeValue {
    fn eq(&self, other: &Value) -> bool {
        matches!(self, Self::Json(value) if value == other)
    }
}
impl AttributeValue {
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Json(Value::Null))
    }
    /// JSON presentation cannot encode nonfinite numbers. The authoritative
    /// observation stays intact; existing JSON renderers retain their null policy.
    pub fn json_view(&self) -> Value {
        match self {
            Self::Json(value) => value.clone(),
            Self::Number(number) => Value::from(number.0),
            Self::Array(values) => Value::Array(values.iter().map(Self::json_view).collect()),
            Self::Object(values) => Value::Object(
                values
                    .iter()
                    .map(|(k, v)| (k.clone(), v.json_view()))
                    .collect(),
            ),
        }
    }
}

/// Only the namespace counter is process-wide; it holds no domain data.
#[derive(Debug)]
pub struct IdentityAllocator {
    namespace: u64,
    serial: u64,
}
impl IdentityAllocator {
    pub fn new() -> Result<Self, StudyError> {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let namespace = NEXT
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| StudyError::CounterExhausted)?;
        Ok(Self {
            namespace,
            serial: 0,
        })
    }
    pub fn source(&mut self) -> Result<SourceInstanceId, StudyError> {
        self.serial = self
            .serial
            .checked_add(1)
            .ok_or(StudyError::CounterExhausted)?;
        Ok(SourceInstanceId {
            namespace: self.namespace,
            serial: self.serial,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StudyKey {
    pub source: SourceInstanceId,
    pub stored_id: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrialKey {
    pub source: SourceInstanceId,
    pub stored_id: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StudyTrialNumber(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectiveIndex(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Minimize,
    Maximize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrialState {
    Running,
    Complete,
    Pruned,
    Fail,
    Waiting,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectiveDescriptor {
    pub index: ObjectiveIndex,
    /// None when the source did not declare a name. Analysis chooses its fallback.
    pub name: Option<String>,
    pub direction: Direction,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StudyMetadata {
    pub name: String,
    pub objectives: Vec<ObjectiveDescriptor>,
    pub observed_parameters: BTreeSet<String>,
    pub declared_bounds: BTreeMap<String, (Number, Number)>,
    pub user_attribute_names: BTreeSet<String>,
    /// Source-level presence evidence independent of individual records. Expected
    /// width is derived from all current records, not a second writable authority.
    pub constrained: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConstraintSchema {
    pub present: bool,
    pub width: usize,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConstraintObservation {
    pub present: bool,
    pub values: Arc<Vec<Number>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum FeasibilityState {
    Feasible,
    Infeasible,
    #[serde(rename = "Feasibility unverified")]
    Unverified,
}
impl FeasibilityState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Feasible => "Feasible",
            Self::Infeasible => "Infeasible",
            Self::Unverified => "Feasibility unverified",
        }
    }
    pub fn classify(values: &[f64], expected: usize) -> Self {
        if values.iter().any(|v| v.is_finite() && *v > 0.0) {
            Self::Infeasible
        } else if expected == 0 || values.len() != expected || values.iter().any(|v| !v.is_finite())
        {
            Self::Unverified
        } else {
            Self::Feasible
        }
    }
    pub fn numeric(self) -> f64 {
        match self {
            Self::Feasible => 1.0,
            Self::Infeasible => 0.0,
            Self::Unverified => f64::NAN,
        }
    }
}
impl ConstraintSchema {
    pub fn classify(self, observation: &ConstraintObservation) -> FeasibilityState {
        if !self.present {
            return FeasibilityState::Feasible;
        }
        let values: Vec<_> = observation.values.iter().map(|n| n.0).collect();
        FeasibilityState::classify(&values, self.width)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParameterValue {
    Numeric(Number),
    Category(Arc<Value>),
    Unavailable(UnavailableValue),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnavailableValue {
    MissingEncodedValue,
    InvalidEncodedValue,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericKind {
    Float,
    Int,
}
impl NumericKind {
    pub fn is_discrete(self, step: Option<f64>) -> bool {
        self == Self::Int || step.is_some_and(|s| s.is_finite() && s > 0.0)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum Distribution {
    Numeric {
        kind: NumericKind,
        low: Option<Number>,
        high: Option<Number>,
        step: Option<Number>,
        log: bool,
        discrete: bool,
    },
    Categorical {
        choices: Arc<Vec<Value>>,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct ParameterObservation {
    pub value: ParameterValue,
    pub distribution: Option<Distribution>,
    /// Inline Journal observations use external category values, whereas ordinary
    /// parameter records encode choice indices. Retained for display compatibility.
    pub external_encoding: bool,
    /// Category index evidence (including invalid values), not category identity.
    /// The temporary projection preserves the existing rounding/display fallback.
    pub encoded_numeric: Option<Number>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ArtifactAssociation {
    pub trial: TrialKey,
    pub reference: String,
    pub name: String,
    pub mime: Option<String>,
    pub order: u64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct TrialRecord {
    pub key: TrialKey,
    pub study: StudyKey,
    pub number: StudyTrialNumber,
    pub state: TrialState,
    pub parameters: Arc<BTreeMap<String, ParameterObservation>>,
    pub objectives: Arc<BTreeMap<ObjectiveIndex, Number>>,
    pub user_attributes: Arc<BTreeMap<String, Arc<AttributeValue>>>,
    pub constraints: ConstraintObservation,
    pub datetime_start: Option<Number>,
    pub datetime_complete: Option<Number>,
    pub intermediate: Arc<Vec<(u64, Number)>>,
    pub artifacts: Arc<Vec<ArtifactAssociation>>,
    pub completed_order: Option<u64>,
}
impl TrialRecord {
    pub fn new(
        study: StudyKey,
        stored_id: u32,
        number: StudyTrialNumber,
        state: TrialState,
    ) -> Self {
        Self {
            key: TrialKey {
                source: study.source,
                stored_id,
            },
            study,
            number,
            state,
            parameters: Default::default(),
            objectives: Default::default(),
            user_attributes: Default::default(),
            constraints: Default::default(),
            datetime_start: None,
            datetime_complete: None,
            intermediate: Default::default(),
            artifacts: Default::default(),
            completed_order: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StudyData {
    pub(crate) metadata: Arc<StudyMetadata>,
    pub(crate) trials: Arc<Vec<Arc<TrialRecord>>>,
    pub(crate) schema: ConstraintSchema,
}
impl StudyData {
    pub fn metadata(&self) -> &Arc<StudyMetadata> {
        &self.metadata
    }
    pub fn trials(&self) -> &Arc<Vec<Arc<TrialRecord>>> {
        &self.trials
    }
    pub fn constraint_schema(&self) -> ConstraintSchema {
        self.schema
    }
    pub fn trial(&self, key: TrialKey) -> Option<&Arc<TrialRecord>> {
        let i = self
            .trials
            .binary_search_by_key(&key.stored_id, |t| t.key.stored_id)
            .ok()?;
        self.trials.get(i).filter(|t| t.key == key)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StudyError {
    CounterExhausted,
    InvalidTicket,
    InvalidSequence,
    StaleRevision,
    InvalidRecord,
    InvalidMetadata,
    NoInput,
}
impl std::fmt::Display for StudyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Study update rejected: {self:?}")
    }
}
impl std::error::Error for StudyError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_serial_exhaustion_does_not_wrap_or_reuse_an_identity() {
        let mut allocator = IdentityAllocator::new().unwrap();
        let first = allocator.source().unwrap();
        let second = allocator.source().unwrap();
        assert_ne!(first, second);
        allocator.serial = u64::MAX;
        assert_eq!(allocator.source(), Err(StudyError::CounterExhausted));
        assert_eq!(allocator.serial, u64::MAX);
    }
}
