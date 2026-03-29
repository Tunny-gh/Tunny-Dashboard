/**
 * HypervolumeHistory — ECharts line chart for hypervolume history (TASK-501)
 *
 * Displays the hypervolume trend over trial numbers as a line chart.
 */

import ReactECharts from 'echarts-for-react'
import { EmptyState } from '../common/EmptyState'

// -------------------------------------------------------------------------
// Props types
// -------------------------------------------------------------------------

export interface HypervolumeDataPoint {
  trial: number
  hypervolume: number
}

export interface HypervolumeHistoryProps {
  /** Hypervolume history data — shows empty state when the array is empty */
  data: HypervolumeDataPoint[]
}

// -------------------------------------------------------------------------
// Component
// -------------------------------------------------------------------------

/**
 * Line chart showing hypervolume history over trials using ECharts.
 * Corresponds to TC-501-05, TC-501-06, TC-501-E03.
 */
export function HypervolumeHistory({ data }: HypervolumeHistoryProps) {
  if (data.length === 0) {
    return <EmptyState />
  }

  const option = {
    xAxis: {
      type: 'value',
      name: 'Trial',
    },
    yAxis: {
      type: 'value',
      name: 'Hypervolume',
    },
    series: [
      {
        type: 'line',
        // Convert {trial, hypervolume} objects to [trial, hypervolume] pairs
        data: data.map((d) => [d.trial, d.hypervolume]),
      },
    ],
    tooltip: {
      trigger: 'axis',
    },
  }

  return <ReactECharts option={option} style={{ width: '100%', height: '100%' }} />
}
