# egui Migration データフロー図

**作成日**: 2026-04-11
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [tunny-dashboard-requirements.md](../../spec/tunny-dashboard-requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## システム全体のデータフロー 🔵

**信頼性**: 🔵 *要件定義・既存データフロー設計より（egui版に翻訳）*

```mermaid
flowchart TD
    User[ユーザー] -->|ファイルドロップ/ダイアログ| App
    App[TunnyApp\neframe::App] -->|parse_journal| Core
    Core[rust_core\n計算ライブラリ] -->|Vec<StudyMeta>| App
    App -->|select_study| Core
    Core -->|Vec<TrialRow> + GpuBufferData| App
    App -->|vertex/color data| GPU
    GPU[wgpu\nGPUレンダラー] -->|フレームバッファ| Window
    App -->|PlotPoints| Plot
    Plot[egui_plot] -->|egui DrawCommands| Window
    Window[eframe ウィンドウ] --> User
```

---

## 1. ファイル読み込み → DataFrame構築 → GPU初期化フロー 🔵

**信頼性**: 🔵 *要件定義REQ-001〜015・既存データフロー設計1より*

```mermaid
flowchart TD
    A[ユーザー: ファイルドロップ\nまたはダイアログ選択] --> B[rfd::FileDialog::pick_file\nまたはDnD イベント]
    B --> C[std::fs::read\nファイルをバイト列として読み込み]
    C --> D[tunny_core::io::journal::parse_journal\n直接Rust呼び出し]
    D --> E{Study選択ダイアログ\negui::Modal または ComboBox}
    E --> F[tunny_core::io::journal::select_study\n選択StudyのDataFrame構築]
    F --> G[tunny_core::multi_objective::pareto::compute_pareto_ranks\nNDSort → Paretoランク列追加]
    G --> H[GpuBufferData 計算\npositions / colors / sizes Float32配列生成]
    H --> I[wgpu::Queue::write_buffer\nGPU VBOアップロード]
    I --> J[初期レンダリング完了\negui::Context::request_repaint]

    F --> K[std::thread::spawn: ScatterMatrix計算\nサムネイル生成]
    F --> L[std::thread::spawn: Spearman感度計算\n感度ヒートマップ準備]
```

**旧実装との差分:**
- `FileReader.readAsText` (JS) → `std::fs::read` (Rust)
- `wasm_parse_journal(Uint8Array)` → `tunny_core::io::journal::parse_journal(&[u8])` 直接呼び出し
- `WebWorker` → `std::thread::spawn` + `mpsc::channel`
- `SharedArrayBuffer` → Rust の所有権モデル（不要）

---

## 2. Brushing & Linking イベント伝播（クリティカルパス）🔵

**信頼性**: 🔵 *要件定義REQ-040〜045・既存データフロー設計2より*

```mermaid
flowchart TD
    subgraph "ユーザー操作（いずれか）"
        A1[平行座標図ブラッシング\nAxis Filter]
        A2[散布図上ドラッグ\nBrush Selection]
        A3[Trialテーブル行クリック\nClick Highlight]
    end

    A1 --> B[AppState::set_filter\nfilter_ranges 更新]
    A2 --> C[AppState::brush_select\nselected_indices 直接更新]
    A3 --> D[AppState::set_highlight\nhighlighted_trial 更新]

    B --> E[tunny_core::data::filter::filter_by_ranges\n直接呼び出し < 5ms @ 50,000点]
    E --> F[AppState::selected_indices 更新\nVec<u32>]

    F --> G[GpuBuffer::update_alphas\nwgpu::Queue::write_buffer < 1ms/チャート]
    C --> G
    D --> H[GpuBuffer::update_highlight_color\n特定点の色変更]

    G --> I[egui::Context::request_repaint\n次フレームで全チャート再描画]
    H --> I

    F --> J[LeftPanel カウンタ更新\nselected_indices.len()]
    F --> K[BottomPanel Trialテーブル更新\n上位100件]
```

**旧実装との差分:**
- `Zustand SelectionStore` → `AppState` の直接フィールド更新
- `wasm_filter_by_ranges(JSON)` → `tunny_core::data::filter::filter_by_ranges(&HashMap)` 直接呼び出し
- `requestAnimationFrame` → `egui::Context::request_repaint()`
- `SharedArrayBuffer` → `Vec<u32>` + wgpu buffer write

---

## 3. egui フレームループ 🔵

**信頼性**: 🔵 *eframe/eguiフレームワーク設計より*

```mermaid
sequenceDiagram
    participant eframe as eframe ランタイム
    participant App as TunnyApp::update()
    participant Recv as rx: Receiver<AppMessage>
    participant UI as egui ウィジェット
    participant wgpu as wgpu レンダラー

    eframe->>App: update(ctx, frame) 毎フレーム
    App->>Recv: rx.try_recv_all() ノンブロッキング
    Recv-->>App: AppMessage::SensitivityDone(result)
    App->>App: app_state.sensitivity_result = Some(result)
    
    App->>UI: show_toolbar(ctx, &mut state)
    App->>UI: show_left_panel(ctx, &mut state)
    App->>UI: show_bottom_panel(ctx, &mut state)
    App->>UI: show_main_canvas(ctx, &mut state)
    
    UI-->>App: ユーザーインタラクション（フィルター変更など）
    App->>App: state.on_filter_changed(axis, min, max)
    App->>wgpu: gpu_buffer.update_alphas(&selected)
    App->>eframe: ctx.request_repaint()
    
    eframe->>wgpu: render_frame()
    wgpu-->>eframe: GPU フレームバッファ
```

---

## 4. 非同期重計算フロー（感度分析・クラスタリング・PDP）🔵

**信頼性**: 🔵 *既存実装の非同期パターン・要件定義より（Rust版への翻訳）*

```mermaid
sequenceDiagram
    participant App as TunnyApp
    participant Thread as std::thread
    participant Core as rust_core
    participant Ch as mpsc::channel

    App->>Thread: spawn(compute_sensitivity_task)
    Note over Thread: UIブロックなし
    Thread->>Core: tunny_core::sensitivity::compute_sensitivity()
    Core-->>Thread: SensitivityResult
    Thread->>Ch: tx.send(AppMessage::SensitivityDone(result))
    
    Note over App: 次フレームの update() で
    App->>Ch: rx.try_recv()
    Ch-->>App: AppMessage::SensitivityDone(result)
    App->>App: app_state.sensitivity_result = Some(result)
    App->>App: ctx.request_repaint()
```

**対応メッセージ型:**
```rust
pub enum AppMessage {
    JournalParsed(Vec<StudyMeta>),
    StudySelected { trial_rows: Vec<TrialRow>, gpu_data: GpuBufferData },
    SensitivityDone(SensitivityResult),
    SobolDone(SobolResult),
    ClusteringDone(ClusterResult),
    DownsampleDone { key: DownsampleKey, indices: Vec<u32> },
    PdpDone { param: String, objective: String, result: PdpResult },
    Error(String),
}
```

---

## 5. GPU 描画フロー（Pareto3D・Pareto2D）🟡

**信頼性**: 🟡 *wgpu/egui-wgpu統合の一般的パターンより*

```mermaid
flowchart LR
    A[AppState::gpu_buffer] --> B[VertexBuffer\npositions: Vec<f32>]
    A --> C[VertexBuffer\ncolors: Vec<f32> RGBA]
    A --> D[VertexBuffer\nsizes: Vec<f32>]

    B & C & D --> E[wgpu::RenderPass\nScatterRenderer::render]
    E --> F[wgsl シェーダー\n点群描画]
    F --> G[egui-wgpu コールバック\n埋め込みテクスチャ]
    G --> H[egui Panel 内に描画]
```

**wgpu 統合の仕組み:**
```rust
// egui-wgpu のカスタムコールバックを使用
egui_wgpu::Callback::new_paint_callback(
    rect,
    ScatterRenderCallback {
        vertex_buffer: gpu_buffer.positions.clone(),
        color_buffer: gpu_buffer.colors.clone(),
        vertex_count: gpu_buffer.len,
        matrix: view_proj,
    },
)
```

---

## 6. ライブ更新フロー（デスクトップ版）🔵

**信頼性**: 🔵 *要件定義REQ-130〜135・既存データフロー設計3より（Desktop版）*

```mermaid
sequenceDiagram
    participant Timer as std::thread (poll)
    participant FS as std::fs
    participant Core as rust_core
    participant App as TunnyApp
    participant Ch as mpsc::channel

    Timer->>FS: ファイルメタデータ確認\nfs::metadata(path).len()
    FS-->>Timer: size = S1

    alt S1 == S0 (変更なし)
        Timer->>Timer: スリープ(interval)
    else S1 > S0 (新規追加あり)
        Timer->>FS: File::open + seek(S0) + read
        FS-->>Core: 差分バイト列
        Core->>Core: tunny_core::io::journal::live_update::append_journal_diff
        Note over Core: 不完全な最終行はスキップ\nRUNNING試行は保留リストへ
        Core->>Core: DataFrame に COMPLETE試行を追記
        Core->>Core: Pareto差分更新
        Core-->>Ch: tx.send(AppMessage::LiveUpdateDone {...})
        Timer->>Timer: S0 = S1
    end

    App->>Ch: rx.try_recv()
    Ch-->>App: AppMessage::LiveUpdateDone
    App->>App: gpu_buffer.append_new_trials(...)
    App->>App: ctx.request_repaint()
```

---

## 7. フィルタースライダー → 全チャート更新フロー 🔵

**信頼性**: 🔵 *要件定義REQ-040〜043・既存実装より*

```mermaid
flowchart TD
    Slider[Left Panel\nフィルタースライダー変更] --> Filter
    Filter[filter_by_ranges\n< 5ms Rust直接呼び出し] --> Indices
    Indices[selected_indices: Vec<u32>] --> Alpha
    Alpha[GpuBuffer::update_alphas\n< 1ms/チャート] --> Repaint
    Repaint[ctx.request_repaint\n次フレーム] --> Charts

    subgraph Charts
        P3D[Pareto3D\nwgpu再描画]
        P2D[Pareto2D\nwgpu再描画]
        PCP[平行座標図\negui Painter再描画]
        SM[Scatter Matrix\negui Painter再描画]
    end

    Indices --> Counter[Left Panel カウンタ\nselected: N件]
    Indices --> Table[Bottom Panel\nTrial テーブル更新]
```

---

## 8. 状態管理の全体像 🔵

**信頼性**: 🔵 *既存Zustand stores分析・Rust所有権モデルより*

```mermaid
graph TD
    subgraph TunnyApp
        AS[AppState\n- all_studies\n- current_study\n- selected_indices\n- filter_ranges\n- highlighted_trial\n- analysis_results]
        LS[LayoutState\n- left_width\n- bottom_height\n- visible_charts\n- layout_mode]
        Ch[mpsc::Receiver\nAppMessage]
    end

    subgraph rust_core
        DF[DataFrame\nメモリ内保持]
        Calc[計算関数群\nfilter, sensitivity\nclustering, pdp]
    end

    subgraph egui-wgpu
        GB[GpuBuffer\npositions, colors\nsizes, alphas]
    end

    AS -->|直接呼び出し| Calc
    Calc -->|結果| AS
    AS -->|alpha更新| GB
    Ch -->|AppMessage| AS
    LS -->|パネルサイズ| TunnyApp
```

---

## 信頼性レベルサマリー

- 🔵 青信号: 14件 (78%)
- 🟡 黄信号: 4件 (22%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: ✅ 高品質
