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

import { useState, useMemo } from 'react'
import ReactECharts from 'echarts-for-react'
import { EmptyState } from '../common/EmptyState'

// -------------------------------------------------------------------------
// Type definitions
// -------------------------------------------------------------------------

/** Documentation. */
export interface SliceTrial {
  /** Documentation. */
  trialId: number
  /** Documentation. */
  params: Record<string, number | string>
  /** Documentation. */
  values: number[] | null
  /** Documentation. */
  paretoRank: number | null
}

/** Documentation. */
export interface SlicePlotProps {
  /** Documentation. */
  trials: SliceTrial[]
  /** Documentation. */
  paramNames: string[]
  /** Documentation. */
  objectiveNames: string[]
  /** Documentation. */
  objectiveIndex?: number
  /** Documentation. */
  selectedIndices?: Uint32Array
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 * Documentation.
 */
export function SlicePlot({
  trials,
  paramNames,
  objectiveNames,
  objectiveIndex: initialObjIdx = 0,
  selectedIndices,
}: SlicePlotProps) {
  // Documentation.
  const [paramIndex, setParamIndex] = useState(0)
  const [objIndex, setObjIndex] = useState(initialObjIdx)

  // Documentation.
  if (trials.length === 0 || paramNames.length === 0) {
    return <EmptyState />
  }

  // Selected Parameter and Objective Name
  const selectedParam = paramNames[paramIndex] ?? paramNames[0]
  const selectedObj = objectiveNames[objIndex] ?? objectiveNames[0]

  // ECharts Option Build: memoize heavy computation
  const option = useMemo(() => {
    // Documentation.
    const scatterData = trials
      .filter(
        (t) =>
          t.values !== null &&
          t.values[objIndex] != null &&
          typeof t.params[selectedParam] === 'number',
      )
      .map((t, i) => [t.params[selectedParam] as number, t.values![objIndex], i])

    // Documentation.
    const isFiltered =
      selectedIndices && selectedIndices.length > 0 && selectedIndices.length < trials.length
    const selectedSet = isFiltered ? new Set(selectedIndices) : null

    // Documentation.
    const validTrialIndices = trials
      .map((t, i) => ({ trial: t, originalIndex: i }))
      .filter(
        ({ trial: t }) =>
          t.values !== null &&
          t.values[objIndex] != null &&
          typeof t.params[selectedParam] === 'number',
      )

    const selectedScatter = selectedSet
      ? scatterData.filter((_, i) => selectedSet.has(validTrialIndices[i].originalIndex))
      : scatterData
    const unselectedScatter = selectedSet
      ? scatterData.filter((_, i) => !selectedSet.has(validTrialIndices[i].originalIndex))
      : []

    const seriesList: object[] = [
      {
        type: 'scatter',
        data: selectedScatter,
        symbolSize: 8,
      },
    ]
    if (unselectedScatter.length > 0) {
      seriesList.push({
        type: 'scatter',
        data: unselectedScatter,
        symbolSize: 8,
        itemStyle: { opacity: 0.08, color: '#94a3b8' },
      })
    }

    return {
      tooltip: {
        trigger: 'item',
        formatter: (params: { value: [number, number, number] }) =>
          `${selectedParam}: ${params.value[0]}<br/>${selectedObj}: ${params.value[1]}`,
      },
      xAxis: {
        type: 'value',
        name: selectedParam,
        nameLocation: 'center',
        nameGap: 24,
      },
      yAxis: {
        type: 'value',
        name: selectedObj,
        nameLocation: 'center',
        nameGap: 40,
      },
      visualMap: selectedSet
        ? undefined
        : {
            min: 0,
            max: Math.max(scatterData.length - 1, 1),
            dimension: 2,
            inRange: { color: ['#5470c6', '#91cc75', '#fac858', '#ee6666'] },
            show: false,
          },
      series: seriesList,
      grid: { containLabel: true },
    }
  }, [trials, selectedParam, selectedObj, objIndex, selectedIndices])

  return (
    <div
      data-testid="slice-plot"
      style={{ height: '100%', display: 'flex', flexDirection: 'column' }}
    >
      {/* Documentation. */}
      <div
        style={{
          display: 'flex',
          gap: 8,
          padding: '4px 8px',
          flexShrink: 0,
          fontSize: 12,
          alignItems: 'center',
        }}
      >
        <label>
          Parameter:{' '}
          <select
            data-testid="slice-param-select"
            value={paramIndex}
            onChange={(e) => setParamIndex(Number(e.target.value))}
            style={{ fontSize: 12 }}
          >
            {paramNames.map((p, i) => (
              <option key={p} value={i}>
                {p}
              </option>
            ))}
          </select>
        </label>

        {/* Documentation. */}
        {objectiveNames.length > 1 && (
          <label>
            Objective:{' '}
            <select
              data-testid="slice-obj-select"
              value={objIndex}
              onChange={(e) => setObjIndex(Number(e.target.value))}
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

      {/* Documentation. */}
      <ReactECharts option={option} style={{ flex: 1 }} lazyUpdate />
    </div>
  )
}

export default SlicePlot
