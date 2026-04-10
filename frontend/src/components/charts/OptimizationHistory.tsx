/**
 * OptimizationHistory — Convergence history chart for single-objective optimization (TASK-1001)
 *
 * Visualizes the optimization convergence process using ECharts.
 */

import { useMemo, useState } from 'react'
import ReactECharts from 'echarts-for-react'

import { buildChartOption, MODE_LABELS } from './OptimizationHistory.options'
import type { HistoryMode, OptimizationHistoryProps } from './OptimizationHistory.types'

export function OptimizationHistory({
  data,
  direction,
  selectedIndices,
}: OptimizationHistoryProps) {
  const [mode, setMode] = useState<HistoryMode>('best')

  const option = useMemo(
    () => buildChartOption(data, mode, direction, selectedIndices),
    [data, mode, direction, selectedIndices],
  )

  return (
    <div
      data-testid="optimization-history"
      style={{ display: 'flex', flexDirection: 'column', height: '100%' }}
    >
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

      <div style={{ flex: 1 }}>
        <ReactECharts option={option} style={{ height: '100%' }} lazyUpdate />
      </div>
    </div>
  )
}
