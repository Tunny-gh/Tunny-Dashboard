use std::collections::HashMap;
use std::sync::Arc;

use arc_swap::ArcSwap;

use super::model::DataFrame;
use crate::data::extras::StudyExtras;

// ============================================================
// TASK-2329: shared study store (thread_local GLOBAL_STATE fully removed)
//
// The DataFrame for each study_id exists "exactly once" in this store.
// Both the UI thread and worker threads can lock-free clone an
// `Arc<DataFrame>` via `snapshot(study_id)`, eliminating the need for a
// persistent copy of row-oriented data (the old `Vec<TrialRow>`) (MEM-001).
// Each slot supports atomic swap on live updates via `ArcSwap` (TASK-2340).
// ============================================================

/// Store shared across all threads, mapping study_id → DataFrame snapshot.
///
/// `extras_slots` is a parallel map keyed by the same study_id that runs
/// alongside `slots` (DataFrame limited to COMPLETE trials) and holds
/// [`StudyExtras`] supplementary information for all trials (all states).
/// Like DataFrame, it supports atomic swap on live updates via `ArcSwap`.
pub struct SharedStudyStore {
    slots: HashMap<u32, ArcSwap<DataFrame>>,
    extras_slots: HashMap<u32, ArcSwap<StudyExtras>>,
    active_study_id: Option<u32>,
}

impl SharedStudyStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        SharedStudyStore {
            slots: HashMap::new(),
            extras_slots: HashMap::new(),
            active_study_id: None,
        }
    }

    /// Stores the DataFrames for all studies keyed by `study_id`. Resets active.
    /// Called as part of a fresh file load, so any stale `extras_slots` content is also discarded.
    pub fn store_all(&mut self, dataframes: Vec<(u32, DataFrame)>) {
        self.slots = dataframes
            .into_iter()
            .map(|(id, df)| (id, ArcSwap::from_pointee(df)))
            .collect();
        self.extras_slots.clear();
        self.active_study_id = None;
    }

    /// Stores `StudyExtras` for all studies keyed by `study_id` (replaces any existing extras).
    pub fn store_extras_all(&mut self, extras: Vec<(u32, StudyExtras)>) {
        self.extras_slots = extras
            .into_iter()
            .map(|(id, ex)| (id, ArcSwap::from_pointee(ex)))
            .collect();
    }

    /// Inserts or replaces `StudyExtras` for a single study (for loads keyed by real study_id).
    pub fn store_extras_for(&mut self, study_id: u32, extras: StudyExtras) {
        match self.extras_slots.get(&study_id) {
            Some(slot) => slot.store(std::sync::Arc::new(extras)),
            None => {
                self.extras_slots
                    .insert(study_id, ArcSwap::from_pointee(extras));
            }
        }
    }

    /// Clones and returns the `StudyExtras` snapshot for `study_id` (lock-free).
    pub fn extras_snapshot(&self, study_id: u32) -> Option<Arc<StudyExtras>> {
        self.extras_slots
            .get(&study_id)
            .map(|slot| slot.load_full())
    }

    /// Returns the `StudyExtras` snapshot for the active study.
    pub fn active_extras_snapshot(&self) -> Option<Arc<StudyExtras>> {
        self.active_study_id.and_then(|id| self.extras_snapshot(id))
    }

    /// Live update: atomically swaps the `StudyExtras` slot. Inserts a new one if absent.
    pub fn swap_extras(&mut self, study_id: u32, new_extras: Arc<StudyExtras>) {
        match self.extras_slots.get(&study_id) {
            Some(slot) => slot.store(new_extras),
            None => {
                self.extras_slots.insert(study_id, ArcSwap::new(new_extras));
            }
        }
    }

    /// Clones and returns the DataFrame snapshot for `study_id` (lock-free).
    pub fn snapshot(&self, study_id: u32) -> Option<Arc<DataFrame>> {
        self.slots.get(&study_id).map(|slot| slot.load_full())
    }

    /// Returns the snapshot for the active study.
    pub fn active_snapshot(&self) -> Option<Arc<DataFrame>> {
        self.active_study_id.and_then(|id| self.snapshot(id))
    }

    /// Live update: atomically swaps the snapshot of an existing slot (TASK-2340).
    /// Inserts a new one if the slot does not exist.
    pub fn swap_snapshot(&mut self, study_id: u32, new_df: Arc<DataFrame>) {
        match self.slots.get(&study_id) {
            Some(slot) => slot.store(new_df),
            None => {
                self.slots.insert(study_id, ArcSwap::new(new_df));
            }
        }
    }

    /// Sets the active study. Returns true if the slot exists.
    pub fn set_active(&mut self, study_id: u32) -> bool {
        if self.slots.contains_key(&study_id) {
            self.active_study_id = Some(study_id);
            true
        } else {
            false
        }
    }

    /// Number of stored studies.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

impl Default for SharedStudyStore {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// Store retention mechanism (cfg branch)
//
// - Production build: a process-global `RwLock<SharedStudyStore>`.
//   A single DataFrame instance is shared across all threads, letting the
//   UI thread obtain an `Arc<DataFrame>` without duplicating row-oriented
//   data (the heart of MEM-001).
// - Test build: thread_local. `cargo test` runs multi-threaded within a
//   single process, so this preserves state isolation between tests (a
//   property of the old thread_local design).
//   Note: the store logic itself is consolidated in `SharedStudyStore`; only
//   the retention mechanism branches here. The production cross-thread
//   sharing path is verified by integration tests on the egui-app side.
// ============================================================

/// A single process-global shared store. read/write share the same instance.
#[cfg(not(test))]
fn global_store() -> &'static std::sync::RwLock<SharedStudyStore> {
    use std::sync::{OnceLock, RwLock};
    static STORE: OnceLock<RwLock<SharedStudyStore>> = OnceLock::new();
    STORE.get_or_init(|| RwLock::new(SharedStudyStore::new()))
}

#[cfg(not(test))]
fn with_store_read<R>(f: impl FnOnce(&SharedStudyStore) -> R) -> R {
    let guard = global_store().read().unwrap_or_else(|p| p.into_inner());
    f(&guard)
}

#[cfg(not(test))]
fn with_store_write<R>(f: impl FnOnce(&mut SharedStudyStore) -> R) -> R {
    let mut guard = global_store().write().unwrap_or_else(|p| p.into_inner());
    f(&mut guard)
}

#[cfg(test)]
thread_local! {
    static TEST_STORE: std::cell::RefCell<SharedStudyStore> =
        std::cell::RefCell::new(SharedStudyStore::new());
}

#[cfg(test)]
fn with_store_read<R>(f: impl FnOnce(&SharedStudyStore) -> R) -> R {
    TEST_STORE.with(|s| f(&s.borrow()))
}

#[cfg(test)]
fn with_store_write<R>(f: impl FnOnce(&mut SharedStudyStore) -> R) -> R {
    TEST_STORE.with(|s| f(&mut s.borrow_mut()))
}

// ============================================================
// Public API (a single implementation independent of the retention mechanism)
// ============================================================

/// Stores the parsed DataFrames keyed by `study_id` (= Vec index).
/// Existing snapshots are replaced and active is reset.
pub fn store_dataframes(dfs: Vec<DataFrame>) {
    let pairs: Vec<(u32, DataFrame)> = dfs
        .into_iter()
        .enumerate()
        .map(|(i, df)| (i as u32, df))
        .collect();
    with_store_write(|store| store.store_all(pairs));
}

/// Sets the active study.
pub fn select_study(study_id: u32) -> Result<(), String> {
    with_store_write(|store| match store.snapshot(study_id) {
        Some(_) => {
            store.set_active(study_id);
            Ok(())
        }
        None => Err(format!(
            "study_id {} not found (total: {})",
            study_id,
            store.len()
        )),
    })
}

/// Applies a closure to the active study's DataFrame.
/// To minimize lock hold time, the `Arc` is obtained first and then applied.
pub fn with_active_df<T, F: FnOnce(&DataFrame) -> T>(f: F) -> Option<T> {
    let arc = with_store_read(|store| store.active_snapshot())?;
    Some(f(&arc))
}

/// Applies a closure to an arbitrary study's DataFrame.
pub fn with_df<T, F: FnOnce(&DataFrame) -> T>(study_id: u32, f: F) -> Option<T> {
    let arc = with_store_read(|store| store.snapshot(study_id))?;
    Some(f(&arc))
}

/// Gets an arbitrary study's DataFrame snapshot (`Arc`).
/// The entry point letting the UI thread reference column data without duplicating row-oriented data (MEM-001).
pub fn snapshot(study_id: u32) -> Option<Arc<DataFrame>> {
    with_store_read(|store| store.snapshot(study_id))
}

/// Gets the active study's DataFrame snapshot (`Arc`).
pub fn active_snapshot() -> Option<Arc<DataFrame>> {
    with_store_read(|store| store.active_snapshot())
}

/// Live update: atomically swaps a study's DataFrame snapshot (TASK-2340).
pub fn swap_snapshot(study_id: u32, new_df: Arc<DataFrame>) {
    with_store_write(|store| store.swap_snapshot(study_id, new_df));
}

/// Stores the parsed `StudyExtras` keyed by `study_id` (= Vec index).
/// Uses the enumerate convention paired with `store_dataframes` for a full journal parse.
pub fn store_extras(extras: Vec<StudyExtras>) {
    let pairs: Vec<(u32, StudyExtras)> = extras
        .into_iter()
        .enumerate()
        .map(|(i, ex)| (i as u32, ex))
        .collect();
    with_store_write(|store| store.store_extras_all(pairs));
}

/// Inserts or replaces a single study's `StudyExtras`, keyed by its real study_id.
/// Used for single-study loads (SQLite / journal streaming).
pub fn store_extras_for(study_id: u32, extras: StudyExtras) {
    with_store_write(|store| store.store_extras_for(study_id, extras));
}

/// Gets an arbitrary study's `StudyExtras` snapshot (`Arc`).
pub fn extras_snapshot(study_id: u32) -> Option<Arc<StudyExtras>> {
    with_store_read(|store| store.extras_snapshot(study_id))
}

/// Gets the active study's `StudyExtras` snapshot (`Arc`).
pub fn active_extras_snapshot() -> Option<Arc<StudyExtras>> {
    with_store_read(|store| store.active_extras_snapshot())
}

/// Live update: atomically swaps a study's `StudyExtras` snapshot.
pub fn swap_extras(study_id: u32, new_extras: Arc<StudyExtras>) {
    with_store_write(|store| store.swap_extras(study_id, new_extras));
}

// ============================================================
// Direct tests of SharedStudyStore logic
// (verifies the store's own correctness, independent of the retention mechanism cfg)
// ============================================================

#[cfg(test)]
mod store_tests {
    use super::*;

    fn df_with_rows(n: usize) -> DataFrame {
        use super::super::types::TrialRow;
        let rows: Vec<TrialRow> = (0..n)
            .map(|i| TrialRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display: std::collections::HashMap::new(),
                param_category_label: std::collections::HashMap::new(),
                objective_values: vec![i as f64],
                user_attrs_numeric: std::collections::HashMap::new(),
                user_attrs_string: std::collections::HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        DataFrame::from_trials(&rows, &[], &["obj0".to_string()], &[], &[], 0)
    }

    #[test]
    fn store_all_and_snapshot_by_id() {
        let mut store = SharedStudyStore::new();
        store.store_all(vec![(0, df_with_rows(3)), (1, df_with_rows(5))]);
        assert_eq!(store.len(), 2);
        assert_eq!(store.snapshot(0).unwrap().row_count(), 3);
        assert_eq!(store.snapshot(1).unwrap().row_count(), 5);
        assert!(store.snapshot(99).is_none());
    }

    #[test]
    fn store_all_resets_active() {
        let mut store = SharedStudyStore::new();
        store.store_all(vec![(0, df_with_rows(1))]);
        assert!(store.set_active(0));
        assert!(store.active_snapshot().is_some());
        store.store_all(vec![(0, df_with_rows(2))]);
        assert!(store.active_snapshot().is_none());
    }

    #[test]
    fn set_active_unknown_id_returns_false() {
        let mut store = SharedStudyStore::new();
        store.store_all(vec![(0, df_with_rows(1))]);
        assert!(!store.set_active(7));
        assert!(store.active_snapshot().is_none());
    }

    #[test]
    fn swap_snapshot_replaces_existing() {
        let mut store = SharedStudyStore::new();
        store.store_all(vec![(0, df_with_rows(2))]);
        store.swap_snapshot(0, Arc::new(df_with_rows(9)));
        assert_eq!(store.snapshot(0).unwrap().row_count(), 9);
    }

    #[test]
    fn swap_snapshot_inserts_when_absent() {
        let mut store = SharedStudyStore::new();
        store.swap_snapshot(4, Arc::new(df_with_rows(6)));
        assert_eq!(store.snapshot(4).unwrap().row_count(), 6);
    }

    #[test]
    fn snapshot_shares_arc_without_clone_of_data() {
        let mut store = SharedStudyStore::new();
        store.store_all(vec![(0, df_with_rows(4))]);
        let a = store.snapshot(0).unwrap();
        let b = store.snapshot(0).unwrap();
        assert!(
            Arc::ptr_eq(&a, &b),
            "同一スナップショットは同じ Arc を共有する"
        );
    }

    // ── extras slots ─────────────────────────────────────────────────

    use crate::data::extras::{StudyExtras, TrialExtra, TrialState};

    fn extras_with(trial_ids: &[u32]) -> StudyExtras {
        StudyExtras {
            trials: trial_ids
                .iter()
                .map(|&id| TrialExtra {
                    trial_id: id,
                    trial_number: id,
                    state: TrialState::Complete,
                    datetime_start: None,
                    datetime_complete: None,
                    intermediate_values: vec![],
                })
                .collect(),
        }
    }

    #[test]
    fn store_extras_all_and_snapshot_by_id() {
        let mut store = SharedStudyStore::new();
        store.store_extras_all(vec![(0, extras_with(&[0, 1])), (1, extras_with(&[2]))]);
        assert_eq!(store.extras_snapshot(0).unwrap().trials.len(), 2);
        assert_eq!(store.extras_snapshot(1).unwrap().trials.len(), 1);
        assert!(store.extras_snapshot(99).is_none());
    }

    #[test]
    fn store_extras_for_inserts_and_replaces() {
        let mut store = SharedStudyStore::new();
        store.store_extras_for(5, extras_with(&[0]));
        assert_eq!(store.extras_snapshot(5).unwrap().trials.len(), 1);
        store.store_extras_for(5, extras_with(&[0, 1, 2]));
        assert_eq!(store.extras_snapshot(5).unwrap().trials.len(), 3);
    }

    #[test]
    fn store_all_clears_stale_extras() {
        let mut store = SharedStudyStore::new();
        store.store_extras_for(0, extras_with(&[0, 1]));
        assert!(store.extras_snapshot(0).is_some());
        // Equivalent to a fresh file load. extras must also be discarded.
        store.store_all(vec![(0, df_with_rows(1))]);
        assert!(store.extras_snapshot(0).is_none());
    }

    #[test]
    fn swap_extras_replaces_and_inserts() {
        let mut store = SharedStudyStore::new();
        store.store_extras_for(0, extras_with(&[0]));
        store.swap_extras(0, Arc::new(extras_with(&[0, 1, 2, 3])));
        assert_eq!(store.extras_snapshot(0).unwrap().trials.len(), 4);
        // A swap onto a nonexistent slot is treated as an insert.
        store.swap_extras(7, Arc::new(extras_with(&[9])));
        assert_eq!(store.extras_snapshot(7).unwrap().trials.len(), 1);
    }

    #[test]
    fn active_extras_snapshot_follows_active_study() {
        let mut store = SharedStudyStore::new();
        store.store_all(vec![(0, df_with_rows(1))]);
        store.store_extras_for(0, extras_with(&[0, 1]));
        assert!(store.active_extras_snapshot().is_none());
        assert!(store.set_active(0));
        assert_eq!(store.active_extras_snapshot().unwrap().trials.len(), 2);
    }
}
