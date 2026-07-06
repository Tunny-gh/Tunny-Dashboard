use std::collections::HashMap;
use std::sync::Arc;

use arc_swap::ArcSwap;

use super::model::DataFrame;
use crate::data::extras::StudyExtras;

// ============================================================
// TASK-2329: 共有 study ストア（thread_local GLOBAL_STATE を全廃）
//
// DataFrame の実体は本ストアに study_id ごと「ただ一度」存在する。
// UI スレッド・ワーカースレッドのいずれからも `snapshot(study_id)` で
// `Arc<DataFrame>` をロックフリーにクローン取得できるため、行指向データの
// 永続複製（旧 `Vec<TrialRow>`）が不要になる（MEM-001）。
// 各スロットは `ArcSwap` でライブ更新時に原子的差替えが可能（TASK-2340）。
// ============================================================

/// study_id → DataFrame スナップショットを全スレッドから共有する store。
///
/// `extras_slots` は `slots`（COMPLETE 限定 DataFrame）と並走する、全 trial（全 state）の
/// 付帯情報 [`StudyExtras`] を同じ study_id キーで保持する並列マップ。DataFrame と同じく
/// `ArcSwap` によりライブ更新時の原子的差替えが可能。
pub struct SharedStudyStore {
    slots: HashMap<u32, ArcSwap<DataFrame>>,
    extras_slots: HashMap<u32, ArcSwap<StudyExtras>>,
    active_study_id: Option<u32>,
}

impl SharedStudyStore {
    /// 空ストアを生成する。
    pub fn new() -> Self {
        SharedStudyStore {
            slots: HashMap::new(),
            extras_slots: HashMap::new(),
            active_study_id: None,
        }
    }

    /// 全 study の DataFrame を `study_id` キーで格納する。active はリセットする。
    /// 新規ファイルロードの一部として呼ばれるため、`extras_slots` の古い内容も破棄する。
    pub fn store_all(&mut self, dataframes: Vec<(u32, DataFrame)>) {
        self.slots = dataframes
            .into_iter()
            .map(|(id, df)| (id, ArcSwap::from_pointee(df)))
            .collect();
        self.extras_slots.clear();
        self.active_study_id = None;
    }

    /// 全 study の `StudyExtras` を `study_id` キーで格納する（既存 extras は置き換え）。
    pub fn store_extras_all(&mut self, extras: Vec<(u32, StudyExtras)>) {
        self.extras_slots = extras
            .into_iter()
            .map(|(id, ex)| (id, ArcSwap::from_pointee(ex)))
            .collect();
    }

    /// 単一 study の `StudyExtras` を挿入または置き換える（実 study_id キーのロード用）。
    pub fn store_extras_for(&mut self, study_id: u32, extras: StudyExtras) {
        match self.extras_slots.get(&study_id) {
            Some(slot) => slot.store(std::sync::Arc::new(extras)),
            None => {
                self.extras_slots
                    .insert(study_id, ArcSwap::from_pointee(extras));
            }
        }
    }

    /// `study_id` の `StudyExtras` スナップショットをクローン取得する（ロックフリー）。
    pub fn extras_snapshot(&self, study_id: u32) -> Option<Arc<StudyExtras>> {
        self.extras_slots
            .get(&study_id)
            .map(|slot| slot.load_full())
    }

    /// アクティブ study の `StudyExtras` スナップショットを取得する。
    pub fn active_extras_snapshot(&self) -> Option<Arc<StudyExtras>> {
        self.active_study_id.and_then(|id| self.extras_snapshot(id))
    }

    /// ライブ更新: `StudyExtras` スロットを原子的に差し替える。存在しなければ新規挿入する。
    pub fn swap_extras(&mut self, study_id: u32, new_extras: Arc<StudyExtras>) {
        match self.extras_slots.get(&study_id) {
            Some(slot) => slot.store(new_extras),
            None => {
                self.extras_slots.insert(study_id, ArcSwap::new(new_extras));
            }
        }
    }

    /// `study_id` の DataFrame スナップショットをクローン取得する（ロックフリー）。
    pub fn snapshot(&self, study_id: u32) -> Option<Arc<DataFrame>> {
        self.slots.get(&study_id).map(|slot| slot.load_full())
    }

    /// アクティブ study のスナップショットを取得する。
    pub fn active_snapshot(&self) -> Option<Arc<DataFrame>> {
        self.active_study_id.and_then(|id| self.snapshot(id))
    }

    /// ライブ更新: 既存スロットのスナップショットを原子的に差し替える（TASK-2340）。
    /// スロットが存在しない場合は新規挿入する。
    pub fn swap_snapshot(&mut self, study_id: u32, new_df: Arc<DataFrame>) {
        match self.slots.get(&study_id) {
            Some(slot) => slot.store(new_df),
            None => {
                self.slots.insert(study_id, ArcSwap::new(new_df));
            }
        }
    }

    /// アクティブ study を設定する。スロットが存在すれば true。
    pub fn set_active(&mut self, study_id: u32) -> bool {
        if self.slots.contains_key(&study_id) {
            self.active_study_id = Some(study_id);
            true
        } else {
            false
        }
    }

    /// 格納 study 数。
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// 空判定。
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
// ストア保持機構（cfg 分岐）
//
// - 本番ビルド: プロセスグローバルな `RwLock<SharedStudyStore>`。
//   全スレッドで単一の DataFrame 実体を共有し、UI スレッドが行指向データを
//   複製せずに `Arc<DataFrame>` を取得できる（MEM-001 の本丸）。
// - テストビルド: thread_local。`cargo test` は1プロセス・マルチスレッド並列で
//   走るため、テスト間の状態分離（旧 thread_local 設計の特性）を保つ。
//   ※ ストアのロジックは `SharedStudyStore` 本体に集約され、保持機構のみ分岐する。
//     クロススレッド共有の本番経路は egui-app 側の統合テストで検証する。
// ============================================================

/// プロセスグローバルな単一の共有ストア。read/write で同一インスタンスを共有する。
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
// 公開 API（保持機構に依存しない単一実装）
// ============================================================

/// パース結果の DataFrame 群を `study_id`（= Vec インデックス）キーで格納する。
/// 既存スナップショットは置き換えられ、active はリセットされる。
pub fn store_dataframes(dfs: Vec<DataFrame>) {
    let pairs: Vec<(u32, DataFrame)> = dfs
        .into_iter()
        .enumerate()
        .map(|(i, df)| (i as u32, df))
        .collect();
    with_store_write(|store| store.store_all(pairs));
}

/// アクティブ study を設定する。
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

/// アクティブ study の DataFrame に対してクロージャを適用する。
/// ロック保持時間を最小化するため、先に `Arc` を取得してから適用する。
pub fn with_active_df<T, F: FnOnce(&DataFrame) -> T>(f: F) -> Option<T> {
    let arc = with_store_read(|store| store.active_snapshot())?;
    Some(f(&arc))
}

/// 任意 study の DataFrame に対してクロージャを適用する。
pub fn with_df<T, F: FnOnce(&DataFrame) -> T>(study_id: u32, f: F) -> Option<T> {
    let arc = with_store_read(|store| store.snapshot(study_id))?;
    Some(f(&arc))
}

/// 任意 study の DataFrame スナップショット（`Arc`）を取得する。
/// UI スレッドが行指向データを複製せずに列データを参照するための入口（MEM-001）。
pub fn snapshot(study_id: u32) -> Option<Arc<DataFrame>> {
    with_store_read(|store| store.snapshot(study_id))
}

/// アクティブ study の DataFrame スナップショット（`Arc`）を取得する。
pub fn active_snapshot() -> Option<Arc<DataFrame>> {
    with_store_read(|store| store.active_snapshot())
}

/// ライブ更新: study の DataFrame スナップショットを原子的に差し替える（TASK-2340）。
pub fn swap_snapshot(study_id: u32, new_df: Arc<DataFrame>) {
    with_store_write(|store| store.swap_snapshot(study_id, new_df));
}

/// パース結果の `StudyExtras` 群を `study_id`（= Vec インデックス）キーで格納する。
/// journal の全体パース（`store_dataframes` と対）で使う enumerate 規約。
pub fn store_extras(extras: Vec<StudyExtras>) {
    let pairs: Vec<(u32, StudyExtras)> = extras
        .into_iter()
        .enumerate()
        .map(|(i, ex)| (i as u32, ex))
        .collect();
    with_store_write(|store| store.store_extras_all(pairs));
}

/// 単一 study の `StudyExtras` を実 study_id キーで挿入または置き換える。
/// 単一 study ロード（SQLite / journal streaming）で使う。
pub fn store_extras_for(study_id: u32, extras: StudyExtras) {
    with_store_write(|store| store.store_extras_for(study_id, extras));
}

/// 任意 study の `StudyExtras` スナップショット（`Arc`）を取得する。
pub fn extras_snapshot(study_id: u32) -> Option<Arc<StudyExtras>> {
    with_store_read(|store| store.extras_snapshot(study_id))
}

/// アクティブ study の `StudyExtras` スナップショット（`Arc`）を取得する。
pub fn active_extras_snapshot() -> Option<Arc<StudyExtras>> {
    with_store_read(|store| store.active_extras_snapshot())
}

/// ライブ更新: study の `StudyExtras` スナップショットを原子的に差し替える。
pub fn swap_extras(study_id: u32, new_extras: Arc<StudyExtras>) {
    with_store_write(|store| store.swap_extras(study_id, new_extras));
}

// ============================================================
// SharedStudyStore ロジックの直接テスト
// （保持機構 cfg に依存せず、ストア本体の正しさを検証する）
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

    // ── extras スロット ─────────────────────────────────────────────────

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
        // 新規ファイルロード相当。extras も破棄されなければならない。
        store.store_all(vec![(0, df_with_rows(1))]);
        assert!(store.extras_snapshot(0).is_none());
    }

    #[test]
    fn swap_extras_replaces_and_inserts() {
        let mut store = SharedStudyStore::new();
        store.store_extras_for(0, extras_with(&[0]));
        store.swap_extras(0, Arc::new(extras_with(&[0, 1, 2, 3])));
        assert_eq!(store.extras_snapshot(0).unwrap().trials.len(), 4);
        // 未存在スロットへの swap は新規挿入。
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
