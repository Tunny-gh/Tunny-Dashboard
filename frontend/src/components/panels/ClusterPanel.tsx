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

import { useState } from 'react'
import type { ChangeEvent } from 'react'
import ReactECharts from 'echarts-for-react'

// -------------------------------------------------------------------------
// Type definitions
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 */
export type ClusterSpace = 'param' | 'objective' | 'all'

/**
 * Documentation.
 */
export interface ElbowResultData {
  /** Documentation. */
  wcssPerK: number[]
  /** Documentation. */
  recommendedK: number
}

/**
 * Documentation.
 */
export interface ClusterPanelProps {
  /** Documentation. */
  onRunClustering: (space: ClusterSpace, k: number) => void
  /** Documentation. */
  isRunning?: boolean
  /** Documentation. */
  progress?: number
  /** Documentation. */
  elbowResult?: ElbowResultData | null
  /** Documentation. */
  error?: string | null
}

// -------------------------------------------------------------------------
// Constants
// -------------------------------------------------------------------------

/** Documentation. */
const K_MIN = 2

/** Documentation. */
const K_DEFAULT = 4

/** Documentation. */
const SPACE_LABELS: Record<ClusterSpace, string> = {
  param: 'Parameters',
  objective: 'Objectives',
  all: 'All',
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 *
 * Design:
 * Documentation.
 * Documentation.
 */
function buildElbowOption(elbow: ElbowResultData): object {
  const kStart = 2
  const kLabels = elbow.wcssPerK.map((_, i) => String(i + kStart))
  const recommendedIdx = elbow.recommendedK - kStart
  const recommendedWcss =
    recommendedIdx >= 0 && recommendedIdx < elbow.wcssPerK.length
      ? elbow.wcssPerK[recommendedIdx]
      : undefined

  return {
    tooltip: { trigger: 'axis' },
    xAxis: {
      type: 'category',
      data: kLabels,
      name: 'k',
      nameLocation: 'middle',
      nameGap: 20,
    },
    yAxis: {
      type: 'value',
      name: 'WCSS',
    },
    series: [
      {
        type: 'line',
        data: elbow.wcssPerK,
        lineStyle: { color: '#4f46e5', width: 2 },
        symbolSize: 5,
        ...(recommendedWcss !== undefined
          ? {
              markPoint: {
                data: [
                  {
                    coord: [String(elbow.recommendedK), recommendedWcss],
                    symbolSize: 14,
                    itemStyle: { color: '#e11d48' },
                    label: {
                      show: true,
                      formatter: `k=${elbow.recommendedK}`,
                      position: 'top',
                      fontSize: 11,
                      color: '#e11d48',
                    },
                  },
                ],
              },
            }
          : {}),
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
 */
export function ClusterPanel({
  onRunClustering,
  isRunning = false,
  progress = 0,
  elbowResult,
  error,
}: ClusterPanelProps) {
  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  /** Documentation. */
  const [space, setSpace] = useState<ClusterSpace>('param')
  /** Documentation. */
  const [k, setK] = useState<number>(K_DEFAULT)
  /** Documentation. */
  const [kError, setKError] = useState<string | null>(null)

  // -------------------------------------------------------------------------
  // event handler
  // -------------------------------------------------------------------------

  /**
   * Documentation.
   */
  const handleKChange = (e: ChangeEvent<HTMLInputElement>) => {
    const val = parseInt(e.target.value, 10)
    const newK = isNaN(val) ? K_DEFAULT : val
    setK(newK)
    if (newK >= K_MIN) setKError(null)
  }

  /**
   * Documentation.
   * Documentation.
   */
  const handleRun = () => {
    if (k < K_MIN) {
      setKError('Please specify k >= 2')
      return
    }
    setKError(null)
    onRunClustering(space, k)
  }

  // -------------------------------------------------------------------------
  // Rendering
  // -------------------------------------------------------------------------

  return (
    <div
      data-testid="cluster-panel"
      style={{ padding: '12px', display: 'flex', flexDirection: 'column', gap: '12px' }}
    >
      {/* Documentation. */}
      <div>
        <div style={{ fontSize: '12px', color: '#6b7280', marginBottom: '4px' }}>Target Space</div>
        {(['param', 'objective', 'all'] as ClusterSpace[]).map((s) => (
          <label
            key={s}
            style={{ display: 'flex', alignItems: 'center', gap: '4px', fontSize: '13px' }}
          >
            <input
              data-testid={`space-${s}`}
              type="radio"
              name="cluster-space"
              value={s}
              checked={space === s}
              onChange={() => setSpace(s)}
            />
            {SPACE_LABELS[s]}
          </label>
        ))}
      </div>

      {/* Documentation. */}
      <div>
        <label style={{ fontSize: '12px', color: '#6b7280' }}>Number of Clusters (k)</label>
        <input
          data-testid="k-input"
          type="number"
          min={1}
          value={k}
          onChange={handleKChange}
          style={{
            display: 'block',
            width: '80px',
            padding: '4px',
            fontSize: '14px',
            border: '1px solid #d1d5db',
            borderRadius: '3px',
            marginTop: '4px',
          }}
        />
        {/* Documentation. */}
        {kError && (
          <div
            data-testid="k-error"
            style={{ marginTop: '4px', fontSize: '12px', color: '#dc2626' }}
          >
            {kError}
          </div>
        )}
      </div>

      {/* Documentation. */}
      <button
        data-testid="run-clustering-btn"
        onClick={handleRun}
        disabled={isRunning}
        style={{
          padding: '6px 16px',
          fontSize: '13px',
          background: isRunning ? '#9ca3af' : '#4f46e5',
          color: '#fff',
          border: 'none',
          borderRadius: '4px',
          cursor: isRunning ? 'not-allowed' : 'pointer',
          alignSelf: 'flex-start',
        }}
      >
        {isRunning ? 'Computing...' : 'Run'}
      </button>

      {/* Documentation. */}
      {isRunning && (
        <div data-testid="progress-container">
          <div
            style={{
              height: '6px',
              background: '#e5e7eb',
              borderRadius: '3px',
              overflow: 'hidden',
            }}
          >
            <div
              data-testid="progress-bar"
              style={{
                height: '100%',
                width: `${progress}%`,
                background: '#4f46e5',
                transition: 'width 0.2s',
              }}
            />
          </div>
          <div
            data-testid="progress-text"
            style={{ fontSize: '12px', color: '#6b7280', marginTop: '4px' }}
          >
            Computing...{progress}%
          </div>
        </div>
      )}

      {/* Documentation. */}
      {error && (
        <div
          data-testid="cluster-error"
          style={{
            padding: '8px',
            background: '#fef2f2',
            border: '1px solid #fca5a5',
            borderRadius: '4px',
            fontSize: '12px',
            color: '#dc2626',
          }}
        >
          {error}
        </div>
      )}

      {/* Documentation. */}
      {elbowResult && (
        <div>
          <div
            data-testid="elbow-recommended"
            style={{ fontSize: '13px', fontWeight: 600, color: '#4f46e5', marginBottom: '4px' }}
          >
            Recommended k = {elbowResult.recommendedK}
          </div>
          <ReactECharts option={buildElbowOption(elbowResult)} style={{ height: '180px' }} />
        </div>
      )}
    </div>
  )
}
