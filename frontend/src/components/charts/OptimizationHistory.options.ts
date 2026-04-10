import type { HistoryMode, OptimizationDirection, TrialData } from './OptimizationHistory.types'
import {
  computeBestSeries,
  computeImprovementRate,
  computeMovingAverage,
} from './OptimizationHistory.utils'

export const MODE_LABELS: Record<HistoryMode, string> = {
  best: 'Best History',
  all: 'All Trials',
  'moving-avg': 'Moving Avg',
  improvement: 'Improvement',
}

const MOVING_AVG_WINDOW = 5

export function buildChartOption(
  data: TrialData[],
  mode: HistoryMode,
  direction: OptimizationDirection,
  selectedIndices?: Uint32Array,
): object {
  if (data.length === 0) {
    return { xAxis: { type: 'value' }, yAxis: { type: 'value' }, series: [] }
  }

  const trials = data.map((d) => d.trial)
  const values = data.map((d) => d.value)
  const bestSeries = computeBestSeries(data, direction)

  switch (mode) {
    case 'best':
      return {
        xAxis: { type: 'category', data: trials },
        yAxis: { type: 'value' },
        series: [{ type: 'line', data: bestSeries, name: 'Best Value' }],
      }

    case 'all': {
      const isFiltered =
        selectedIndices && selectedIndices.length > 0 && selectedIndices.length < data.length
      const selectedSet = isFiltered ? new Set(selectedIndices) : null
      const allPoints = trials.map((t, i) => [t, values[i]])
      const selectedPoints = selectedSet
        ? allPoints.filter((_, i) => selectedSet.has(i))
        : allPoints
      const unselectedPoints = selectedSet ? allPoints.filter((_, i) => !selectedSet.has(i)) : []

      const seriesList: object[] = [{ type: 'scatter', data: selectedPoints, name: 'All Trials' }]
      if (unselectedPoints.length > 0) {
        seriesList.push({
          type: 'scatter',
          data: unselectedPoints,
          name: 'Unselected',
          itemStyle: { opacity: 0.08, color: '#94a3b8' },
        })
      }

      return {
        xAxis: { type: 'value' },
        yAxis: { type: 'value' },
        series: seriesList,
      }
    }

    case 'moving-avg': {
      const movingAvg = computeMovingAverage(values, MOVING_AVG_WINDOW)
      return {
        xAxis: { type: 'category', data: trials },
        yAxis: { type: 'value' },
        series: [
          { type: 'line', data: values, name: 'All Trials', opacity: 0.4 },
          { type: 'line', data: movingAvg, name: `Moving Avg(${MOVING_AVG_WINDOW})` },
        ],
      }
    }

    case 'improvement': {
      const improvementRate = computeImprovementRate(bestSeries)
      return {
        xAxis: { type: 'category', data: trials },
        yAxis: { type: 'value' },
        series: [{ type: 'bar', data: improvementRate, name: 'Improvement(%)' }],
      }
    }

    default:
      return { xAxis: { type: 'value' }, yAxis: { type: 'value' }, series: [] }
  }
}
