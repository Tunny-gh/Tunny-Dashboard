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
import {
  useLayoutStore,
  DEFAULT_FREE_LAYOUT,
} from '../../stores/layoutStore';
import type { ChartId, FreeModeLayout, LayoutMode } from '../../types';

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
    <div data-testid="free-layout-canvas" className="flex flex-col gap-2 p-2 h-full">
      {/* ---------------------------------------------------------------- */}
      {/* エラーメッセージ                                                  */}
      {/* ---------------------------------------------------------------- */}
      {layoutLoadError && (
        <div
          data-testid="layout-error-msg"
          className="px-3 py-1.5 text-sm text-red-700 bg-red-50 border border-red-200 rounded"
        >
          {layoutLoadError}
        </div>
      )}

      {/* ---------------------------------------------------------------- */}
      {/* ツールバー                                                        */}
      {/* ---------------------------------------------------------------- */}
      <div className="flex items-center gap-2 flex-wrap">
        {/* レイアウト保存ボタン */}
        <button
          data-testid="save-free-layout-btn"
          onClick={handleSave}
          className="px-3 py-1 text-sm bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
        >
          レイアウト保存
        </button>

        {/* プリセットボタン */}
        <span className="text-xs text-gray-400">プリセット:</span>
        {(['A', 'B', 'C'] as const).map((preset) => (
          <button
            key={preset}
            data-testid={`free-layout-preset-${preset}`}
            onClick={() => handlePreset(preset)}
            className="px-2 py-0.5 text-xs border border-gray-300 rounded text-gray-600 hover:bg-gray-50"
          >
            Mode {preset}
          </button>
        ))}

        {/* 保存成功トースト */}
        {showToast && (
          <span
            data-testid="layout-saved-toast"
            className="ml-2 text-xs text-green-600 bg-green-50 border border-green-200 px-2 py-0.5 rounded"
          >
            レイアウトを保存しました
          </span>
        )}
      </div>

      {/* ---------------------------------------------------------------- */}
      {/* グリッドエリア                                                    */}
      {/* ---------------------------------------------------------------- */}
      <div className="relative flex-1 min-h-0" style={{ minHeight: '400px' }}>
        {/* ドロップゾーン層（背景、絶対配置）*/}
        <div className="absolute inset-0">
          {Array.from({ length: GRID_SIZE }, (_, r) =>
            Array.from({ length: GRID_SIZE }, (_, c) => {
              const row = r + 1;
              const col = c + 1;
              return (
                <div
                  key={`dz-${row}-${col}`}
                  data-testid={`free-layout-dropzone-${row}-${col}`}
                  className="absolute border border-dashed border-gray-200 hover:bg-blue-50/30 transition-colors"
                  style={{
                    top: `${r * 25}%`,
                    left: `${c * 25}%`,
                    width: '25%',
                    height: '25%',
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
          className="absolute inset-0 pointer-events-none"
          style={{
            display: 'grid',
            gridTemplateColumns: `repeat(${GRID_SIZE}, 1fr)`,
            gridTemplateRows: `repeat(${GRID_SIZE}, 1fr)`,
          }}
        >
          {layout.cells.map(({ chartId, gridRow, gridCol }) => (
            <div
              key={chartId}
              data-testid={`free-layout-card-${chartId}`}
              className="pointer-events-auto border border-gray-200 rounded bg-white shadow-sm overflow-hidden"
              style={{
                gridArea: `${gridRow[0]} / ${gridCol[0]} / ${gridRow[1]} / ${gridCol[1]}`,
                zIndex: 1,
              }}
            >
              {/* ドラッグハンドル（タイトルバー） */}
              <div
                data-testid={`free-layout-drag-handle-${chartId}`}
                draggable
                onDragStart={() => setDraggingChartId(chartId)}
                onDragEnd={() => setDraggingChartId(null)}
                className="px-2 py-1 text-xs font-semibold text-gray-600 bg-gray-50 border-b border-gray-200 cursor-grab select-none"
              >
                {CHART_LABELS[chartId] ?? chartId}
              </div>

              {/* チャートコンテンツエリア（プレースホルダー） */}
              <div className="flex-1 bg-gray-50 flex items-center justify-center text-xs text-gray-300">
                {chartId}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};

export default FreeLayoutCanvas;
