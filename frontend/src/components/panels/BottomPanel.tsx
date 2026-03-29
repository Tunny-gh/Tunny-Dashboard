/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 * Documentation.
 */

import { useSelectionStore } from '../../stores/selectionStore'
import { useStudyStore } from '../../stores/studyStore'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Test Coverage: TC-402-05〜07, TC-402-E02
 */
export function BottomPanel() {
  // Documentation.
  const selectedIndices = useSelectionStore((s) => s.selectedIndices)
  const highlighted = useSelectionStore((s) => s.highlighted)
  const setHighlight = useSelectionStore((s) => s.setHighlight)

  // Documentation.
  const currentStudy = useStudyStore((s) => s.currentStudy)
  const trialRows = useStudyStore((s) => s.trialRows)

  // Documentation.
  if (!currentStudy) {
    return (
      <div style={{ padding: '12px' }}>
        <span>Data not loaded</span>
      </div>
    )
  }

  // Documentation.
  const columns = ['trial_id', ...currentStudy.paramNames, ...currentStudy.objectiveNames]

  // Documentation.
  return (
    <div style={{ height: '100%', overflowY: 'auto' }}>
      <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px' }}>
        <thead>
          <tr>
            {columns.map((col) => (
              <th
                key={col}
                style={{
                  padding: '4px 8px',
                  borderBottom: '1px solid #e5e7eb',
                  textAlign: 'left',
                  position: 'sticky',
                  top: 0,
                  background: '#f9fafb',
                }}
              >
                {col}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {/* Documentation. */}
          {Array.from(selectedIndices).map((idx) => (
            <tr
              key={idx}
              data-testid={`trial-row-${idx}`}
              onClick={() => setHighlight(idx)}
              style={{
                cursor: 'pointer',
                background: highlighted === idx ? '#eff6ff' : undefined,
              }}
            >
              {/* Documentation. */}
              <td style={{ padding: '3px 8px', borderBottom: '1px solid #f3f4f6' }}>
                {trialRows[idx]?.trialId ?? idx}
              </td>
              {/* Documentation. */}
              {currentStudy.paramNames.map((name) => {
                const trial = trialRows[idx]
                const raw = trial?.params[name]
                const value = raw !== undefined ? Number(raw) : undefined
                return (
                  <td key={name} style={{ padding: '3px 8px', borderBottom: '1px solid #f3f4f6' }}>
                    {value !== undefined && Number.isFinite(value) ? value.toPrecision(6) : '—'}
                  </td>
                )
              })}
              {/* Documentation. */}
              {currentStudy.objectiveNames.map((name, objIdx) => {
                const trial = trialRows[idx]
                const value = trial?.values[objIdx]
                return (
                  <td key={name} style={{ padding: '3px 8px', borderBottom: '1px solid #f3f4f6' }}>
                    {value !== undefined && Number.isFinite(value) ? value.toFixed(4) : '—'}
                  </td>
                )
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
