/**
 * FreeLayoutCanvas — フリーレイアウト（Mode D）キャンバス (TASK-1501)
 *
 * 【役割】: 4×4グリッドのドラッグ&ドロップでチャートを自由配置する
 * 【設計方針】:
 *   - 4×4 CSS Grid（絶対配置ドロップゾーン + CSS Grid チャートカード）
 *   - ドラッグ状態を useState で管理（dataTransfer 不使用でテスト容易）
 *   - freeModeLayout が null のときは DEFAULT_FREE_LAYOUT を使用
 *   - レイアウト保存後に 2 秒間トースト通知を表示
 *   - プリセット（A/B/C）適用は window.confirm で確認後に適用
 * 🟢 REQ-032, NFR-031, NFR-032 に準拠
 */

import React, { useState } from 'react';
import ReactECharts from 'echarts-for-react';
import {
  useLayoutStore,
  DEFAULT_FREE_LAYOUT,
} from '../../stores/layoutStore';
import { useStudyStore } from '../../stores/studyStore';
import type { ChartId, FreeModeLayout, LayoutMode } from '../../types';
import { OptimizationHistory } from '../charts/OptimizationHistory';
import { ParallelCoordinates } from '../charts/ParallelCoordinates';
import { EmptyState } from '../common/EmptyState';

// -------------------------------------------------------------------------
// 定数
// -------------------------------------------------------------------------

/** 【チャートラベル】: ChartId → 表示名マッピング */
const CHART_LABELS: Partial<Record<ChartId, string>> = {
  'pareto-front': 'Pareto Front',
  'parallel-coords': 'Parallel Coordinates',
  'scatter-matrix': 'Scatter Matrix',
  'history': 'History',
  'hypervolume': 'Hypervolume',
  'importance': 'Importance',
  'objective-pair-matrix': 'Objective Pair Matrix',
  'pdp': 'PDP',
  'sensitivity-heatmap': 'Sensitivity',
  'cluster-view': 'Cluster View',
  'umap': 'UMAP',
  // 🟢 optuna-dashboard 相当の追加チャート
  'slice': 'Slice Plot',
  'edf': 'EDF',
  'contour': 'Contour Plot',
};

/**
 * 【プリセットレイアウト】: Mode A〜C 相当の FreeModeLayout 定義
 * プリセットボタンクリック時に適用される
 */
const PRESET_LAYOUTS: Record<Exclude<LayoutMode, 'D'>, FreeModeLayout> = {
  A: {
    cells: [
      { chartId: 'pareto-front', gridRow: [1, 3], gridCol: [1, 3] },
      { chartId: 'parallel-coords', gridRow: [1, 3], gridCol: [3, 5] },
      { chartId: 'scatter-matrix', gridRow: [3, 5], gridCol: [1, 3] },
      { chartId: 'history', gridRow: [3, 5], gridCol: [3, 5] },
    ],
  },
  B: {
    cells: [
      { chartId: 'pareto-front', gridRow: [1, 5], gridCol: [1, 3] },
      { chartId: 'parallel-coords', gridRow: [1, 3], gridCol: [3, 5] },
      { chartId: 'hypervolume', gridRow: [3, 5], gridCol: [3, 5] },
    ],
  },
  C: {
    cells: [
      { chartId: 'pareto-front', gridRow: [1, 3], gridCol: [1, 3] },
      { chartId: 'parallel-coords', gridRow: [1, 3], gridCol: [3, 5] },
      { chartId: 'scatter-matrix', gridRow: [3, 5], gridCol: [1, 5] },
    ],
  },
};

/** グリッドの次元数（4×4） */
const GRID_SIZE = 4;

// -------------------------------------------------------------------------
// ChartContent — chartId に応じて実コンポーネントを返す
// -------------------------------------------------------------------------

function ChartContent({ chartId }: { chartId: ChartId }) {
  const currentStudy = useStudyStore((s) => s.currentStudy);
  const gpuBuffer = useStudyStore((s) => s.gpuBuffer);

  if (!currentStudy || !gpuBuffer) {
    return <EmptyState />;
  }

  switch (chartId) {
    case 'pareto-front': {
      // positions[i*2], positions[i*2+1] からECharts散布図を構築
      // 単目的: [normalized_idx, obj0]  多目的: [obj0, obj1]
      const scatterData = Array.from({ length: gpuBuffer.trialCount }, (_, i) => [
        gpuBuffer.positions[i * 2],
        gpuBuffer.positions[i * 2 + 1],
      ]);
      const isMulti = currentStudy.directions.length > 1;
      const xLabel = isMulti ? (currentStudy.objectiveNames[0] ?? 'obj0') : 'trial';
      const yLabel = currentStudy.objectiveNames[isMulti ? 1 : 0] ?? 'value';
      const option = {
        grid: { left: '12%', right: '4%', top: '8%', bottom: '14%' },
        xAxis: { type: 'value', name: xLabel, nameLocation: 'middle' as const, nameGap: 24 },
        yAxis: { type: 'value', name: yLabel, nameLocation: 'middle' as const, nameGap: 40 },
        tooltip: { trigger: 'item' as const },
        series: [{ type: 'scatter' as const, data: scatterData, symbolSize: 5, itemStyle: { opacity: 0.7 } }],
      };
      return <ReactECharts option={option} style={{ height: '100%', width: '100%' }} />;
    }
    case 'parallel-coords':
      return <ParallelCoordinates gpuBuffer={gpuBuffer} currentStudy={currentStudy} />;
    case 'history': {
      // positions から TrialData を導出
      // 単目的: [norm_idx, obj0]  多目的: [obj0, obj1]
      const isMulti = currentStudy.directions.length > 1;
      const data = Array.from({ length: gpuBuffer.trialCount }, (_, i) => ({
        trial: i + 1,
        value: isMulti
          ? gpuBuffer.positions[i * 2]       // multi: x = obj0
          : gpuBuffer.positions[i * 2 + 1],  // single: y = obj0
      }));
      const direction = currentStudy.directions[0] === 'minimize' ? 'minimize' : 'maximize';
      return <OptimizationHistory data={data} direction={direction} />;
    }
    default:
      return <EmptyState message={`${chartId} チャートは準備中です`} />;
  }
}

// -------------------------------------------------------------------------
// FreeLayoutCanvas コンポーネント
// -------------------------------------------------------------------------

/**
 * 【機能概要】: 4×4グリッドのドラッグ&ドロップレイアウト編集コンポーネント
 */
export const FreeLayoutCanvas: React.FC = () => {
  const { freeModeLayout, layoutLoadError, setFreeModeLayout, updateCellPosition } =
    useLayoutStore();

  // 【ドラッグ中チャートID】: dataTransfer を使わず state で管理（テスト容易性向上）
  const [draggingChartId, setDraggingChartId] = useState<ChartId | null>(null);
  // 【保存トースト表示状態】
  const [showToast, setShowToast] = useState(false);

  // 【実効レイアウト】: null のときはデフォルトを使用
  const layout = freeModeLayout ?? DEFAULT_FREE_LAYOUT;

  /**
   * 【ドロップ処理】: ドロップゾーン (row, col) への配置を確定する
   * 現在のスパン（行/列の幅）を維持しつつグリッド境界内にクランプする
   */
  const handleDrop = (row: number, col: number) => {
    if (!draggingChartId) return;

    const cell = layout.cells.find((c) => c.chartId === draggingChartId);
    if (!cell) return;

    const spanRow = cell.gridRow[1] - cell.gridRow[0];
    const spanCol = cell.gridCol[1] - cell.gridCol[0];

    // 【境界クランプ】: グリッド外にはみ出さないよう制限
    const newRowEnd = Math.min(row + spanRow, GRID_SIZE + 1);
    const newColEnd = Math.min(col + spanCol, GRID_SIZE + 1);

    updateCellPosition(draggingChartId, [row, newRowEnd], [col, newColEnd]);
    setDraggingChartId(null);
  };

  /**
   * 【レイアウト保存】: freeModeLayout を確定してトーストを表示する
   * 実際の永続化は sessionStore.saveSession() を通じて行う
   */
  const handleSave = () => {
    // freeModeLayout は常に最新状態が store に入っているため追加処理不要
    setShowToast(true);
    setTimeout(() => setShowToast(false), 2000);
  };

  /**
   * 【プリセット適用】: confirm 確認後にプリセットレイアウトを適用する
   */
  const handlePreset = (preset: Exclude<LayoutMode, 'D'>) => {
    if (window.confirm(`Mode ${preset} のプリセットでレイアウトを上書きしますか？`)) {
      setFreeModeLayout(PRESET_LAYOUTS[preset]);
    }
  };

  return (
    <div
      data-testid="free-layout-canvas"
      style={{ display: 'flex', flexDirection: 'column', gap: '8px', padding: '8px', height: '100%', boxSizing: 'border-box' }}
    >
      {/* ---------------------------------------------------------------- */}
      {/* エラーメッセージ                                                  */}
      {/* ---------------------------------------------------------------- */}
      {layoutLoadError && (
        <div
          data-testid="layout-error-msg"
          style={{ padding: '6px 12px', fontSize: '13px', color: '#b91c1c', background: '#fef2f2', border: '1px solid #fecaca', borderRadius: '4px' }}
        >
          {layoutLoadError}
        </div>
      )}

      {/* ---------------------------------------------------------------- */}
      {/* ツールバー                                                        */}
      {/* ---------------------------------------------------------------- */}
      <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flexWrap: 'wrap', flexShrink: 0 }}>
        {/* レイアウト保存ボタン */}
        <button
          data-testid="save-free-layout-btn"
          onClick={handleSave}
          style={{ padding: '2px 10px', fontSize: '13px', background: 'var(--accent)', color: '#fff', border: 'none', borderRadius: '4px', cursor: 'pointer' }}
        >
          レイアウト保存
        </button>

        {/* プリセットボタン */}
        <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>プリセット:</span>
        {(['A', 'B', 'C'] as const).map((preset) => (
          <button
            key={preset}
            data-testid={`free-layout-preset-${preset}`}
            onClick={() => handlePreset(preset)}
            style={{ padding: '1px 8px', fontSize: '12px', border: '1px solid var(--border)', borderRadius: '4px', color: 'var(--accent)', background: 'var(--bg)', cursor: 'pointer' }}
          >
            Mode {preset}
          </button>
        ))}

        {/* 保存成功トースト */}
        {showToast && (
          <span
            data-testid="layout-saved-toast"
            style={{ marginLeft: '8px', fontSize: '12px', color: '#16a34a', background: '#f0fdf4', border: '1px solid #bbf7d0', padding: '1px 8px', borderRadius: '4px' }}
          >
            レイアウトを保存しました
          </span>
        )}
      </div>

      {/* ---------------------------------------------------------------- */}
      {/* グリッドエリア                                                    */}
      {/* ---------------------------------------------------------------- */}
      <div style={{ position: 'relative', flex: 1, minHeight: 0 }}>
        {/* ドロップゾーン層（背景、絶対配置）*/}
        <div style={{ position: 'absolute', inset: 0 }}>
          {Array.from({ length: GRID_SIZE }, (_, r) =>
            Array.from({ length: GRID_SIZE }, (_, c) => {
              const row = r + 1;
              const col = c + 1;
              return (
                <div
                  key={`dz-${row}-${col}`}
                  data-testid={`free-layout-dropzone-${row}-${col}`}
                  style={{
                    position: 'absolute',
                    top: `${r * 25}%`,
                    left: `${c * 25}%`,
                    width: '25%',
                    height: '25%',
                    border: '1px dashed #e5e7eb',
                  }}
                  onDragOver={(e) => e.preventDefault()}
                  onDrop={() => handleDrop(row, col)}
                />
              );
            }),
          )}
        </div>

        {/* チャートカード層（CSS Grid、ドロップゾーンの上に重ねる）*/}
        <div
          style={{
            position: 'absolute',
            inset: 0,
            pointerEvents: 'none',
            display: 'grid',
            gridTemplateColumns: `repeat(${GRID_SIZE}, 1fr)`,
            gridTemplateRows: `repeat(${GRID_SIZE}, 1fr)`,
          }}
        >
          {layout.cells.map(({ chartId, gridRow, gridCol }) => (
            <div
              key={chartId}
              data-testid={`free-layout-card-${chartId}`}
              style={{
                gridArea: `${gridRow[0]} / ${gridCol[0]} / ${gridRow[1]} / ${gridCol[1]}`,
                zIndex: 1,
                pointerEvents: 'auto',
                border: '1px solid var(--border)',
                borderRadius: '4px',
                background: 'var(--bg)',
                boxShadow: '0 1px 3px rgba(37,99,235,0.08)',
                overflow: 'hidden',
                display: 'flex',
                flexDirection: 'column',
              }}
            >
              {/* ドラッグハンドル（タイトルバー） */}
              <div
                data-testid={`free-layout-drag-handle-${chartId}`}
                draggable
                onDragStart={() => setDraggingChartId(chartId)}
                onDragEnd={() => setDraggingChartId(null)}
                style={{ padding: '3px 8px', fontSize: '12px', fontWeight: 600, color: 'var(--accent)', background: 'var(--bg-panel)', borderBottom: '1px solid var(--border)', cursor: 'grab', userSelect: 'none', flexShrink: 0 }}
              >
                {CHART_LABELS[chartId] ?? chartId}
              </div>

              {/* チャートコンテンツエリア */}
              <div style={{ flex: 1, overflow: 'hidden', minHeight: 0 }}>
                <ChartContent chartId={chartId} />
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

export default FreeLayoutCanvas;
