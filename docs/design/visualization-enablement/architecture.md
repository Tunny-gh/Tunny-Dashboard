# 可視化機能有効化 アーキテクチャ設計

**作成日**: 2026-03-29
**関連要件定義**: [requirements.md](../../spec/visualization-enablement/requirements.md)
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *要件定義書・note.md より*

Rust/WASM で実装済みの感度分析（`sensitivity.rs`）・クラスタリング（`clustering.rs`）関数を、wasm-bindgen 経由で JavaScript に公開し、Zustand ストア経由でチャートコンポーネントに接続する。既存の 4 チャート（`importance`・`sensitivity-heatmap`・`cluster-view`・`umap`）を "This chart is under development" 状態から実データ表示に切り替える。

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存 tunny-dashboard アーキテクチャ・wasm-api.md より*

- **パターン**: 5 層レイヤードアーキテクチャ（WASM → TypeScript型宣言 → WasmLoader → Zustand Store → React Component）
- **選択理由**: 既存コードベースが同パターンで実装済み。`parseJournal`・`selectStudy`・`computeHvHistory` 等が同一フローを踏んでいる

---

## コンポーネント構成

### 層 1: WASM バインディング（`rust_core/src/lib.rs`） 🔵

**信頼性**: 🔵 *EARS要件定義 REQ-VE-001〜006 / wasm-api.md より*

新規 `#[wasm_bindgen]` 関数 6 本を追加する。シリアライズは既存関数と同じ `serde_wasm_bindgen::Serializer::json_compatible()` を使用する。

| JS名 | Rust関数名 | Rust実装 | 入力 |
|---|---|---|---|
| `computeSensitivity` | `wasm_compute_sensitivity` | `sensitivity::compute_sensitivity()` | なし |
| `computeSensitivitySelected` | `wasm_compute_sensitivity_selected` | `sensitivity::compute_sensitivity_selected()` | `Uint32Array` |
| `runPca` | `wasm_run_pca` | `clustering::run_pca()` | `u32, &str` |
| `runKmeans` | `wasm_run_kmeans` | `clustering::run_kmeans()` | `u32, Float64Array, u32` |
| `estimateKElbow` | `wasm_estimate_k_elbow` | `clustering::estimate_k_elbow()` | `Float64Array, u32, u32` |
| `computeClusterStats` | `wasm_compute_cluster_stats` | `clustering::compute_cluster_stats()` | `Int32Array` |

**重要注意**: `wasm_compute_cluster_stats` での `Int32Array → Vec<usize>` 変換は `i32 as usize` で行う（k-means ラベルは常に ≥ 0）。将来 HDBSCAN 対応時は `labels[i] >= 0` フィルタリングを追加すること。

### 層 2: TypeScript 型宣言（`frontend/src/wasm/pkg/tunny_core.d.ts`） 🔵

**信頼性**: 🔵 *REQ-VE-010〜016 / 既存 tunny_core.d.ts パターンより*

6 関数のシグネチャ + 対応 Result インターフェース 5 件を手動追加（wasm-pack ビルドでは型が `any` になるため）。

### 層 3: WasmLoader（`frontend/src/wasm/wasmLoader.ts`） 🔵

**信頼性**: 🔵 *REQ-VE-020〜022 / 既存 WasmLoader パターンより*

既存 `WasmLoader` クラスに 6 プロパティ + `_initialize()` でのバインド処理を追加する。パターンは既存の `parseJournal`・`computeHvHistory` と同一。

### 層 4a: `analysisStore`（新設） 🔵

**信頼性**: 🔵 *REQ-VE-030〜034 / 設計ヒアリングより*

- `frontend/src/stores/analysisStore.ts` を Zustand ストアとして新設
- `useStudyStore.subscribe` で Study 変更を検知し、`sensitivityResult` を null にリセット（`comparisonStore.ts` と同パターン）
- `computeSensitivity()` / `computeSensitivitySelected()` が `WasmLoader.getInstance()` 経由で WASM を呼び出す

### 層 4b: `clusterStore`（新設） 🔵

**信頼性**: 🔵 *REQ-VE-040〜045 / 設計ヒアリングより*

- `frontend/src/stores/clusterStore.ts` を Zustand ストアとして新設
- `useStudyStore.subscribe` で Study 変更を検知し、全クラスタ状態を null にリセット
- `runClustering(space, k)` が PCA → k-means → cluster stats を順次実行
- `estimateK(space)` が PCA → elbow を順次実行

**`runClustering` の実行順序**:
```
runPca(2, space) → pcaProjections [N×2]
  ↓
pcaProjections を Float64Array (flatten) に変換
  ↓
runKmeans(k, flatProjections, 2) → clusterLabels [N]
  ↓
clusterLabels を Int32Array に変換
  ↓
computeClusterStats(Int32ArrayLabels) → clusterStats
```

### 層 5: React チャートコンポーネント 🔵

**信頼性**: 🔵 *設計ヒアリング・REQ-VE-050〜087 より*

各チャートを独立ファイルとして実装（SensitivityHeatmap.tsx と同パターン）。

| コンポーネント | ファイル | 依存ストア |
|---|---|---|
| `ImportanceChart` | `frontend/src/components/charts/ImportanceChart.tsx` | `analysisStore` |
| `ClusterScatter` | `frontend/src/components/charts/ClusterScatter.tsx` | `clusterStore` |
| `DimReductionScatter` | `frontend/src/components/charts/DimReductionScatter.tsx` | `clusterStore`, `selectionStore` |
| *(既存)* `SensitivityHeatmap` | `frontend/src/components/charts/SensitivityHeatmap.tsx` | `analysisStore` |

`FreeLayoutCanvas.tsx` の `ChartContent` switch 文に 4 ケースを追加し、各コンポーネントを呼び出す。

---

## システム構成図

```
FreeLayoutCanvas.tsx (Mode D)
│
├── case 'sensitivity-heatmap'
│     └── <SensitivityHeatmap /> ← analysisStore.sensitivityResult
├── case 'importance'
│     └── <ImportanceChart /> ← analysisStore.sensitivityResult
├── case 'cluster-view'
│     └── <ClusterScatter /> ← clusterStore.pcaProjections/clusterLabels
└── case 'umap'
      └── <DimReductionScatter /> ← clusterStore.pcaProjections / wasm.runPca

                    ↑                              ↑
              analysisStore                  clusterStore
                    │                              │
                    └──────────────────────────────┘
                              useStudyStore.subscribe (Study変更でリセット)
                                         │
                                  WasmLoader.getInstance()
                                         │
                            ┌────────────────────────┐
                            │   tunny_core.wasm       │
                            │  ├── computeSensitivity │
                            │  ├── runPca             │
                            │  ├── runKmeans          │
                            │  ├── estimateKElbow     │
                            │  └── computeClusterStats│
                            └────────────────────────┘
```

**信頼性**: 🔵 *要件定義・既存設計より*

---

## ディレクトリ構造

**信頼性**: 🔵 *既存プロジェクト構造 / note.md より*

**変更ファイル**:
```
rust_core/src/
└── lib.rs                    ← 6 新規 wasm_bindgen 関数追加

frontend/src/
├── wasm/
│   ├── pkg/
│   │   └── tunny_core.d.ts   ← 6 関数シグネチャ + 5 インターフェース追加
│   └── wasmLoader.ts         ← 6 プロパティ + バインド追加
├── stores/
│   ├── analysisStore.ts      ← 新規作成
│   └── clusterStore.ts       ← 新規作成
├── components/
│   ├── charts/
│   │   ├── ImportanceChart.tsx    ← 新規作成
│   │   ├── ClusterScatter.tsx     ← 新規作成
│   │   └── DimReductionScatter.tsx← 新規作成
│   ├── layout/
│   │   └── FreeLayoutCanvas.tsx  ← 4 ケース追加
│   └── panels/
│       └── LeftPanel.tsx         ← ClusterPanel → clusterStore 接続
```

---

## 非機能要件の実現方法

### パフォーマンス 🔵

**信頼性**: 🔵 *NFR-VE-001〜003 / TASK-801/901 memo より*

- `computeSensitivity`: Rust 単体テスト済み（50,000 × 30 × 4 で 500ms 以内）
- `runPca + runKmeans`: Rust 単体テスト済み（50,000 × 34 で 400ms 以内）
- チャート初回描画目標: 1,500ms 以内（WASM 計算 + ECharts 描画）
- ローディングスピナーを WASM 計算中に表示してユーザー待機感を軽減

### スタイル 🔵

**信頼性**: 🔵 *NFR-VE-010 / wasm-phase2-requirements.md NFR-201 より*

新規コンポーネントは Tailwind CSS を使用しない。インラインスタイルまたは CSS 変数のみ使用する。

### エラー耐性 🔵

**信頼性**: 🔵 *NFR-VE-020 / 既存チャートパターンより*

- WASM エラーは try/catch で捕捉し、`sensitivityError` / `clusterError` にセット
- チャートは `EmptyState` コンポーネントでエラーメッセージを表示
- アプリ全体はクラッシュしない

---

## wasm-pack ビルド手順

**信頼性**: 🔵 *note.md より*

`lib.rs` に新規バインディングを追加後:
```bash
cd rust_core
wasm-pack build --target web --out-dir ../frontend/src/wasm/pkg
```

ビルド後 `tunny_core.d.ts` が自動更新されるが、型定義（インターフェース）は手動追加が必要。

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.ts](interfaces.ts)
- **要件定義**: [requirements.md](../../spec/visualization-enablement/requirements.md)
- **WASM API 仕様**: [wasm-api.md](../tunny-dashboard/wasm-api.md)
- **既存アーキテクチャ**: [architecture.md](../tunny-dashboard/architecture.md)

## 信頼性レベルサマリー

- 🔵 青信号: 11件 (92%)
- 🟡 黄信号: 1件 (8%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
