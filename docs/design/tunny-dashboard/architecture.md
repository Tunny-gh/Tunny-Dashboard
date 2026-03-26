# Tunny Dashboard アーキテクチャ設計

## システム概要

ブラウザ完結型のポストプロセッシング分析ダッシュボード。サーバー・データベース不要。単一HTMLファイルとして配布可能。

## アーキテクチャパターン

- **パターン**: 4層クライアントサイドアーキテクチャ
- **理由**: 配布の容易さ・セキュリティ（ファイルがブラウザ外に出ない）・5万点の高速処理を両立するため

## 4層構造

```
┌─────────────────────────────────────────────┐
│  Layer 4: UI / Rendering                    │
│  React + TypeScript                          │
│  deck.gl（WebGL散布図/3D）                   │
│  regl / ECharts WebGL（PCP）                 │
│  OffscreenCanvas（Scatter Matrix）           │
├─────────────────────────────────────────────┤
│  Layer 3: State Management                  │
│  Zustand                                    │
│  SelectionStore / LayoutStore / StudyStore  │
├─────────────────────────────────────────────┤
│  Layer 2: JS Bridge                         │
│  wasm-bindgen 生成コード                    │
│  SharedArrayBuffer（ゼロコピー転送）         │
│  WebWorker プール                           │
├─────────────────────────────────────────────┤
│  Layer 1: WASM Core (Rust)                  │
│  Journalパーサ                              │
│  DataFrame（WASMメモリ常駐）                 │
│  フィルタ・集計・統計計算                    │
│  Pareto/クラスタリング/感度分析              │
└─────────────────────────────────────────────┘
```

## モジュール構成

### Layer 1: WASM Core（Rust）

| モジュール | 責務 | 主要関数 |
|---|---|---|
| `journal_parser` | JSONLパース・ステートマシン | `parse_journal()` |
| `dataframe` | WASMメモリ内DataFrame管理 | `get_column()`, `row_count()` |
| `filter` | 範囲クエリ・インデックス生成 | `filter_by_ranges()` |
| `pareto` | NDSort・Hypervolume計算 | `compute_pareto_ranks()`, `compute_hypervolume()` |
| `clustering` | PCA・k-means・統計量計算 | `run_pca()`, `run_kmeans()`, `compute_cluster_stats()` |
| `sensitivity` | Spearman相関・Ridge回帰・R² | `compute_spearman()`, `compute_ridge()` |
| `pdp` | PDP/ICE計算（Ridge簡易版） | `compute_pdp()`, `compute_ice()` |
| `sampling` | ダウンサンプリング戦略 | `downsample_for_thumbnail()` |
| `export` | CSV/JSONシリアライズ | `serialize_csv()`, `serialize_json()` |
| `live_update` | 差分パース・保留リスト管理 | `append_journal_diff()` |

### Layer 2: JS Bridge

| モジュール | 責務 |
|---|---|
| `wasmLoader` | WASMモジュールの初期化・シングルトン管理 |
| `workerPool` | Scatter Matrix用WebWorker×4の管理 |
| `umapWorker` | UMAP計算用WebWorker（umap-js） |
| `micWorker` | MIC計算用WebWorker |
| `onnxWorker` | ONNX Runtime Web（高精度PDP）用WebWorker |
| `fsapiPoller` | File System Access APIポーリング管理 |
| `gpuBuffer` | Float32Array（positions/colors/sizes）管理 |

### Layer 3: State Management（Zustand）

| Store | 状態 | 主なアクション |
|---|---|---|
| `SelectionStore` | selectedIndices, filterRanges, highlighted, colorMode | brushSelect, addAxisFilter, clearSelection, setHighlight |
| `StudyStore` | currentStudy, allStudies, studyMode | selectStudy, loadJournal, switchMode |
| `LayoutStore` | layoutMode, visibleCharts, panelSizes, freeModeLayout | setLayoutMode, toggleChart, saveLayout |
| `ClusterStore` | clusterConfig, clusterLabels, clusterStats | runClustering, setClusterConfig |
| `AnalysisStore` | sensitivityResult, pdpCache, importanceData | computeSensitivity, computePDP |
| `ExportStore` | pinnedTrials, sessionState | pinTrial, saveSession, loadSession |
| `LiveUpdateStore` | isLive, pollInterval, lastUpdate, newTrialCount | toggleLive, setPollInterval |

### Layer 4: UI / Rendering

| コンポーネント | 担当レイヤー | 描画技術 |
|---|---|---|
| `ToolBar` | React | React DOM |
| `LeftPanel` | React | React DOM |
| `ParetoScatter3D` | WebGL（ReactサイクルOS切り離し） | deck.gl PointCloudLayer |
| `ParetoScatter2D` | WebGL | deck.gl ScatterplotLayer |
| `ParallelCoordinates` | WebGL | regl / ECharts WebGL |
| `ScatterMatrix` | OffscreenCanvas | WebWorker + OffscreenCanvas |
| `ObjectivePairMatrix` | WebGL | deck.gl ScatterplotLayer |
| `OptimizationHistory` | Canvas | ECharts |
| `HypervolumeHistory` | Canvas | ECharts |
| `SensitivityHeatmap` | Canvas | ECharts HeatMap |
| `PDPChart` | Canvas | ECharts |
| `BottomTable` | React | React DOM（仮想スクロール） |
| `ArtifactGallery` | React | React DOM |
| `ReportBuilder` | React | React DOM |

## WebWorkerトポロジー

```
メインスレッド
├── WASM Core（SharedArrayBuffer経由）
├── ScatterMatrixWorker[0]  → 行 0〜9  （x1〜x10）
├── ScatterMatrixWorker[1]  → 行 10〜19（x11〜x20）
├── ScatterMatrixWorker[2]  → 行 20〜29（x21〜x30）
├── ScatterMatrixWorker[3]  → 行 30〜33（目的・対角）
├── UMAPWorker              → UMAP計算（非同期・完了後有効化）
├── MICWorker               → MIC計算（10,000点ダウンサンプリング）
└── ONNXWorker              → ONNX Runtime Web推論
```

## ビルドパイプライン

```
Rust（rust_core/）
    ↓ wasm-pack build
WASM バイナリ + wasm-bindgen バインディング
    ↓
TypeScript + React（src/）
    ↓ Vite
dist/index.html（単一ファイル配布）
```

### Cargo.tomlのターゲット設計

```toml
# rust_core/Cargo.toml
[lib]
crate-type = ["cdylib", "rlib"]  # WASM + Native両対応

[features]
wasm = ["wasm-bindgen", "js-sys", "web-sys"]
```

## ディレクトリ構造

```
tunny-dashboard/
├── rust_core/                  # Rustコア
│   ├── src/
│   │   ├── lib.rs
│   │   ├── journal_parser.rs
│   │   ├── dataframe.rs
│   │   ├── filter.rs
│   │   ├── pareto.rs
│   │   ├── clustering.rs
│   │   ├── sensitivity.rs
│   │   ├── pdp.rs
│   │   ├── sampling.rs
│   │   ├── export.rs
│   │   └── live_update.rs
│   └── Cargo.toml
├── src/                        # TypeScript + React
│   ├── wasm/                   # Layer 2: JS Bridge
│   │   ├── wasmLoader.ts
│   │   ├── gpuBuffer.ts
│   │   ├── workers/
│   │   │   ├── scatterMatrixWorker.ts
│   │   │   ├── umapWorker.ts
│   │   │   ├── micWorker.ts
│   │   │   └── onnxWorker.ts
│   │   └── fsapiPoller.ts
│   ├── stores/                 # Layer 3: Zustand
│   │   ├── selectionStore.ts
│   │   ├── studyStore.ts
│   │   ├── layoutStore.ts
│   │   ├── clusterStore.ts
│   │   ├── analysisStore.ts
│   │   ├── exportStore.ts
│   │   └── liveUpdateStore.ts
│   ├── components/             # Layer 4: React UI
│   │   ├── layout/
│   │   ├── charts/
│   │   ├── panels/
│   │   └── export/
│   ├── types/                  # 共通型定義
│   │   └── index.ts
│   └── main.tsx
├── docs/
│   ├── spec/
│   └── design/
└── scripts/
    ├── generate_surrogates.py
    └── generate_shap.py
```

## 非機能要件との対応

| 要件 | 実現手段 |
|---|---|
| フィルタ 5ms以内 | WASM `filter_by_ranges()` + Uint32Array |
| GPU更新 1ms以内 | alpha値のみ書き換え（positions変更なし） |
| 5万点 60fps | deck.gl GPUバッファ一括描画 |
| ブラウザ完結 | wasm-pack + Vite 単一HTML配布 |
| ファイル外部送信なし | File API（ブラウザサンドボックス内） |
