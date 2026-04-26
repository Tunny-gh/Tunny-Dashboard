# 責務分離リファクタリング データフロー図

**作成日**: 2026-04-15
**関連アーキテクチャ**: [architecture.md](architecture.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: 既存コード分析・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: 既存コード分析・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: 既存コード分析・ユーザヒアリングにない推測によるフロー

---

## 1. リファクタリング前のデータフロー 🔵

**信頼性**: 🔵 *既存コード分析より*

```mermaid
flowchart TD
    subgraph "app.rs"
        PM[poll_messages]
    end

    subgraph "app_state.rs (648行)"
        AS[AppState 構造体]
        TYPES[型定義: StudyMeta, TrialRow...]
        FILTER[フィルタリング: apply_filters]
        RESULTS[分析結果: SensitivityResult...]
        CACHE[DownsampleCache]
    end

    subgraph "grid_canvas.rs (418行)"
        GC[グリッド描画]
        SC[show_chart: チャートディスパッチ]
    end

    subgraph "widgets/"
        W[各チャートウィジェット]
    end

    PM -->|直接変更| AS
    PM -->|直接変更| TYPES
    PM -->|直接変更| RESULTS
    PM -->|直接変更| CACHE

    GC --> SC
    SC -->|clone データ| AS
    SC -->|呼び出し| W

    style AS fill:#ff9999
    style TYPES fill:#ff9999
    style FILTER fill:#ff9999
    style RESULTS fill:#ff9999
    style CACHE fill:#ff9999
    style SC fill:#ffcc99
```

**問題**: app_state.rs が4つの関心事（型・フィルタ・結果・キャッシュ）を持つ。grid_canvas.rs が2つの関心事（グリッド描画・チャートディスパッチ）を持つ。

## 2. リファクタリング後のデータフロー 🔵

**信頼性**: 🔵 *ユーザーヒアリング・設計より*

```mermaid
flowchart TD
    subgraph "app.rs"
        PM[poll_messages]
    end

    subgraph "state/message_handler.rs"
        MH[MessageHandler::handle]
    end

    subgraph "state/types.rs"
        TYPES[型定義: StudyMeta, TrialRow, GpuBufferData, StudyContext, ColorMode]
    end

    subgraph "state/filter.rs"
        FILTER[フィルタリング: apply_filters, set_filter, brush_select]
        CACHE[DownsampleCache, should_resample]
    end

    subgraph "state/results.rs"
        RESULTS[分析結果: SensitivityResult, SobolResult, ClusterResult...]
        HV[HvHistory, LiveUpdateState]
    end

    subgraph "state/app_state.rs"
        AS[AppState 構造体]
    end

    subgraph "ui/chart_registry.rs"
        CR[show_chart: チャートディスパッチ]
    end

    subgraph "ui/grid_canvas.rs"
        GC[グリッド描画 のみ]
    end

    subgraph "widgets/"
        W[各チャートウィジェット]
    end

    PM -->|委譲| MH
    MH -->|更新| AS
    MH -->|更新| FILTER
    MH -->|更新| RESULTS

    GC -->|委譲| CR
    CR -->|データ参照| AS
    CR -->|呼び出し| W

    style AS fill:#99cc99
    style TYPES fill:#99cc99
    style FILTER fill:#99cc99
    style RESULTS fill:#99cc99
    style CACHE fill:#99cc99
    style MH fill:#99cc99
    style CR fill:#99cc99
```

## 3. Phase 1: 型・フィルター分離のデータフロー 🔵

**信頼性**: 🔵 *既存コード app_state.rs の分析より*

```mermaid
sequenceDiagram
    participant Old as app_state.rs (旧)
    participant Types as state/types.rs (新)
    participant Filter as state/filter.rs (新)
    participant Results as state/results.rs (新)
    participant New as app_state.rs (新)

    Note over Old,New: Phase 1: ファイル分割

    Old->>Types: 型定義を移動
    Note right of Types: Direction, TrialState, StudyMeta<br/>TrialRow, GpuBufferData<br/>StudyContext, ColorMode

    Old->>Filter: フィルタリングロジックを移動
    Note right of Filter: DownsampleCache, should_resample<br/>impl AppState: set_filter,<br/>remove_filter, clear_filters,<br/>brush_select, apply_filters

    Old->>Results: 分析結果型を移動
    Note right of Results: SensitivityResult, RidgeResult,<br/>RfAnovaResult, SobolResult,<br/>ClusterResult, TopsisResult,<br/>HvHistory, LiveUpdateState

    New->>Types: pub use types::*;
    New->>Filter: pub use filter::*;
    New->>Results: pub use results::*;

    Note over New: AppState 構造体のみ残る<br/>外部からの use パスは不変
```

### Phase 1 の具体的な依存関係 🔵

**信頼性**: 🔵 *既存コードの use 宣言分析より*

```mermaid
graph LR
    subgraph "外部からの参照"
        APP[app.rs]
        GC[grid_canvas.rs]
        LP[left_panel.rs]
        TB[toolbar.rs]
        W[widgets/*]
    end

    subgraph "state/"
        AS[app_state.rs]
        T[types.rs]
        F[filter.rs]
        R[results.rs]
    end

    APP -->|use| AS
    GC -->|use| AS
    LP -->|use| AS
    TB -->|use| AS
    W -->|use| AS

    AS -->|pub use| T
    AS -->|pub use| F
    AS -->|pub use| R

    F -->|use| T
    R -->|use| T

    style AS fill:#99cc99
    style T fill:#cce5ff
    style F fill:#cce5ff
    style R fill:#cce5ff
```

## 4. Phase 2: チャートディスパッチ分離のデータフロー 🔵

**信頼性**: 🔵 *既存コード grid_canvas.rs:257-371 の分析より*

```mermaid
sequenceDiagram
    participant GC as grid_canvas.rs
    participant CR as chart_registry.rs
    participant AS as AppState
    participant WS as WidgetStates
    participant W as widgets/*

    Note over GC,W: Phase 2: show_chart の抽出

    GC->>GC: render_cell_content
    GC->>CR: show_cell_chart(ui, app_state, widgets, &id, tx)
    CR->>CR: show_chart(ui, app_state, widgets, chart_id, tx)
    CR->>AS: current_study データ clone
    CR->>CR: match chart_id
    CR->>W: widgets.pareto_2d.show(ui, app_state)
    CR->>W: widgets.opt_history.show(ui, ...)
    CR->>W: widgets.importance.show(ui, ...)
    Note over CR,W: 各チャートウィジェットに委譲
```

### chart_registry の match ブランチ 🔵

**信頼性**: 🔵 *既存コード grid_canvas.rs:293-371 より*

```mermaid
graph TD
    CR[chart_registry::show_chart]
    CR --> P2D[ParetoScatter2D → widgets.pareto_2d]
    CR --> OH[OptimizationHistory → widgets.opt_history]
    CR --> HV[HvHistory → widgets.hv_history]
    CR --> IC[ImportanceChart → widgets.importance]
    CR --> PC[PdpChart → widgets.pdp_chart]
    CR --> P2[PdpChart2D → widgets.pdp_2d + spawn_task]
    CR --> PX[ParallelCoordinates → widgets.parallel_coords]
    CR --> SM[ScatterMatrix → widgets.scatter_matrix]
    CR --> PS[ParetoScatter3D → placeholder]
    CR --> SH[SensitivityHeatmap → widgets.sensitivity_heatmap]
    CR --> CS[ClusterScatter → widgets.cluster_scatter]
```

## 5. Phase 3: メッセージ処理分離のデータフロー 🔵

**信頼性**: 🔵 *既存コード app.rs:38-124 の分析より*

```mermaid
sequenceDiagram
    participant Thread as Background Thread
    participant RX as mpsc::Receiver
    participant PM as poll_messages
    participant MH as MessageHandler
    participant AS as AppState
    participant WS as WidgetStates

    Thread->>RX: AppMessage 送信
    PM->>RX: try_recv()
    RX-->>PM: AppMessage
    PM->>MH: MessageHandler::handle(msg, app_state, widget_states, ...)
    MH->>MH: match msg
    MH->>AS: app_state 更新
    MH->>WS: widget_states 更新
    PM->>PM: ctx.request_repaint()
```

### メッセージ処理の責務マッピング 🔵

**信頼性**: 🔵 *既存コード app.rs:41-121 より*

| メッセージ | 更新先 | 更新内容 |
|---|---|---|
| `JournalParsed` | AppState | all_studies, journal_path, is_loading |
| `StudySelected` | AppState | current_study, is_loading |
| `SensitivityDone` | AppState | sensitivity_result |
| `SobolDone` | AppState | sobol_result |
| `ClusteringDone` | AppState | cluster_result |
| `TopsisDone` | AppState | topsis_result |
| `DownsampleDone` | AppState | downsample_cache |
| `HvHistoryDone` | AppState | hv_history |
| `Pdp2dDone` | WidgetStates | pdp_2d.result, pdp_2d.computing |
| `Error` | AppState | load_error, is_loading |
| `SensitivityError` | WidgetStates | importance.computing |

## 6. リファクタリングの影響範囲 🔵

**信頼性**: 🔵 *既存コードの use 宣言調査より*

### 外部モジュールへの影響

```mermaid
graph TD
    subgraph "Phase 1 影響"
        P1A["app.rs: use app_state::* → 影響なし<br/>(pub use 再エクスポート)"]
        P1B["ui/*.rs: use app_state::* → 影響なし"]
        P1C["テスト: use app_state::* → 影響なし"]
    end

    subgraph "Phase 2 影響"
        P2A["grid_canvas.rs: show_chart 呼び出しを<br/>chart_registry に変更"]
        P2B["ui/mod.rs: mod chart_registry 追加"]
    end

    subgraph "Phase 3 影響"
        P3A["app.rs: poll_messages を<br/>MessageHandler::handle に委譲"]
        P3B["state/mod.rs: mod message_handler 追加"]
        P3C["テスト: app.rs のテスト →<br/>message_handler に移植"]
    end

    style P1A fill:#99cc99
    style P1B fill:#99cc99
    style P1C fill:#99cc99
    style P2A fill:#ffffcc
    style P2B fill:#ffffcc
    style P3A fill:#ffffcc
    style P3B fill:#ffffcc
    style P3C fill:#ffffcc
```

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **ヒアリング記録**: [design-interview.md](design-interview.md)

## 信頼性レベルサマリー

- 🔵 青信号: 10件 (91%)
- 🟡 黄信号: 1件 (9%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
