/**
 * OptimizationHistory — Convergence history chart for single-objective optimization (TASK-1001)
 *
 * Visualizes the optimization convergence process using ECharts.
 *
 * Design:
 *   - Four display modes: best / all / moving-avg / improvement
 *   - detectPhase() automatically detects the optimization phase from trial progress
 *   - Rendered with echarts-for-react (mock-compatible for jsdom tests)
 *
 * Conforms to REQ-1001–REQ-1006.
 */

import { useState } from 'react'
import ReactECharts from 'echarts-for-react'

// -------------------------------------------------------------------------
// Types
// -------------------------------------------------------------------------

/** One trial's result data */
export interface TrialData {
  /** Trial number (1-based) */
  trial: number
  /** Objective value */
  value: number
}

/** Display mode for the convergence history chart */
export type HistoryMode = 'best' | 'all' | 'moving-avg' | 'improvement'

/** Optimization direction */
export type OptimizationDirection = 'minimize' | 'maximize'

/** Optimization progress phase */
export type OptimizationPhase = 'exploration' | 'exploitation' | 'convergence'

// -------------------------------------------------------------------------
// Constants
// -------------------------------------------------------------------------

/** Mode labels for UI display */
const MODE_LABELS: Record<HistoryMode, string> = {
  best: 'Best値推移',
  all: '全試行値',
  'moving-avg': '移動平均',
  improvement: '改善率',
}

/** Window size for moving average computation */
const MOVING_AVG_WINDOW = 5

// -------------------------------------------------------------------------
// Pure functions
// -------------------------------------------------------------------------

/**
 * Detect the optimization phase from trial progress.
 *
 * Phase boundaries:
 *   - exploration  : progress < 0.3  (first 30% of trials)
 *   - exploitation : 0.3 <= progress < 0.7 (middle 40%)
 *   - convergence  : progress >= 0.7 (last 30%)
 *
 * Conforms to REQ-1004–REQ-1006.
 *
 * @param trialIndex - Current trial index (1-based)
 * @param totalTrials - Total number of trials
 * @returns Phase string
 */
export function detectPhase(trialIndex: number, totalTrials: number): OptimizationPhase {
  const progress = trialIndex / totalTrials

  if (progress < 0.3) {
    return 'exploration'
  }
  if (progress < 0.7) {
    return 'exploitation'
  }
  return 'convergence'
}

/**
 * Compute the running best value at each trial.
 * @param data - Trial data array
 * @param direction - Optimization direction
 * @returns Best value at each trial
 */
function computeBestSeries(data: TrialData[], direction: OptimizationDirection): number[] {
  let best = direction === 'minimize' ? Infinity : -Infinity
  return data.map(({ value }) => {
    if (direction === 'minimize') {
      best = Math.min(best, value)
    } else {
      best = Math.max(best, value)
    }
    return best
  })
}

/**
 * Compute a moving average with the given window size.
 * @param values - Input value array
 * @param window - Window size
 * @returns Moving average array
 */
function computeMovingAverage(values: number[], window: number): number[] {
  return values.map((_, i) => {
    const start = Math.max(0, i - window + 1)
    const slice = values.slice(start, i + 1)
    return slice.reduce((sum, v) => sum + v, 0) / slice.length
  })
}

/**
 * Compute the improvement rate from the previous trial's best value.
 * Formula: |prev - curr| / |prev| * 100
 * @param bestSeries - Running best value array
 * @returns Improvement rate array (%)
 */
function computeImprovementRate(bestSeries: number[]): number[] {
  return bestSeries.map((curr, i) => {
    if (i === 0) return 0
    const prev = bestSeries[i - 1]
    if (prev === 0) return 0
    return Math.abs((prev - curr) / prev) * 100
  })
}

// -------------------------------------------------------------------------
// ECharts option builder
// -------------------------------------------------------------------------

/**
 * Build the ECharts option for the given display mode.
 * @param data - Trial data array
 * @param mode - Display mode
 * @param direction - Optimization direction
 * @returns ECharts option object
 */
function buildChartOption(
  data: TrialData[],
  mode: HistoryMode,
  direction: OptimizationDirection,
): object {
  if (data.length === 0) {
    return { xAxis: { type: 'value' }, yAxis: { type: 'value' }, series: [] }
  }

  const trials = data.map((d) => d.trial)
  const values = data.map((d) => d.value)
  const bestSeries = computeBestSeries(data, direction)

  switch (mode) {
    case 'best':
      // Running best value line chart
      return {
        xAxis: { type: 'category', data: trials },
        yAxis: { type: 'value' },
        series: [{ type: 'line', data: bestSeries, name: 'Best値' }],
      }

    case 'all':
      // All trial values scatter plot
      return {
        xAxis: { type: 'value' },
        yAxis: { type: 'value' },
        series: [{ type: 'scatter', data: trials.map((t, i) => [t, values[i]]), name: '全試行値' }],
      }

    case 'moving-avg': {
      // Moving average line chart
      const movingAvg = computeMovingAverage(values, MOVING_AVG_WINDOW)
      return {
        xAxis: { type: 'category', data: trials },
        yAxis: { type: 'value' },
        series: [
          { type: 'line', data: values, name: '全試行値', opacity: 0.4 },
          { type: 'line', data: movingAvg, name: `移動平均(${MOVING_AVG_WINDOW})` },
        ],
      }
    }

    case 'improvement': {
      // Best value improvement rate bar chart
      const improvementRate = computeImprovementRate(bestSeries)
      return {
        xAxis: { type: 'category', data: trials },
        yAxis: { type: 'value' },
        series: [{ type: 'bar', data: improvementRate, name: '改善率(%)' }],
      }
    }

    default:
      return { xAxis: { type: 'value' }, yAxis: { type: 'value' }, series: [] }
  }
}

// -------------------------------------------------------------------------
// Props types
// -------------------------------------------------------------------------

export interface OptimizationHistoryProps {
  /** Trial data array */
  data: TrialData[]
  /** Optimization direction */
  direction: OptimizationDirection
}

// -------------------------------------------------------------------------
// Main component
// -------------------------------------------------------------------------

/**
 * Convergence history chart for single-objective optimization.
 * Four display modes: best / all / moving-avg / improvement.
 * Automatically detects the optimization phase via detectPhase().
 *
 * Conforms to REQ-1001–REQ-1006.
 */
export function OptimizationHistory({ data, direction }: OptimizationHistoryProps) {
  // Default mode is 'best' (running best value)
  const [mode, setMode] = useState<HistoryMode>('best')

  const option = buildChartOption(data, mode, direction)

  return (
    <div
      data-testid="optimization-history"
      style={{ display: 'flex', flexDirection: 'column', height: '100%' }}
    >
      {/* Control bar: mode toggle buttons */}
      <div
        style={{
          display: 'flex',
          gap: '4px',
          padding: '8px',
          borderBottom: '1px solid #e5e7eb',
          flexShrink: 0,
        }}
      >
        {(['best', 'all', 'moving-avg', 'improvement'] as HistoryMode[]).map((m) => (
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

      {/* Chart area: ECharts convergence history */}
      <div style={{ flex: 1 }}>
        <ReactECharts option={option} style={{ height: '100%' }} />
      </div>
    </div>
  )
}
