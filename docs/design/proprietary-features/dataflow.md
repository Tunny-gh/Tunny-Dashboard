# プロプライエタリな汎用最適化ソフト機能拡充 データフロー図

**作成日**: 2026-04-04
**関連アーキテクチャ**: [architecture.md](architecture.md)
**関連要件定義**: （作成予定）

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実なフロー
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測によるフロー
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測によるフロー

---

## システム全体のデータフロー 🔵

**信頼性**: 🔵 *既存アーキテクチャ・ユーザヒアリングより*

```mermaid
flowchart TD
    U[ユーザー] --> FLC[FreeLayoutCanvas]
    FLC -->|case 'surface3d'| S3D[SurfacePlot3D]
    FLC -->|case 'topsis-ranking'| TRC[TopsisRankingChart]

    S3D --> AS[analysisStore（拡張）]
    TRC --> MS[mcdmStore（新設）]

    AS --> WL[WasmLoader]
    MS --> WL
    WL --> WASM[(tunny_core.wasm)]

    WASM -->|Pdp2dResult| WL
    WASM -->|TopsisResult| WL
    WL --> AS
    WL --> MS
    AS --> S3D
    MS --> TRC
    TRC -->|setHighlight| SEL[selectionStore]
    SEL -->|ハイライト連動| FLC
```

---

## 機能1: 3D応答曲面プロット (surface3d) 🔵

**信頼性**: 🔵 *ユーザヒアリング・既存pdp.rs・wasm-api.md より*

**関連要件**: （設計ヒアリングQ2-Q3より）

```mermaid
sequenceDiagram
    participant U as ユーザー
    participant C as SurfacePlot3D
    participant A as analysisStore
    participant W as WasmLoader
    participant R as tunny_core.wasm

    U->>C: パラメータ1/2・目的選択 or モデル種別変更
    C->>A: computeSurface3d(param1, param2, obj)
    A->>A: cacheKey確認（surface3dCache）
    alt キャッシュヒット
        A-->>C: キャッシュデータ返却
    else キャッシュミス
        A->>A: isComputingSurface = true
        A->>W: wasm.computePdp2d(p1, p2, obj, 50)
        W->>R: wasm_compute_pdp_2d(p1, p2, obj, 50)
        R-->>W: Pdp2dResult { grid1, grid2, heatmap }
        W-->>A: Surface3DResult
        A->>A: surface3dCache.set(key, result)
        A->>A: isComputingSurface = false
        A-->>C: Surface3DResult
    end
    C->>C: deck.gl GridLayer にグリッドデータを渡す
    C-->>U: 3D曲面レンダリング
```

**詳細ステップ:**
1. `SurfacePlot3D` コンポーネントはドロップダウンからパラメータ1・パラメータ2・目的関数を選択させる
2. 選択変更時に `analysisStore.computeSurface3d()` を呼び出す
3. `analysisStore` は `surface3dCache` でキャッシュ確認し、ミス時のみWASM計算
4. `wasmLoader.computePdp2d()` → `wasm_compute_pdp_2d()` でRidgeモデルの50×50グリッドを計算
5. 結果の `heatmap` フラット配列をdeck.gl `GridLayer` の高さ属性として渡す
6. X/Y軸に `grid1`・`grid2` のスケールを適用し、Z値で色分けとバー高さを表現

**モデル種別切り替えフロー:**
```mermaid
flowchart TD
    M{surrogateModelType} -->|ridge| R1[computePdp2d via Ridge回帰]
    M -->|random_forest| R2[computePdp2d via ONNXWorker]
    M -->|kriging| R3[disabled - 将来実装]
    R1 --> G[GridLayer描画]
    R2 --> G
    R3 -->|表示なし| E[EmptyState 将来対応予定]
```

---

## 機能2: TOPSISランキング (topsis-ranking) 🔵

**信頼性**: 🔵 *ユーザヒアリング・TOPSIS理論より*

**関連要件**: （設計ヒアリングQ4より）

```mermaid
sequenceDiagram
    participant U as ユーザー
    participant T as TopsisRankingChart
    participant M as mcdmStore
    participant W as WasmLoader
    participant R as tunny_core.wasm
    participant S as selectionStore

    Note over U,R: 初回表示または重み変更時
    U->>T: 重みスライダー操作
    T->>M: setTopsisWeights(newWeights)
    M->>M: 重みを合計1.0に正規化
    T->>M: computeTopsis()
    M->>M: isComputing = true
    M->>W: wasm.computeTopsis(weights, isMinimize)
    W->>R: wasm_compute_topsis(weights, is_minimize)
    R->>R: 正規化 → 重み付き → 理想解計算 → 距離計算 → スコア算出
    R-->>W: TopsisResult { scores, rankedIndices, positiveIdeal, negativeIdeal }
    W-->>M: TopsisRankingResult
    M->>M: topsisResult = result, isComputing = false
    M-->>T: topsisResult

    T->>T: ECharts BarChart レンダリング（上位N件ハイライト）

    Note over U,S: 棒クリック時
    U->>T: バー（試行）クリック
    T->>S: selectionStore.setHighlight(trialIndex)
    S-->>T: highlight更新
```

**TOPSIS計算ステップ（Rust実装詳細）:**
```mermaid
flowchart TD
    A[目的関数値行列 N×M] --> B[ベクトル正規化]
    B --> C[重み付き正規化行列]
    C --> D{方向確認 is_minimize}
    D -->|minimize| E[正理想解=列最小値, 負理想解=列最大値]
    D -->|maximize| F[正理想解=列最大値, 負理想解=列最小値]
    E --> G[各試行の正理想解へのユークリッド距離 D+]
    F --> G
    E --> H[各試行の負理想解へのユークリッド距離 D-]
    F --> H
    G --> I[スコア = D- / (D+ + D-)]
    H --> I
    I --> J[スコア降順ソート]
    J --> K[TopsisResult]
```

---

## データ処理パターン

### キャッシュ戦略 (surface3d) 🔵

**信頼性**: 🔵 *既存pdpCache実装パターンより*

```typescript
// cacheKey = `${surrogateModelType}_${param1}_${param2}_${objective}_${nGrid}`
surface3dCache: Map<string, Surface3DResult>
```

- Studyが変更された際はキャッシュをクリア（`studyStore.subscribe` で検知）
- 同一パラメータ組み合わせの再計算を防ぐ
- メモリ上限: 最大20エントリ（LRU方式）🟡

### Study変更時のリセット 🔵

**信頼性**: 🔵 *既存analysisStore・clusterStoreパターンより*

```mermaid
flowchart LR
    SS[studyStore] -->|subscribe| AS[analysisStore]
    SS -->|subscribe| MS[mcdmStore]
    AS -->|clearSurface3dCache()| A2[surface3dCache = new Map()]
    MS -->|reset()| M2[topsisResult = null]
```

---

## エラーハンドリングフロー 🔵

**信頼性**: 🔵 *既存clusterStore・analysisStoreエラーパターンより*

```mermaid
flowchart TD
    A[WASM呼び出し] -->|try/catch| B{エラー発生?}
    B -->|No| C[結果をStoreに格納]
    B -->|Yes| D[console.error出力]
    D --> E[surface3dError / topsisError をStoreにセット]
    E --> F[EmptyState コンポーネント表示]
    C --> G[チャート描画]
```

**エラーケース:**
- `EmptyState` message="曲面計算エラー" — computePdp2d失敗時
- `EmptyState` message="ランキング計算エラー" — computeTopsis失敗時
- `EmptyState` message="目的関数が2つ以上必要です" — 単目的Studyでtopsis-ranking表示時
- `EmptyState` message="パラメータが2つ以上必要です" — surface3dでパラメータが1個以下の時

---

## 状態管理フロー

### SurfacePlot3D の状態管理 🔵

**信頼性**: 🔵 *ユーザヒアリング・既存コンポーネントパターンより*

```mermaid
stateDiagram-v2
    [*] --> 初期状態
    初期状態 --> パラメータ選択待ち: コンポーネントマウント
    パラメータ選択待ち --> 計算中: param1/param2/objective選択完了
    計算中 --> キャッシュヒット: surface3dCache確認
    計算中 --> WASM計算中: キャッシュミス
    WASM計算中 --> 描画完了: 計算成功
    WASM計算中 --> エラー: 計算失敗
    キャッシュヒット --> 描画完了: キャッシュデータ使用
    描画完了 --> 計算中: パラメータ変更 or モデル変更
    エラー --> パラメータ選択待ち: リトライ
```

### TopsisRankingChart の状態管理 🔵

**信頼性**: 🔵 *ユーザヒアリング・既存コンポーネントパターンより*

```mermaid
stateDiagram-v2
    [*] --> 初期状態
    初期状態 --> 重み設定中: コンポーネントマウント（デフォルト均等重み）
    重み設定中 --> 計算中: computeTopsis()呼び出し
    計算中 --> 描画完了: 計算成功
    計算中 --> エラー: 計算失敗
    描画完了 --> 計算中: 重み変更
    描画完了 --> ハイライト更新: バークリック
    ハイライト更新 --> 描画完了: selectionStore更新完了
    エラー --> 重み設定中: リトライ
```

---

## 関連文書

- **アーキテクチャ**: [architecture.md](architecture.md)
- **型定義**: [interfaces.ts](interfaces.ts)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **WASM API仕様**: [../tunny-dashboard/wasm-api.md](../tunny-dashboard/wasm-api.md)

## 信頼性レベルサマリー

- 🔵 青信号: 10件 (91%)
- 🟡 黄信号: 1件 (9%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
