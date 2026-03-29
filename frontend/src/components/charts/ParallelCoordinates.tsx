/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 * Documentation.
 */

import { useMemo } from 'react'
import ReactECharts from 'echarts-for-react'
import { useSelectionStore } from '../../stores/selectionStore'
import { useStudyStore } from '../../stores/studyStore'
import { COLORMAPS } from '../../colormaps'
import type { ColormapName } from '../../colormaps'
import type { GpuBuffer } from '../../wasm/gpuBuffer'
import type { Study } from '../../types'
import { EmptyState } from '../common/EmptyState'

// -------------------------------------------------------------------------
// Props Type definitions
// -------------------------------------------------------------------------

export interface ParallelCoordinatesProps {
  /** Documentation. */
  gpuBuffer: GpuBuffer | null
  /** Documentation. */
  currentStudy: Study | null
}

// -------------------------------------------------------------------------
// Documentation.
// Documentation.
// -------------------------------------------------------------------------

interface AxisAreaSelectedEvent {
  axesInfo: Array<{
    axisIndex: number
    /** Documentation. */
    intervals: number[][]
  }>
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Test Coverage: TC-601-01〜04, TC-601-E01〜E02, TC-601-B01
 */
export function ParallelCoordinates({ gpuBuffer, currentStudy }: ParallelCoordinatesProps) {
  // Documentation.
  const addAxisFilter = useSelectionStore((s) => s.addAxisFilter)
  const removeAxisFilter = useSelectionStore((s) => s.removeAxisFilter)
  const selectedIndices = useSelectionStore((s) => s.selectedIndices)
  const colormapName = useSelectionStore((s) => s.colorMode) as ColormapName
  const trialRows = useStudyStore((s) => s.trialRows)

  // Documentation.
  if (!gpuBuffer || !currentStudy) {
    return <EmptyState message="Data not loaded" />
  }

  // 【ECharts option build】: memoize heavy computation
  const { option, axisNames: memoAxisNames } = useMemo(() => {
    // Documentation.
    const _axisNames = [...currentStudy.paramNames, ...currentStudy.objectiveNames]
    const _paramCount = currentStudy.paramNames.length

    // Series Data Build
    const _seriesData: number[][] =
      trialRows.length > 0
        ? trialRows.map((trial) =>
            _axisNames.map((axisName, dim) => {
              const raw =
                dim < _paramCount ? trial.params[axisName] : trial.values[dim - _paramCount]
              const value = Number(raw)
              return Number.isFinite(value) ? value : Number.NaN
            }),
          )
        : Array.from({ length: gpuBuffer.trialCount }, (_, i) => {
            const row = new Array(_axisNames.length).fill(Number.NaN)
            if (gpuBuffer.positions.length >= (i + 1) * 2) {
              row[0] = gpuBuffer.positions[i * 2]
              if (_axisNames.length > 1) {
                row[1] = gpuBuffer.positions[i * 2 + 1]
              }
            }
            return row
          })

    // parallelAxis Definition
    const _parallelAxis = _axisNames.map((name, dim) => {
      let min = Number.POSITIVE_INFINITY
      let max = Number.NEGATIVE_INFINITY
      for (const row of _seriesData) {
        const value = row[dim]
        if (Number.isFinite(value)) {
          if (value < min) min = value
          if (value > max) max = value
        }
      }
      const hasValidRange = Number.isFinite(min) && Number.isFinite(max)
      if (!hasValidRange) return { dim, name, type: 'value' as const }
      if (min === max) {
        const delta = Math.abs(min) * 0.01 || 1
        return { dim, name, type: 'value' as const, min: min - delta, max: max + delta }
      }
      return { dim, name, type: 'value' as const, min, max }
    })

    // Documentation.
    const _selectedSet = new Set(selectedIndices)
    const _isFiltered = selectedIndices.length > 0 && selectedIndices.length < _seriesData.length

    const _selectedData: number[][] = []
    const _unselectedData: number[][] = []
    for (let i = 0; i < _seriesData.length; i++) {
      if (!_isFiltered || _selectedSet.has(i)) {
        _selectedData.push(_seriesData[i])
      } else {
        _unselectedData.push(_seriesData[i])
      }
    }

    // Compute first-objective min/max for rainbow visualMap
    let _objMin = Number.POSITIVE_INFINITY
    let _objMax = Number.NEGATIVE_INFINITY
    const _objDim = _paramCount // first objective is the dimension right after params
    for (const row of _seriesData) {
      const v = row[_objDim]
      if (Number.isFinite(v)) {
        if (v < _objMin) _objMin = v
        if (v > _objMax) _objMax = v
      }
    }
    const _hasObjRange = Number.isFinite(_objMin) && Number.isFinite(_objMax)

    // Use the selected colormap stops for visualMap
    const _colormapStops = COLORMAPS[colormapName].stops

    return {
      axisNames: _axisNames,
      option: {
        parallel: { left: '5%', right: '13%', top: '10%', bottom: '10%' },
        parallelAxis: _parallelAxis,
        ...(_hasObjRange
          ? {
              visualMap: {
                show: true,
                dimension: _objDim,
                min: _objMin,
                max: _objMax,
                inRange: {
                  color: _colormapStops,
                },
                right: 0,
                top: 'center',
                text: ['High', 'Low'],
                textStyle: { fontSize: 10 },
              },
            }
          : {}),
        series: [
          ...(_unselectedData.length > 0
            ? [
                {
                  type: 'parallel',
                  data: _unselectedData,
                  lineStyle: { width: 1, opacity: 0.08, color: '#94a3b8' },
                  silent: true,
                },
              ]
            : []),
          {
            type: 'parallel',
            data: _selectedData,
            lineStyle: { width: 1, opacity: 0.4 },
          },
        ],
      },
    }
  }, [currentStudy, trialRows, gpuBuffer, selectedIndices, colormapName])

  /**
   * Documentation.
   * Documentation.
   * 🟢 REQ-041 support: axis brushing → addAxisFilter → WASM filterByRanges
   */
  const handleAxisAreaSelected = (params: unknown) => {
    const event = params as AxisAreaSelectedEvent
    if (!event?.axesInfo) return

    event.axesInfo.forEach(({ axisIndex, intervals }) => {
      const axisName = memoAxisNames[axisIndex]
      if (!axisName) return

      if (intervals.length === 0) {
        // Documentation.
        removeAxisFilter(axisName)
      } else {
        // Documentation.
        const [min, max] = intervals[0] // Documentation.
        addAxisFilter(axisName, min, max)
      }
    })
  }

  // Documentation.
  return (
    <ReactECharts
      option={option}
      style={{ width: '100%', height: '100%' }}
      lazyUpdate
      onEvents={{
        // Documentation.
        axisareaselected: handleAxisAreaSelected,
      }}
    />
  )
}
