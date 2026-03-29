/**
 * EdfPlot — Empirical Cumulative Distribution Function chart (equivalent to Optuna-Dashboard EDF)
 *
 * Visualizes the empirical CDF of objective values as a step line.
 *
 * Design:
 *   - computeEdf() sorts ascending and computes cumulative probability (pure function, easy to test)
 *   - Supports multiple objectives simultaneously (useful for multi-objective comparison)
 *   - ECharts step: 'end' represents the staircase shape of a CDF
 */

import ReactECharts from 'echarts-for-react'
import { EmptyState } from '../common/EmptyState'

// -------------------------------------------------------------------------
// Types
// -------------------------------------------------------------------------

/** EDF data for one objective */
export interface EdfSeries {
  /** Objective name (shown as legend label) */
  name: string
  /** Objective values (unsorted is fine) */
  values: number[]
}

/** Props */
export interface EdfPlotProps {
  /** Series to display (pass multiple for multi-objective) */
  series: EdfSeries[]
}

// -------------------------------------------------------------------------
// Pure functions
// -------------------------------------------------------------------------

/**
 * Compute empirical CDF coordinates from a list of values.
 *
 * Steps:
 *   1. Sort ascending
 *   2. Cumulative probability of each point = rank / total
 *
 * @param values - Objective values (any order)
 * @returns Array of [value, cumulative probability] pairs
 */
export function computeEdf(values: number[]): [number, number][] {
  if (values.length === 0) return []

  // Sort without mutating the original array
  const sorted = [...values].sort((a, b) => a - b)
  const n = sorted.length

  // Cumulative probability for the i-th point (1-based) = i / n
  return sorted.map((v, i) => [v, (i + 1) / n])
}

// -------------------------------------------------------------------------
// Component
// -------------------------------------------------------------------------

/**
 * Draws the empirical CDF of objective values as a step line.
 * Pass multiple series to display several lines on the same chart.
 */
export function EdfPlot({ series }: EdfPlotProps) {
  // Show empty state when there are no series or all series have no values
  if (series.length === 0 || series.every((s) => s.values.length === 0)) {
    return <EmptyState />
  }

  const option = {
    tooltip: {
      trigger: 'axis',
      formatter: (params: Array<{ seriesName: string; value: [number, number] }>) =>
        params.map((p) => `${p.seriesName}: ${p.value[0].toFixed(4)}`).join('<br/>'),
    },
    legend: {
      data: series.map((s) => s.name),
    },
    xAxis: {
      type: 'value',
      name: 'Objective Value',
      nameLocation: 'center',
      nameGap: 24,
    },
    yAxis: {
      type: 'value',
      name: 'Cumulative Probability',
      nameLocation: 'center',
      nameGap: 40,
      min: 0,
      max: 1,
    },
    series: series.map((s) => ({
      name: s.name,
      type: 'line',
      step: 'end', // Horizontal segments give the CDF staircase appearance
      data: computeEdf(s.values),
      symbolSize: 0, // Hide point markers, show line only
    })),
  }

  return (
    <div data-testid="edf-plot" style={{ height: '100%' }}>
      <ReactECharts option={option} style={{ height: '100%' }} />
    </div>
  )
}

export default EdfPlot
