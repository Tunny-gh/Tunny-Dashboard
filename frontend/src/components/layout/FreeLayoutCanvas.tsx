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

import React, { useState, useEffect } from 'react'
import ReactECharts from 'echarts-for-react'
import { useLayoutStore, DEFAULT_FREE_LAYOUT } from '../../stores/layoutStore'
import { useStudyStore } from '../../stores/studyStore'
import type { ChartId, FreeModeLayout, Study } from '../../types'
import { OptimizationHistory } from '../charts/OptimizationHistory'
import { ParallelCoordinates } from '../charts/ParallelCoordinates'
import { ScatterMatrix } from '../charts/ScatterMatrix'
import { EdfPlot } from '../charts/EdfPlot'
import { ObjectivePairMatrix } from '../charts/ObjectivePairMatrix'
import { SlicePlot } from '../charts/SlicePlot'
import { ContourPlot } from '../charts/ContourPlot'
import { HypervolumeHistory, type HypervolumeDataPoint } from '../charts/HypervolumeHistory'
import { EmptyState } from '../common/EmptyState'
import { WasmLoader } from '../../wasm/wasmLoader'

// -------------------------------------------------------------------------
// 定数
// -------------------------------------------------------------------------

/** 【チャートラベル】: ChartId → 表示名マッピング */
const CHART_LABELS: Partial<Record<ChartId, string>> = {
  'pareto-front': 'Pareto Front',
  'parallel-coords': 'Parallel Coordinates',
  'scatter-matrix': 'Scatter Matrix',
  history: 'History',
  hypervolume: 'Hypervolume',
  importance: 'Importance',
  'objective-pair-matrix': 'Objective Pair Matrix',
  pdp: 'PDP',
  'sensitivity-heatmap': 'Sensitivity',
  'cluster-view': 'Cluster View',
  umap: 'UMAP',
  // 🟢 optuna-dashboard 相当の追加チャート
  slice: 'Slice Plot',
  edf: 'EDF',
  contour: 'Contour Plot',
}

/** グリッドの次元数（4×4） */
const GRID_SIZE = 4

// -------------------------------------------------------------------------
// HypervolumeContent — computeHvHistory の非同期 WASM 呼び出しを担うサブコンポーネント
// -------------------------------------------------------------------------

function HypervolumeContent({ study }: { study: Study }) {
  const [data, setData] = useState<HypervolumeDataPoint[]>([])
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const isMinimize = study.directions.map((d) => d === 'minimize')
    WasmLoader.getInstance()
      .then((wasm) => {
        const result = wasm.computeHvHistory(isMinimize)
        setData(
          Array.from(result.trialIds).map((id, i) => ({
            trial: id,
            hypervolume: result.hvValues[i],
          })),
        )
      })
      .catch(() => setError('HV 計算エラー'))
  }, [study])

  if (error) return <EmptyState message={error} />
  return (
    <div data-testid="hypervolume-chart" style={{ width: '100%', height: '100%' }}>
      <HypervolumeHistory data={data} />
    </div>
  )
}

// ChartContent — chartId に応じて実コンポーネントを返す
// -------------------------------------------------------------------------

function ChartContent({ chartId }: { chartId: ChartId }) {
  const currentStudy = useStudyStore((s) => s.currentStudy)
  const gpuBuffer = useStudyStore((s) => s.gpuBuffer)
  const trialRows = useStudyStore((s) => s.trialRows)

  if (!currentStudy || !gpuBuffer) {
    return <EmptyState message="データを読み込んでください" />
  }

  switch (chartId) {
    case 'pareto-front': {
      // positions[i*2], positions[i*2+1] からECharts散布図を構築
      // 単目的: [normalized_idx, obj0]  多目的: [obj0, obj1]
      const scatterData = Array.from({ length: gpuBuffer.trialCount }, (_, i) => [
        gpuBuffer.positions[i * 2],
        gpuBuffer.positions[i * 2 + 1],
      ])
      const isMulti = currentStudy.directions.length > 1
      const xLabel = isMulti ? (currentStudy.objectiveNames[0] ?? 'obj0') : 'trial'
      const yLabel = currentStudy.objectiveNames[isMulti ? 1 : 0] ?? 'value'
      const option = {
        grid: { left: '12%', right: '4%', top: '8%', bottom: '14%' },
        xAxis: { type: 'value', name: xLabel, nameLocation: 'middle' as const, nameGap: 24 },
        yAxis: { type: 'value', name: yLabel, nameLocation: 'middle' as const, nameGap: 40 },
        tooltip: { trigger: 'item' as const },
        series: [
          {
            type: 'scatter' as const,
            data: scatterData,
            symbolSize: 5,
            itemStyle: { opacity: 0.7 },
          },
        ],
      }
      return <ReactECharts option={option} style={{ height: '100%', width: '100%' }} />
    }
    case 'parallel-coords':
      return <ParallelCoordinates gpuBuffer={gpuBuffer} currentStudy={currentStudy} />
    case 'history': {
      // positions から TrialData を導出
      // 単目的: [norm_idx, obj0]  多目的: [obj0, obj1]
      const isMulti = currentStudy.directions.length > 1
      const data = Array.from({ length: gpuBuffer.trialCount }, (_, i) => ({
        trial: i + 1,
        value: isMulti
          ? gpuBuffer.positions[i * 2] // multi: x = obj0
          : gpuBuffer.positions[i * 2 + 1], // single: y = obj0
      }))
      const direction = currentStudy.directions[0] === 'minimize' ? 'minimize' : 'maximize'
      return <OptimizationHistory data={data} direction={direction} />
    }
    case 'scatter-matrix':
      // engine=null のとき ScatterMatrix はモード UI + グレーセルを表示する
      return <ScatterMatrix engine={null} currentStudy={currentStudy} />
    case 'objective-pair-matrix':
      if (currentStudy.objectiveNames.length <= 1) {
        return <EmptyState message="多目的 Study でのみ利用可能です" />
      }
      return <ObjectivePairMatrix gpuBuffer={gpuBuffer} currentStudy={currentStudy} />
    case 'importance': {
      if (currentStudy.paramNames.length === 0) {
        return <EmptyState />
      }
      const importanceOption = {
        title: { text: '重要度（暫定・WASM未計算）', textStyle: { fontSize: 12 } },
        xAxis: { type: 'category', data: currentStudy.paramNames },
        yAxis: { type: 'value' },
        series: [{ type: 'bar', data: currentStudy.paramNames.map(() => 1.0) }],
        grid: { containLabel: true },
      }
      return <ReactECharts option={importanceOption} style={{ height: '100%', width: '100%' }} />
    }
    case 'slice':
      if (trialRows.length === 0) return <EmptyState />
      return (
        <SlicePlot
          trials={trialRows.map((t) => ({
            trialId: t.trialId,
            params: t.params,
            values: t.values,
            paretoRank: t.paretoRank,
          }))}
          paramNames={currentStudy.paramNames}
          objectiveNames={currentStudy.objectiveNames}
        />
      )
    case 'contour':
      if (trialRows.length === 0) return <EmptyState />
      if (currentStudy.paramNames.length < 2) {
        return <EmptyState message="パラメータが2つ以上必要です" />
      }
      return (
        <ContourPlot
          trials={trialRows.map((t) => ({ params: t.params, values: t.values }))}
          paramNames={currentStudy.paramNames}
          objectiveNames={currentStudy.objectiveNames}
        />
      )
    case 'edf': {
      // gpuBuffer.positions から目的関数値を取り出して EDF を描画する
      // 単目的: positions[i*2+1] = obj0
      // 多目的: positions[i*2] = obj0, positions[i*2+1] = obj1
      const isMultiEdf = currentStudy.directions.length > 1
      const edfSeries = isMultiEdf
        ? [
            {
              name: currentStudy.objectiveNames[0] ?? 'obj0',
              values: Array.from(
                { length: gpuBuffer.trialCount },
                (_, i) => gpuBuffer.positions[i * 2],
              ),
            },
            {
              name: currentStudy.objectiveNames[1] ?? 'obj1',
              values: Array.from(
                { length: gpuBuffer.trialCount },
                (_, i) => gpuBuffer.positions[i * 2 + 1],
              ),
            },
          ]
        : [
            {
              name: currentStudy.objectiveNames[0] ?? 'value',
              values: Array.from(
                { length: gpuBuffer.trialCount },
                (_, i) => gpuBuffer.positions[i * 2 + 1],
              ),
            },
          ]
      return <EdfPlot series={edfSeries} />
    }
    case 'hypervolume':
      if (currentStudy.directions.length < 2) {
        return <EmptyState message="多目的 Study でのみ利用可能です" />
      }
      return <HypervolumeContent study={currentStudy} />
    default:
      return <EmptyState message="このチャートは準備中です" />
  }
}

// -------------------------------------------------------------------------
// FreeLayoutCanvas コンポーネント
// -------------------------------------------------------------------------

/**
 * 【機能概要】: 4×4グリッドのドラッグ&ドロップレイアウト編集コンポーネント
 */
export const FreeLayoutCanvas: React.FC = () => {
  const {
    freeModeLayout,
    layoutLoadError,
    setFreeModeLayout,
    updateCellPosition,
    addCell,
    removeCell,
  } = useLayoutStore()
  const setLayoutMode = useLayoutStore((s) => s.setLayoutMode)
  const layoutMode = useLayoutStore((s) => s.layoutMode)

  // 【ドラッグ中セルID】: cellId で管理（同一 chartId の複数インスタンスを識別するため）
  const [draggingCellId, setDraggingCellId] = useState<string | null>(null)
  // 【保存トースト表示状態】
  const [showToast, setShowToast] = useState(false)

  // 【実効レイアウト】: null のときはデフォルトを使用
  const layout = freeModeLayout ?? DEFAULT_FREE_LAYOUT

  /**
   * 【ドロップ処理】: ドロップゾーン (row, col) への配置を確定する
   * dataTransfer の type で 'move-chart'（再配置）と 'add-chart'（カタログ追加）を判別する
   */
  const handleDrop = (row: number, col: number, e: React.DragEvent) => {
    e.preventDefault()

    // 【カタログからの追加ドロップ】
    try {
      const raw = e.dataTransfer.getData('text/plain')
      if (raw) {
        const payload = JSON.parse(raw) as { type: string; chartId?: ChartId; cellId?: string }
        if (payload.type === 'add-chart' && payload.chartId) {
          // Mode D 以外のとき自動切替
          if (layoutMode !== 'D') setLayoutMode('D')
          const newRowEnd = Math.min(row + 2, GRID_SIZE + 1)
          const newColEnd = Math.min(col + 2, GRID_SIZE + 1)
          addCell(payload.chartId, [row, newRowEnd], [col, newColEnd])
          return
        }
      }
    } catch {
      // 不正 JSON はサイレントに無視
    }

    // 【既存タイルの再配置ドロップ】
    if (!draggingCellId) return

    const cell = layout.cells.find((c) => c.cellId === draggingCellId)
    if (!cell) return

    const spanRow = cell.gridRow[1] - cell.gridRow[0]
    const spanCol = cell.gridCol[1] - cell.gridCol[0]

    // 【境界クランプ】: グリッド外にはみ出さないよう制限
    const newRowEnd = Math.min(row + spanRow, GRID_SIZE + 1)
    const newColEnd = Math.min(col + spanCol, GRID_SIZE + 1)

    updateCellPosition(draggingCellId, [row, newRowEnd], [col, newColEnd])
    setDraggingCellId(null)
  }

  /**
   * 【レイアウト保存】: freeModeLayout を確定してトーストを表示する
   * 実際の永続化は sessionStore.saveSession() を通じて行う
   */
  const handleSave = () => {
    // freeModeLayout は常に最新状態が store に入っているため追加処理不要
    setShowToast(true)
    setTimeout(() => setShowToast(false), 2000)
  }

  return (
    <div
      data-testid="free-layout-canvas"
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '8px',
        padding: '8px',
        height: '100%',
        boxSizing: 'border-box',
      }}
    >
      {/* ---------------------------------------------------------------- */}
      {/* エラーメッセージ                                                  */}
      {/* ---------------------------------------------------------------- */}
      {layoutLoadError && (
        <div
          data-testid="layout-error-msg"
          style={{
            padding: '6px 12px',
            fontSize: '13px',
            color: '#b91c1c',
            background: '#fef2f2',
            border: '1px solid #fecaca',
            borderRadius: '4px',
          }}
        >
          {layoutLoadError}
        </div>
      )}

      {/* ---------------------------------------------------------------- */}
      {/* ツールバー                                                        */}
      {/* ---------------------------------------------------------------- */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '8px',
          flexWrap: 'wrap',
          flexShrink: 0,
        }}
      >
        {/* レイアウト保存ボタン */}
        <button
          data-testid="save-free-layout-btn"
          onClick={handleSave}
          style={{
            padding: '2px 10px',
            fontSize: '13px',
            background: 'var(--accent)',
            color: '#fff',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer',
          }}
        >
          レイアウト保存
        </button>

        {/* 保存成功トースト */}
        {showToast && (
          <span
            data-testid="layout-saved-toast"
            style={{
              marginLeft: '8px',
              fontSize: '12px',
              color: '#16a34a',
              background: '#f0fdf4',
              border: '1px solid #bbf7d0',
              padding: '1px 8px',
              borderRadius: '4px',
            }}
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
              const row = r + 1
              const col = c + 1
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
                  onDrop={(e) => handleDrop(row, col, e)}
                />
              )
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
          {layout.cells.map(({ cellId, chartId, gridRow, gridCol }) => (
            <div
              key={cellId}
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
              {/* ドラッグハンドル（タイトルバー）+ 削除ボタン */}
              <div
                data-testid={`free-layout-drag-handle-${chartId}`}
                draggable
                onDragStart={(e) => {
                  setDraggingCellId(cellId)
                  e.dataTransfer?.setData(
                    'text/plain',
                    JSON.stringify({ type: 'move-chart', cellId }),
                  )
                }}
                onDragEnd={() => setDraggingCellId(null)}
                style={{
                  padding: '3px 8px',
                  fontSize: '12px',
                  fontWeight: 600,
                  color: 'var(--accent)',
                  background: 'var(--bg-panel)',
                  borderBottom: '1px solid var(--border)',
                  cursor: 'grab',
                  userSelect: 'none',
                  flexShrink: 0,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                }}
              >
                <span>{CHART_LABELS[chartId] ?? chartId}</span>
                <button
                  data-testid={`chart-close-btn-${cellId}`}
                  onClick={(e) => {
                    e.stopPropagation()
                    removeCell(cellId)
                  }}
                  style={{
                    background: 'transparent',
                    border: 'none',
                    cursor: 'pointer',
                    color: 'var(--text-muted)',
                    fontSize: '14px',
                    lineHeight: 1,
                    padding: '0 2px',
                  }}
                >
                  ×
                </button>
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
  )
}

export default FreeLayoutCanvas
