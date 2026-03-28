# TASK-302 要件定義書: Zustand Store群の実装

## 1. 機能の概要

- 🟢 `useSelectionStore` — Brushing & Linking の中核（selectedIndices・filterRanges・colorMode）
- 🟢 `useStudyStore` — Journal読み込み・Study選択・DataFrame管理
- 🟢 `useLayoutStore` — レイアウトモード・表示チャート・パネルサイズ管理
- 🟡 `useClusterStore` / `useAnalysisStore` / `useExportStore` / `useLiveUpdateStore` — スタブ実装（後続TASKで完成）
- 🟢 **想定ユーザー**: TASK-401〜TASK-601 の UIコンポーネントが各Storeを参照・更新する
- 🟢 **システム内位置づけ**: TASK-301（WASMローダー）の後段。TASK-401（UIシェル）より前

**参照した EARS 要件**: REQ-040〜REQ-045
**参照した設計文書**: dataflow.md § Brushing & Linking、types/index.ts

---

## 2. 入出力仕様

### `useSelectionStore` 🟢

Zustand store。外部 subscribe で GpuBuffer の alpha を直接更新する（Reactサイクル外）。

```typescript
// State
selectedIndices: Uint32Array     // 現在選択中の trial インデックス
filterRanges: Record<string, Range>   // 軸名 → {min, max}
highlighted: number | null       // ハイライト中 trial インデックス
colorMode: ColorMode             // 'objective' | 'cluster' | 'rank' | 'generation'

// Internal state（公開インターフェース外）
_trialCount: number              // clearSelection で全インデックス生成に使用

// Actions
brushSelect(indices: Uint32Array): void
addAxisFilter(axis: string, min: number, max: number): void
removeAxisFilter(axis: string): void
clearSelection(): void
setHighlight(index: number | null): void
setColorMode(mode: ColorMode): void
```

**addAxisFilter / removeAxisFilter の動作**:
1. filterRanges を同期的に更新
2. `WasmLoader.getInstance()` → `filterByRanges(rangesJson)` を非同期呼び出し（Promise）
3. 結果の `Uint32Array` を `selectedIndices` に set
4. WASM 未初期化（reject）の場合は filterRanges のみ更新し selectedIndices は変更しない

**clearSelection の動作**:
1. `_trialCount` から全インデックス配列 `[0, 1, ..., N-1]` を生成
2. `selectedIndices` に set
3. `filterRanges` を空 `{}` に reset

### `useStudyStore` 🟢

```typescript
// State
currentStudy: Study | null
allStudies: Study[]
studyMode: StudyMode             // 'single-objective' | 'multi-objective'
isLoading: boolean
loadError: string | null

// Actions
loadJournal(file: File): Promise<void>   // WASM parseJournal → allStudies 更新
selectStudy(studyId: number): void       // WASM selectStudy → GpuBuffer 生成
setComparisonStudies(studyIds: number[]): void   // スタブ
getDataFrameInfo(): DataFrameInfo | null         // スタブ
```

**loadJournal の動作**:
1. `isLoading = true`
2. `file.arrayBuffer()` → `WasmLoader.getInstance()` → `parseJournal(data)`
3. `allStudies = result.studies` / `isLoading = false`
4. 失敗時: `loadError = errorMessage` / `isLoading = false`

**selectStudy の動作**:
1. `WasmLoader.getInstance()` → `wasm.selectStudy(studyId)` → `GpuBuffer` 生成
2. `selectionStore._trialCount` を `gpuBuffer.trialCount` で初期化
3. `selectionStore.selectedIndices` を全インデックスで初期化（clearSelection状態）

### `useLayoutStore` 🟢

```typescript
// State
layoutMode: LayoutMode           // 'A' | 'B' | 'C' | 'D'
visibleCharts: Set<ChartId>
panelSizes: PanelSizes           // { leftPanel: 280, bottomPanel: 200 }
freeModeLayout: FreeModeLayout | null

// Actions
setLayoutMode(mode: LayoutMode): void
toggleChart(chartId: ChartId): void   // Set への追加/削除
saveLayout(): LayoutConfig
loadLayout(config: LayoutConfig): void
```

---

## 3. 制約条件

### Brushing & Linking — React サイクル外更新（REQ-044）

- 🟢 `useSelectionStore.subscribe()` で GpuBuffer.updateAlphas を呼び出す
- 🟢 この subscribe は React コンポーネントの外（chart 初期化時）で設定する
- 🟢 Zustand の `subscribeWithSelector` ミドルウェアを使用可能

### パフォーマンス（REQ-042, REQ-043）

- 🟢 `addAxisFilter()` の WASM 呼び出しは非同期（UI をブロックしない）
- 🟢 filterByRanges の呼び出しから selectedIndices 更新まで ≤ 5ms（WASM側の責任）

### WASM 未初期化耐性（エラーハンドリング要件）

- 🟡 `addAxisFilter()` / `removeAxisFilter()` で WASM が未初期化（reject）の場合:
  filterRanges のみ更新し、クラッシュしない

### Zustand バージョン（5.0.x）

- 🟢 Zustand 5 では `create` のカリー化が必要: `create<State>()(...)`
- 🟢 `subscribeWithSelector` は `zustand/middleware` からインポート

---

## 4. EARS 要件・設計文書との対応関係

| 実装要素 | EARS 要件 ID | 設計文書 |
|---|---|---|
| SelectionStore | REQ-040〜REQ-043 | dataflow.md § Brushing & Linking |
| Reactサイクル外GPU更新 | REQ-044 | dataflow.md § Brushing & Linking |
| 選択カウンタリアルタイム更新 | REQ-045 | types/index.ts § SelectionStore |

---

## 品質判定

✅ **高品質**
- Brushing & Linking を単一 SelectionStore に集約
- WASM 呼び出しを非同期で分離し UI をブロックしない
- subscribe パターンで Reactサイクル外の GPU 直接更新を実現
