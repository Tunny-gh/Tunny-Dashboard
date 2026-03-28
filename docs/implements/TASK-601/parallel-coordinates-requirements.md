# TASK-601 要件定義: Parallel Coordinates 30軸

## 1. 機能概要

- **対象コンポーネント**: `ParallelCoordinates` — ECharts parallel座標を使った30軸表示
- **参照要件**: REQ-051, REQ-041
- **依存**: TASK-501（ECharts環境）, TASK-302（selectionStore）

### 役割

全変数（最大30軸）＋全目的（最大4軸）を平行座標系で表示し、
軸上のブラシ操作 → `selectionStore.addAxisFilter()` → WASM フィルタ → GPUバッファ alpha 更新
という Brushing & Linking パイプラインを実現する。

## 2. 入出力仕様

### Props

```ts
interface ParallelCoordinatesProps {
  gpuBuffer: GpuBuffer | null;    // null → 空状態UI表示
  currentStudy: Study | null;     // 軸名取得用
}
```

### 内部動作

1. `currentStudy.paramNames` + `currentStudy.objectiveNames` → ECharts `parallelAxis` 配列
2. `gpuBuffer.positions` などを ECharts `series[{type:'parallel'}]` の data に変換
3. ECharts `axisareaselected` イベント → `addAxisFilter(axisName, min, max)` 呼び出し
4. 軸のフィルタが全て除去された → `removeAxisFilter(axisName)`

### ECharts axisareaselected イベントデータ形式（設計）

```ts
interface AxisAreaSelectedEvent {
  axesInfo: Array<{
    axisIndex: number;
    intervals: number[][];  // [[min, max], ...]
  }>;
}
```

## 3. 制約条件

- 🟢 **データなし時**: `"データが読み込まれていません"` を表示（REQ-051）
- 🟢 **フィルタで0件時**: `"絞り込み結果: 0件"` を表示
- 🟢 **30軸 + 4軸**: paramNames（最大30）+ objectiveNames（最大4）を全て表示
- 🟡 **カテゴリ変数**: ECharts の `type: 'value'` で数値インデックスとして表示（実数値は TASK-602 で対応）

## 4. EARS要件対応

| REQ | 内容 | 対応 |
|-----|------|------|
| REQ-051 | Parallel Coordinates 表示 | ECharts parallel type |
| REQ-041 | Axis Filter → addAxisFilter | axisareaselected イベントで実装 |
