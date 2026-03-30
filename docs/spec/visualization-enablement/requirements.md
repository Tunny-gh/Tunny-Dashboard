# 可視化機能有効化 要件定義書

## 概要

FreeLayoutCanvas (Mode D) に登録済みだが `default` フォールバック（"This chart is under development"）となっている 4 チャート — **Importance**・**SensitivityHeatmap**・**ClusterView**・**UMAP** — を実際に動作させる。

Rust/WASM 側の実装（`sensitivity.rs`, `clustering.rs`）は既に完了・テスト済みであり、本スコープは「WASM バインディング公開 → TypeScript 型宣言 → WasmLoader バインディング → Zustand ストア新設 → FreeLayoutCanvas チャート配線」の 4 層を一気通貫で実装することにある。

### 既実装の Rust 関数

| Rust 関数 | ファイル | テスト | 用途 |
|---|---|---|---|
| `compute_sensitivity()` | `sensitivity.rs` | 16件合格 | Spearman + Ridge 全試行 |
| `compute_sensitivity_selected(indices)` | `sensitivity.rs` | 同上 | Brushing サブセット用 |
| `run_pca(n_components, space)` | `clustering.rs` | 11件合格 | PCA 次元削減 |
| `run_kmeans(k, flat_data, n_cols)` | `clustering.rs` | 同上 | k-means クラスタリング |
| `estimate_k_elbow(flat_data, n_cols, max_k)` | `clustering.rs` | 同上 | Elbow 法 k 自動推定 |
| `compute_cluster_stats(labels)` | `clustering.rs` | 同上 | クラスタ統計量 |

### 現状ギャップ

`tunny_core.d.ts` の現在の公開関数一覧:
`parseJournal` / `selectStudy` / `filterByRanges` / `serializeCsv` / `computeHvHistory` / `appendJournalDiff` / `computeReportStats` / `getTrials`

上記 6 関数の WASM bindgen エクスポートが存在しない。`analysisStore` および `clusterStore` の Zustand 実装も存在しない。

## 関連文書

- **ヒアリング記録**: [💬 interview-record.md](interview-record.md)
- **ユーザストーリー**: [📖 user-stories.md](user-stories.md)
- **受け入れ基準**: [✅ acceptance-criteria.md](acceptance-criteria.md)
- **元仕様**: [docs/spec/tunny-dashboard-requirements.md](../tunny-dashboard-requirements.md)
- **WASM API 仕様**: [docs/design/tunny-dashboard/wasm-api.md](../../design/tunny-dashboard/wasm-api.md)
- **WASM Phase 2 実績**: [docs/spec/wasm-phase2-requirements.md](../wasm-phase2-requirements.md)

## 機能要件（EARS 記法）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: PRD・EARS 要件定義書・設計文書・ユーザヒアリングを参考にした確実な要件
- 🟡 **黄信号**: PRD・EARS 要件定義書・設計文書・ユーザヒアリングから妥当な推測による要件
- 🔴 **赤信号**: PRD・EARS 要件定義書・設計文書・ユーザヒアリングにない推測による要件

---

### REQ-VE-001〜006: WASM バインディング（lib.rs）

#### REQ-VE-001: `computeSensitivity` WASM バインディング 🔵

*tunny-dashboard-requirements.md REQ-090〜092 / wasm-api.md `compute_spearman`+`compute_ridge` より*

- REQ-VE-001-A: `rust_core/src/lib.rs` に `#[wasm_bindgen(js_name = "computeSensitivity")]` 関数を追加しなければならない
- REQ-VE-001-B: `wasm_compute_sensitivity()` は `sensitivity::compute_sensitivity()` を呼び出し、以下 JSON 形式で返さなければならない:
  ```typescript
  {
    spearman: number[][];       // [nParams][nObjectives]
    ridge: { beta: number[]; rSquared: number }[]; // [nObjectives]
    paramNames: string[];
    objectiveNames: string[];
    durationMs: number;
  }
  ```
- REQ-VE-001-C: `compute_sensitivity()` が `None` を返した場合（アクティブ Study なし）は `Err(JsValue::from_str("No active study"))` を返さなければならない

#### REQ-VE-002: `computeSensitivitySelected` WASM バインディング 🔵

*tunny-dashboard-requirements.md REQ-098 / wasm-api.md `compute_sensitivity_selected` より*

- REQ-VE-002-A: `rust_core/src/lib.rs` に `#[wasm_bindgen(js_name = "computeSensitivitySelected")]` 関数を追加しなければならない
- REQ-VE-002-B: `wasm_compute_sensitivity_selected(indices: js_sys::Uint32Array)` は `sensitivity::compute_sensitivity_selected(&indices)` を呼び出し、REQ-VE-001-B と同形式で返さなければならない
- REQ-VE-002-C: `indices` が空の場合は `Err(JsValue::from_str("Empty selection"))` を返さなければならない

#### REQ-VE-003: `runPca` WASM バインディング 🔵

*wasm-api.md `run_pca` より*

- REQ-VE-003-A: `rust_core/src/lib.rs` に `#[wasm_bindgen(js_name = "runPca")]` 関数を追加しなければならない
- REQ-VE-003-B: `wasm_run_pca(n_components: u32, space: &str)` は `space` を `PcaSpace` に変換（`"param"` / `"objective"` / `"all"`）し `clustering::run_pca(n_components, space)` を呼び出し、以下 JSON 形式で返さなければならない:
  ```typescript
  {
    projections: number[][];     // [N][n_components]
    explainedVariance: number[]; // [n_components]
    featureNames: string[];
    durationMs: number;
  }
  ```
- REQ-VE-003-C: `run_pca` が `None` を返した場合（データ不足）は `Err(JsValue::from_str("Insufficient data for PCA"))` を返さなければならない
- REQ-VE-003-D: 無効な `space` 文字列の場合は `Err(JsValue::from_str("Invalid space"))` を返さなければならない

#### REQ-VE-004: `runKmeans` WASM バインディング 🔵

*wasm-api.md `run_kmeans` より*

- REQ-VE-004-A: `rust_core/src/lib.rs` に `#[wasm_bindgen(js_name = "runKmeans")]` 関数を追加しなければならない
- REQ-VE-004-B: `wasm_run_kmeans(k: u32, data: js_sys::Float64Array, n_cols: u32)` は `data` を `Vec<f64>` に変換し `clustering::run_kmeans(k, &data, n_cols)` を呼び出し、以下 JSON 形式で返さなければならない:
  ```typescript
  {
    labels: number[];    // [N] クラスタラベル (0-indexed)
    centroids: number[][]; // [k][n_cols]
    wcss: number;
    durationMs: number;
  }
  ```

#### REQ-VE-005: `estimateKElbow` WASM バインディング 🔵

*wasm-api.md `estimate_k_elbow` より*

- REQ-VE-005-A: `rust_core/src/lib.rs` に `#[wasm_bindgen(js_name = "estimateKElbow")]` 関数を追加しなければならない
- REQ-VE-005-B: `wasm_estimate_k_elbow(data: js_sys::Float64Array, n_cols: u32, max_k: u32)` は `clustering::estimate_k_elbow(&data, n_cols, max_k)` を呼び出し、以下 JSON 形式で返さなければならない:
  ```typescript
  {
    wcssPerK: number[];     // [max_k - 1] (k=2〜max_k)
    recommendedK: number;
    durationMs: number;
  }
  ```

#### REQ-VE-006: `computeClusterStats` WASM バインディング 🔵

*wasm-api.md `compute_cluster_stats` より*

- REQ-VE-006-A: `rust_core/src/lib.rs` に `#[wasm_bindgen(js_name = "computeClusterStats")]` 関数を追加しなければならない
- REQ-VE-006-B: `wasm_compute_cluster_stats(labels: js_sys::Int32Array)` は `labels` を `Vec<usize>` に変換し `clustering::compute_cluster_stats(&labels)` を呼び出し、以下 JSON 形式で返さなければならない:
  ```typescript
  {
    stats: Array<{
      clusterId: number;
      size: number;
      centroid: Record<string, number>;
      std: Record<string, number>;
      significantDiffs: string[];
    }>;
    durationMs: number;
  }
  ```

---

### REQ-VE-010〜016: TypeScript 型宣言（tunny_core.d.ts）

🔵 *wasm-api.md および wasm-phase2-requirements.md のパターンより*

- REQ-VE-010: `tunny_core.d.ts` に `computeSensitivity(): SensitivityWasmResult` を追加しなければならない
- REQ-VE-011: `tunny_core.d.ts` に `computeSensitivitySelected(indices: Uint32Array): SensitivityWasmResult` を追加しなければならない
- REQ-VE-012: `tunny_core.d.ts` に `runPca(n_components: number, space: string): PcaWasmResult` を追加しなければならない
- REQ-VE-013: `tunny_core.d.ts` に `runKmeans(k: number, data: Float64Array, n_cols: number): KmeansWasmResult` を追加しなければならない
- REQ-VE-014: `tunny_core.d.ts` に `estimateKElbow(data: Float64Array, n_cols: number, max_k: number): ElbowWasmResult` を追加しなければならない
- REQ-VE-015: `tunny_core.d.ts` に `computeClusterStats(labels: Int32Array): ClusterStatsWasmResult` を追加しなければならない
- REQ-VE-016: 上記型インターフェース `SensitivityWasmResult`・`PcaWasmResult`・`KmeansWasmResult`・`ElbowWasmResult`・`ClusterStatsWasmResult` を `tunny_core.d.ts` に定義しなければならない

---

### REQ-VE-020〜026: WasmLoader バインディング（wasmLoader.ts）

🔵 *wasmLoader.ts 既存パターンより*

- REQ-VE-020: `wasmLoader.ts` に以下のメソッドシグネチャを追加しなければならない:
  - `computeSensitivity: () => SensitivityWasmResult`
  - `computeSensitivitySelected: (indices: Uint32Array) => SensitivityWasmResult`
  - `runPca: (nComponents: number, space: string) => PcaWasmResult`
  - `runKmeans: (k: number, data: Float64Array, nCols: number) => KmeansWasmResult`
  - `estimateKElbow: (data: Float64Array, nCols: number, maxK: number) => ElbowWasmResult`
  - `computeClusterStats: (labels: Int32Array) => ClusterStatsWasmResult`
- REQ-VE-021: `wasmLoader.ts` の `_initialize()` で各メソッドを対応する wasm 関数にバインドしなければならない
- REQ-VE-022: wasm-pack でビルドされた `tunny_core` の import 文に新規関数を追加しなければならない

---

### REQ-VE-030〜035: analysisStore 新設（Zustand）

🔵 *tunny-dashboard-requirements.md REQ-090〜098 / ユーザヒアリングより*

- REQ-VE-030: `frontend/src/stores/analysisStore.ts` を Zustand ストアとして新設しなければならない
- REQ-VE-031: `analysisStore` は以下の状態を持たなければならない:
  - `sensitivityResult: SensitivityWasmResult | null`
  - `isComputingSensitivity: boolean`
  - `sensitivityError: string | null`
- REQ-VE-032: `analysisStore.computeSensitivity()` は `WasmLoader.getInstance()` を通じて WASM を呼び出し、結果を `sensitivityResult` にセットしなければならない
- REQ-VE-033: `analysisStore.computeSensitivitySelected(indices: Uint32Array)` はサブセット感度再計算を行い、結果を `sensitivityResult` にセットしなければならない
- REQ-VE-034: Study 変更時（`studyStore.selectStudy` 後）に `sensitivityResult` を `null` にリセットしなければならない 🟡 *既存 `selectionStore` の Study 変更パターンから妥当な推測*

---

### REQ-VE-040〜046: clusterStore 新設（Zustand）

🔵 *tunny-dashboard-requirements.md REQ-080〜087 / ユーザヒアリングより*

- REQ-VE-040: `frontend/src/stores/clusterStore.ts` を Zustand ストアとして新設しなければならない
- REQ-VE-041: `clusterStore` は以下の状態を持たなければならない:
  - `pcaProjections: number[][] | null`  // [N][2]
  - `clusterLabels: number[] | null`     // [N]
  - `clusterStats: ClusterStatsWasmResult | null`
  - `elbowResult: ElbowWasmResult | null`
  - `isRunning: boolean`
  - `clusterError: string | null`
  - `clusterSpace: 'param' | 'objective' | 'all'`
  - `k: number`
- REQ-VE-042: `clusterStore.runClustering(space, k)` は以下の順で WASM を呼び出さなければならない:
  1. `wasm.runPca(2, space)` → `pcaProjections` にセット
  2. `wasm.runKmeans(k, flat_projections, 2)` → `clusterLabels` にセット
  3. `wasm.computeClusterStats(labels)` → `clusterStats` にセット
- REQ-VE-043: `clusterStore.estimateK(space)` は `wasm.runPca(2, space)` → `wasm.estimateKElbow(...)` を呼び出し、`elbowResult` にセットしなければならない
- REQ-VE-044: `LeftPanel.tsx` の `ClusterPanel` への配線を `clusterStore` 経由に更新しなければならない。`ClusterPanel` の `onRunClustering` コールバックが `clusterStore.runClustering(space, k)` を呼び出すよう接続しなければならない 🔵 *TASK-902 memo/ClusterPanel 設計より*
- REQ-VE-045: Study 変更時に `pcaProjections`・`clusterLabels`・`clusterStats`・`elbowResult` を `null` にリセットしなければならない 🟡 *既存パターンから妥当な推測*

---

### REQ-VE-050〜055: SensitivityHeatmap チャート配線

🔵 *TASK-802 memo / SensitivityHeatmap.tsx 既実装より*

- REQ-VE-050: `FreeLayoutCanvas.tsx` の `ChartContent` switch 文に `case 'sensitivity-heatmap':` を追加しなければならない
- REQ-VE-051: `sensitivity-heatmap` チャートマウント時、`analysisStore.sensitivityResult` が `null` であれば `analysisStore.computeSensitivity()` を自動実行しなければならない
- REQ-VE-052: `analysisStore.isComputingSensitivity` が `true` の間は `SensitivityHeatmap` に `isLoading={true}` を渡さなければならない
- REQ-VE-053: `sensitivityResult` が揃ったら `SensitivityData` 形式に変換して `SensitivityHeatmap` コンポーネント（既存実装）に渡さなければならない。変換は `spearman: SensitivityWasmResult.spearman` / `ridge: SensitivityWasmResult.ridge` を用いなければならない
- REQ-VE-054: WASM エラー時は `EmptyState message="Sensitivity computation failed"` を表示しなければならない

---

### REQ-VE-060〜066: Importance チャート（ダミー実装の置き換え）

🔵 *tunny-dashboard-requirements.md REQ-090〜092 / ユーザヒアリングより*

- REQ-VE-060: `FreeLayoutCanvas.tsx` の `case 'importance':` のダミー実装（全値 1.0）を実際の WASM 計算に置き換えなければならない
- REQ-VE-061: Importance チャートは **重要度指標ドロップダウン** を持たなければならない。選択肢:
  - `"spearman"` — Spearman |ρ| の目的関数全体平均
  - `"beta"` — Ridge β の目的関数全体平均絶対値
- REQ-VE-062: 選択された指標に基づき、パラメータ × 目的関数の重要度を平均し、降順ソートした水平バーチャートを表示しなければならない
- REQ-VE-063: チャートタイトルは "Parameter Importance ({metric})" 形式で表示しなければならない（例: "Parameter Importance (Spearman |ρ|)"）
- REQ-VE-064: `analysisStore.sensitivityResult` が `null` の場合は自動実行し、ローディング表示を行わなければならない
- REQ-VE-065: `currentStudy.paramNames.length === 0` の場合は `EmptyState` を表示しなければならない

---

### REQ-VE-070〜077: ClusterView チャート

🔵 *tunny-dashboard-requirements.md REQ-080〜087 / ユーザヒアリングより*

- REQ-VE-070: `FreeLayoutCanvas.tsx` の `ChartContent` switch 文に `case 'cluster-view':` を追加しなければならない
- REQ-VE-071: `clusterStore.pcaProjections` が `null` の場合は `EmptyState message="Run clustering in the left panel first"` を表示しなければならない
- REQ-VE-072: `pcaProjections` が揃っている場合、各試行の 2D 座標（PCA）と `clusterLabels` による色分け散布図を ECharts で表示しなければならない
- REQ-VE-073: クラスタ色は `ClusterList.tsx` に実装済みの `getClusterColor(clusterId)` を使用しなければならない 🔵 *TASK-902 memo より*
- REQ-VE-074: `clusterStore.isRunning` が `true` の間はローディング表示を行わなければならない
- REQ-VE-075: `clusterStore.clusterLabels` が `null`（クラスタリング未実行）だが `pcaProjections` がある場合は、単色でプロットしても構わない 🟡 *推測・実装上の簡略化として妥当*
- REQ-VE-076: X 軸ラベル "PC1"・Y 軸ラベル "PC2"・凡例「Cluster {n}」を表示しなければならない
- REQ-VE-077: `clusterStore.clusterError` がある場合は `EmptyState message={error}` を表示しなければならない

---

### REQ-VE-080〜087: UMAP チャート（PCA 次元削減散布図として実装）

🔵 *tunny-dashboard-requirements.md REQ-082・REQ-086 / ユーザヒアリングより*

> **設計根拠**: REQ-082 は「前処理として PCA（WASM 実装）を適用し、**オプションで** UMAP（umap-js）を提供」と定める。UMAP は Phase 12 スコープであり、本フェーズでは PCA 2D 散布図として UMAP チャートを実現し、UMAP は将来の拡張として留保する。

- REQ-VE-080: `FreeLayoutCanvas.tsx` の `ChartContent` switch 文に `case 'umap':` を追加しなければならない
- REQ-VE-081: UMAP チャートは **次元削減方式セレクター** を持たなければならない。選択肢:
  - `"pca"` — WASM PCA（デフォルト、本フェーズで実装）
  - `"umap"` — umap-js（"Coming Soon" として disabled 表示）
- REQ-VE-082: PCA モードでは `clusterStore.pcaProjections` が利用可能な場合はそれを再利用し、ない場合は `wasm.runPca(2, 'all')` を呼び出して取得しなければならない
- REQ-VE-083: 取得した 2D 投影データを散布図（ECharts scatter）で表示しなければならない
- REQ-VE-084: 点の色は `selectionStore.colorMode` による既存カラーマップを使用しなければならない 🔵 *既存 ParallelCoordinates 等のパターンより*
- REQ-VE-085: チャートタイトルは `"Dimensionality Reduction (PCA)"` と表示しなければならない
- REQ-VE-086: `clusterStore.isRunning` が `true` の間はローディング表示を行わなければならない
- REQ-VE-087: `EmptyState` は PCA データが取得できない場合のみ表示しなければならない

---

## 非機能要件

### パフォーマンス

- NFR-VE-001: `computeSensitivity()` は 50,000 試行 × 30 変数 × 4 目的で 500 ms 以内に完了しなければならない（Rust 単体テスト TC-801-P01/P02 済み） 🔵 *TASK-801 memo より*
- NFR-VE-002: `runPca(2, "param") + runKmeans(4, ...)` の合計は 50,000 試行で 400 ms 以内に完了しなければならない（Rust 単体テスト TC-901-P01/P02 済み） 🔵 *TASK-901 memo より*
- NFR-VE-003: 各チャートの初回描画（WASM 計算含む）は 1,500 ms 以内に完了することを目標とする 🟡 *既存 Hypervolume チャートパターンから妥当な推測*

### スタイル

- NFR-VE-010: 新規追加コンポーネントは Tailwind CSS を使用してはならない（インラインスタイルまたは CSS 変数のみ） 🔵 *wasm-phase2-requirements.md NFR-201 より*

### エラー耐性

- NFR-VE-020: WASM 呼び出しが失敗した場合、フロントエンドはクラッシュせず `EmptyState` でエラーメッセージを表示しなければならない 🔵 *既存チャートの共通パターンより*

---

## Edge ケース

### WASM バインディング共通

- EDGE-VE-001: アクティブ Study が未選択の状態で WASM を呼び出した場合は適切なエラーを返し、チャートは `EmptyState` を表示しなければならない 🔵 *既存パターンより*
- EDGE-VE-002: パラメータ数が 0 の Study で感度計算・クラスタリングを呼び出した場合は `EmptyState message="No parameters"` を表示しなければならない 🔵 *tunny-dashboard-requirements.md EDGE-102 より*

### SensitivityHeatmap

- EDGE-VE-010: 試行数が 2 未満の場合、Spearman 計算は 0.0 を返すため `EmptyState message="Insufficient trials (min 2)"` を表示しなければならない 🔵 *sensitivity.rs 境界値テスト TC-801-05 より*

### Importance

- EDGE-VE-020: 目的関数が 1 つの場合は平均をとらずその 1 目的分の重要度をそのまま表示しなければならない 🟡 *推測・単目的スタディの考慮*

### ClusterView

- EDGE-VE-030: k=1 でクラスタリングした場合（ClusterPanel 側でバリデーション済み）はクラスタ色が 1 色になるが表示は継続しなければならない 🔵 *TASK-902 memo TC-902-E01 より*
- EDGE-VE-031: 試行数が k より少ない場合の k-means エラーは `clusterStore.clusterError` にセットして `EmptyState` で表示しなければならない 🔵 *clustering.rs の既存エラー処理より*

### UMAP チャート

- EDGE-VE-040: UMAP モードを選択した場合（`disabled` 解除前）は「Coming Soon」メッセージを表示しなければならない 🔵 *ユーザヒアリング・REQ-082 より*
