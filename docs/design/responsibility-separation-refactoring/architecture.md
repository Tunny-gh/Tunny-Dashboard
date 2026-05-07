# 責務分離リファクタリング アーキテクチャ設計

**作成日**: 2026-04-15
**関連要件**: コード保守性向上・責務の分離
**ヒアリング記録**: [design-interview.md](design-interview.md)
**層境界契約**: [layer-contract.md](layer-contract.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 既存コード分析・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: 既存コード分析・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: 既存コード分析・ユーザヒアリングにない推測による設計

---

## 1. リファクタリング概要 🔵

**信頼性**: 🔵 *既存コード分析・ユーザヒアリングより*

egui-app クレート内の単一責任原則（SRP）違反を解消し、保守性を向上させるリファクタリング。
3フェーズで段階的に実施し、各フェーズでテストが通る状態を維持する。

### 現状の問題点

| ファイル | 行数 | 問題 |
|---|---|---|
| `app_state.rs` | 648行 | データモデル・フィルタリングロジック・分析結果・キャッシュが1ファイルに混在 |
| `grid_canvas.rs` | 418行 | グリッド描画 + チャートディスパッチ（show_chart）が混在 |
| `app.rs` | 212行 | メッセージハンドリングが AppState/WidgetStates を直接変更 |

## 2. アーキテクチャパターン 🔵

**信頼性**: 🔵 *ユーザーヒアリングより*

- **パターン**: egui MVU + Registry パターン
- **選択理由**: 段階的リファクタリングで既存テストを維持しつつ、責務境界を明確化するため

### 設計原則

- **単一責任原則**: 各モジュールは1つの関心事のみを持つ
- **段階的移行**: 各フェーズ完了時に `cargo test` が通ることを保証
- **後方互換性**: public API のシグネチャ変更は最小限

## 3. フェーズ別アーキテクチャ変更

### Phase 1: 型・フィルター分離 🔵

**信頼性**: 🔵 *ユーザーヒアリング・既存コード分析より*

`app_state.rs` を3ファイルに分割:

```
state/
├── types.rs          # データモデル（StudyMeta, TrialRow, GpuBufferData 等）
├── filter.rs         # フィルタリングロジック（apply_filters, set_filter, brush_select 等）
├── results.rs        # 分析結果型（SensitivityResult, SobolResult, ClusterResult 等）+ キャッシュ
├── app_state.rs      # AppState 構造体のみ（各モジュールを再エクスポート）
├── layout_state.rs   # （変更なし）
└── messages.rs       # （変更なし）
```

#### types.rs に移動する型 🔵

**信頼性**: 🔵 *既存コード app_state.rs:8-101 より*

```rust
// types.rs
pub enum Direction { Minimize, Maximize }
pub enum TrialState { Complete, Running, Pruned, Fail, Waiting }
pub struct StudyMeta { ... }
pub struct TrialRow { ... }
pub struct GpuBufferData { ... }
pub struct StudyContext { ... }
pub enum ColorMode { ... }
```

#### filter.rs に移動するロジック 🔵

**信頼性**: 🔵 *既存コード app_state.rs:246-327 より*

```rust
// filter.rs
pub struct DownsampleCache { ... }
pub fn should_resample(current_rate: f64, last_rate: f64) -> bool { ... }

impl AppState {
    pub fn set_filter(&mut self, param: &str, min: f64, max: f64) { ... }
    pub fn remove_filter(&mut self, param: &str) { ... }
    pub fn clear_filters(&mut self) { ... }
    pub fn brush_select(&mut self, indices: Vec<u32>) { ... }
    fn apply_filters(&mut self) { ... }
}
```

#### results.rs に移動する型 🔵

**信頼性**: 🔵 *既存コード app_state.rs:107-206 より*

```rust
// results.rs
pub struct SensitivityResult { ... }
pub struct RidgeResult { ... }
pub struct RfAnovaResult { ... }
pub struct SobolResult { ... }
pub struct ClusterResult { ... }
pub struct TopsisResult { ... }
pub struct HvHistory { ... }
pub struct LiveUpdateState { ... }
```

#### app_state.rs の変更後 🔵

**信頼性**: 🔵 *既存コード分析より*

```rust
// app_state.rs
mod types; mod filter; mod results;
pub use types::*;
pub use filter::*;
pub use results::*;

pub struct AppState {
    pub all_studies: Vec<StudyMeta>,
    pub journal_path: Option<PathBuf>,
    pub current_study: Option<StudyContext>,
    pub selected_indices: Vec<u32>,
    pub filter_ranges: HashMap<String, (f64, f64)>,
    pub highlighted_trial: Option<u32>,
    pub color_mode: ColorMode,
    pub sensitivity_result: Option<SensitivityResult>,
    pub sobol_result: Option<SobolResult>,
    pub cluster_result: Option<ClusterResult>,
    pub downsample_cache: DownsampleCache,
    pub live_update: LiveUpdateState,
    pub topsis_result: Option<TopsisResult>,
    pub hv_history: Option<HvHistory>,
}
```

### Phase 2: チャートディスパッチ分離 🔵

**信頼性**: 🔵 *ユーザーヒアリング（レジストリパターン採用）・既存コード grid_canvas.rs:257-371 より*

`grid_canvas.rs` の `show_chart()` 関数を `ui/chart_registry.rs` に抽出:

```
ui/
├── chart_registry.rs    # 新規: チャートディスパッチロジック
├── grid_canvas.rs       # グリッド描画に専任（show_chart 呼び出しを chart_registry に委譲）
├── layout.rs
├── left_panel.rs
├── main_canvas.rs
├── toolbar.rs
├── bottom_panel.rs
└── right_panel.rs
```

#### chart_registry.rs の設計 🔵

**信頼性**: 🔵 *既存コード grid_canvas.rs:257-371 の抽出・ユーザーヒアリングより*

```rust
// chart_registry.rs
use crate::state::app_state::AppState;
use crate::state::layout_state::ChartId;
use crate::state::messages::AppMessage;
use crate::ui::widget_states::WidgetStates;
use std::sync::mpsc;

/// ChartId に対応するチャートウィジェットを描画する
pub fn show_chart(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    chart_id: &ChartId,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    // grid_canvas.rs から show_chart() の中身をそのまま移動
    match chart_id {
        ChartId::ParetoScatter2D => { ... }
        ChartId::OptimizationHistory => { ... }
        // ... 各チャート
    }
}

/// タイトルと区切り線付きでチャートを描画する
pub fn show_cell_chart(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    chart_id: &ChartId,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    ui.label(egui::RichText::new(chart_id.label()).strong());
    ui.separator();
    show_chart(ui, app_state, widgets, chart_id, tx);
}
```

#### grid_canvas.rs の変更後 🔵

**信頼性**: 🔵 *既存コード分析より*

`grid_canvas.rs` はグリッド描画に専念し、チャートの描画は `chart_registry::show_cell_chart()` に委譲する:

```rust
// grid_canvas.rs の render_cell_content 内
fn render_cell_content(...) {
    match &cell.content {
        Some(PanelItem::Chart(id)) => {
            let id = id.clone();
            crate::ui::chart_registry::show_cell_chart(ui, app_state, widgets, &id, tx);
        }
        Some(PanelItem::TrialTable) => { ... }
        None => { ... }
    }
}
```

### Phase 3: メッセージ処理分離 🔵

**信頼性**: 🔵 *ユーザーヒアリング（MessageHandler 抽出）・既存コード app.rs:38-124 より*

`app.rs` の `poll_messages()` を `state/message_handler.rs` に抽出:

```
state/
├── message_handler.rs   # 新規: メッセージ処理ロジック
├── app_state.rs
├── filter.rs
├── results.rs
├── types.rs
├── layout_state.rs
└── messages.rs
```

#### message_handler.rs の設計 🔵

**信頼性**: 🔵 *既存コード app.rs:38-124 の抽出・ユーザーヒアリングより*

```rust
// message_handler.rs
use crate::state::app_state::{AppState, StudyContext};
use crate::state::messages::{AppMessage, DownsampleKey};
use crate::ui::widget_states::WidgetStates;

/// バックグラウンドタスクからのメッセージを処理する
pub struct MessageHandler;

impl MessageHandler {
    /// 単一メッセージを処理し、AppState と WidgetStates を更新する
    pub fn handle(
        msg: AppMessage,
        app_state: &mut AppState,
        widget_states: &mut WidgetStates,
        is_loading: &mut bool,
        load_error: &mut Option<String>,
    ) {
        match msg {
            AppMessage::JournalParsed { studies, path } => {
                app_state.all_studies = studies;
                app_state.journal_path = Some(path);
                *is_loading = false;
            }
            AppMessage::StudySelected { meta, trial_rows, gpu_data, pareto_indices } => {
                app_state.clear();
                app_state.current_study = Some(StudyContext { meta, trial_rows, gpu_data, pareto_indices });
                *is_loading = false;
            }
            AppMessage::SensitivityDone(result) => {
                app_state.sensitivity_result = Some(result);
            }
            AppMessage::SobolDone(result) => {
                app_state.sobol_result = Some(result);
            }
            AppMessage::ClusteringDone(result) => {
                app_state.cluster_result = Some(result);
            }
            AppMessage::TopsisDone(result) => {
                app_state.topsis_result = Some(result);
            }
            AppMessage::DownsampleDone { key, indices } => {
                match key {
                    DownsampleKey::Scatter => app_state.downsample_cache.scatter = Some(indices),
                    DownsampleKey::Pcp => app_state.downsample_cache.pcp = Some(indices),
                    DownsampleKey::Thumbnail => app_state.downsample_cache.thumbnail = Some(indices),
                    DownsampleKey::Hover => app_state.downsample_cache.hover = Some(indices),
                }
            }
            AppMessage::HvHistoryDone { trial_ids, hv_values } => {
                use crate::state::results::HvHistory;
                app_state.hv_history = Some(HvHistory { trial_ids, hv_values });
            }
            AppMessage::Pdp2dDone(result) => {
                widget_states.pdp_2d.result = Some(result);
                widget_states.pdp_2d.computing = false;
            }
            AppMessage::Error(e) => {
                *load_error = Some(e);
                *is_loading = false;
            }
            AppMessage::SensitivityError(_e) => {
                widget_states.importance.computing = false;
            }
            AppMessage::LiveUpdateDone { .. } | AppMessage::PdpDone { .. } => {
                // TODO: 今後のタスクで実装
            }
        }
    }
}
```

#### app.rs の変更後 🔵

**信頼性**: 🔵 *既存コード分析より*

```rust
// app.rs
use crate::state::message_handler::MessageHandler;

impl TunnyApp {
    pub fn poll_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx.try_recv() {
            MessageHandler::handle(
                msg,
                &mut self.app_state,
                &mut self.widget_states,
                &mut self.is_loading,
                &mut self.load_error,
            );
            ctx.request_repaint();
        }
    }
}
```

## 4. リファクタリング後のモジュール構成 🔵

**信頼性**: 🔵 *ユーザーヒアリング・既存コード分析より*

```
egui-app/src/
├── main.rs                    # （変更なし）
├── app.rs                     # TunnyApp + eframe::App のみ（薄いラッパー）
├── state/
│   ├── mod.rs                 # （変更なし）
│   ├── types.rs               # 新規: データモデル型
│   ├── filter.rs              # 新規: フィルタリングロジック
│   ├── results.rs             # 新規: 分析結果型 + キャッシュ
│   ├── message_handler.rs     # 新規: メッセージ処理ロジック
│   ├── app_state.rs           # AppState 構造体のみ + 再エクスポート
│   ├── layout_state.rs        # （変更なし）
│   └── messages.rs            # （変更なし）
├── ui/
│   ├── mod.rs                 # chart_registry 追加
│   ├── chart_registry.rs      # 新規: チャートディスパッチ
│   ├── grid_canvas.rs         # グリッド描画に専任
│   ├── layout.rs              # （変更なし）
│   ├── left_panel.rs          # （変更なし）
│   ├── main_canvas.rs         # （変更なし）
│   ├── toolbar.rs             # （変更なし）
│   ├── bottom_panel.rs        # （変更なし）
│   ├── right_panel.rs         # （変更なし）
│   ├── widgets/               # （変更なし）
│   └── widget_states.rs       # （変更なし）
└── render/                    # （変更なし）
```

## 5. 各フェーズのテスト戦略 🟡

**信頼性**: 🟡 *一般的なリファクタリング手法より*

### Phase 1 テスト戦略

- `app_state.rs` の既存テスト（18件）をそのまま維持
- `types.rs`, `filter.rs`, `results.rs` にテストを移動（モジュール名変更のみ）
- `pub use` で再エクスポートするため、外部からの `use crate::state::app_state::StudyMeta` はそのまま動作
- 各フェーズ終了時に `cargo test` でグリーン確認

### Phase 2 テスト戦略

- `grid_canvas.rs` の既存テスト（8件）はそのまま維持
- `chart_registry.rs` に新しいテストは不要（純粋な移動のため）
- `render_cell_content` の呼び出し先が変わるだけ

### Phase 3 テスト戦略

- `app.rs` の既存テスト（4件）は `poll_messages` のシグネチャ変更に追従
- `message_handler.rs` に handle のユニットテストを追加
- テスト内容は app.rs の既存テストを移植

## 6. 移行時のリスクと対策 🔵

**信頼性**: 🔵 *既存コード分析より*

| リスク | 対策 |
|---|---|
| `pub use` の再エクスポート漏れ | 各フェーズで `cargo check` + `cargo test` を実行 |
| テストのモジュールパス変更 | `use super::*` を活用し、パス変更を最小化 |
| Borrow checker エラー | grid_canvas.rs は既に clone ベースの設計のため影響少 |
| フィルタリングロジックの impl 分離 | filter.rs で `impl AppState` を定義（Rustの孤児impl問題なし: 同クレート内） |

## 7. 非対象（スコープ外）🟡

**信頼性**: 🟡 *ユーザーヒアリングより*

以下は今回のリファクタリング対象外とする:

- `rust_core/src/lib.rs` の WASM バインディング分離（別タスク）
- `widget_states.rs` の構造変更
- 各 widget ファイル（scatter_matrix.rs等）の内部リファクタリング
- `layout_state.rs` の分割（既に良好な分離）

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **egui移行設計**: [../egui-migration/architecture.md](../egui-migration/architecture.md)

## 信頼性レベルサマリー

- 🔵 青信号: 14件 (88%)
- 🟡 黄信号: 2件 (12%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
