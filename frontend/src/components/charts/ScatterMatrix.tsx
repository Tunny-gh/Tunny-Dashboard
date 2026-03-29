/**
 * Documentation.
 *
 * Documentation.
 * Design:
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */

import { useState, useEffect } from 'react'
import type { ScatterMatrixEngine, ScatterCellSize } from '../../wasm/workers/ScatterMatrixEngine'
import type { Study } from '../../types'

// -------------------------------------------------------------------------
// Constants・Type definitions
// -------------------------------------------------------------------------

/** Documentation. */
export type ScatterMode = 'mode1' | 'mode2' | 'mode3'

/** Documentation. */
export type SortOrder = 'alphabetical' | 'correlation' | 'importance'

/** Documentation. */
const MODE_LABELS: Record<ScatterMode, string> = {
  mode1: 'Params×Params', // Documentation.
  mode2: 'Params×Objectives', // Documentation.
  mode3: 'All', // Documentation.
}

/** Documentation. */
const SORT_LABELS: Record<SortOrder, string> = {
  alphabetical: 'Alphabetical', // Documentation.
  correlation: 'By Correlation', // Documentation.
  importance: 'By Importance', // Documentation.
}

// -------------------------------------------------------------------------
// Props Type definitions
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
export interface ScatterMatrixProps {
  /** Documentation. */
  engine: ScatterMatrixEngine | null
  /** Documentation. */
  currentStudy: Study | null
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * @param study - current Study
 * Documentation.
 * Documentation.
 * Documentation.
 */
export function getAxesForMode(
  study: Study,
  mode: ScatterMode,
  sortOrder: SortOrder,
): { rowAxes: string[]; colAxes: string[] } {
  // Documentation.
  const applySort = (axes: string[]): string[] => {
    if (sortOrder === 'alphabetical') {
      return [...axes].sort()
    }
    // Documentation.
    return [...axes]
  }

  const params = applySort(study.paramNames)
  const objectives = applySort(study.objectiveNames)
  const all = applySort([...study.paramNames, ...study.objectiveNames])

  switch (mode) {
    case 'mode1':
      // Documentation.
      return { rowAxes: params, colAxes: params }
    case 'mode2':
      // Documentation.
      return { rowAxes: params, colAxes: objectives }
    case 'mode3':
    default:
      // Documentation.
      return { rowAxes: all, colAxes: all }
  }
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
interface ScatterCellProps {
  row: number
  col: number
  xAxis: string
  yAxis: string
  engine: ScatterMatrixEngine | null
  size?: ScatterCellSize
}

/**
 * Documentation.
 * 【RenderingState】:
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */
function ScatterCell({ row, col, xAxis, yAxis, engine, size = 'thumbnail' }: ScatterCellProps) {
  const [imageUrl, setImageUrl] = useState<string | null>(null)
  const [error, setError] = useState(false)

  useEffect(() => {
    // Documentation.
    if (!engine) return

    let cancelled = false

    // Documentation.
    engine
      .renderCell(row, col, size)
      .then((imageData) => {
        if (cancelled || !imageData) return

        // Documentation.
        const canvas = document.createElement('canvas')
        canvas.width = imageData.width
        canvas.height = imageData.height
        const ctx = canvas.getContext('2d')
        ctx?.putImageData(imageData, 0, 0)
        setImageUrl(canvas.toDataURL())
      })
      .catch(() => {
        // Documentation.
        if (!cancelled) setError(true)
      })

    // Documentation.
    return () => {
      cancelled = true
    }
  }, [engine, row, col, size])

  const pixelSize = size === 'thumbnail' ? 80 : size === 'preview' ? 300 : 600

  return (
    <div
      data-testid={`scatter-cell-${row}-${col}`}
      title={`${xAxis} vs ${yAxis} — Click to expand`}
      style={{
        width: `${pixelSize}px`,
        height: `${pixelSize}px`,
        position: 'relative',
        overflow: 'hidden',
        cursor: 'pointer',
        border: '1px solid #e5e7eb',
      }}
    >
      {error ? (
        // Documentation.
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            width: '100%',
            height: '100%',
            fontSize: '24px',
          }}
        >
          ❌
        </div>
      ) : imageUrl ? (
        // Documentation.
        <img
          src={imageUrl}
          alt={`${xAxis} vs ${yAxis}`}
          style={{ width: '100%', height: '100%', objectFit: 'fill' }}
        />
      ) : (
        // Documentation.
        <div
          style={{
            width: '100%',
            height: '100%',
            background: '#e5e7eb',
          }}
        />
      )}
    </div>
  )
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Test Coverage: TC-702-01〜05, TC-702-E01〜E02
 */
export function ScatterMatrix({ engine, currentStudy }: ScatterMatrixProps) {
  // Documentation.
  const [mode, setMode] = useState<ScatterMode>('mode1')

  // Documentation.
  const [sortOrder, setSortOrder] = useState<SortOrder>('alphabetical')

  // Documentation.
  if (!currentStudy) {
    return (
      <div style={{ padding: '12px' }}>
        <span>Data not loaded</span>
      </div>
    )
  }

  // Documentation.
  const { rowAxes, colAxes } = getAxesForMode(currentStudy, mode, sortOrder)

  // -------------------------------------------------------------------------
  // Rendering
  // -------------------------------------------------------------------------

  return (
    <div
      data-testid="scatter-matrix"
      style={{ display: 'flex', flexDirection: 'column', height: '100%' }}
    >
      {/* Documentation. */}
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
        {/* Documentation. */}
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

        {/* Documentation. */}
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

      {/* Documentation. */}
      <div
        data-testid="scatter-grid"
        style={{
          display: 'grid',
          gridTemplateColumns: `repeat(${colAxes.length}, 80px)`,
          gridAutoRows: '80px',
          overflow: 'auto',
          flex: 1,
        }}
      >
        {/* Documentation. */}
        {rowAxes.flatMap((yAxis, row) =>
          colAxes.map((xAxis, col) => (
            <ScatterCell
              key={`${row}-${col}`}
              row={row}
              col={col}
              xAxis={xAxis}
              yAxis={yAxis}
              engine={engine}
            />
          )),
        )}
      </div>
    </div>
  )
}
