//! Caller-owned loaded-Study collection. All formats emit owned I/O batches;
//! #252 replaces projections; #253 removes legacy presentation-store mirrors.
use super::app_state::StudyContext;
use super::messages::AppMessage;
use std::collections::{HashMap, VecDeque};
use std::sync::{mpsc, Arc};
use tdashboard_study::*;

#[derive(Debug)]
pub struct JournalStudies {
    pub source: SourceInstanceId,
    pub scan: u64,
    pub bytes: Option<Arc<Vec<u8>>>,
    pub path: Option<std::path::PathBuf>,
    pub csv_metadata: Option<super::app_state::StudyMeta>,
    pub managers: HashMap<u32, StudyManager>,
    pub selected: Option<u32>,
    /// Acquisition bytes used by each attempt. A storage reload leaves unrelated
    /// inputs valid, but their next activation must read the newly acquired bytes.
    requested_scans: HashMap<u32, u64>,
    /// Presentation handoff follows request order, not worker completion order.
    comparisons: VecDeque<ComparisonRequest>,
    pub projections: HashMap<u32, tunny_core::journal_parser::study_projection::TableProjection>,
}

#[derive(Debug)]
struct ComparisonRequest {
    key: StudyKey,
    /// None = pending; Some(None) = failed; Some(Some(view)) = ready.
    result: Option<Option<StudyContext>>,
}

impl JournalStudies {
    pub fn new(scan: u64) -> Result<Self, StudyError> {
        let source = IdentityAllocator::new()?.source()?;
        Ok(Self {
            source,
            scan,
            bytes: None,
            path: None,
            csv_metadata: None,
            managers: HashMap::new(),
            selected: None,
            requested_scans: HashMap::new(),
            comparisons: VecDeque::new(),
            projections: HashMap::new(),
        })
    }
    pub fn invalidate_loaders(&mut self, scan: u64) {
        self.scan = scan;
        self.bytes = None;
        self.clear_comparisons();
        for manager in self.managers.values_mut() {
            if let Some(state) = manager
                .load_state()
                .filter(|s| s.phase == LoadPhase::Loading)
            {
                let ticket = state.ticket;
                let _ = manager.supersede(ticket);
            }
        }
    }
    pub fn clear_comparisons(&mut self) {
        self.comparisons.clear();
    }
    pub fn has_comparison(&self, key: StudyKey) -> bool {
        self.comparisons.iter().any(|r| r.key == key)
    }
    fn reserve_comparison(&mut self, key: StudyKey) {
        if let Some(request) = self.comparisons.iter_mut().find(|r| r.key == key) {
            request.result = None;
        } else {
            self.comparisons
                .push_back(ComparisonRequest { key, result: None });
        }
    }
    /// A failed request releases its slot without adding a view or assigning a
    /// color. Later ready views drain in the same order as the old serial worker.
    pub fn complete_comparison(
        &mut self,
        key: StudyKey,
        context: Option<StudyContext>,
    ) -> Vec<StudyContext> {
        let Some(request) = self.comparisons.iter_mut().find(|r| r.key == key) else {
            return Vec::new();
        };
        request.result = Some(context);
        let mut ready = Vec::new();
        while self.comparisons.front().is_some_and(|r| r.result.is_some()) {
            if let Some(Some(context)) = self.comparisons.pop_front().unwrap().result {
                ready.push(context);
            }
        }
        ready
    }
    pub fn request(
        &mut self,
        id: u32,
        comparison: bool,
        force: bool,
        tx: mpsc::SyncSender<AppMessage>,
    ) -> Result<Option<AppMessage>, String> {
        let bytes = self
            .bytes
            .clone()
            .ok_or("No journal is loaded yet. Please open a journal first.")?;
        let source = if let Some(path) = &self.path {
            if let Some(url) = crate::io::rdb::path_as_rdb_url(path) {
                tdashboard_io::storage::ReadSource::Rdb(url)
            } else if crate::io::sqlite::is_sqlite_path(path) {
                tdashboard_io::storage::ReadSource::Sqlite(path.clone())
            } else if crate::io::flat_csv::is_csv_path(path) {
                tdashboard_io::storage::ReadSource::Csv {
                    bytes: bytes.clone(),
                    name: path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("csv")
                        .to_string(),
                }
            } else {
                tdashboard_io::storage::ReadSource::Journal(bytes.clone())
            }
        } else {
            tdashboard_io::storage::ReadSource::Journal(bytes.clone())
        };
        let csv_metadata = self.csv_metadata.clone();
        let key = StudyKey {
            source: self.source,
            stored_id: id,
        };
        if comparison && !force && self.has_comparison(key) {
            return Ok(None);
        }
        if !comparison {
            self.selected = Some(id);
        }
        let manager = self.managers.entry(id).or_insert_with(|| {
            StudyManager::new(StudyKey {
                source: self.source,
                stored_id: id,
            })
        });
        // Primary selection and comparison are interests in the same acquisition,
        // not competing writers. Keep its original full/streaming order and ticket.
        if !force
            && self.requested_scans.get(&id) == Some(&self.scan)
            && manager
                .load_state()
                .is_some_and(|s| s.phase == LoadPhase::Loading)
        {
            if comparison {
                self.reserve_comparison(key);
            }
            return Ok(None);
        }
        if !force
            && self.requested_scans.get(&id) == Some(&self.scan)
            && manager
                .load_state()
                .is_some_and(|s| s.phase == LoadPhase::Complete)
            && manager
                .acquire()
                .is_some_and(|i| i.load_state().phase == LoadPhase::Complete)
        {
            let key = manager.key();
            if comparison {
                self.reserve_comparison(key);
            }
            // The caller is the GUI/channel consumer. Return local work rather
            // than waiting for space in its own bounded delivery channel.
            return Ok(Some(AppMessage::JournalActivate { key }));
        }
        let kind = if manager.acquire().is_some() || manager.load_state().is_some() {
            LoadKind::Reload
        } else {
            LoadKind::Initial
        };
        let ticket = manager.begin(kind).map_err(|e| e.to_string())?;
        self.requested_scans.insert(id, self.scan);
        let mut header = manager.header(ticket).map_err(|e| e.to_string())?;
        if comparison || self.has_comparison(key) {
            // A forced replacement transfers any pending comparison interest to
            // the new ticket, even when the replacement was requested as primary.
            self.reserve_comparison(key);
        }
        std::thread::spawn(move || {
            let mut deliver = |mut batch| {
                if let (Some(meta), tdashboard_study::StudyBatch::Replace { metadata, .. }) =
                    (&csv_metadata, &mut batch)
                {
                    metadata
                        .objectives
                        .iter_mut()
                        .zip(&meta.directions)
                        .for_each(|(o, d)| {
                            o.direction = match d {
                                super::app_state::Direction::Minimize => Direction::Minimize,
                                super::app_state::Direction::Maximize => Direction::Maximize,
                            };
                        });
                    metadata.declared_bounds = meta
                        .param_bounds
                        .iter()
                        .map(|(k, (l, h))| (k.clone(), (Number(*l), Number(*h))))
                        .collect();
                }
                let (reply, ack) = mpsc::sync_channel(1);
                tx.send(AppMessage::JournalBatch {
                    header,
                    batch,
                    reply,
                })
                .map_err(|e| e.to_string())?;
                let receipt = ack.recv().map_err(|e| e.to_string())??;
                header = receipt.next_header(ticket);
                Ok(())
            };
            let result = source.read(ticket.study(), !comparison, &mut deliver);
            let _ = tx.send(AppMessage::JournalFinished { header, result });
        });
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::StudyMeta;

    fn context(id: u32) -> StudyContext {
        StudyContext::from_rows_for_test(
            StudyMeta {
                study_id: id,
                name: format!("s{id}"),
                directions: vec![],
                completed_trials: 0,
                param_names: vec![],
                objective_names: vec![],
                param_bounds: Default::default(),
            },
            vec![],
        )
    }

    #[test]
    fn comparison_failure_releases_later_ready_views_and_reset_discards_pending_views() {
        let mut journal = JournalStudies::new(0).unwrap();
        let first = StudyKey {
            source: journal.source,
            stored_id: 1,
        };
        let second = StudyKey {
            stored_id: 2,
            ..first
        };
        journal.reserve_comparison(first);
        journal.reserve_comparison(second);
        assert!(journal
            .complete_comparison(second, Some(context(2)))
            .is_empty());
        let ready = journal.complete_comparison(first, None);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].meta.study_id, 2);
        journal.reserve_comparison(first);
        journal.reserve_comparison(second);
        assert!(journal
            .complete_comparison(second, Some(context(2)))
            .is_empty());
        journal.clear_comparisons();
        assert!(journal
            .complete_comparison(first, Some(context(1)))
            .is_empty());
    }
}
