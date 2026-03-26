# Tunny Dashboard WASM API 仕様

Rust/WASMコア（`rust_core`）がwasm-bindgen経由でJavaScript層に公開する関数の仕様。

## 命名規則

- 関数名: `snake_case`（wasm-bindgenがJS側に `camelCase` でも公開可）
- 戻り値: `JsValue` 経由のJSONまたは `SharedArrayBuffer`
- エラー: `Result<T, JsValue>` でthrowableなJSエラーとして伝播

---

## Journalパーサ

### `parse_journal(data: Uint8Array) -> JsValue`

Journalファイル全体をパースし、全StudyのDataFrameをWASMメモリに構築する。

**入力:**
- `data`: Journalファイルのバイト列

**出力 (JSON):**
```typescript
{
  studies: Study[];        // 全Studyのメタ情報
  durationMs: number;
}
```

**処理:**
1. 1行1JSONのJSONLをop_codeベースのステートマシンで処理
2. 各StudyのDataFrameをWASMメモリに構築
3. 各パラメータの分布情報に基づき内部スケール値を表示値に逆変換
4. `RUNNING`・`PRUNED`・`FAIL`トライアルを保留リストで管理

**性能目標:** 50,000行で5秒以内

---

### `select_study(study_id: u32) -> JsValue`

アクティブなStudyを切り替える。

**入力:** `study_id`

**出力 (JSON):**
```typescript
{
  dataFrameInfo: DataFrameInfo;
  gpuBufferData: {
    positions: ArrayBuffer;   // Float32Array (N×2)
    positions3d: ArrayBuffer; // Float32Array (N×3)
    sizes: ArrayBuffer;       // Float32Array (N×1)
    trialCount: number;
  };
}
```

---

### `append_journal_diff(data: Uint8Array, byte_offset: u64) -> JsValue`

ライブ更新時の差分パース。指定バイトオフセット以降の新規行のみ処理する。

**入力:**
- `data`: 差分テキストのバイト列（S0〜S1の範囲）
- `byte_offset`: 前回読み込み終了位置

**出力 (JSON):**
```typescript
{
  newCompleteTrials: number;
  paretoUpdated: boolean;
  newTrialIndices: Uint32Array;  // SharedArrayBuffer
  newBestValue: number | null;   // 単目的の場合
  durationMs: number;
}
```

**性能目標:** 1,000行差分で20ms以内

---

## フィルタ・クエリ

### `filter_by_ranges(ranges_json: string) -> Uint32Array`

範囲条件を満たすtrial_idのインデックス配列を返す。

**入力 (JSON文字列):**
```typescript
{
  "x1": { "min": 2.0, "max": 8.0 },
  "obj1": { "min": null, "max": 0.5 }
}
```

**出力:** `Uint32Array`（条件を満たすtrialのインデックス）

**性能目標:** 50,000件で5ms以内

---

### `get_trial(index: u32) -> JsValue`

1試行分の詳細データを返す。

**出力 (JSON):**
```typescript
{
  trialId: number;
  params: Record<string, number | string>;
  values: number[] | null;
  paretoRank: number | null;
  clusterId: number | null;
  isFeasible: boolean | null;
  userAttrs: Record<string, number | string>;
  artifactIds: string[];
}
```

---

### `get_trials_batch(indices: Uint32Array) -> JsValue`

複数試行のデータをまとめて返す（Bottom Table表示用）。

**性能目標:** 1,000件で10ms以内

---

## Pareto計算

### `compute_pareto_ranks() -> JsValue`

現在選択中StudyのDataFrameに対してNDSortを実行する。

**出力 (JSON):**
```typescript
{
  ranks: Uint32Array;         // SharedArrayBuffer: 各trialのParetoランク
  paretoIndices: Uint32Array; // Rank1のインデックス
  hypervolume: number | null; // 2目的以上の場合のみ計算
  durationMs: number;
}
```

**性能目標:** 50,000点で100ms以内

---

### `compute_hypervolume_history() -> JsValue`

試行番号順にHypervolume推移を計算する。

**出力 (JSON):**
```typescript
{
  trialIds: number[];
  hvValues: number[];
  durationMs: number;
}
```

---

### `score_tradeoff_navigator(weights: Float64Array) -> Uint32Array`

Trade-off Navigator用。重みベクトルで全trialをスコアリングし、最良インデックスを返す。

**入力:** `weights` - 目的数分の重みベクトル（合計1.0に正規化）

**出力:** `Uint32Array` - スコア昇順のtrial_idインデックス

**性能目標:** 50,000点で1ms以内

---

## クラスタリング

### `run_pca(n_components: u32, space: &str) -> JsValue`

次元削減（PCA）を実行する。

**入力:**
- `n_components`: 削減後の次元数
- `space`: `"objective"` | `"variable"` | `"combined"`

**出力 (JSON):**
```typescript
{
  reducedData: Float64Array;   // [N × n_components] flattened
  explainedVariance: number[]; // 各主成分の寄与率
  durationMs: number;
}
```

**性能目標:** 50ms以内（50,000点 × 34次元）

---

### `run_kmeans(k: u32, data: Float64Array, n_cols: u32) -> JsValue`

k-meansクラスタリングを実行する。

**入力:**
- `k`: クラスタ数
- `data`: 入力データ（`run_pca`の出力 `reducedData`）
- `n_cols`: 列数

**出力 (JSON):**
```typescript
{
  labels: Int32Array;      // 各trialのクラスタラベル
  centroids: number[][];
  wcss: number;
  durationMs: number;
}
```

**性能目標:** 200ms以内（50,000点）

---

### `estimate_k_elbow(data: Float64Array, n_cols: u32, max_k: u32) -> JsValue`

Elbow法でkを自動推定する。

**出力 (JSON):**
```typescript
{
  ks: number[];
  wcss: number[];
  recommendedK: number;
  durationMs: number;
}
```

---

### `compute_cluster_stats(labels: Int32Array) -> JsValue`

クラスタ統計量（平均・標準偏差・有意差）を計算する。

**出力 (JSON):**
```typescript
{
  stats: ClusterStats[];
  durationMs: number;
}
```

---

## 感度分析

### `compute_spearman() -> JsValue`

Spearman順位相関係数（全変数×全目的）を計算する。

**出力 (JSON):**
```typescript
{
  matrix: Float64Array; // [nParams × nObjectives] flattened, row-major
  paramNames: string[];
  objectiveNames: string[];
  durationMs: number;
}
```

**性能目標:** 500ms以内（50,000点 × 30変数 × 4目的）

---

### `compute_ridge() -> JsValue`

Ridge回帰で標準化偏回帰係数βとR²を計算する。

**出力 (JSON):**
```typescript
{
  betaMatrix: Float64Array; // [nParams × nObjectives] flattened
  r2Values: Float64Array;   // [nObjectives]
  durationMs: number;
}
```

**性能目標:** 300ms以内

---

### `compute_sensitivity_selected(selected_indices: Uint32Array) -> JsValue`

Brushingで絞り込んだサブセットに対して感度を再計算する。

**性能目標:** 50ms以内（Spearman + β）

---

## PDP（Partial Dependence Plot）

### `compute_pdp(param_name: &str, objective_name: &str, n_grid: u32, n_samples: u32) -> JsValue`

Ridge回帰（簡易版）でPDPを計算する。

**入力:**
- `param_name`: 対象変数名
- `objective_name`: 対象目的名
- `n_grid`: グリッド点数（デフォルト50）
- `n_samples`: サブサンプリング数（デフォルト500）

**出力 (JSON):**
```typescript
{
  gridPoints: Float64Array;
  pdpValues: Float64Array;
  confidenceLow: Float64Array;
  confidenceHigh: Float64Array;
  iceLines: Float64Array | null; // [n_samples × n_grid] flattened, null if not requested
  rugValues: Float64Array;
  durationMs: number;
}
```

**性能目標:** 簡易版20ms以内

---

### `compute_pdp_2d(param1: &str, param2: &str, objective: &str, n_grid: u32) -> JsValue`

2変数PDP（交互作用ヒートマップ）を計算する。

**出力 (JSON):**
```typescript
{
  grid1: Float64Array; // param1のグリッド点
  grid2: Float64Array; // param2のグリッド点
  heatmap: Float64Array; // [n_grid × n_grid] flattened
  durationMs: number;
}
```

**性能目標:** 簡易版100ms以内

---

## ダウンサンプリング

### `downsample_for_thumbnail(max_points: u32) -> JsValue`

Scatter Matrixサムネイル用のダウンサンプリング。

**処理:**
1. Pareto点は必ず含める
2. 残りを空間的均等サンプリング（グリッド分割）
3. `selectedIndices`に含まれる点は必ず含める

**出力 (JSON):**
```typescript
{
  indices: Uint32Array; // ダウンサンプリング後のtrial_idインデックス
  durationMs: number;
}
```

**性能目標:** 1ms以内

---

## エクスポート

### `serialize_csv(indices: Uint32Array | null, columns_json: &str) -> JsValue`

CSVをUint8Arrayとして出力する。

**入力:**
- `indices`: 対象trialのインデックス（nullの場合は全件）
- `columns_json`: 出力する列名の配列（JSON文字列）

**出力:**
```typescript
{
  data: Uint8Array; // UTF-8 CSV
  rowCount: number;
  durationMs: number;
}
```

---

### `compute_report_stats() -> JsValue`

HTMLレポート生成用の統計サマリーを計算する。

**出力 (JSON):**
```typescript
{
  studySummary: {
    totalTrials: number;
    completeTrials: number;
    paretoSize: number;
    hypervolume: number | null;
  };
  objectiveStats: Array<{
    name: string;
    min: number;
    max: number;
    mean: number;
    std: number;
  }>;
  paramStats: Array<{
    name: string;
    min: number;
    max: number;
    mean: number;
  }>;
  durationMs: number;
}
```

---

## 性能目標サマリー

| 関数 | 目標時間 | 実行コンテキスト |
|---|---|---|
| `parse_journal()` | < 5,000ms | メインスレッド → WASM |
| `filter_by_ranges()` | < 5ms | メインスレッド → WASM |
| `compute_pareto_ranks()` | < 100ms | WASM |
| `score_tradeoff_navigator()` | < 1ms | WASM |
| `run_pca()` | < 50ms | WASM |
| `run_kmeans()` | < 200ms | WASM |
| `compute_spearman()` | < 500ms | WASM |
| `compute_ridge()` | < 300ms | WASM |
| `compute_sensitivity_selected()` | < 50ms | WASM |
| `compute_pdp()` (Ridge) | < 20ms | WASM |
| `compute_pdp_2d()` (Ridge) | < 100ms | WASM |
| `downsample_for_thumbnail()` | < 1ms | WASM |
| `append_journal_diff()` | < 20ms | WASM |
| `compute_report_stats()` | < 100ms | WASM |
