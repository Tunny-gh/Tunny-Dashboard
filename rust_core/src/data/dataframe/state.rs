use std::collections::HashMap;
use std::sync::Arc;

use arc_swap::ArcSwap;

use super::model::DataFrame;
use super::types::SelectStudyResult;

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
pub struct SharedStudyStore {
    slots: HashMap<u32, ArcSwap<DataFrame>>,
    active_study_id: Option<u32>,
}

impl SharedStudyStore {
    /// 空ストアを生成する。
    pub fn new() -> Self {
        SharedStudyStore {
            slots: HashMap::new(),
            active_study_id: None,
        }
    }

    /// 全 study の DataFrame を `study_id` キーで格納する。active はリセットする。
    pub fn store_all(&mut self, dataframes: Vec<(u32, DataFrame)>) {
        self.slots = dataframes
            .into_iter()
            .map(|(id, df)| (id, ArcSwap::from_pointee(df)))
            .collect();
        self.active_study_id = None;
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

#[cfg(not(test))]
fn with_store_read<R>(f: impl FnOnce(&SharedStudyStore) -> R) -> R {
    use std::sync::{OnceLock, RwLock};
    static STORE: OnceLock<RwLock<SharedStudyStore>> = OnceLock::new();
    let lock = STORE.get_or_init(|| RwLock::new(SharedStudyStore::new()));
    let guard = lock.read().unwrap_or_else(|p| p.into_inner());
    f(&guard)
}

#[cfg(not(test))]
fn with_store_write<R>(f: impl FnOnce(&mut SharedStudyStore) -> R) -> R {
    use std::sync::{OnceLock, RwLock};
    static STORE: OnceLock<RwLock<SharedStudyStore>> = OnceLock::new();
    let lock = STORE.get_or_init(|| RwLock::new(SharedStudyStore::new()));
    let mut guard = lock.write().unwrap_or_else(|p| p.into_inner());
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

/// アクティブ study を設定し、`DataFrameInfo` と GPU バッファ初期値を返す。
pub fn select_study(study_id: u32) -> Result<SelectStudyResult, String> {
    with_store_write(|store| match store.snapshot(study_id) {
        Some(df) => {
            let result = SelectStudyResult {
                data_frame_info: df.info(),
                gpu_buffer_data: df.gpu_buffers(),
            };
            store.set_active(study_id);
            Ok(result)
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
        assert!(Arc::ptr_eq(&a, &b), "同一スナップショットは同じ Arc を共有する");
    }
}
