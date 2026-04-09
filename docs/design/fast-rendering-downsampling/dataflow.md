# 高速描画ダウンサンプリング データフロー図

**作成日**: 2026-04-07
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: [requirements.md](../../spec/tunny-dashboard-requirements.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## システム全体のデータフロー 🔵

**信頼性**: 🔵 *既存 tunny-dashboard アーキテクチャ / ユーザヒアリングより*

```mermaid
flowchart TD
    A[Journal読み込み完了] --> B[compute_pareto_ranks WASM]
    B --> C[downsampleStore.recompute]
    C --> D[sampling.rs WASM × 6種]
    D --> E[インデックスキャッシュ更新]

    E --> F1[ParetoScatter2D/3D\n scatter 10k点]
    E --> F2[ObjectivePairMatrix\n scatter 10k点]
    E --> F3[ScatterMatrix thumbnail\n thumbnail 500点]
    E --> F4[ScatterMatrix hover\n hover 3k点]
    E --> F5[ParallelCoordinates\n pcp 5k点]
    E --> F6[SlicePlot/SurfacePlot3D\n data_points 5k点]
    E --> F7[ClusterScatter\n cluster 10k点]
```

---

## 主要フロー

### フロー1: Study選択時のダウンサンプリング初期化 🔵

**信頼性**: 🔵 *既存 studyStore.selectStudy / compute_pareto_ranks フローより*

**関連要件**: REQ-060, REQ-063, REQ-072

```mermaid
sequenceDiagram
    participant U as ユーザー
    participant SS as studyStore
    participant DS as downsampleStore
    participant WL as WasmLoader
    participant SM as sampling.rs WASM

    U->>SS: selectStudy(studyId)
    SS->>WL: select_study(studyId)
    WL-->>SS: DataFrameInfo + GPUバッファ
    SS->>WL: compute_pareto_ranks()
    WL->>SM: (内部) Paretoランク計算
    WL-->>SS: paretoIndices

    SS->>DS: notifyStudyChanged(paretoIndices)
    DS->>WL: downsampleSmart(10000, true)      ← scatter
    DS->>WL: downsampleForThumbnail(500)        ← thumbnail
    DS->>WL: downsampleForThumbnail(3000)       ← hover
    DS->>WL: downsampleStratifiedByRank(5000,5) ← pcp
    DS->>WL: downsampleSmart(5000, false)       ← data_points
    DS->>WL: downsampleSmart(10000, true)       ← cluster (Pareto優先)
    WL-->>DS: 各 DownsampleResult（Uint32Array × 6）
    DS-->>DS: キャッシュ更新
```

**詳細ステップ**:
1. `selectStudy` → Pareto 計算 → `paretoIndices` を downsampleStore に通知
2. downsampleStore が 6 種のダウンサンプリングを WASM に並列発行（または順次）
3. 各 Uint32Array を種別キーで内部マップに保存
4. チャートコンポーネントが `getIndices(key)` で取得して描画

---

### フロー2: フィルタ変更時の再ダウンサンプリング 🔵

**信頼性**: 🔵 *REQ-042〜043 / selectionStore パターンより*

**関連要件**: REQ-040, REQ-042, REQ-043

```mermaid
sequenceDiagram
    participant U as ユーザー
    participant SES as selectionStore
    participant WL as WasmLoader
    participant DS as downsampleStore

    U->>SES: addAxisFilter(range)
    SES->>WL: filter_by_ranges(ranges)
    WL-->>SES: filteredIndices (Uint32Array)
    SES->>SES: GPU alpha値更新（REQ-015）

    SES->>DS: notifyFilterChanged(filteredIndices)
    alt filteredCount > threshold × 1.2 (大幅変化)
        DS->>WL: downsampleSmart(10000, true, filteredIndices)
        DS->>WL: downsampleForThumbnail(500, filteredIndices)
        Note over DS: フィルタ後インデックスに対して再ダウンサンプリング
        WL-->>DS: 更新済みインデックス
        DS-->>DS: キャッシュ更新
    else 小幅変化
        Note over DS: 既存キャッシュを維持（パフォーマンス優先）
    end
```

**備考**: フィルタ変化量が閾値（±20%）以内の場合は再計算せず既存キャッシュを使用する。これはフィルタのスライダー操作中のリアルタイム応答性を維持するため。

---

### フロー3: ダウンサンプリング処理内部フロー（WASM） 🔵

**信頼性**: 🔵 *REQ-063 / wasm-api.md `downsample_for_thumbnail` 定義より*

**`downsample_smart` 処理フロー（Rust）**:

```mermaid
flowchart TD
    A[downsample_smart 呼び出し\nmax_points, include_pareto] --> B{N ≤ max_points?}
    B -->|Yes| C[全インデックスを返す]
    B -->|No| D{include_pareto?}
    D -->|Yes| E[Pareto Rank1 インデックスを取得]
    E --> F{pareto_count ≤ max_points?}
    F -->|No| G[Pareto点のみから\nmax_points 点を返す]
    F -->|Yes| H[残り予算 = max_points - pareto_count]
    D -->|No| H2[残り予算 = max_points]
    H --> I[非Pareto点からランダムサンプリング\n残り予算分]
    H2 --> I
    I --> J[Pareto + サンプル 結合]
    J --> K[DownsampleResult返却]
```

**`downsample_for_thumbnail` 処理フロー（Rust・REQ-063準拠）**:

```mermaid
flowchart TD
    A[downsample_for_thumbnail\nmax_points] --> B[Pareto Rank1 インデックス取得]
    B --> C{pareto_count ≤ max_points/2?}
    C -->|Yes| D[全Pareto点確保]
    C -->|No| E[Pareto点からmax_points/2点をランダム確保]
    D --> F[残り予算 = max_points - pareto_count]
    E --> F
    F --> G[目的変数空間を\nsqrt残予算 × sqrt残予算 グリッドに分割]
    G --> H[各グリッドセルから1点をランダム選択]
    H --> I[Pareto + グリッドサンプル 結合]
    I --> J[DownsampleResult 返却]
```

---

### フロー4: チャートコンポーネントのインデックス取得 🔵

**信頼性**: 🔵 *既存チャートパターン / ユーザヒアリングより*

```mermaid
sequenceDiagram
    participant C as チャートコンポーネント
    participant DS as downsampleStore
    participant G as GPUバッファ (Float32Array)

    C->>DS: getIndices('scatter')
    DS-->>C: Uint32Array (≤10,000)
    C->>G: positions[indices] でデータ抽出
    Note over C: deck.gl ScatterplotLayer に\n絞り込み済み positions を渡す
    C-->>C: renderFrame()
```

---

## データ変換パターン

### ダウンサンプリング前後のデータ型 🔵

**信頼性**: 🔵 *wasm-api.md / 既存 GPUバッファ設計より*

```
WASM DataFrame（全 N 点）
    ↓ downsample_smart / downsample_for_thumbnail
DownsampleResult.indices: Uint32Array（サイズ ≤ max_points）
    ↓ JS Bridge（wasmLoader）
downsampleStore.cache[key]: Uint32Array
    ↓ getIndices(key)
チャートコンポーネント
    ↓ GPU positions/colors バッファのインデックス参照
WebGL 描画（deck.gl / Canvas）
```

### フィルタとダウンサンプリングの合成 🔵

**信頼性**: 🔵 *REQ-015 / 既存 alpha 値更新パターンより*

```
全試行インデックス N
├── ダウンサンプリング → renderIndices (≤ max_points)
│       ↓ 描画する点の集合
└── フィルタリング → filteredAlpha (N 長 Uint8Array)
        ↓ alpha=0 で非表示

最終表示 = renderIndices ∩ filteredAlpha（alpha > 0）
（alpha 値の更新は既存 REQ-015 のまま。描画自体は renderIndices に絞る）
```

---

## エラーハンドリングフロー 🟡

**信頼性**: 🟡 *既存 analysisStore・clusterStore エラーパターンから妥当な推測*

```mermaid
flowchart TD
    A[downsampleStore.recompute] --> B[WASM 呼び出し]
    B --> C{エラー?}
    C -->|Yes| D[downsampleError にセット]
    D --> E[全チャートが全インデックスにフォールバック]
    Note1["パフォーマンスは低下するが\n描画機能は維持される"]
    C -->|No| F[キャッシュ正常更新]
```

---

## 性能目標サマリー 🔵

**信頼性**: 🔵 *NFR-001・NFR-002 / wasm-api.md 性能目標より*

| 処理 | 目標時間 | 備考 |
|---|---|---|
| `downsample_smart` (50,000 点) | < 5ms | filter_by_ranges と同等 |
| `downsample_for_thumbnail` (50,000 点) | < 5ms | グリッド計算込み |
| Study 選択時 6 種一括計算 | < 20ms | compute_pareto_ranks 後 |
| フィルタ変更時の再計算 | < 5ms | フィルタ後 N 点に対して |
| チャートコンポーネント取得 | < 1ms | キャッシュ参照のみ |

---

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **型定義**: [interfaces.ts](interfaces.ts)
- **既存 WASM API**: [wasm-api.md](../tunny-dashboard/wasm-api.md)
- **ヒアリング記録**: [design-interview.md](design-interview.md)

---

## 信頼性レベルサマリー

- 🔵 青信号: 7件 (88%)
- 🟡 黄信号: 1件 (12%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
