/**
 * ObjectivePairMatrix — N×N objective pair matrix (TASK-502)
 *
 * Displays a scatter matrix of objective pairs:
 *   - Diagonal cells: 1D distribution histogram (objective name label)
 *   - Lower-triangle cells: 2D scatter plot (deck.gl ScatterplotLayer)
 *
 * Conforms to REQ-070, REQ-075.
 */

import { DeckGL, ScatterplotLayer } from 'deck.gl'
import type { GpuBuffer } from '../../wasm/gpuBuffer'
import type { Study } from '../../types'

// -------------------------------------------------------------------------
// Props types
// -------------------------------------------------------------------------

export interface ObjectivePairMatrixProps {
  /** GPU buffer — scatter cells show a placeholder when null */
  gpuBuffer: GpuBuffer | null
  /** Current study — used to obtain objective names and count */
  currentStudy: Study | null
  /**
   * Cell click callback: notifies the caller of the selected xAxisName and yAxisName.
   * Used by AppShell (etc.) to assign axes in the 3D view.
   */
  onCellClick?: (xAxisName: string, yAxisName: string) => void
}

// -------------------------------------------------------------------------
// Component
// -------------------------------------------------------------------------

/**
 * Scatter matrix for each pair of objectives.
 *
 * Grid structure:
 *   - Diagonal (row === col): objective name label / histogram placeholder
 *   - Lower triangle (row > col): 2D scatter via deck.gl ScatterplotLayer
 *   - Upper triangle (row < col): empty (reserved for future statistics)
 *
 * Visibility rules:
 *   - 1 or fewer objectives: returns null (component hidden)
 * Documentation.
 *
 * Corresponds to TC-502-01–04, TC-502-E01–E02.
 */
export function ObjectivePairMatrix({
  gpuBuffer,
  currentStudy,
  onCellClick,
}: ObjectivePairMatrixProps) {
  if (!currentStudy) {
    return (
      <div style={{ padding: '12px' }}>
        <span>Data not loaded</span>
      </div>
    )
  }

  const { objectiveNames } = currentStudy
  const n = objectiveNames.length

  // Hide the matrix when there is only one objective
  if (n <= 1) return null

  // -------------------------------------------------------------------------
  // Cell generation
  // -------------------------------------------------------------------------

  /**
   * Generate all n×n cells as a flat array.
   * Index mapping: row = Math.floor(idx / n), col = idx % n
   */
  const cells = Array.from({ length: n * n }, (_, idx) => {
    const row = Math.floor(idx / n)
    const col = idx % n
    const xAxis = objectiveNames[col] // column → x-axis
    const yAxis = objectiveNames[row] // row → y-axis

    return (
      <div
        key={`${row}-${col}`}
        data-testid={`matrix-cell-${row}-${col}`}
        onClick={() => onCellClick?.(xAxis, yAxis)}
        style={{
          cursor: 'pointer',
          position: 'relative',
          border: '1px solid #e5e7eb',
          overflow: 'hidden',
          minHeight: '80px',
        }}
      >
        {row === col ? (
          // Diagonal cell: objective name label (1D histogram planned for future)
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              height: '100%',
              padding: '4px',
              fontSize: '11px',
              fontWeight: 'bold',
              background: '#f9fafb',
              color: '#374151',
            }}
          >
            {objectiveNames[row]}
          </div>
        ) : row > col ? (
          // Lower-triangle cell: 2D scatter via deck.gl ScatterplotLayer
          gpuBuffer ? (
            <DeckGL
              layers={[
                new ScatterplotLayer({
                  id: `scatter-${row}-${col}`,
                  data: { length: gpuBuffer.trialCount },
                  getPosition: (_: unknown, { index }: { index: number }) => [
                    gpuBuffer.positions[index * 2],
                    gpuBuffer.positions[index * 2 + 1],
                    0,
                  ],
                  getColor: [79, 70, 229, 180], // Indigo, semi-transparent
                  getRadius: 3,
                  pickable: false,
                }),
              ]}
              controller={false}
              style={{ width: '100%', height: '100%' }}
            />
          ) : (
            // Placeholder shown when gpuBuffer is not yet loaded
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                height: '100%',
                color: '#9ca3af',
                fontSize: '10px',
              }}
            >
              —
            </div>
          )
        ) : // Upper-triangle cell: currently empty (reserved for future statistics)
        null}
      </div>
    )
  })

  // -------------------------------------------------------------------------
  // Render
  // -------------------------------------------------------------------------

  return (
    // n×n CSS Grid layout
    <div
      data-testid="objective-pair-matrix"
      style={{
        display: 'grid',
        gridTemplateColumns: `repeat(${n}, 1fr)`,
        gridTemplateRows: `repeat(${n}, 1fr)`,
        width: '100%',
        height: '100%',
      }}
    >
      {cells}
    </div>
  )
}
