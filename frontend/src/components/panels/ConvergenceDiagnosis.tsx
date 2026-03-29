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

/** Documentation. */
const MINIMUM_TRIALS = 10

/** Documentation. */
const TAIL_WINDOW_RATE = 0.2

/** Documentation. */
const CONVERGED_THRESHOLD = 0.001 // 0.1%

/** Documentation. */
const CONVERGING_THRESHOLD = 0.01 // 1%

// -------------------------------------------------------------------------
// Type definitions
// -------------------------------------------------------------------------

/** Documentation. */
export type ConvergenceStatus = 'converged' | 'converging' | 'not-converged' | 'insufficient'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */
export function diagnoseConvergence(
  data: TrialData[],
  direction: OptimizationDirection,
): ConvergenceStatus {
  // Documentation.
  if (data.length < MINIMUM_TRIALS) {
    return 'insufficient'
  }

  // Documentation.
  let best = direction === 'minimize' ? Infinity : -Infinity
  const bestSeries = data.map(({ value }) => {
    if (direction === 'minimize') {
      best = Math.min(best, value)
    } else {
      best = Math.max(best, value)
    }
    return best
  })

  // Documentation.
  const tailStart = Math.floor(data.length * (1 - TAIL_WINDOW_RATE))
  const tailBest = bestSeries.slice(tailStart)

  // Documentation.
  const firstBest = tailBest[0]
  const lastBest = tailBest[tailBest.length - 1]

  // Documentation.
  const improvementRate =
    firstBest !== 0 ? Math.abs((firstBest - lastBest) / firstBest) : Math.abs(firstBest - lastBest)

  // Documentation.
  if (improvementRate < CONVERGED_THRESHOLD) {
    return 'converged'
  }
  if (improvementRate < CONVERGING_THRESHOLD) {
    return 'converging'
  }
  return 'not-converged'
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/** Documentation. */
interface BadgeConfig {
  testId: string
  label: string
  color: string
  background: string
}

/** Documentation. */
const BADGE_CONFIG: Record<ConvergenceStatus, BadgeConfig> = {
  converged: {
    testId: 'badge-converged',
    label: 'Converged',
    color: '#fff',
    background: '#16a34a', // Documentation.
  },
  converging: {
    testId: 'badge-converging',
    label: 'Converging',
    color: '#92400e',
    background: '#fbbf24', // Documentation.
  },
  'not-converged': {
    testId: 'badge-not-converged',
    label: 'Not Converged',
    color: '#fff',
    background: '#dc2626', // Documentation.
  },
  insufficient: {
    testId: 'badge-insufficient',
    label: 'Insufficient',
    color: '#374151',
    background: '#e5e7eb', // Documentation.
  },
}

// -------------------------------------------------------------------------
// Props Type definitions
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
export interface ConvergenceDiagnosisProps {
  /** Documentation. */
  data: TrialData[]
  /** Documentation. */
  direction: OptimizationDirection
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
export function ConvergenceDiagnosis({ data, direction }: ConvergenceDiagnosisProps) {
  // Documentation.
  const status = diagnoseConvergence(data, direction)

  // Documentation.
  if (status === 'insufficient') {
    return (
      <div
        data-testid="convergence-diagnosis"
        style={{ padding: '12px', display: 'flex', alignItems: 'center', gap: '8px' }}
      >
        <span style={{ fontSize: '13px', color: '#6b7280' }}>Convergence:</span>
        <span
          data-testid="badge-insufficient"
          style={{
            fontSize: '12px',
            padding: '2px 8px',
            borderRadius: '9999px',
            background: BADGE_CONFIG.insufficient.background,
            color: BADGE_CONFIG.insufficient.color,
          }}
        >
          Insufficient (not enough trials)
        </span>
      </div>
    )
  }

  const config = BADGE_CONFIG[status]

  return (
    <div
      data-testid="convergence-diagnosis"
      style={{ padding: '12px', display: 'flex', alignItems: 'center', gap: '8px' }}
    >
      {/* Documentation. */}
      <span style={{ fontSize: '13px', color: '#6b7280' }}>Convergence:</span>

      {/* Documentation. */}
      <span
        data-testid={config.testId}
        style={{
          fontSize: '12px',
          fontWeight: 600,
          padding: '2px 8px',
          borderRadius: '9999px',
          background: config.background,
          color: config.color,
        }}
      >
        {config.label}
      </span>
    </div>
  )
}
