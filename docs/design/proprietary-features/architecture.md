# プロプライエタリな汎用最適化ソフト機能拡充 アーキテクチャ設計

**作成日**: 2026-04-04
**関連要件定義**: なし（本設計がベース。要件定義書は別途 `docs/spec/mode-frontier-features/requirements.md` として作成予定）
**ヒアリング記録**: [design-interview.md](design-interview.md)

**【信頼性レベル凡例】**:
- 🔵 **青信号**: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な設計
- 🟡 **黄信号**: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による設計
- 🔴 **赤信号**: EARS要件定義書・設計文書・ユーザヒアリングにない推測による設計

---

## システム概要 🔵

**信頼性**: 🔵 *ユーザヒアリングより*

プロプライエタリな汎用最適化ソフトが提供する分析機能のうち、以下2機能をTunny Dashboardに追加する：

1. **サロゲートモデル可視化（3D応答曲面プロット）** — モデル種別（Ridge/RF）をUI上で選択し、任意の2パラメータ×1目的の3D曲面をdeck.glで描画
2. **TOPSIS多基準意思決定** — 全試行をTOPSIS（Technique for Order Preference by Similarity to Ideal Solution）でランキングし、最良解を明示

既存の5層アーキテクチャ（WASM → TypeScript型宣言 → WasmLoader → Zustand Store → React Component）を踏襲し、新ChartId 2種を追加する。サーバー不要・ブラウザ完結の制約は維持する。

## アーキテクチャパターン 🔵

**信頼性**: 🔵 *既存 tunny-dashboard アーキテクチャ設計・visualization-enablement 設計より*

- **パターン**: 5層レイヤードアーキテクチャ（既存パターン踏襲）
- **選択理由**: `computeHvHistory`・`computeSensitivity` などの先行実装が同一フローで完成しており、開発コストが低く整合性が高い

```
WASM (rust_core)
  ↓ wasm-bindgen
tunny_core.d.ts （型宣言手動追加）
  ↓
wasmLoader.ts （バインディング追加）
  ↓
Zustand Store （mcdmStore / analysisStore拡張）
  ↓
React Component （SurfacePlot3D / TopsisRankingChart）
  ↓
FreeLayoutCanvas ChartContent switch 追加
```

## コンポーネント構成

### 層 1: WASM Core（Rust） 🔵

**信頼性**: 🔵 *wasm-api.md・既存pdp.rs・ユーザヒアリングより*

#### 既存転用（変更なし）

| JS名 | Rust実装ファイル | 用途 |
|---|---|---|
| `computePdp2d` | `pdp.rs::compute_pdp_2d()` | 3D曲面のグリッドデータ生成（Ridge回帰、50×50グリッド） |

`compute_pdp_2d()` は2変数PDP（n_grid × n_grid のヒートマップ）を返す。3D曲面プロットのZ値として転用する。

#### 新規追加（Rust実装必要）

| JS名 | Rust関数名 | 新規ファイル | 概要 |
|---|---|---|---|
| `computeTopsis` | `wasm_compute_topsis` | `topsis.rs` | TOPSIS法で全試行をスコアリング・ランキング |
| `computePdp2d` | `wasm_compute_pdp_2d` | `pdp.rs`（追加） | 2変数PDP — wasm_bindgenエクスポートが未実装なら追加 |

**TOPSIS アルゴリズム概要（Rust実装）:**
```
1. 目的関数値行列を正規化（ベクトル正規化）
2. 重み付き正規化行列を計算
3. 正理想解（各目的のベスト値）と負理想解（ワースト値）を決定
4. 各試行から正・負理想解へのユークリッド距離を計算
5. スコア = D⁻ / (D⁺ + D⁻)（0〜1、高い方がベター）
6. スコア降順でランキング
```

**パフォーマンス目標:** 50,000点 × 4目的で 100ms 以内

### 層 2: TypeScript型宣言（tunny_core.d.ts） 🔵

**信頼性**: 🔵 *既存 tunny_core.d.ts パターン・ユーザヒアリングより*

追加するシグネチャ:

```typescript
export interface Pdp2dResult {
  grid1: Float64Array;   // param1のグリッド点（n_grid点）
  grid2: Float64Array;   // param2のグリッド点（n_grid点）
  heatmap: Float64Array; // [n_grid × n_grid] flattened（行major）
  durationMs: number;
}

export interface TopsisResult {
  scores: Float64Array;       // 各試行のTOPSISスコア（0〜1）
  rankedIndices: Uint32Array; // スコア降順の試行インデックス
  positiveIdeal: Float64Array; // 正理想解（目的数次元）
  negativeIdeal: Float64Array; // 負理想解（目的数次元）
  durationMs: number;
}

export function computePdp2d(
  param1: string,
  param2: string,
  objective: string,
  n_grid: number
): Pdp2dResult;

export function computeTopsis(
  weights: Float64Array,
  is_minimize: boolean[]
): TopsisResult;
```

### 層 3: WasmLoader（wasmLoader.ts） 🔵

**信頼性**: 🔵 *既存wasmLoader.tsパターン・ユーザヒアリングより*

既存 `WasmLoader` クラスに以下を追加:

```typescript
computePdp2d: (
  param1: string,
  param2: string,
  objective: string,
  nGrid: number
) => Pdp2dResult

computeTopsis: (
  weights: Float64Array,
  isMinimize: boolean[]
) => TopsisResult
```

### 層 4: Zustand Store 🔵

**信頼性**: 🔵 *既存 analysisStore・clusterStore パターン・ユーザヒアリングより*

#### 4a: AnalysisStore拡張（`analysisStore.ts`）

サロゲートモデル選択状態を `AnalysisStore` に追加する:

```typescript
// 追加するState
surrogateModelType: SurrogateModelType  // 選択中のモデル種別
surface3dCache: Map<string, Surface3DResult>  // key = `${p1}_${p2}_${obj}`
isComputingSurface: boolean

// 追加するAction
setSurrogateModelType: (type: SurrogateModelType) => void
computeSurface3d: (param1: string, param2: string, objective: string) => Promise<void>
```

```typescript
export type SurrogateModelType = 'ridge' | 'random_forest' | 'kriging'
// ridge: 常時利用可能（WASM）
// random_forest: .onnxファイル読み込み後に利用可能
// kriging: 将来実装（現時点では disabled）

export interface Surface3DResult {
  param1: string
  param2: string
  objective: string
  grid1: number[]   // param1のグリッド点
  grid2: number[]   // param2のグリッド点
  heatmap: number[] // [n_grid × n_grid] flattened Z値
  modelType: SurrogateModelType
  durationMs: number
}
```

#### 4b: McdmStore（新設、`mcdmStore.ts`）

TOPSIS状態管理:

```typescript
export interface McdmStore {
  // State
  topsisWeights: number[]         // 目的数分の重み（合計1.0に正規化）
  topsisResult: TopsisRankingResult | null
  isComputing: boolean
  topN: number                    // 上位N件を強調表示（デフォルト10）

  // Actions
  setTopsisWeights: (weights: number[]) => void
  computeTopsis: () => Promise<void>
  setTopN: (n: number) => void
}

export interface TopsisRankingResult {
  scores: number[]          // 全試行のスコア（trial順）
  rankedIndices: number[]   // スコア降順のインデックス
  positiveIdeal: number[]   // 正理想解
  negativeIdeal: number[]   // 負理想解
  durationMs: number
}
```

### 層 5: React コンポーネント 🔵

**信頼性**: 🔵 *ユーザヒアリング・既存コンポーネントパターンより*

#### SurfacePlot3D（新規）

- **ファイル**: `frontend/src/components/charts/SurfacePlot3D.tsx`
- **ChartId**: `'surface3d'`
- **描画技術**: deck.gl `GridLayer`（グリッドセルのZ値でカラーマップ）
- **ユーザー操作**:
  - X軸パラメータ選択（`currentStudy.paramNames` から）
  - Y軸パラメータ選択（同上）
  - 目的関数選択（`currentStudy.objectiveNames` から）
  - モデル種別選択ドロップダウン（Ridge / Random Forest / Kriging）
- **依存ストア**: `analysisStore`（surface3dCache, surrogateModelType）, `studyStore`

#### TopsisRankingChart（新規）

- **ファイル**: `frontend/src/components/charts/TopsisRankingChart.tsx`
- **ChartId**: `'topsis-ranking'`
- **描画技術**: ECharts（縦棒グラフ or 横棒グラフ、上位N件をハイライト）
- **ユーザー操作**:
  - 各目的の重みスライダー（合計を常に1.0に正規化）
  - 上位N件表示件数（5/10/20/全件）
  - 棒クリックで全グラフハイライト連動（`selectionStore.setHighlight`）
- **依存ストア**: `mcdmStore`, `studyStore`, `selectionStore`

---

## システム構成図 🔵

**信頼性**: 🔵 *ユーザヒアリング・既存アーキテクチャより*

```
FreeLayoutCanvas.tsx (Mode D)
│
├── case 'surface3d'
│     └── <SurfacePlot3D />
│           ├── Model selector UI (Ridge / RF / Kriging)
│           ├── Axis selector UI (param1, param2, objective)
│           └── deck.gl GridLayer ← analysisStore.surface3dCache
│
└── case 'topsis-ranking'
      └── <TopsisRankingChart />
            ├── Weight sliders (目的数 × スライダー)
            ├── Top-N selector
            └── ECharts BarChart ← mcdmStore.topsisResult

                    ↑                              ↑
              analysisStore (拡張)           mcdmStore (新設)
                    │                              │
                    └──────────────────────────────┘
                              WasmLoader.getInstance()
                                         │
                            ┌────────────────────────────┐
                            │      tunny_core.wasm        │
                            │  ├── computePdp2d (既存転用)│
                            │  └── computeTopsis (新規)   │
                            └────────────────────────────┘
```

---

## ディレクトリ構造 🔵

**信頼性**: 🔵 *既存プロジェクト構造より*

**変更ファイル:**
```
rust_core/src/
├── lib.rs              ← computePdp2d / computeTopsis wasm_bindgen追加
├── pdp.rs              ← compute_pdp_2d() が既実装か確認・なければ追加
└── topsis.rs           ← 新規: TOPSISアルゴリズム実装

frontend/src/
├── wasm/
│   ├── pkg/
│   │   └── tunny_core.d.ts  ← Pdp2dResult, TopsisResult, 関数シグネチャ追加
│   └── wasmLoader.ts         ← computePdp2d, computeTopsis バインド追加
├── stores/
│   ├── analysisStore.ts      ← surrogateModelType, surface3dCache 追加
│   └── mcdmStore.ts          ← 新規作成
├── components/charts/
│   ├── SurfacePlot3D.tsx     ← 新規作成
│   └── TopsisRankingChart.tsx← 新規作成
├── components/layout/
│   └── FreeLayoutCanvas.tsx  ← case 'surface3d', case 'topsis-ranking' 追加
└── types/index.ts            ← ChartId union, SurrogateModelType, 型定義追加
```

---

## 非機能要件の実現方法

### パフォーマンス 🔵

**信頼性**: 🔵 *既存NFRと同等基準・ユーザヒアリングより*

- `compute_pdp_2d()`: 50×50グリッド、Ridge回帰、100ms以内（既存仕様より）
- `compute_topsis()`: 50,000点×4目的で100ms以内（Rust実装）
- `SurfacePlot3D`の初回描画: WASM計算 + deck.gl描画 合計1,500ms以内
- ローディングスピナーをWASM計算中に表示

### セキュリティ 🔵

**信頼性**: 🔵 *既存NFR-020・021より*

- ファイルデータはブラウザ外に送信しない（完全ローカル処理）
- 既存制約を変更しない

### スタイル制約 🔵

**信頼性**: 🔵 *既存NFR-201（Tailwind禁止）より*

- 新規コンポーネントはすべてインラインスタイルまたはCSS変数のみ使用
- Tailwind CSS クラスは使用禁止

### エラー耐性 🔵

**信頼性**: 🔵 *既存エラーハンドリングパターンより*

- WASM計算エラーはtry/catchで捕捉し、`EmptyState`コンポーネントで表示
- RF（.onnx）未読み込み時はモデル選択UIでdisabledを表示
- Kriging未実装時は「将来対応予定」ラベルでdisabled表示

---

## 技術的制約

### 3D描画技術について 🔵

**信頼性**: 🔵 *ユーザヒアリング・既存技術スタックより*

- deck.gl `GridLayer` はZ値の高さ表現（棒グラフ形式）が可能
- 滑らかな3Dサーフェス（メッシュ）にはdeck.glのnativeサポートが限定的なため、`GridLayer`で50×50グリッドセルを棒グラフ状に表現する
- 将来的に滑らかなサーフェスが必要な場合はThree.jsへの切り替えを検討

### computePdp2d既存実装確認 🟡

**信頼性**: 🟡 *wasm-api.md仕様より（WASM bindgen公開は未確認）*

- `compute_pdp_2d()` はwasm-api.mdに仕様定義済み
- `wasmLoader.ts` に `computePdp2d` バインドが存在するか実装時に確認必要
- 未公開の場合は `lib.rs` にwasm_bindgenエクスポートを追加

### TOPSIS重みの初期値 🟡

**信頼性**: 🟡 *プロプライエタリな汎用最適化ソフト参考・ユーザヒアリングから妥当な推測*

- 初期値は全目的に均等配分（1/n ずつ）
- ユーザーがスライダーで調整可能
- 重みの合計が常に1.0になるよう正規化

---

## 実装フェーズ案 🟡

**信頼性**: 🟡 *ユーザヒアリングと既存実装パターンから妥当な推測*

| Phase | 内容 |
|---|---|
| Phase 1 | `topsis.rs` Rust実装 + `computeTopsis` WASM公開 + `mcdmStore` + `TopsisRankingChart` |
| Phase 2 | `computePdp2d` WASM公開確認・追加 + `AnalysisStore`拡張 + `SurfacePlot3D` |
| Phase 3 | モデル選択UI（RF/.onnx連携、Kriging将来対応stub） |

---

## 関連文書

- **データフロー**: [dataflow.md](dataflow.md)
- **型定義**: [interfaces.ts](interfaces.ts)
- **ヒアリング記録**: [design-interview.md](design-interview.md)
- **既存アーキテクチャ**: [../tunny-dashboard/architecture.md](../tunny-dashboard/architecture.md)
- **WASM API仕様**: [../tunny-dashboard/wasm-api.md](../tunny-dashboard/wasm-api.md)
- **可視化有効化設計**: [../visualization-enablement/architecture.md](../visualization-enablement/architecture.md)

## 信頼性レベルサマリー

- 🔵 青信号: 14件 (87%)
- 🟡 黄信号: 2件 (13%)
- 🔴 赤信号: 0件 (0%)

**品質評価**: 高品質
