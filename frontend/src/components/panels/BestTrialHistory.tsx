/**
 * Documentation.
 *
 * Documentation.
 * Design:
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */

import type { TrialData, OptimizationDirection } from '../charts/OptimizationHistory'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */
function extractBestTrials(data: TrialData[], direction: OptimizationDirection): TrialData[] {
  let best = direction === 'minimize' ? Infinity : -Infinity
  const result: TrialData[] = []

  for (const trial of data) {
    // Documentation.
    const isBetter = direction === 'minimize' ? trial.value < best : trial.value > best

    if (isBetter) {
      best = trial.value
      result.push(trial)
    }
  }

  return result
}

// -------------------------------------------------------------------------
// Props Type definitions
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
export interface BestTrialHistoryProps {
  /** Documentation. */
  data: TrialData[]
  /** Documentation. */
  direction: OptimizationDirection
  /** Documentation. */
  onRowClick?: (trial: TrialData) => void
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */
export function BestTrialHistory({ data, direction, onRowClick }: BestTrialHistoryProps) {
  // Documentation.
  const bestTrials = extractBestTrials(data, direction)

  return (
    <div data-testid="best-trial-table" style={{ overflow: 'auto', maxHeight: '300px' }}>
      {/* Documentation. */}
      <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px' }}>
        <thead>
          <tr style={{ background: '#f3f4f6', borderBottom: '1px solid #e5e7eb' }}>
            <th style={{ padding: '6px 12px', textAlign: 'left', fontWeight: 600 }}>Trial #</th>
            <th style={{ padding: '6px 12px', textAlign: 'right', fontWeight: 600 }}>Objective</th>
          </tr>
        </thead>
        <tbody>
          {/* Documentation. */}
          {bestTrials.map((trial) => (
            <tr
              key={trial.trial}
              data-testid={`best-row-${trial.trial}`}
              onClick={() => onRowClick?.(trial)}
              style={{
                borderBottom: '1px solid #f3f4f6',
                cursor: onRowClick ? 'pointer' : 'default',
              }}
            >
              {/* Documentation. */}
              <td style={{ padding: '5px 12px' }}>{trial.trial}</td>

              {/* Documentation. */}
              <td style={{ padding: '5px 12px', textAlign: 'right', fontFamily: 'monospace' }}>
                {trial.value.toFixed(4)}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
