//! memory-efficiency 型定義（設計案）
//!
//! 作成日: 2026-05-29
//! 関連設計: architecture.md / dataflow.md
//!
//! 本ファイルは実装ではなく **設計上の型シグネチャ案** である。
//! 既存型（`egui-app/src/state/types.rs`, `rust_core/src/data/dataframe/model.rs`）からの
//! 変更点を中心に記述する。
//!
//! 信頼性レベル:
//! - 🔵 青信号: 要件定義・コード調査・ユーザヒアリングを参考にした確実な型定義
//! - 🟡 黄信号: それらから妥当な推測による型定義
//! - 🔴 赤信号: 根拠資料にない推測による型定義

use std::collections::HashMap;
use std::sync::Arc;
// 🟡 arc-swap クレートの追加が前提（実装時に Cargo.toml で確認 / design-interview.md 残課題）
// use arc_swap::ArcSwap;

// ============================================================
// rust_core: 共有 Arc ストア（state.rs 刷新）
// ============================================================

/// study_id をキーに、各 study の DataFrame スナップショットを共有保持するストア。
/// 🔵 ヒアリングQ1/Q2 2026-05-29 — 現行 thread_local GLOBAL_STATE を置換
///
/// 設計意図:
/// - 全スレッド（UI/ワーカー）から `Arc<DataFrame>` を安全にクローン取得できる。
/// - 各スロットは `ArcSwap` でライブ更新時に原子的に差し替える。
/// - 現行 `store_dataframes` / `select_study` / `with_active_df`（state.rs:27-80）の責務を引き継ぐ。
pub struct SharedStudyStore {
    /// study_id → 列指向 DataFrame の差替え可能スナップショット
    /// 🔵 ヒアリングQ1（ArcSwap）/ Q2（全 study 常駐）
    slots: HashMap<u32, ArcSwapDataFrame>, // ArcSwap<DataFrame> のエイリアス（下記）
    /// アクティブ study（現行 active_study_id 相当）
    /// 🔵 state.rs:9 より
    active_study_id: Option<u32>,
}

/// `ArcSwap<DataFrame>` のプレースホルダ型エイリアス。
/// 🟡 arc-swap 採用前提（design-interview.md 残課題）
pub type ArcSwapDataFrame = (); // 実装では `arc_swap::ArcSwap<DataFrame>`

impl SharedStudyStore {
    /// 全 study の DataFrame を初回パースで格納する。
    /// 🔵 ヒアリングQ2（全 study 常駐）/ 現行 store_dataframes(state.rs:27) を置換
    pub fn store_all(&mut self, _dataframes: Vec<(u32, /* DataFrame */ ())>) {
        unimplemented!("設計案: study_id ごとに ArcSwac スロットを構築")
    }

    /// study_id のスナップショットを Arc でクローン取得する（ロックフリー読み取り）。
    /// 🔵 ヒアリングQ1（load）/ 現行 with_active_df(state.rs:66) を置換
    pub fn snapshot(&self, _study_id: u32) -> Option<Arc<()/* DataFrame */>> {
        unimplemented!("設計案: slots[study_id].load_full()")
    }

    /// ライブ更新: 新スナップショットを原子的に差し替える。
    /// 🔵 ヒアリングQ1（ArcSwap store）/ message_handler.rs:209-268 の置換先
    pub fn swap_snapshot(&self, _study_id: u32, _new_df: Arc<()/* DataFrame */>) {
        unimplemented!("設計案: slots[study_id].store(new_df)")
    }
}

// ============================================================
// egui-app: StudyView（新規・UI 側軽量ビュー）
// ============================================================

/// 列指向 DataFrame スナップショットをラップし、UI 算出の行属性を並行配列で保持する軽量ビュー。
/// 🔵 ヒアリングQ3 2026-05-29 — 現行 `Vec<TrialRow>`（state/types.rs:61）を置換
///
/// 設計意図:
/// - 行指向 `Vec<TrialRow>` と per-row HashMap を保持しない（MEM-001 / REQ-001, REQ-101）。
/// - `DataFrame` にない UI 算出値（pareto_rank/cluster_id/state/trial_number）を並行配列で持つ。
/// - 列値はラップした `Arc<DataFrame>` のアクセサ（get_numeric_column 等）から借用で取得する。
pub struct StudyView {
    /// 共有ストアから取得した列データの不変スナップショット
    /// 🔵 ヒアリングQ1/Q3 — Arc 共有（複製なし）
    pub df: Arc<()/* tunny_core::dataframe::DataFrame */>,

    /// 行 index → trial_id（現行 TrialRow.trial_id 相当）
    /// 🔵 model.rs:175 get_trial_id 由来。DataFrame 内 trial_ids でも代替可
    pub trial_ids: Vec<u32>,

    /// Pareto ランク（行 index 順）。アプリ層算出。
    /// 🔵 現行 TrialRow.pareto_rank（types.rs:43）の置換先
    pub pareto_rank: Vec<u32>,

    /// クラスタ ID（行 index 順）。未割当は None。
    /// 🔵 現行 TrialRow.cluster_id（types.rs:44）の置換先
    pub cluster_id: Vec<Option<i32>>,

    /// 試行状態（行 index 順）。
    /// 🔵 現行 TrialRow.state（types.rs:45）の置換先
    pub state: Vec<TrialState>,
}

impl StudyView {
    /// 行数（= DataFrame.row_count）。
    /// 🔵 model.rs:210 row_count より
    pub fn row_count(&self) -> usize {
        unimplemented!("設計案: self.df.row_count()")
    }

    /// パラメータ列の借用スライス（行ごとの HashMap を作らない）。
    /// 🔵 MEM-004 / poll_chart.rs:16-22 の HashMap 参照を置換（REQ-008, REQ-009）
    pub fn numeric_column(&self, _name: &str) -> Option<&[f64]> {
        unimplemented!("設計案: self.df.get_numeric_column(name)")
    }

    /// 単一行の TrialRow を必要箇所だけ一時生成する互換ヘルパー（移行用・任意）。
    /// 🟡 既存ウィジェットの段階移行を想定した妥当な推測。最終的には列アクセスへ寄せる
    pub fn row_at(&self, _index: usize) -> TrialRow {
        unimplemented!("設計案: 列 + 並行配列から一時的に TrialRow を組み立て（永続保持しない）")
    }
}

// ============================================================
// egui-app: StudyContext 再設計（state/types.rs:59 を置換）
// ============================================================

/// 選択中 study のアプリ状態。
/// 🔵 現行 StudyContext（types.rs:59）から trial_rows / gpu_data を撤廃
///
/// 変更点:
/// - `trial_rows: Vec<TrialRow>` を削除 → `view: StudyView`（MEM-001）。
/// - `gpu_data: GpuBufferData` を削除（MEM-007 / REQ-013、描画側で未読）。
pub struct StudyContext {
    /// 🔵 現行どおり（types.rs:60）
    pub meta: StudyMeta,
    /// 🔵 ヒアリングQ3 — trial_rows の置換
    pub view: StudyView,
    /// 🔵 現行どおり（types.rs:63）— Pareto 前線インデックス
    pub pareto_indices: Vec<u32>,
    // pub trial_rows: Vec<TrialRow>,   // ❌ 削除（MEM-001）
    // pub gpu_data: GpuBufferData,     // ❌ 削除（MEM-007）
}

/// 比較対象 study の軽量表現。
/// 🔵 MEM-005 / 現行 comparison_studies: Vec<StudyContext>（app_state.rs:43）を置換
///
/// 設計意図:
/// - フル StudyContext を持たず、study_id 参照＋遅延 StudyView（REQ-010, REQ-011）。
/// - 比較削除で StudyView をドロップし Arc 参照を解放（REQ-201）。
pub struct ComparisonStudy {
    /// 🔵 共有ストア参照キー（再パース不要・ヒアリングQ2）
    pub study_id: u32,
    /// 🔵 一覧表示用の軽量メタ
    pub meta: StudyMeta,
    /// 🟡 描画に入った時点で遅延構築（妥当な推測）
    pub view: Option<StudyView>,
}

// ============================================================
// egui-app: メッセージ型の変更（state/messages.rs）
// ============================================================

/// study 選択完了メッセージ（現行 AppMessage::StudySelected の置換案）。
/// 🔵 ヒアリングQ1/Q3 — trial_rows / gpu_data を運ばない
///
/// 変更点:
/// - `trial_rows: Vec<TrialRow>` / `gpu_data: GpuBufferData` を撤廃。
/// - 列データは共有ストアにあるため、study_id と派生属性のみ運ぶ。
pub struct StudySelectedPayload {
    /// 🔵 message_handler.rs:25-43 より
    pub meta: StudyMeta,
    /// 🔵 共有ストア参照キー
    pub study_id: u32,
    /// 🔵 派生属性（並行配列）。pareto_rank/state 等
    pub pareto_rank: Vec<u32>,
    pub state: Vec<TrialState>,
    /// 🔵 現行どおり（messages 由来）
    pub pareto_indices: Vec<u32>,
}

// ============================================================
// 既存（参照のため再掲・変更なし）
// ============================================================

/// 🔵 現行どおり（state/types.rs:13-21）
#[derive(Debug, Clone, Default, PartialEq)]
pub enum TrialState {
    #[default]
    Complete,
    Running,
    Pruned,
    Fail,
    Waiting,
}

/// 🔵 現行どおり（state/types.rs:23-34）
pub struct StudyMeta {
    pub study_id: u32,
    pub name: String,
    pub directions: Vec<Direction>,
    pub completed_trials: usize,
    pub total_trials: usize,
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub user_attr_names: Vec<String>,
    pub has_constraints: bool,
}

/// 🟡 移行期のみ存在しうる互換型（最終的には列アクセスへ寄せ、永続保持はしない）
/// 現行 state/types.rs:37-47。MEM-001 により StudyContext からは除去される
pub struct TrialRow {
    pub trial_id: u32,
    pub trial_number: u32,
    pub params: HashMap<String, f64>,
    pub objectives: Vec<f64>,
    pub pareto_rank: u32,
    pub cluster_id: Option<i32>,
    pub state: TrialState,
    pub user_attrs: HashMap<String, String>,
}

pub use Direction as _Direction; // placeholder
/// 🔵 現行どおり（state/types.rs:7-11）
pub enum Direction {
    Minimize,
    Maximize,
}

// ============================================================
// ウィジェットキャッシュの変更方針（型コメントのみ）
// ============================================================
//
// 🔵 MEM-002: pareto_2d.rs:38 / pareto_3d.rs:99
//   - `display_rows_cache: Option<Vec<TrialRow>>` を撤廃。
//   + 代替: `point_cache: Option<Vec<[f32; 2]>>`（または 3D は [f32;3]）＋ `rank_slice` 等の最小データ。
//
// 🔵 MEM-003: parallel_coords.rs:71 / scatter_matrix.rs:31
//   - 各々の `col_data_cache: Option<Vec<Vec<f64>>>` を撤廃または共有化。
//   + 代替: StudyView の列スライス借用、もしくは一元管理された派生列キャッシュへの参照。
//
// 🔵 MEM-004: poll_chart.rs:16-22
//   - `r.params.get(p)` からの `Vec<Vec<f64>>` 実行時再構築を撤廃。
//   + 代替: `StudyView::numeric_column()` の借用スライス／フラットバッファ。

// ============================================================
// 信頼性レベルサマリー
// ============================================================
// - 🔵 青信号: 28件 (90%)
// - 🟡 黄信号: 3件 (10%)  // arc-swap 依存、row_at 互換ヘルパー、ComparisonStudy.view 遅延
// - 🔴 赤信号: 0件 (0%)
//
// 品質評価: 高品質
