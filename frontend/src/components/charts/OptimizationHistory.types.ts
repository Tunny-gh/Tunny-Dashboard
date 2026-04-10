export interface TrialData {
  trial: number
  value: number
}

export type HistoryMode = 'best' | 'all' | 'moving-avg' | 'improvement'

export type OptimizationDirection = 'minimize' | 'maximize'

export type OptimizationPhase = 'exploration' | 'exploitation' | 'convergence'

export interface OptimizationHistoryProps {
  data: TrialData[]
  direction: OptimizationDirection
  selectedIndices?: Uint32Array
}
