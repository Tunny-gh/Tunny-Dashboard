/**
 * FreeLayoutCanvas — Free-layout (Mode D) canvas (TASK-1501)
 *
 * Documentation.
 * Design:
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */

import React, { useState, useEffect, useMemo } from 'react'
import ReactECharts from 'echarts-for-react'
import { useLayoutStore, DEFAULT_FREE_LAYOUT } from '../../stores/layoutStore'
import { useStudyStore } from '../../stores/studyStore'
import { useSelectionStore } from '../../stores/selectionStore'
import type { ChartId, Study } from '../../types'
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
// Constants
// -------------------------------------------------------------------------

/** Documentation. */
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
  // Documentation.
  slice: 'Slice Plot',
  edf: 'EDF',
  contour: 'Contour Plot',
}

/** Grid dimension count（4×4） */
const GRID_SIZE = 4

// -------------------------------------------------------------------------
// Documentation.
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
      .catch(() => setError('HV computation error'))
  }, [study])

  if (error) return <EmptyState message={error} />
  return (
    <div data-testid="hypervolume-chart" style={{ width: '100%', height: '100%' }}>
      <HypervolumeHistory data={data} />
    </div>
  )
}

// Documentation.
// -------------------------------------------------------------------------

function ChartContent({ chartId }: { chartId: ChartId }) {
  const currentStudy = useStudyStore((s) => s.currentStudy)
  const gpuBuffer = useStudyStore((s) => s.gpuBuffer)
  const trialRows = useStudyStore((s) => s.trialRows)
  const selectedIndices = useSelectionStore((s) => s.selectedIndices)

  // Build a Set of selected indices for quick lookup (memoized to avoid O(n) on every render)
  // Must be before early return to satisfy Rules of Hooks
  const selectedSet = useMemo(() => {
    if (gpuBuffer && selectedIndices.length > 0 && selectedIndices.length < gpuBuffer.trialCount) {
      return new Set(selectedIndices)
    }
    return null
  }, [selectedIndices, gpuBuffer])

  if (!currentStudy || !gpuBuffer) {
    return <EmptyState message="Please load data" />
  }

  switch (chartId) {
    case 'pareto-front': {
      if (currentStudy.directions.length < 2) {
        return <EmptyState message="Available for multi-objective studies only" />
      }
      const xLabel = currentStudy.objectiveNames[0] ?? 'obj0'
      const yLabel = currentStudy.objectiveNames[1] ?? 'obj1'

      // Build objective arrays with direction sign (minimize = as-is, maximize = negate)
      const signs = currentStudy.directions.map((d) => (d === 'minimize' ? 1 : -1))
      const pts: { xy: number[]; norm: number[] }[] = []
      for (let i = 0; i < trialRows.length; i++) {
        const v = trialRows[i].values
        if (!v || v.length < 2) continue
        const norm = v.map((val, j) => (signs[j] ?? 1) * val)
        pts.push({ xy: [v[0], v[1]], norm })
      }

      // Non-dominated sort (rank 1 only)
      const isNonDominated = pts.map((a, ai) => {
        for (let bi = 0; bi < pts.length; bi++) {
          if (ai === bi) continue
          const b = pts[bi]
          let bBetter = false
          let aNotWorse = true
          for (let k = 0; k < a.norm.length; k++) {
            if (b.norm[k] < a.norm[k]) bBetter = true
            if (b.norm[k] > a.norm[k]) { aNotWorse = false; break }
          }
          if (bBetter && aNotWorse) return false // a is dominated by b
        }
        return true
      })

      const paretoPoints: number[][] = []
      const dominatedPoints: number[][] = []
      for (let i = 0; i < pts.length; i++) {
        if (isNonDominated[i]) {
          paretoPoints.push(pts[i].xy)
        } else {
          dominatedPoints.push(pts[i].xy)
        }
      }

      // Sort pareto front by x for the connecting line
      const sortedPareto = [...paretoPoints].sort((a, b) => a[0] - b[0])

      const seriesList: object[] = [
        {
          name: 'Dominated',
          type: 'scatter' as const,
          data: dominatedPoints,
          symbolSize: 5,
          itemStyle: { color: '#5470c6', opacity: 0.7 },
        },
        {
          name: 'Pareto Front',
          type: 'scatter' as const,
          data: paretoPoints,
          symbolSize: 7,
          itemStyle: { color: '#ee6666', opacity: 0.9 },
          z: 10,
        },
        {
          name: 'Pareto Line',
          type: 'line' as const,
          data: sortedPareto,
          showSymbol: false,
          lineStyle: { color: '#ee6666', opacity: 0.35, width: 1.5 },
          z: 5,
        },
      ]

      const option = {
        grid: { left: '12%', right: '4%', top: '8%', bottom: '14%' },
        xAxis: { type: 'value', name: xLabel, nameLocation: 'middle' as const, nameGap: 24 },
        yAxis: { type: 'value', name: yLabel, nameLocation: 'middle' as const, nameGap: 40 },
        tooltip: { trigger: 'item' as const },
        legend: { show: false },
        series: seriesList,
      }
      return <ReactECharts option={option} style={{ height: '100%', width: '100%' }} lazyUpdate />
    }
    case 'parallel-coords':
      return <ParallelCoordinates gpuBuffer={gpuBuffer} currentStudy={currentStudy} />
    case 'history': {
      // Documentation.
      // single-objective: [norm_idx, obj0]  multi-objective: [obj0, obj1]
      const isMulti = currentStudy.directions.length > 1
      const data = Array.from({ length: gpuBuffer.trialCount }, (_, i) => ({
        trial: i + 1,
        value: isMulti
          ? gpuBuffer.positions[i * 2] // multi: x = obj0
          : gpuBuffer.positions[i * 2 + 1], // single: y = obj0
      }))
      const direction = currentStudy.directions[0] === 'minimize' ? 'minimize' : 'maximize'
      return (
        <OptimizationHistory data={data} direction={direction} selectedIndices={selectedIndices} />
      )
    }
    case 'scatter-matrix':
      return <ScatterMatrix trialRows={trialRows} currentStudy={currentStudy} />
    case 'objective-pair-matrix':
      if (currentStudy.objectiveNames.length <= 1) {
        return <EmptyState message="Available for multi-objective studies only" />
      }
      return <ObjectivePairMatrix gpuBuffer={gpuBuffer} currentStudy={currentStudy} />
    case 'importance': {
      if (currentStudy.paramNames.length === 0) {
        return <EmptyState />
      }
      const importanceOption = {
        title: { text: 'Importance (tentative, WASM not computed)', textStyle: { fontSize: 12 } },
        xAxis: { type: 'category', data: currentStudy.paramNames },
        yAxis: { type: 'value' },
        series: [{ type: 'bar', data: currentStudy.paramNames.map(() => 1.0) }],
        grid: { containLabel: true },
      }
      return (
        <ReactECharts
          option={importanceOption}
          style={{ height: '100%', width: '100%' }}
          lazyUpdate
        />
      )
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
          selectedIndices={selectedIndices}
        />
      )
    case 'contour':
      if (trialRows.length === 0) return <EmptyState />
      if (currentStudy.paramNames.length < 2) {
        return <EmptyState message="At least 2 parameters required" />
      }
      return (
        <ContourPlot
          trials={trialRows.map((t) => ({ params: t.params, values: t.values }))}
          paramNames={currentStudy.paramNames}
          objectiveNames={currentStudy.objectiveNames}
          selectedIndices={selectedIndices}
        />
      )
    case 'edf': {
      // Documentation.
      // single-objective: positions[i*2+1] = obj0
      // multi-objective: positions[i*2] = obj0, positions[i*2+1] = obj1
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
        return <EmptyState message="Available for multi-objective studies only" />
      }
      return <HypervolumeContent study={currentStudy} />
    default:
      return <EmptyState message="This chart is under development" />
  }
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
export const FreeLayoutCanvas: React.FC = () => {
  const { freeModeLayout, layoutLoadError, updateCellPosition, addCell, removeCell } =
    useLayoutStore()
  const setLayoutMode = useLayoutStore((s) => s.setLayoutMode)
  const layoutMode = useLayoutStore((s) => s.layoutMode)

  // Documentation.
  const [draggingCellId, setDraggingCellId] = useState<string | null>(null)
  // Documentation.
  const [showToast, setShowToast] = useState(false)

  // Documentation.
  const layout = freeModeLayout ?? DEFAULT_FREE_LAYOUT

  /**
   * Documentation.
   * Documentation.
   */
  const handleDrop = (row: number, col: number, e: React.DragEvent) => {
    e.preventDefault()

    // Documentation.
    try {
      const raw = e.dataTransfer.getData('text/plain')
      if (raw) {
        const payload = JSON.parse(raw) as { type: string; chartId?: ChartId; cellId?: string }
        if (payload.type === 'add-chart' && payload.chartId) {
          // Documentation.
          if (layoutMode !== 'D') setLayoutMode('D')
          const newRowEnd = Math.min(row + 2, GRID_SIZE + 1)
          const newColEnd = Math.min(col + 2, GRID_SIZE + 1)
          addCell(payload.chartId, [row, newRowEnd], [col, newColEnd])
          return
        }
      }
    } catch {
      // Documentation.
    }

    // Documentation.
    if (!draggingCellId) return

    const cell = layout.cells.find((c) => c.cellId === draggingCellId)
    if (!cell) return

    const spanRow = cell.gridRow[1] - cell.gridRow[0]
    const spanCol = cell.gridCol[1] - cell.gridCol[0]

    // Documentation.
    const newRowEnd = Math.min(row + spanRow, GRID_SIZE + 1)
    const newColEnd = Math.min(col + spanCol, GRID_SIZE + 1)

    updateCellPosition(draggingCellId, [row, newRowEnd], [col, newColEnd])
    setDraggingCellId(null)
  }

  /**
   * Documentation.
   * Documentation.
   */
  const handleSave = () => {
    // Documentation.
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
      {/* error message                                                  */}
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
      {/* Documentation. */}
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
        {/* Documentation. */}
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
          Save Layout
        </button>

        {/* Documentation. */}
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
            Layout saved
          </span>
        )}
      </div>

      {/* ---------------------------------------------------------------- */}
      {/* Documentation. */}
      {/* ---------------------------------------------------------------- */}
      <div style={{ position: 'relative', flex: 1, minHeight: 0 }}>
        {/* Documentation. */}
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

        {/* Documentation. */}
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
              {/* Documentation. */}
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

              {/* Documentation. */}
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
