# egui UI リファクタリング データフロー図

**作成日**: 2026-05-08
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/egui-ui-refactor/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## REQ-001: chart_registry.rs 分割後のデータフロー 🔵

**信頼性**: 🔵 *REQ-001・既存コード chart_registry.rs 分析より*

### 変更前のフロー

```mermaid
flowchart TD
    GC[grid_canvas.rs] -->|show_chart| CR["chart_registry.rs\n(~750行)\nrender + poll 混在"]
    CR -->|draw| UI[egui::Ui]
    CR -->|spawn_task| BG[Background Thread]
```

### 変更後のフロー

```mermaid
flowchart TD
    GC[grid_canvas.rs] -->|show_chart| CR["chart_registry.rs\n(~20行)\n薄いラッパー"]
    CR -->|render_chart| RC["render_chart.rs\n描画専用"]
    CR -->|poll_chart_work| PC["poll_chart.rs\nディスパッチ専用"]
    RC -->|&mut Ui| UI[egui::Ui]
    PC -->|spawn_task| BG[Background Thread]

    style RC fill:#cce5ff
    style PC fill:#cce5ff
```

### 呼び出しシーケンス

```mermaid
sequenceDiagram
    participant GC as grid_canvas.rs
    participant CR as chart_registry.rs
    participant RC as render_chart.rs
    participant PC as poll_chart.rs
    participant UI as egui::Ui
    participant TX as mpsc::SyncSender

    GC->>CR: show_chart(ui, app_state, widgets, chart_id, tx)
    CR->>RC: render_chart(ui, app_state, widgets, chart_id)
    RC->>UI: widgets.xxx.show(ui, ...)
    CR->>PC: poll_chart_work(app_state, widgets, chart_id, tx)
    PC->>TX: spawn_task(tx, || AppMessage::XxxDone)
```

**関連要件**: REQ-001

---

## REQ-002: 計算ロジック移動後のデータフロー 🔵

**信頼性**: 🔵 *REQ-002・ユーザヒアリング Q3・left_panel.rs 分析より*

### normalize_weights のフロー

**変更前** (`left_panel.rs` 内で完結):
```
show_tradeoff_navigator → normalize_weights() [左パネル内定義]
```

**変更後** (rust_core 経由):
```mermaid
flowchart LR
    LP[left_panel.rs\nor tradeoff_navigator.rs] -->|呼び出し| NW["tunny_core::multi_objective\n::weights::normalize_weights"]
    NW --> |&mut [f64]| Result[正規化済みウェイト]
```

### build_best_trial_history のフロー

**変更前** (`left_panel.rs` の TrialRow を直接受け取る):
```
build_best_trial_history(&[state::TrialRow], ...) → Vec<(u32, f64)>
```

**変更後** (プリミティブ型 slice で型依存を排除):
```mermaid
sequenceDiagram
    participant LP as left_panel.rs / convergence_card.rs
    participant CV as tunny_core::convergence
    participant ST as state::types::TrialRow

    LP->>ST: trial_ids と obj_values を抽出
    Note right of LP: let ids: Vec<u32> = ...<br/>let vals: Vec<f64> = ...
    LP->>CV: build_best_trial_history(&ids, &vals, is_minimize)
    CV-->>LP: Vec<(u32, f64)>
```

**型変換の詳細**:
```rust
// egui-app 側での抽出（convergence_card.rs 内）
let trial_ids: Vec<u32> = trials.iter().map(|t| t.trial_id).collect();
let obj_values: Vec<f64> = trials
    .iter()
    .filter_map(|t| t.objectives.get(objective_idx).copied())
    .collect();
let history = tunny_core::convergence::build_best_trial_history(
    &trial_ids, &obj_values, is_minimize
);
```

**関連要件**: REQ-002

---

## REQ-003: HTML レポート構築フロー 🔵

**信頼性**: 🔵 *REQ-003・app.rs:77-108 分析・html_report.rs 分析より*

### 変更前のフロー

```mermaid
flowchart TD
    TB[ToolbarAction::GenerateHtmlReport] -->|inline ~30行| BUILD["HtmlReportSnapshot 構築\napp.rs 内"]
    BUILD --> GEN[generate_html_report_async]
    GEN --> TX[mpsc::SyncSender]
```

### 変更後のフロー

```mermaid
flowchart TD
    TB[ToolbarAction::GenerateHtmlReport] -->|3行| BASM["io::html_report\n::build_and_send_report(ctx, indices, tx)"]
    BASM -->|内部で構築| BUILD["HtmlReportSnapshot 構築\nhtml_report.rs 内"]
    BUILD --> GEN[generate_html_report_async]
    GEN --> TX[mpsc::SyncSender]

    style BASM fill:#cce5ff
```

### build_and_send_report の内部フロー

```mermaid
sequenceDiagram
    participant APP as app.rs
    participant HR as io/html_report.rs
    participant TX as mpsc::SyncSender<AppMessage>

    APP->>HR: build_and_send_report(ctx, selected_indices, tx)
    HR->>HR: 選択 Trial を HtmlTrialRow に変換
    HR->>HR: TrialStatistics を計算
    HR->>HR: HtmlReportSnapshot { ... } を構築
    HR->>HR: generate_html_report_async(snap, tx)
    HR->>TX: AppMessage::HtmlReportDone(html_string)
```

**関連要件**: REQ-003

---

## REQ-004: 左パネル分割後のデータフロー 🔵

**信頼性**: 🔵 *REQ-004・left_panel.rs:221-344 分析より*

### 変更前

```mermaid
flowchart TD
    LP[left_panel.rs\n~450行] -->|定義| TN[show_tradeoff_navigator]
    LP -->|定義| CC[show_convergence_card]
    LP -->|定義| NW[normalize_weights]
    LP -->|定義| CI[compute_improvement_rate]
    LP -->|定義| BH[build_best_trial_history]
```

### 変更後

```mermaid
flowchart TD
    LP[left_panel.rs\n~250行] -->|呼び出し| TN["ui/widgets\n/tradeoff_navigator.rs"]
    LP -->|呼び出し| CC["ui/widgets\n/convergence_card.rs"]
    TN -->|呼び出し| NW["tunny_core::multi_objective\n::weights::normalize_weights"]
    CC -->|呼び出し| CI["tunny_core::convergence\n::compute_improvement_rate"]
    CC -->|呼び出し| BH["tunny_core::convergence\n::build_best_trial_history"]

    style TN fill:#cce5ff
    style CC fill:#cce5ff
    style NW fill:#d4edda
    style CI fill:#d4edda
    style BH fill:#d4edda
```

### Widget 呼び出しシーケンス（左パネル）

```mermaid
sequenceDiagram
    participant LP as left_panel.rs
    participant TN as tradeoff_navigator.rs
    participant CC as convergence_card.rs
    participant CORE as tunny_core

    LP->>LP: obj_names, is_minimize を AppState から抽出
    alt 多目的 Study (len >= 2)
        LP->>TN: show_tradeoff_navigator(ui, app_state, &obj_names, &is_minimize, tx)
        TN->>CORE: normalize_weights(&mut weights)
        TN->>CORE: trigger_tradeoff_computation(...)
    else 単目的 Study (len == 1)
        LP->>CC: show_convergence_card(ui, app_state)
        CC->>CORE: build_best_trial_history(&ids, &vals, is_minimize)
        CC->>CORE: compute_improvement_rate(&history, 100)
    end
```

**関連要件**: REQ-004

---

## 全体の層境界データフロー（変更後） 🔵

**信頼性**: 🔵 *layer-contract.md・全 REQ 設計より*

```mermaid
flowchart TB
    subgraph rust_core["rust_core (Pure Logic/Data)"]
        CONV[convergence.rs\ncompute_improvement_rate\nbuild_best_trial_history]
        WEIGHTS[multi_objective/weights.rs\nnormalize_weights]
    end

    subgraph state["egui-app/src/state (App State)"]
        AS[app_state.rs]
        MSG[message_handler.rs]
    end

    subgraph ui["egui-app/src/ui (UI View)"]
        CR[chart_registry.rs]
        RC[render_chart.rs]
        PC[poll_chart.rs]
        LP[left_panel.rs]
        TN[widgets/tradeoff_navigator.rs]
        CC[widgets/convergence_card.rs]
    end

    subgraph io_layer["egui-app/src/io"]
        HR[html_report.rs\nbuild_and_send_report]
    end

    subgraph app["egui-app/src/app.rs"]
        APP[TunnyApp]
    end

    APP -->|MessageHandler::handle| MSG
    APP -->|build_and_send_report| HR
    CR -->|render_chart| RC
    CR -->|poll_chart_work| PC
    LP -->|show_tradeoff_navigator| TN
    LP -->|show_convergence_card| CC
    TN -->|normalize_weights| WEIGHTS
    CC -->|compute_improvement_rate| CONV
    CC -->|build_best_trial_history| CONV
    RC -->|read| AS
    PC -->|read/write| AS

    style rust_core fill:#d4edda
    style state fill:#fff3cd
    style ui fill:#cce5ff
    style app fill:#f8d7da
```

---

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **型定義**: [interfaces.rs](interfaces.rs)
- **ヒアリング記録**: [design-interview.md](design-interview.md)

## 信頼性レベルサマリー

- 🔵 青信号: 6件 (100%)
- 🟡 黄信号: 0件 (0%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
