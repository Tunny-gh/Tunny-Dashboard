use crate::*;
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputStamp {
    pub study: StudyKey,
    pub generation: u64,
    pub revision: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AcquiredStamp {
    pub input: InputStamp,
    pub load_epoch: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadTicket {
    study: StudyKey,
    generation: u64,
    operation: u64,
}
impl LoadTicket {
    pub fn study(self) -> StudyKey {
        self.study
    }
    pub fn generation(self) -> u64 {
        self.generation
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BatchHeader {
    pub ticket: LoadTicket,
    pub sequence: u64,
    pub base_revision: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApplyReceipt {
    pub next_sequence: u64,
    pub stamp: AcquiredStamp,
}
impl ApplyReceipt {
    pub fn next_header(self, ticket: LoadTicket) -> BatchHeader {
        BatchHeader {
            ticket,
            sequence: self.next_sequence,
            base_revision: self.stamp.input.revision,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadKind {
    Initial,
    Reload,
    Refresh,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadPhase {
    Loading,
    Complete,
    Failed,
    Superseded,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadState {
    pub ticket: LoadTicket,
    pub phase: LoadPhase,
    pub last_accepted_sequence: Option<u64>,
}
#[derive(Clone, Debug)]
pub struct StudyInput {
    stamp: AcquiredStamp,
    state: LoadState,
    data: Arc<StudyData>,
}
impl StudyInput {
    pub fn stamp(&self) -> AcquiredStamp {
        self.stamp
    }
    pub fn load_state(&self) -> &LoadState {
        &self.state
    }
    pub fn data(&self) -> &Arc<StudyData> {
        &self.data
    }
    pub fn matches(&self, stamp: AcquiredStamp) -> bool {
        self.stamp == stamp
    }
}
#[derive(Clone, Debug, Default)]
pub enum MetadataEdit {
    #[default]
    Unchanged,
    Set(StudyMetadata),
    Remove,
}
#[derive(Clone, Debug)]
pub enum StudyBatch {
    Delta {
        upserts: Vec<Arc<TrialRecord>>,
        removals: Vec<TrialKey>,
        metadata: MetadataEdit,
    },
    Replace {
        trials: Vec<Arc<TrialRecord>>,
        metadata: StudyMetadata,
    },
}
#[derive(Debug)]
struct Operation {
    state: LoadState,
    next_sequence: u64,
    revision: u64,
    fresh: bool,
}

/// Single writer on the caller's orchestration thread. Workers retain only inputs.
#[derive(Debug)]
pub struct StudyManager {
    key: StudyKey,
    generation: u64,
    operation_id: u64,
    epoch: u64,
    operation: Option<Operation>,
    visible: Option<StudyInput>,
}
fn next(n: u64) -> Result<u64, StudyError> {
    n.checked_add(1).ok_or(StudyError::CounterExhausted)
}
impl StudyManager {
    pub fn new(key: StudyKey) -> Self {
        Self {
            key,
            generation: 0,
            operation_id: 0,
            epoch: 0,
            operation: None,
            visible: None,
        }
    }
    pub fn key(&self) -> StudyKey {
        self.key
    }
    pub fn acquire(&self) -> Option<StudyInput> {
        self.visible.clone()
    }
    pub fn load_state(&self) -> Option<&LoadState> {
        self.operation.as_ref().map(|op| &op.state)
    }
    pub fn begin(&mut self, kind: LoadKind) -> Result<LoadTicket, StudyError> {
        let fresh = kind != LoadKind::Refresh;
        let generation = match kind {
            LoadKind::Initial if self.generation == 0 => 1,
            LoadKind::Reload if self.generation != 0 => next(self.generation)?,
            LoadKind::Refresh
                if self.visible.is_some()
                    && self.visible.as_ref().unwrap().stamp.input.generation == self.generation =>
            {
                self.generation
            }
            _ => return Err(StudyError::InvalidTicket),
        };
        let operation_id = next(self.operation_id)?;
        let epoch = next(self.epoch)?;
        let ticket = LoadTicket {
            study: self.key,
            generation,
            operation: operation_id,
        };
        let state = LoadState {
            ticket,
            phase: LoadPhase::Loading,
            last_accepted_sequence: None,
        };
        let revision = if fresh {
            0
        } else {
            self.visible.as_ref().unwrap().stamp.input.revision
        };
        self.operation = Some(Operation {
            state: state.clone(),
            next_sequence: 0,
            revision,
            fresh,
        });
        self.generation = generation;
        self.operation_id = operation_id;
        self.epoch = epoch;
        if !fresh {
            let visible = self.visible.as_mut().unwrap();
            visible.state = state;
            visible.stamp.load_epoch = epoch;
        } else if let Some(visible) = self
            .visible
            .as_mut()
            .filter(|v| v.state.phase == LoadPhase::Loading)
        {
            visible.state.phase = LoadPhase::Superseded;
            visible.stamp.load_epoch = epoch;
        }
        Ok(ticket)
    }
    pub fn header(&self, ticket: LoadTicket) -> Result<BatchHeader, StudyError> {
        let op = self.checked(ticket)?;
        Ok(BatchHeader {
            ticket,
            sequence: op.next_sequence,
            base_revision: op.revision,
        })
    }
    fn checked(&self, ticket: LoadTicket) -> Result<&Operation, StudyError> {
        self.operation
            .as_ref()
            .filter(|op| op.state.ticket == ticket && op.state.phase == LoadPhase::Loading)
            .ok_or(StudyError::InvalidTicket)
    }
    pub fn apply(
        &mut self,
        header: BatchHeader,
        batch: StudyBatch,
    ) -> Result<ApplyReceipt, StudyError> {
        let op = self.checked(header.ticket)?;
        if header.sequence != op.next_sequence {
            return Err(StudyError::InvalidSequence);
        }
        if header.base_revision != op.revision {
            return Err(StudyError::StaleRevision);
        }
        let first = op.fresh && op.next_sequence == 0;
        let base = if first {
            Arc::new(StudyData {
                metadata: Default::default(),
                trials: Default::default(),
                schema: Default::default(),
            })
        } else {
            self.visible
                .as_ref()
                .ok_or(StudyError::NoInput)?
                .data
                .clone()
        };
        let mut candidate = (*base).clone();
        let replacing = matches!(&batch, StudyBatch::Replace { .. });
        if first && matches!(&batch, StudyBatch::Delta { upserts, .. } if upserts.is_empty()) {
            return Err(StudyError::InvalidRecord);
        }
        let (upserts, removals) = match batch {
            StudyBatch::Replace { trials, metadata } => {
                candidate.trials = Default::default();
                candidate.metadata = Arc::new(metadata);
                (trials, Vec::new())
            }
            StudyBatch::Delta {
                upserts,
                removals,
                metadata,
            } => {
                match metadata {
                    MetadataEdit::Set(m) => candidate.metadata = Arc::new(m),
                    MetadataEdit::Remove => candidate.metadata = Default::default(),
                    MetadataEdit::Unchanged => {}
                }
                (upserts, removals)
            }
        };
        let mut changed_keys = BTreeSet::new();
        for key in removals {
            if key.source != self.key.source || !changed_keys.insert(key) {
                return Err(StudyError::InvalidRecord);
            }
            if let Ok(i) = candidate
                .trials
                .binary_search_by_key(&key.stored_id, |t| t.key.stored_id)
            {
                Arc::make_mut(&mut candidate.trials).remove(i);
            }
        }
        for mut record in upserts {
            if record.study != self.key
                || record.key.source != self.key.source
                || !changed_keys.insert(record.key)
            {
                return Err(StudyError::InvalidRecord);
            }
            if let Some(old) = base.trial(record.key) {
                if old.number != record.number || old.study != record.study {
                    return Err(StudyError::InvalidRecord);
                }
                if !replacing
                    && old.completed_order.is_some()
                    && old.completed_order != record.completed_order
                {
                    return Err(StudyError::InvalidRecord);
                }
                let record = Arc::make_mut(&mut record);
                if record.parameters == old.parameters {
                    record.parameters = old.parameters.clone();
                } else {
                    for (key, observation) in Arc::make_mut(&mut record.parameters) {
                        if let Some(previous) =
                            old.parameters.get(key).filter(|p| *p == observation)
                        {
                            *observation = previous.clone();
                        }
                    }
                }
                if record.objectives == old.objectives {
                    record.objectives = old.objectives.clone();
                }
                if record.constraints == old.constraints {
                    record.constraints = old.constraints.clone();
                }
                if record.intermediate == old.intermediate {
                    record.intermediate = old.intermediate.clone();
                }
                if record.artifacts == old.artifacts {
                    record.artifacts = old.artifacts.clone();
                }
                if record.user_attributes == old.user_attributes {
                    record.user_attributes = old.user_attributes.clone();
                } else {
                    for (key, value) in Arc::make_mut(&mut record.user_attributes) {
                        if let Some(previous) = old.user_attributes.get(key).filter(|p| *p == value)
                        {
                            *value = previous.clone();
                        }
                    }
                }
            }
            match candidate
                .trials
                .binary_search_by_key(&record.key.stored_id, |t| t.key.stored_id)
            {
                Ok(i) if *candidate.trials[i] == *record => {}
                Ok(i) => Arc::make_mut(&mut candidate.trials)[i] = record,
                Err(i) => Arc::make_mut(&mut candidate.trials).insert(i, record),
            }
        }
        let mut numbers = BTreeSet::new();
        let mut orders = BTreeSet::new();
        let mut schema = ConstraintSchema {
            present: candidate.metadata.constrained,
            width: 0,
        };
        for trial in candidate.trials.iter() {
            if !numbers.insert(trial.number) || trial.artifacts.iter().any(|a| a.trial != trial.key)
            {
                return Err(StudyError::InvalidRecord);
            }
            if trial
                .intermediate
                .windows(2)
                .any(|pair| pair[0].0 > pair[1].0)
            {
                return Err(StudyError::InvalidRecord);
            }
            for observation in trial.parameters.values() {
                match (&observation.distribution, &observation.value) {
                    (Some(Distribution::Categorical { .. }), ParameterValue::Numeric(_))
                    | (Some(Distribution::Numeric { .. }), ParameterValue::Category(_)) => {
                        return Err(StudyError::InvalidRecord)
                    }
                    (
                        Some(Distribution::Numeric {
                            kind,
                            step,
                            discrete,
                            ..
                        }),
                        _,
                    ) if *discrete != kind.is_discrete(step.map(|s| s.0)) => {
                        return Err(StudyError::InvalidRecord)
                    }
                    _ => {}
                }
            }
            if trial.state == TrialState::Complete {
                let order = trial.completed_order.ok_or(StudyError::InvalidRecord)?;
                if !orders.insert(order) {
                    return Err(StudyError::InvalidRecord);
                }
            }
            schema.present |= trial.constraints.present || !trial.constraints.values.is_empty();
            schema.width = schema.width.max(trial.constraints.values.len());
        }
        let mut objectives = BTreeSet::new();
        if candidate
            .metadata
            .objectives
            .iter()
            .any(|o| !objectives.insert(o.index))
        {
            return Err(StudyError::InvalidMetadata);
        }
        if candidate.metadata == base.metadata {
            candidate.metadata = base.metadata.clone();
        }
        if candidate.trials == base.trials {
            candidate.trials = base.trials.clone();
        }
        candidate.schema = schema;
        let changed = first || *base != candidate;
        let revision = if changed {
            next(op.revision)?
        } else {
            op.revision
        };
        let sequence = next(op.next_sequence)?;
        let epoch = next(self.epoch)?;
        let stamp = AcquiredStamp {
            input: InputStamp {
                study: self.key,
                generation: header.ticket.generation,
                revision,
            },
            load_epoch: epoch,
        };
        let op = self.operation.as_mut().unwrap();
        op.revision = revision;
        op.next_sequence = sequence;
        op.state.last_accepted_sequence = Some(header.sequence);
        self.visible = Some(StudyInput {
            stamp,
            state: op.state.clone(),
            data: if changed { Arc::new(candidate) } else { base },
        });
        self.epoch = epoch;
        Ok(ApplyReceipt {
            next_sequence: sequence,
            stamp,
        })
    }
    pub fn finish(
        &mut self,
        ticket: LoadTicket,
        next_sequence: u64,
    ) -> Result<AcquiredStamp, StudyError> {
        let op = self.checked(ticket)?;
        if next_sequence != op.next_sequence {
            return Err(StudyError::InvalidSequence);
        }
        if op.next_sequence == 0 {
            return Err(StudyError::NoInput);
        }
        self.terminal(ticket, LoadPhase::Complete)?;
        Ok(self.visible.as_ref().unwrap().stamp)
    }
    pub fn fail(&mut self, ticket: LoadTicket) -> Result<(), StudyError> {
        self.terminal(ticket, LoadPhase::Failed)
    }
    pub fn supersede(&mut self, ticket: LoadTicket) -> Result<(), StudyError> {
        self.terminal(ticket, LoadPhase::Superseded)
    }
    fn terminal(&mut self, ticket: LoadTicket, phase: LoadPhase) -> Result<(), StudyError> {
        self.checked(ticket)?;
        let epoch = next(self.epoch)?;
        let op = self.operation.as_mut().unwrap();
        op.state.phase = phase;
        if let Some(visible) = self
            .visible
            .as_mut()
            .filter(|v| v.stamp.input.generation == ticket.generation)
        {
            visible.state = op.state.clone();
            visible.stamp.load_epoch = epoch;
        }
        self.epoch = epoch;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_counter_exhaustion_never_publishes_or_consumes_a_batch() {
        let key = StudyKey {
            source: IdentityAllocator::new().unwrap().source().unwrap(),
            stored_id: 0,
        };
        for counter in ["operation", "epoch", "generation"] {
            let mut manager = StudyManager::new(key);
            match counter {
                "operation" => manager.operation_id = u64::MAX,
                "epoch" => manager.epoch = u64::MAX,
                _ => manager.generation = u64::MAX,
            }
            assert_eq!(
                manager.begin(if counter == "generation" {
                    LoadKind::Reload
                } else {
                    LoadKind::Initial
                }),
                Err(StudyError::CounterExhausted)
            );
            assert!(manager.operation.is_none() && manager.acquire().is_none());
        }
        for counter in ["sequence", "revision", "epoch"] {
            let mut manager = StudyManager::new(key);
            let ticket = manager.begin(LoadKind::Initial).unwrap();
            manager
                .apply(
                    manager.header(ticket).unwrap(),
                    StudyBatch::Replace {
                        trials: vec![],
                        metadata: Default::default(),
                    },
                )
                .unwrap();
            let previous = manager.acquire().unwrap();
            match counter {
                "sequence" => manager.operation.as_mut().unwrap().next_sequence = u64::MAX,
                "revision" => manager.operation.as_mut().unwrap().revision = u64::MAX,
                _ => manager.epoch = u64::MAX,
            }
            let header = manager.header(ticket).unwrap();
            assert_eq!(
                manager.apply(
                    header,
                    StudyBatch::Replace {
                        trials: vec![],
                        metadata: StudyMetadata {
                            name: "changed".into(),
                            ..Default::default()
                        }
                    }
                ),
                Err(StudyError::CounterExhausted)
            );
            assert_eq!(manager.header(ticket).unwrap(), header);
            assert_eq!(manager.acquire().unwrap().stamp(), previous.stamp());
        }
    }
}
