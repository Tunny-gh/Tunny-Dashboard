/**
 * ScatterMatrix — renders a grid of small scatter plot thumbnails.
 *
 * Each cell shows one axis vs another, drawn directly on a <canvas> element
 * using trial data (no WebWorker needed for 80 px thumbnails).
 */

import { useState, useEffect, useRef, useMemo } from 'react'
import type { Study, TrialData } from '../../types'

// -------------------------------------------------------------------------
// Constants / Types
// -------------------------------------------------------------------------

export type ScatterMode = 'mode1' | 'mode2' | 'mode3'

export type SortOrder = 'alphabetical' | 'correlation' | 'importance'

const MODE_LABELS: Record<ScatterMode, string> = {
  mode1: 'Params×Params',
  mode2: 'Params×Objectives',
  mode3: 'All',
}

const SORT_LABELS: Record<SortOrder, string> = {
  alphabetical: 'Alphabetical',
  correlation: 'By Correlation',
  importance: 'By Importance',
}

const CELL_PX = 80

// -------------------------------------------------------------------------
// Props
// -------------------------------------------------------------------------

export interface ScatterMatrixProps {
  trialRows: TrialData[]
  currentStudy: Study | null
}

// -------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------

export function getAxesForMode(
  study: Study,
  mode: ScatterMode,
  sortOrder: SortOrder,
): { rowAxes: string[]; colAxes: string[] } {
  const applySort = (axes: string[]): string[] => {
    if (sortOrder === 'alphabetical') {
      return [...axes].sort()
    }
    return [...axes]
  }

  const params = applySort(study.paramNames)
  const objectives = applySort(study.objectiveNames)
  const all = applySort([...study.paramNames, ...study.objectiveNames])

  switch (mode) {
    case 'mode1':
      return { rowAxes: params, colAxes: params }
    case 'mode2':
      return { rowAxes: params, colAxes: objectives }
    case 'mode3':
    default:
      return { rowAxes: all, colAxes: all }
  }
}

/** Extract a numeric value for the given axis name from a trial. */
function getAxisValue(trial: TrialData, axisName: string, study: Study): number | null {
  if (axisName in trial.params) {
    const v = trial.params[axisName]
    return typeof v === 'number' ? v : null
  }
  const objIdx = study.objectiveNames.indexOf(axisName)
  if (objIdx >= 0 && trial.values && objIdx < trial.values.length) {
    return trial.values[objIdx]
  }
  return null
}

// -------------------------------------------------------------------------
// ScatterCell — draws a single scatter thumbnail on <canvas>
// -------------------------------------------------------------------------

interface ScatterCellProps {
  row: number
  col: number
  xAxis: string
  yAxis: string
  trialRows: TrialData[]
  study: Study
}

function ScatterCell({ row, col, xAxis, yAxis, trialRows, study }: ScatterCellProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)

  // Pre-compute points array (memoised so canvas effect doesn't re-extract every render)
  const points = useMemo(() => {
    const pts: [number, number][] = []
    for (const trial of trialRows) {
      const x = getAxisValue(trial, xAxis, study)
      const y = getAxisValue(trial, yAxis, study)
      if (x !== null && y !== null) pts.push([x, y])
    }
    return pts
  }, [trialRows, xAxis, yAxis, study])

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    const size = CELL_PX
    ctx.clearRect(0, 0, size, size)

    // Diagonal cell: show axis label instead of scatter
    if (xAxis === yAxis) {
      ctx.fillStyle = '#f3f4f6'
      ctx.fillRect(0, 0, size, size)
      ctx.fillStyle = '#374151'
      ctx.textAlign = 'center'

      // Word-wrap: split on _ / - / camelCase boundaries, then greedily pack lines
      const tokens = xAxis.split(/[_\-]|(?<=[a-z])(?=[A-Z])/).filter(Boolean)
      const maxWidth = size - 8 // 4px padding each side
      const fontSize = 9
      ctx.font = `${fontSize}px sans-serif`

      const lines: string[] = []
      let cur = tokens[0] ?? ''
      for (let i = 1; i < tokens.length; i++) {
        const candidate = cur + '_' + tokens[i]
        if (ctx.measureText(candidate).width <= maxWidth) {
          cur = candidate
        } else {
          lines.push(cur)
          cur = tokens[i]
        }
      }
      if (cur) lines.push(cur)

      const lineHeight = fontSize + 2
      const totalHeight = lines.length * lineHeight
      const startY = (size - totalHeight) / 2 + fontSize / 2

      for (let i = 0; i < lines.length; i++) {
        ctx.fillText(lines[i], size / 2, startY + i * lineHeight, maxWidth)
      }
      return
    }

    // Background
    ctx.fillStyle = '#f9fafb'
    ctx.fillRect(0, 0, size, size)

    if (points.length === 0) return

    // Compute bounds
    let xMin = Infinity,
      xMax = -Infinity,
      yMin = Infinity,
      yMax = -Infinity
    for (const [x, y] of points) {
      if (x < xMin) xMin = x
      if (x > xMax) xMax = x
      if (y < yMin) yMin = y
      if (y > yMax) yMax = y
    }
    const xRange = xMax - xMin || 1
    const yRange = yMax - yMin || 1
    const pad = 4
    const plotSize = size - pad * 2

    // Draw dots
    ctx.fillStyle = 'rgba(79, 70, 229, 0.55)' // indigo-600
    for (const [x, y] of points) {
      const px = pad + ((x - xMin) / xRange) * plotSize
      const py = pad + (1 - (y - yMin) / yRange) * plotSize
      ctx.beginPath()
      ctx.arc(px, py, 1.5, 0, Math.PI * 2)
      ctx.fill()
    }
  }, [points, xAxis, yAxis])

  return (
    <div
      data-testid={`scatter-cell-${row}-${col}`}
      title={`${xAxis} vs ${yAxis}`}
      style={{
        width: `${CELL_PX}px`,
        height: `${CELL_PX}px`,
        overflow: 'hidden',
        border: '1px solid #e5e7eb',
      }}
    >
      <canvas
        ref={canvasRef}
        width={CELL_PX}
        height={CELL_PX}
        style={{ width: '100%', height: '100%', display: 'block' }}
      />
    </div>
  )
}

// -------------------------------------------------------------------------
// Main component
// -------------------------------------------------------------------------

export function ScatterMatrix({ trialRows, currentStudy }: ScatterMatrixProps) {
  const [mode, setMode] = useState<ScatterMode>('mode1')
  const [sortOrder, setSortOrder] = useState<SortOrder>('alphabetical')

  if (!currentStudy) {
    return (
      <div style={{ padding: '12px' }}>
        <span>Data not loaded</span>
      </div>
    )
  }

  const { rowAxes, colAxes } = getAxesForMode(currentStudy, mode, sortOrder)

  return (
    <div
      data-testid="scatter-matrix"
      style={{ display: 'flex', flexDirection: 'column', height: '100%' }}
    >
      {/* Mode & sort controls */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '8px',
          padding: '8px',
          borderBottom: '1px solid #e5e7eb',
          flexShrink: 0,
        }}
      >
        <div style={{ display: 'flex', gap: '4px' }}>
          {(['mode1', 'mode2', 'mode3'] as ScatterMode[]).map((m) => (
            <button
              key={m}
              data-testid={`mode-btn-${m}`}
              aria-pressed={mode === m}
              onClick={() => setMode(m)}
              style={{
                padding: '4px 10px',
                fontSize: '12px',
                background: mode === m ? '#4f46e5' : '#f3f4f6',
                color: mode === m ? '#fff' : '#374151',
                border: '1px solid',
                borderColor: mode === m ? '#4f46e5' : '#d1d5db',
                borderRadius: '4px',
                cursor: 'pointer',
              }}
            >
              {MODE_LABELS[m]}
            </button>
          ))}
        </div>

        <select
          data-testid="sort-select"
          value={sortOrder}
          onChange={(e) => setSortOrder(e.target.value as SortOrder)}
          style={{
            fontSize: '12px',
            padding: '4px',
            border: '1px solid #d1d5db',
            borderRadius: '4px',
          }}
        >
          {(['alphabetical', 'correlation', 'importance'] as SortOrder[]).map((s) => (
            <option key={s} value={s}>
              {SORT_LABELS[s]}
            </option>
          ))}
        </select>
      </div>

      {/* Grid of scatter cells */}
      <div
        data-testid="scatter-grid"
        style={{
          display: 'grid',
          gridTemplateColumns: `repeat(${colAxes.length}, ${CELL_PX}px)`,
          gridAutoRows: `${CELL_PX}px`,
          overflow: 'auto',
          flex: 1,
        }}
      >
        {rowAxes.flatMap((yAxis, row) =>
          colAxes.map((xAxis, col) => (
            <ScatterCell
              key={`${row}-${col}`}
              row={row}
              col={col}
              xAxis={xAxis}
              yAxis={yAxis}
              trialRows={trialRows}
              study={currentStudy}
            />
          )),
        )}
      </div>
    </div>
  )
}
