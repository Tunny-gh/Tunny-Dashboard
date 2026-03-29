/**
 * ContourPlot — 2D parameter correlation scatter plot (simplified contour plot)
 *
 * Displays a scatter plot of two parameters colored by objective value.
 *
 * Note: optuna-dashboard's contour plot interpolates a surface using Gradient
 * Boosting Tree (scikit-learn) and draws contour lines (requires Python).
 * This component only shows actual trial points without interpolation.
 *
 * Design:
 *   - Dropdowns to select X/Y parameters and objective
 *   - visualMap maps objective values to a color scale
 *   - Numeric parameters only (string parameters are excluded)
 */

import { useState } from 'react'
import ReactECharts from 'echarts-for-react'
import { EmptyState } from '../common/EmptyState'

// -------------------------------------------------------------------------
// Types
// -------------------------------------------------------------------------

/** One trial's data as received by ContourPlot */
export interface ContourTrial {
  /** Parameter value map (numeric or string values) */
  params: Record<string, number | string>
  /** Objective values (null = incomplete trial) */
  values: number[] | null
}

/** Props */
export interface ContourPlotProps {
  /** List of trials to display */
  trials: ContourTrial[]
  /** Parameter name list */
  paramNames: string[]
  /** Objective name list */
  objectiveNames: string[]
}

// -------------------------------------------------------------------------
// Component
// -------------------------------------------------------------------------

/**
 * Scatter plot of 2 parameters colored by objective value.
 * Dropdowns for X/Y parameters and objective update the chart in real time.
 */
export function ContourPlot({ trials, paramNames, objectiveNames }: ContourPlotProps) {
  const [xParamIdx, setXParamIdx] = useState(0)
  const [yParamIdx, setYParamIdx] = useState(Math.min(1, paramNames.length - 1))
  const [objIdx, setObjIdx] = useState(0)

  // Show empty state when there are no trials or fewer than 2 parameters
  if (trials.length === 0 || paramNames.length < 2) {
    return <EmptyState message="No data available (at least 2 parameters required)" />
  }

  const xParam = paramNames[xParamIdx]
  const yParam = paramNames[yParamIdx]

  // Filter to trials where X/Y params are numeric and objective value exists
  const validTrials = trials.filter(
    (t) =>
      t.values !== null &&
      t.values[objIdx] != null &&
      typeof t.params[xParam] === 'number' &&
      typeof t.params[yParam] === 'number',
  )

  // Compute objective value range for visualMap min/max
  const objValues = validTrials.map((t) => t.values![objIdx])
  const minObj = objValues.length > 0 ? Math.min(...objValues) : 0
  const maxObj = objValues.length > 0 ? Math.max(...objValues) : 1

  // ECharts option: scatter + visualMap (color scale by objective value)
  const option = {
    tooltip: {
      trigger: 'item',
      formatter: (params: { value: [number, number, number] }) =>
        `${xParam}: ${params.value[0].toFixed(4)}<br/>` +
        `${yParam}: ${params.value[1].toFixed(4)}<br/>` +
        `${objectiveNames[objIdx]}: ${params.value[2].toFixed(4)}`,
    },
    xAxis: {
      type: 'value',
      name: xParam,
      nameLocation: 'center',
      nameGap: 24,
    },
    yAxis: {
      type: 'value',
      name: yParam,
      nameLocation: 'center',
      nameGap: 40,
    },
    // Color scale: blue (low) → red (high)
    visualMap: {
      min: minObj,
      max: maxObj,
      dimension: 2,
      inRange: {
        color: [
          '#313695',
          '#4575b4',
          '#74add1',
          '#abd9e9',
          '#e0f3f8',
          '#ffffbf',
          '#fee090',
          '#fdae61',
          '#f46d43',
          '#d73027',
          '#a50026',
        ],
      },
      calculable: true,
      orient: 'vertical',
      right: 10,
      top: 'center',
    },
    series: [
      {
        type: 'scatter',
        // Data format: [x, y, objective value] — dimension=2 used by visualMap
        data: validTrials.map((t) => [
          t.params[xParam] as number,
          t.params[yParam] as number,
          t.values![objIdx],
        ]),
        symbolSize: 8,
      },
    ],
    grid: { containLabel: true, right: 80 },
  }

  return (
    <div
      data-testid="contour-plot"
      style={{ height: '100%', display: 'flex', flexDirection: 'column' }}
    >
      {/* Warning banner: contour interpolation requires Python/scikit-learn */}
      <div
        data-testid="contour-note"
        style={{
          padding: '2px 8px',
          background: '#fffbeb',
          borderBottom: '1px solid #fde68a',
          fontSize: 11,
          color: '#92400e',
          flexShrink: 0,
        }}
      >
        ⚠️ Contour interpolation (requires Python / scikit-learn) from optuna-dashboard is not
        supported. Only actual trial points are shown.
      </div>

      {/* Control bar: X/Y parameter and objective selectors */}
      <div
        style={{
          display: 'flex',
          gap: 8,
          padding: '4px 8px',
          flexShrink: 0,
          fontSize: 12,
          alignItems: 'center',
          flexWrap: 'wrap',
        }}
      >
        <label>
          X:{' '}
          <select
            data-testid="contour-x-select"
            value={xParamIdx}
            onChange={(e) => setXParamIdx(Number(e.target.value))}
            style={{ fontSize: 12 }}
          >
            {paramNames.map((p, i) => (
              <option key={p} value={i}>
                {p}
              </option>
            ))}
          </select>
        </label>

        <label>
          Y:{' '}
          <select
            data-testid="contour-y-select"
            value={yParamIdx}
            onChange={(e) => setYParamIdx(Number(e.target.value))}
            style={{ fontSize: 12 }}
          >
            {paramNames.map((p, i) => (
              <option key={p} value={i}>
                {p}
              </option>
            ))}
          </select>
        </label>

        {/* Objective selector: shown only when there are 2 or more objectives */}
        {objectiveNames.length > 1 && (
          <label>
            Objective:{' '}
            <select
              data-testid="contour-obj-select"
              value={objIdx}
              onChange={(e) => setObjIdx(Number(e.target.value))}
              style={{ fontSize: 12 }}
            >
              {objectiveNames.map((o, i) => (
                <option key={o} value={i}>
                  {o}
                </option>
              ))}
            </select>
          </label>
        )}
      </div>

      {/* Chart: ECharts scatter + visualMap */}
      <ReactECharts option={option} style={{ flex: 1 }} />
    </div>
  )
}

export default ContourPlot
