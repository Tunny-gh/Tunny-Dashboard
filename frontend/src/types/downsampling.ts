/**
 * Downsampling types and constants for chart rendering performance.
 */

// ========================================
// Cache key and strategy types
// ========================================

export type DownsampleKey =
  | 'scatter' // ParetoScatter2D/3D, ObjectivePairMatrix (max 10,000)
  | 'thumbnail' // ScatterMatrix thumbnails (max 500, REQ-063)
  | 'hover' // ScatterMatrix hover expand (max 3,000)
  | 'pcp' // ParallelCoordinates (max 1,000)
  | 'data_points' // SlicePlot, SurfacePlot3D data points (max 5,000)
  | 'cluster' // ClusterScatter, DimReductionScatter (max 10,000)

export type DownsampleStrategy =
  | 'smart' // Pareto-reserving + random fill
  | 'grid_thumbnail' // Pareto-reserving + grid spatial sampling
  | 'stratified_by_rank' // Pareto-rank stratified sampling
  | 'by_cluster' // Equal-per-cluster sampling (fallback: smart)

export interface DownsampleConfig {
  maxPoints: number
  includePareto: boolean
  strategy: DownsampleStrategy
  nStrata?: number // for 'stratified_by_rank' strategy only
}

// ========================================
// Store state types
// ========================================

export interface DownsampleState {
  cache: Partial<Record<DownsampleKey, Uint32Array>>
  isComputing: boolean
  error: string | null
  lastTotalCount: number
}

export interface DownsampleActions {
  recompute: () => Promise<void>
  recomputeIfNeeded: (filteredIndices: Uint32Array) => Promise<void>
  getIndices: (key: DownsampleKey) => Uint32Array
  reset: () => void
}

// ========================================
// Configuration constants
// ========================================

export const DOWNSAMPLE_CONFIGS: Record<DownsampleKey, DownsampleConfig> = {
  scatter: {
    maxPoints: 10_000,
    includePareto: true,
    strategy: 'smart',
  },
  thumbnail: {
    maxPoints: 500,
    includePareto: true,
    strategy: 'grid_thumbnail',
  },
  hover: {
    maxPoints: 3_000,
    includePareto: true,
    strategy: 'grid_thumbnail',
  },
  pcp: {
    maxPoints: 1_000,
    includePareto: true,
    strategy: 'stratified_by_rank',
    nStrata: 5,
  },
  data_points: {
    maxPoints: 5_000,
    includePareto: false,
    strategy: 'smart',
  },
  cluster: {
    maxPoints: 10_000,
    includePareto: true,
    strategy: 'by_cluster',
  },
} as const
