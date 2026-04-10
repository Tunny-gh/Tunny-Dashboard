import type {
  OptimizationDirection,
  OptimizationPhase,
  TrialData,
} from './OptimizationHistory.types'

export function detectPhase(trialIndex: number, totalTrials: number): OptimizationPhase {
  const progress = trialIndex / totalTrials

  if (progress < 0.3) {
    return 'exploration'
  }
  if (progress < 0.7) {
    return 'exploitation'
  }
  return 'convergence'
}

export function computeBestSeries(data: TrialData[], direction: OptimizationDirection): number[] {
  let best = direction === 'minimize' ? Infinity : -Infinity
  return data.map(({ value }) => {
    if (direction === 'minimize') {
      best = Math.min(best, value)
    } else {
      best = Math.max(best, value)
    }
    return best
  })
}

export function computeMovingAverage(values: number[], window: number): number[] {
  return values.map((_, i) => {
    const start = Math.max(0, i - window + 1)
    const slice = values.slice(start, i + 1)
    return slice.reduce((sum, v) => sum + v, 0) / slice.length
  })
}

export function computeImprovementRate(bestSeries: number[]): number[] {
  return bestSeries.map((curr, i) => {
    if (i === 0) return 0
    const prev = bestSeries[i - 1]
    if (prev === 0) return 0
    return Math.abs((prev - curr) / prev) * 100
  })
}
