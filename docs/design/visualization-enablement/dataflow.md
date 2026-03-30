# 可視化機能有効化 データフロー図

**作成日**: 2026-03-29
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/visualization-enablement/requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## フロー 1: SensitivityHeatmap 初回表示 🔵

**信頼性**: 🔵 *REQ-VE-050〜054 / ユーザーストーリー 1.1 より*

**関連要件**: REQ-VE-001, REQ-VE-030〜032, REQ-VE-050〜054

```
ユーザー: Mode D でキャンバスに "Sensitivity" チャートを追加
  │
  ▼
FreeLayoutCanvas: case 'sensitivity-heatmap' レンダリング
  │
  ▼
useEffect: analysisStore.sensitivityResult === null ?
  │ YES
  ▼
analysisStore.computeSensitivity() 呼び出し
  │
  ├─ set({ isComputingSensitivity: true })
  │
  ▼
WasmLoader.getInstance().then(wasm => wasm.computeSensitivity())
  │
  ▼ [WASM: sensitivity::compute_sensitivity()]
  │
  ├─ 成功: sensitivityResult にセット, isComputingSensitivity = false
  │      ↓
  │   <SensitivityHeatmap data={sensitivityResult} isLoading={false} />
  │      ↓
  │   ECharts ヒートマップ表示 (Spearman/Ridge切替・閾値スライダー付き)
  │
  └─ 失敗: sensitivityError にセット, isComputingSensitivity = false
         ↓
      <EmptyState message="Sensitivity computation failed" />
```

**計算中フロー**:
```
isComputingSensitivity === true
  │
  ▼
<SensitivityHeatmap isLoading={true} />
  │
  ▼
ローディングスピナー表示
```

---

## フロー 2: Importance チャート表示 🔵

**信頼性**: 🔵 *REQ-VE-060〜065 / ユーザーストーリー 1.3 より*

**関連要件**: REQ-VE-060〜065

```
FreeLayoutCanvas: case 'importance' レンダリング
  │
  ▼
ImportanceChart マウント
  │
  ├─ analysisStore.sensitivityResult !== null → 即時描画
  │
  └─ analysisStore.sensitivityResult === null
       │
       ▼
     analysisStore.computeSensitivity() 自動呼び出し
       │         (フロー1と同じ WASM 呼び出し)
       ▼
     sensitivityResult 取得後:
       │
       ▼
     ユーザーが指標ドロップダウンを選択
       │
       ├─ "spearman" → spearman[param][obj] の |ρ| を全 obj で平均
       │
       └─ "beta" → ridge[obj].beta[param] の |β| を全 obj で平均
             │
             ▼
           降順ソート → 水平バーチャート (ECharts)
           タイトル: "Parameter Importance (Spearman |ρ|)" 等
```

**EmptyState 条件**:
- `currentStudy.paramNames.length === 0` → `EmptyState message="No parameters"`
- WASM エラー → `EmptyState message="Sensitivity computation failed"`

---

## フロー 3: ClusterView 表示（クラスタリング実行後） 🔵

**信頼性**: 🔵 *REQ-VE-040〜046, REQ-VE-070〜077 / ユーザーストーリー 2.1 より*

**関連要件**: REQ-VE-003〜006, REQ-VE-040〜043, REQ-VE-070〜077

### 3a: クラスタリング実行（LeftPanel → clusterStore）

```
ユーザー: LeftPanel > ClusterPanel で Space/k を設定し「Run」ボタン押下
  │
  ▼
ClusterPanel.onRunClustering(space, k) コールバック
  │
  ▼
clusterStore.runClustering(space, k)
  │
  ├─ set({ isRunning: true, clusterError: null })
  │
  ▼
Step 1: WasmLoader → wasm.runPca(2, space)
  │       [WASM: clustering::run_pca(2, PcaSpace::from(space))]
  │       → { projections: number[][], explainedVariance: number[], ... }
  ├─ set({ pcaProjections: projections })
  │
  ▼
Step 2: projections を Float64Array(flat) に変換
  │
  ▼
Step 3: WasmLoader → wasm.runKmeans(k, flatProjections, 2)
  │       [WASM: clustering::run_kmeans(k, &flat_data, 2)]
  │       → { labels: number[], centroids: number[][], wcss: number, ... }
  ├─ set({ clusterLabels: labels })
  │
  ▼
Step 4: Int32Array(labels) を作成
  │
  ▼
Step 5: WasmLoader → wasm.computeClusterStats(Int32ArrayLabels)
  │       [WASM: compute_cluster_stats(&labels_as_usize)]
  │       → { stats: ClusterStat[], durationMs: number }
  ├─ set({ clusterStats: stats, isRunning: false })
  │
  └─ エラー時: set({ clusterError: err, isRunning: false })
```

### 3b: ClusterView チャート表示

```
FreeLayoutCanvas: case 'cluster-view' レンダリング
  │
  ▼
ClusterScatter マウント
  │
  ├─ clusterStore.pcaProjections === null
  │     ↓
  │   <EmptyState message="Run clustering in the left panel first" />
  │
  ├─ clusterStore.clusterError !== null
  │     ↓
  │   <EmptyState message={clusterError} />
  │
  ├─ clusterStore.isRunning === true
  │     ↓
  │   ローディングスピナー
  │
  └─ pcaProjections あり
       │
       ▼
     ECharts scatter:
       - X軸: PC1, Y軸: PC2
       - 点ごとに clusterLabels[i] → getClusterColor(clusterId) で色付け
       - 凡例: "Cluster 0" 〜 "Cluster k-1"
```

---

## フロー 4: UMAP チャート（PCA モード）表示 🔵

**信頼性**: 🔵 *REQ-VE-080〜087 / ユーザーストーリー 3.1 より*

**関連要件**: REQ-VE-080〜087

```
FreeLayoutCanvas: case 'umap' レンダリング
  │
  ▼
DimReductionScatter マウント
  │
  ├─ clusterStore.pcaProjections !== null
  │     ↓
  │   既存データを再利用（追加 WASM 呼び出しなし）
  │
  └─ clusterStore.pcaProjections === null
       │
       ▼
     wasm.runPca(2, 'all') 直接呼び出し（clusterStore を経由しない）
       │
       ▼
     projections 取得後:
       │
       ▼
     ECharts scatter:
       - タイトル: "Dimensionality Reduction (PCA)"
       - 点の色: selectionStore.colorMode カラーマップ
       - 次元削減方式セレクター: "PCA" (選択中) / "UMAP (Coming Soon)" (disabled)
```

**セレクター切り替え**:
```
ユーザー: "UMAP (Coming Soon)" を選択
  │
  ▼
disabled のため操作無効 → "Coming Soon" メッセージ表示
```

---

## フロー 5: Study 変更によるリセット 🔵

**信頼性**: 🔵 *REQ-VE-034, REQ-VE-045 / 設計ヒアリング（subscribe パターン）より*

**関連要件**: REQ-VE-034, REQ-VE-045

```
studyStore.selectStudy(newStudyId) 呼び出し
  │
  ▼
studyStore.currentStudy が更新される
  │
  ▼ [useStudyStore.subscribe で購読]
  │
  ├─ analysisStore の subscriber が発火
  │     └─ set({ sensitivityResult: null, sensitivityError: null,
  │              isComputingSensitivity: false })
  │
  └─ clusterStore の subscriber が発火
        └─ set({ pcaProjections: null, clusterLabels: null,
                 clusterStats: null, elbowResult: null,
                 clusterError: null, isRunning: false })
```

**購読初期化**（各ストアのトップレベル）:
```typescript
// analysisStore.ts 末尾
useStudyStore.subscribe(
  (state) => state.currentStudy,
  (_newStudy) => {
    useAnalysisStore.setState({ sensitivityResult: null, sensitivityError: null })
  }
)

// clusterStore.ts 末尾
useStudyStore.subscribe(
  (state) => state.currentStudy,
  (_newStudy) => {
    useClusterStore.setState({
      pcaProjections: null, clusterLabels: null,
      clusterStats: null, elbowResult: null, clusterError: null,
    })
  }
)
```

---

## フロー 6: Elbow 法 k 推定 🔵

**信頼性**: 🔵 *REQ-VE-043 / ユーザーストーリー 2.2 より*

**関連要件**: REQ-VE-005, REQ-VE-043

```
ユーザー: ClusterPanel で「Estimate k」ボタン押下
  │
  ▼
clusterStore.estimateK(space)
  │
  ├─ set({ isRunning: true })
  │
  ▼
Step 1: wasm.runPca(2, space) → projections
  │
  ▼
Step 2: projections を Float64Array(flat) に変換
  │
  ▼
Step 3: wasm.estimateKElbow(flat, 2, 10)
  │       [WASM: estimate_k_elbow(&data, 2, 10)]
  │       → { wcssPerK: number[], recommendedK: number, durationMs: number }
  │
  ▼
set({ elbowResult: { wcssPerK, recommendedK }, isRunning: false })
  │
  ▼
ClusterPanel に elbowResult が渡され Elbow グラフ表示
推奨 k が入力欄に反映される（ClusterPanel 側の責務）
```

---

## Brushing 後の感度再計算フロー 🔵

**信頼性**: 🔵 *REQ-VE-002, REQ-VE-033 / ユーザーストーリー 1.2 より*

**関連要件**: REQ-VE-002, REQ-VE-033

```
ユーザー: Pareto 散布図でブラシ選択
  │
  ▼
selectionStore.brushSelect(indices: Uint32Array)
  │
  ▼
[SensitivityHeatmap 内の「再計算」ボタン or 自動トリガー]
  │
  ▼
analysisStore.computeSensitivitySelected(indices)
  │
  ▼
wasm.computeSensitivitySelected(Uint32Array(indices))
  │   [WASM: sensitivity::compute_sensitivity_selected(&indices)]
  │
  ▼
sensitivityResult 更新 → ヒートマップ再描画
```

---

## エラーハンドリングフロー 🔵

**信頼性**: 🔵 *NFR-VE-020 / EDGE-VE-001〜040 より*

```
WASM 呼び出し
  │
  ├─ Success: Result<JsValue>
  │     └─ JSON.parse → ストア state 更新 → チャート描画
  │
  └─ Error: throws JS Error (Err(JsValue::from_str(...)))
        │
        ▼
      try/catch → error.message を sensitivityError / clusterError にセット
        │
        ▼
      <EmptyState message={error} /> 表示

特殊エラーケース:
  - "No active study"  → EmptyState "Please select a study"
  - "Empty selection"  → EmptyState "Select trials first"
  - "Insufficient data for PCA" → EmptyState "Insufficient trials (min 2)"
  - "Invalid space"    → 内部エラー（ログのみ）
```

---

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **型定義**: [interfaces.ts](interfaces.ts)
- **ヒアリング記録**: [design-interview.md](design-interview.md)

## 信頼性レベルサマリー

- 🔵 青信号: 7件 (100%)
- 🟡 黄信号: 0件 (0%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
