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

import { useState } from 'react'
import ReactECharts from 'echarts-for-react'

// -------------------------------------------------------------------------
// Type definitions
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
export interface SensitivityData {
  /** Documentation. */
  paramNames: string[]
  /** Documentation. */
  objectiveNames: string[]
  /** Documentation. */
  spearman: number[][]
  /** Documentation. */
  ridge: { beta: number[]; rSquared: number }[]
}

/** Documentation. */
export type SensitivityMetric = 'spearman' | 'beta'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/** Documentation. */
const METRIC_LABELS: Record<SensitivityMetric, string> = {
  spearman: 'Spearman',
  beta: 'Ridge β',
}

// -------------------------------------------------------------------------
// Props Type definitions
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
export interface SensitivityHeatmapProps {
  /** Documentation. */
  data: SensitivityData | null
  /** Documentation. */
  metric: SensitivityMetric
  /** Documentation. */
  threshold: number
  /** Documentation. */
  isLoading?: boolean
  /** Documentation. */
  onThresholdChange?: (threshold: number) => void
  /** Documentation. */
  onCellClick?: (paramName: string, objectiveName: string) => void
}

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
function buildHeatmapOption(
  data: SensitivityData,
  metric: SensitivityMetric,
  threshold: number,
): object {
  const { paramNames, objectiveNames } = data

  // Documentation.
  const matrix: number[][] =
    metric === 'spearman'
      ? data.spearman
      : // Documentation.
        paramNames.map((_, pIdx) =>
          objectiveNames.map((_, oIdx) => data.ridge[oIdx]?.beta[pIdx] ?? 0),
        )

  // Documentation.
  const heatmapData: [number, number, number][] = []
  for (let pIdx = 0; pIdx < paramNames.length; pIdx++) {
    // Documentation.
    const maxAbs = Math.max(...(matrix[pIdx] ?? [0]).map(Math.abs))
    const isFiltered = maxAbs < threshold

    for (let oIdx = 0; oIdx < objectiveNames.length; oIdx++) {
      const value = matrix[pIdx]?.[oIdx] ?? 0
      // Documentation.
      heatmapData.push([oIdx, pIdx, isFiltered ? 0 : value])
    }
  }

  return {
    tooltip: {
      trigger: 'item',
      formatter: (params: { data: [number, number, number] }) => {
        const [oIdx, pIdx, val] = params.data
        return `${paramNames[pIdx]} × ${objectiveNames[oIdx]}: ${val.toFixed(3)}`
      },
    },
    xAxis: {
      type: 'category',
      data: objectiveNames,
      splitArea: { show: true },
    },
    yAxis: {
      type: 'category',
      data: paramNames,
      splitArea: { show: true },
    },
    visualMap: {
      min: -1,
      max: 1,
      calculable: true,
      orient: 'horizontal',
      left: 'center',
      bottom: '0%',
      // Documentation.
      inRange: {
        color: ['#2563eb', '#ffffff', '#dc2626'],
      },
    },
    series: [
      {
        name: METRIC_LABELS[metric],
        type: 'heatmap',
        data: heatmapData,
        label: {
          show: true,
          formatter: (params: { data: [number, number, number] }) => params.data[2].toFixed(2),
        },
        emphasis: {
          itemStyle: { shadowBlur: 10, shadowColor: 'rgba(0, 0, 0, 0.5)' },
        },
      },
    ],
  }
}

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
export function SensitivityHeatmap({
  data,
  metric,
  threshold,
  isLoading = false,
  onThresholdChange,
  onCellClick,
}: SensitivityHeatmapProps) {
  // Documentation.
  const [activeMetric, setActiveMetric] = useState<SensitivityMetric>(metric)

  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  if (isLoading) {
    return (
      <div
        data-testid="sensitivity-heatmap"
        style={{ padding: '24px', display: 'flex', alignItems: 'center', gap: '12px' }}
      >
        {/* Documentation. */}
        <div
          style={{
            width: '16px',
            height: '16px',
            borderRadius: '50%',
            border: '2px solid #4f46e5',
            borderTopColor: 'transparent',
            animation: 'spin 1s linear infinite',
          }}
        />
        <span style={{ fontSize: '13px', color: '#6b7280' }}>Computing (WASM)...</span>
      </div>
    )
  }

  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  if (!data) {
    return (
      <div data-testid="sensitivity-heatmap" style={{ padding: '12px' }}>
        <span style={{ fontSize: '13px', color: '#6b7280' }}>Data not loaded</span>
      </div>
    )
  }

  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  const option = buildHeatmapOption(data, activeMetric, threshold)

  // -------------------------------------------------------------------------
  // Rendering
  // -------------------------------------------------------------------------

  return (
    <div
      data-testid="sensitivity-heatmap"
      style={{ display: 'flex', flexDirection: 'column', height: '100%' }}
    >
      {/* Documentation. */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '12px',
          padding: '8px',
          borderBottom: '1px solid #e5e7eb',
          flexShrink: 0,
        }}
      >
        {/* Documentation. */}
        <div style={{ display: 'flex', gap: '4px' }}>
          {(['spearman', 'beta'] as SensitivityMetric[]).map((m) => (
            <button
              key={m}
              data-testid={`metric-btn-${m}`}
              aria-pressed={activeMetric === m}
              onClick={() => setActiveMetric(m)}
              style={{
                padding: '4px 10px',
                fontSize: '12px',
                background: activeMetric === m ? '#4f46e5' : '#f3f4f6',
                color: activeMetric === m ? '#fff' : '#374151',
                border: '1px solid',
                borderColor: activeMetric === m ? '#4f46e5' : '#d1d5db',
                borderRadius: '4px',
                cursor: 'pointer',
              }}
            >
              {METRIC_LABELS[m]}
            </button>
          ))}
        </div>

        {/* Documentation. */}
        <label style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '12px' }}>
          <span style={{ color: '#6b7280' }}>Threshold:</span>
          <input
            data-testid="threshold-slider"
            type="range"
            min="0"
            max="1"
            step="0.05"
            value={threshold}
            onChange={(e) => onThresholdChange?.(parseFloat(e.target.value))}
            style={{ width: '80px' }}
          />
          <span style={{ color: '#374151', minWidth: '32px' }}>{threshold.toFixed(2)}</span>
        </label>
      </div>

      {/* Documentation. */}
      <div
        style={{ flex: 1 }}
        onClick={(e) => {
          // Documentation.
          // Documentation.
          void e
        }}
      >
        <ReactECharts
          option={option}
          style={{ height: '100%' }}
          onEvents={
            onCellClick
              ? {
                  click: (params: { data?: [number, number, number] }) => {
                    if (params.data) {
                      const [oIdx, pIdx] = params.data
                      const paramName = data.paramNames[pIdx]
                      const objectiveName = data.objectiveNames[oIdx]
                      if (paramName && objectiveName) {
                        onCellClick(paramName, objectiveName)
                      }
                    }
                  },
                }
              : undefined
          }
        />
      </div>
    </div>
  )
}
