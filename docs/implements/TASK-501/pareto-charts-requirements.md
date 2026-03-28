# TASK-501 要件定義: Pareto 3D/2D Scatter & HypervolumeHistory

## 1. 機能概要

- **対象コンポーネント**:
  1. `ParetoScatter3D` — deck.gl `PointCloudLayer` ベースの3D散布図
  2. `ParetoScatter2D` — deck.gl `ScatterplotLayer` ベースの2D散布図
  3. `HypervolumeHistory` — ECharts折れ線グラフ（HV推移）

- **参照要件**: REQ-050, REQ-070〜REQ-075
- **依存**: TASK-301（GpuBuffer）, TASK-302（Zustand Stores）

## 2. 入出力仕様

### 2.1 ParetoScatter3D / ParetoScatter2D 共通

**Props**:
```ts
interface ParetoScatterProps {
  gpuBuffer: GpuBuffer | null;   // null → 空状態UI表示
  currentStudy: Study | null;    // 軸名取得用
}
```

**内部動作**:
- `selectionStore.subscribe((s) => s.selectedIndices, (indices) => gpuBuffer.updateAlphas(indices))` でGPUバッファを直接更新（React再レンダリングなし）
- 軸割り当て（X/Y/Z）は `useState` でコンポーネント内管理

### 2.2 HypervolumeHistory

**Props**:
```ts
interface HypervolumeHistoryProps {
  data: Array<{ trial: number; hypervolume: number }>;
}
```

## 3. 制約条件

- 🟢 **WebGL非対応時**: `"WebGL非対応のブラウザです"` メッセージを表示（REQ-070）
- 🟢 **データ0件時**: `"データがありません"` メッセージを表示
- 🟢 **アンマウント時**: selectionStore の unsubscribe を呼び出すこと（メモリリーク防止）
- 🟡 **deck.gl バージョン**: 9.x を使用（`DeckGL` コンポーネント + Layer クラス）

## 4. 使用例

```tsx
// ParetoScatter3D の使用例
const gpuBuffer = useStudyStore((s) => s.gpuBuffer);
const currentStudy = useStudyStore((s) => s.currentStudy);
<ParetoScatter3D gpuBuffer={gpuBuffer} currentStudy={currentStudy} />

// HypervolumeHistory の使用例
<HypervolumeHistory data={[{ trial: 1, hypervolume: 0.5 }, ...]} />
```

## 5. EARS要件対応

| REQ | 内容 | 対応 |
|-----|------|------|
| REQ-050 | Brushing & Linking — selectionStore経由でGPU更新 | subscribe パターン |
| REQ-070 | Paretoランク視覚化 | sizes配列でPareto点大きく表示 |
| REQ-072 | Hypervolume履歴 | HypervolumeHistory コンポーネント |
| REQ-074 | 3D/2D Scatter | PointCloudLayer / ScatterplotLayer |
