/**
 * PDPChart — Partial Dependence Plot (PDP) component (TASK-804)
 *
 * Visualizes a Ridge-based simplified PDP using ECharts.
 */

import ReactECharts from 'echarts-for-react'

import { buildPdp2dOption, buildPdpOption } from './PDPChart.options'
import { getModelQuality, R2_GOOD_THRESHOLD } from './PDPChart.quality'
import type { PDPChartProps } from './PDPChart.types'

export function PDPChart({
  data1d,
  data2d,
  isLoading = false,
  useOnnx = false,
  highlightedIndices = [],
  onOnnxRequest,
}: PDPChartProps) {
  if (isLoading) {
    return (
      <div
        data-testid="pdp-chart"
        style={{ padding: '24px', display: 'flex', alignItems: 'center', gap: '12px' }}
      >
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
        <span style={{ fontSize: '13px', color: '#6b7280' }}>Computing PDP...</span>
      </div>
    )
  }

  if (!data1d && !data2d) {
    return (
      <div data-testid="pdp-chart" style={{ padding: '12px' }}>
        <span style={{ fontSize: '13px', color: '#6b7280' }}>Data not loaded</span>
      </div>
    )
  }

  const rSquared = data1d?.rSquared ?? data2d?.rSquared ?? 0
  const quality = getModelQuality(rSquared)
  const qualityLabel =
    quality === 'Good' ? '✓ Good' : quality === 'Caution' ? '△ Caution' : '✕ Not Recommended'
  const qualityColor =
    quality === 'Good' ? '#16a34a' : quality === 'Caution' ? '#d97706' : '#dc2626'

  const highlightedSet = new Set(highlightedIndices)

  return (
    <div data-testid="pdp-chart" style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
      {!useOnnx && (
        <div
          data-testid="linear-approx-banner"
          style={{
            padding: '8px 12px',
            background: '#fef3c7',
            border: '1px solid #f59e0b',
            borderRadius: '4px',
            fontSize: '12px',
            color: '#92400e',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
          }}
        >
          <span>Displaying linear approximation / Load .onnx file for higher accuracy PDP</span>
          {onOnnxRequest && (
            <button
              data-testid="onnx-request-btn"
              onClick={onOnnxRequest}
              style={{
                padding: '2px 8px',
                fontSize: '11px',
                background: '#f59e0b',
                color: '#fff',
                border: 'none',
                borderRadius: '3px',
                cursor: 'pointer',
              }}
            >
              Load .onnx
            </button>
          )}
        </div>
      )}

      {rSquared < R2_GOOD_THRESHOLD && (
        <div
          data-testid="r2-warning-badge"
          style={{
            padding: '4px 10px',
            background: '#fef2f2',
            border: '1px solid #fca5a5',
            borderRadius: '4px',
            fontSize: '12px',
            color: '#dc2626',
          }}
        >
          Caution: PDP interpretation may be unreliable (R²={rSquared.toFixed(2)})
        </div>
      )}

      <div
        data-testid="model-quality-panel"
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '12px',
          padding: '6px 10px',
          background: '#f9fafb',
          border: '1px solid #e5e7eb',
          borderRadius: '4px',
          fontSize: '12px',
        }}
      >
        <span style={{ color: '#6b7280' }}>R²:</span>
        <span data-testid="r2-value" style={{ fontWeight: 600 }}>
          {rSquared.toFixed(3)}
        </span>
        <span data-testid="quality-label" style={{ color: qualityColor, fontWeight: 600 }}>
          {qualityLabel}
        </span>
      </div>

      {data1d && !data2d && (
        <ReactECharts option={buildPdpOption(data1d, highlightedSet)} style={{ height: '300px' }} />
      )}

      {data2d && <ReactECharts option={buildPdp2dOption(data2d)} style={{ height: '300px' }} />}
    </div>
  )
}
