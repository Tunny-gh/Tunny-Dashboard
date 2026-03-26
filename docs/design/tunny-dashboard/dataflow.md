# Tunny Dashboard データフロー設計

## 1. ファイル読み込み → DataFrame構築 → GPUバッファ初期化

```mermaid
flowchart TD
    A[ユーザー: ファイル選択] --> B[File API / FSAPI]
    B --> C[FileReader.readAsText]
    C --> D[WASM: parse_journal]
    D --> E{Study選択ダイアログ}
    E --> F[WASM: build_dataframe\n選択StudyのDataFrameをWASMメモリに構築]
    F --> G[WASM: compute_pareto_ranks\nNDSort → Paretoランク列追加]
    G --> H[JS: initGpuBuffers\npositions / colors / sizes を Float32Array初期化]
    H --> I[JS: deck.gl / regl へGPUバッファを渡す]
    I --> J[初期レンダリング完了]

    F --> K[WebWorker×4: ScatterMatrix\nMode2優先 120セル → 1秒以内]
    F --> L[WASM: compute_spearman\n感度ヒートマップ → 500ms以内]
```

**時間目標:**
- Journalパース（50,000行）: 5秒以内
- GPU初期化: 100ms以内
- Scatter Matrix Mode2（120セル）: パース完了後1秒以内

---

## 2. Brushing & Linking イベント伝播（クリティカルパス）

```mermaid
flowchart TD
    subgraph "ユーザー操作 (いずれか)"
        A1[PCPブラシドラッグ\nAxis Filter]
        A2[グラフ上ドラッグ\nBrush Selection]
        A3[テーブル行クリック\nClick Highlight]
    end

    A1 --> B[SelectionStore: addAxisFilter\nfilterRanges 更新]
    A2 --> C[SelectionStore: brushSelect\nselectedIndices 直接更新]
    A3 --> D[SelectionStore: setHighlight\nhighlighted 更新]

    B --> E[WASM: filter_by_ranges\n< 5ms @ 50,000点]
    E --> F[selectedIndices: Uint32Array 更新\nSharedArrayBuffer経由]

    F --> G[GPU alpha値のみ更新\n< 1ms / グラフ]
    C --> G
    D --> H[ハイライト点の color 更新]

    G --> I[requestAnimationFrame\n次フレームで全グラフ描画]
    H --> I

    F --> J[Left Panel: カウンタ更新\nselected: N件]
    F --> K[Bottom Table: 仮想スクロール更新\n上位100件のみ即時表示]
```

**重要な設計原則:**
- グラフコンポーネントはReactのre-renderを経由しない（`subscribe` → GPU直接更新）
- `positions` / `sizes` は変更しない（GPUコピー不要）
- `colors[i*4+3]`（alpha値）のみ書き換える

---

## 3. ライブ更新 差分フロー（FSAPI）

```mermaid
sequenceDiagram
    participant Timer as setInterval (N秒)
    participant FSAPI as File System Access API
    participant WASM as WASM Core (Rust)
    participant Store as Zustand Store
    participant Graph as グラフコンポーネント

    Timer->>FSAPI: ファイルサイズ確認
    FSAPI-->>Timer: size = S1

    alt S1 == S0 (変更なし)
        Timer->>Timer: スキップ
    else S1 > S0 (新規追加あり)
        Timer->>FSAPI: バイト範囲 S0〜S1 を読み込み
        FSAPI-->>WASM: 差分テキスト（新規行のみ）
        WASM->>WASM: 差分JSONLパース
        note over WASM: 不完全な最終行はスキップ\nRUNNING試行は保留リストへ
        WASM->>WASM: COMPLETE試行をDataFrameに追記
        WASM->>WASM: Pareto差分更新（NDSort）
        WASM-->>Store: paretoUpdated, newTrialIndices, trialCount
        Store->>Graph: History/HV → 末尾に点を追記
        Store->>Graph: Pareto図 → paretoUpdated時のみ更新
        note over Graph: Brushing選択・フィルタ・視点は変更しない
        Timer->>Timer: S0 = S1 に更新
    end
```

**更新戦略（グラフ別）:**

| グラフ | 更新タイミング | 更新方法 |
|---|---|---|
| History / Best値推移 | 毎回 | 末尾に点を追記 |
| Hypervolume推移 | Pareto更新時のみ | 末尾に点を追記 |
| Paretoフロント | Pareto解が変化した時のみ | GPUバッファ全更新 |
| Parallel Coordinates | 毎回 | 新規点のみGPUバッファに追記 |
| Scatter Matrix | 毎回 | サムネイルのみ再生成 |
| 感度ヒートマップ | 試行数が10%増加したら | WASM再計算 + 再描画 |
| クラスタリング結果 | 自動更新しない | 手動ボタンで再実行 |

---

## 4. クラスタリング実行フロー

```mermaid
flowchart LR
    A[ユーザー: クラスタリング設定\n対象空間・アルゴリズム・k] --> B[WASM: run_pca\n30〜34次元 → 5次元 ~50ms]
    B --> C[WASM: run_kmeans\nLloyd's algorithm ~200ms]
    C --> D[WASM: compute_cluster_stats\nmean / std / t検定 ~150ms]
    D --> E[ClusterStore: clusterLabels 更新]
    E --> F[GPUバッファ: colors をクラスタ色で更新]
    F --> G[全グラフ: クラスタ色で再描画]
    E --> H[クラスタ一覧パネル: 特徴サマリー表示]

    A --> I{UMAPオプション有効?}
    I -->|Yes| J[UMAPWorker: umap-js 非同期 5〜30秒]
    J --> K[UMAPWorker完了後: UMAP 2D ビュー有効化]
```

---

## 5. HTMLレポート生成フロー

```mermaid
flowchart TD
    A[ユーザー: レポートビルダー\nセクション選択・基本情報入力] --> B[WASM: 対象データ絞り込み\nPareto解 + クラスタ代表点 ~100ms]
    B --> C[WASM: 統計サマリー計算]
    C --> D[JS: 各図をPlotly.jsで再描画\n→ JSON化 ~500ms]
    D --> E[JS: HTMLテンプレートに\nデータ・図を埋め込み]
    E --> F[Blob生成 → a download トリガー]
    F --> G[report.html ダウンロード\n5〜15MB 目標]
```

---

## 6. PDP計算フロー（モデル別）

```mermaid
flowchart TD
    A[ユーザー: 変数・目的を選択] --> B{.onnxファイル読み込み済み?}

    B -->|No 簡易版| C[WASM: Ridge回帰モデル\nグリッド点生成 50点\nダウンサンプリング 500サンプル]
    C --> D[WASM: 予測 25,000行 ~20ms]

    B -->|Yes 高精度版| E[ONNXWorker: .onnxロード\n~500ms 初回のみ]
    E --> F[ONNXWorker: バッチ推論 ~160ms]

    D --> G[WASM: グリッド点ごとに平均化\n→ PDP曲線 50点 + 95%CI]
    F --> G
    G --> H[ECharts: PDP + ICE + rugプロット描画]
```

---

## 7. 感度分析 計算フロー（3層）

```mermaid
flowchart TD
    A[分析開始] --> B[WASM Layer1-A: Spearman相関\n30変数 × 4目的 ~500ms]
    A --> C[WASM Layer1-B: 標準化偏回帰係数β\nRidge回帰 ~300ms]
    A --> D[WASM Layer1-C: R²適合度\n~100ms]
    A --> E[MICWorker: MIC計算\n10,000点ダウンサンプリング ~5秒]

    B & C & D --> F[ECharts: 感度ヒートマップ\n30行×4列]
    E --> F

    G{.onnxあり?} -->|Yes| H[metadata.json: RF重要度を即時読み込み]
    H --> F

    I{shap_values.jsonあり?} -->|Yes| J[JS: SHAP Summary Plot描画]
    J --> F
```
