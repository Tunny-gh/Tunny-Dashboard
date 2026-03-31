import { describe, it, expect, vi, beforeEach } from 'vitest'
import { act } from 'react'

// ---------------------------------------------------------------------------
// Hoisted mocks
// ---------------------------------------------------------------------------

const {
  mockRunPca,
  mockRunKmeans,
  mockComputeClusterStats,
  mockEstimateKElbow,
  mockGetInstance,
  capturedStudySubscribers,
} = vi.hoisted(() => {
  const mockRunPca = vi.fn()
  const mockRunKmeans = vi.fn()
  const mockComputeClusterStats = vi.fn()
  const mockEstimateKElbow = vi.fn()
  const mockGetInstance = vi.fn().mockResolvedValue({
    runPca: mockRunPca,
    runKmeans: mockRunKmeans,
    computeClusterStats: mockComputeClusterStats,
    estimateKElbow: mockEstimateKElbow,
  })
  const capturedStudySubscribers: Array<(state: { currentStudy: unknown }) => void> = []
  return {
    mockRunPca,
    mockRunKmeans,
    mockComputeClusterStats,
    mockEstimateKElbow,
    mockGetInstance,
    capturedStudySubscribers,
  }
})

vi.mock('../wasm/wasmLoader', () => ({
  WasmLoader: {
    getInstance: mockGetInstance,
    reset: vi.fn(),
  },
}))

vi.mock('./studyStore', () => ({
  useStudyStore: {
    getState: vi.fn().mockReturnValue({ currentStudy: null }),
    subscribe: vi.fn().mockImplementation((cb: (state: { currentStudy: unknown }) => void) => {
      capturedStudySubscribers.push(cb)
      return () => {}
    }),
  },
}))

import { useClusterStore } from './clusterStore'

// ---------------------------------------------------------------------------
// Test data
// ---------------------------------------------------------------------------

const mockPcaResult = {
  projections: [
    [1.0, 2.0],
    [3.0, 4.0],
    [5.0, 6.0],
  ],
  explainedVariance: [0.6, 0.3],
  featureNames: ['x', 'y'],
  durationMs: 5,
}

const mockKmeansResult = {
  labels: [0, 1, 0],
  centroids: [
    [1.0, 2.0],
    [3.0, 4.0],
  ],
  wcss: 0.5,
  durationMs: 3,
}

const mockClusterStatsResult = {
  stats: [
    { clusterId: 0, size: 2, centroid: { x: 1.0 }, std: { x: 0.1 }, significantDiffs: [] },
    { clusterId: 1, size: 1, centroid: { x: 3.0 }, std: { x: 0.0 }, significantDiffs: ['x'] },
  ],
  durationMs: 2,
}

const mockElbowResult = {
  wcssPerK: [10.0, 5.0, 2.0, 1.5],
  recommendedK: 3,
  durationMs: 8,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function resetStore() {
  useClusterStore.setState({
    pcaProjections: null,
    clusterLabels: null,
    clusterStats: null,
    elbowResult: null,
    isRunning: false,
    clusterError: null,
    clusterSpace: 'param',
    k: 3,
  })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('clusterStore', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
    mockGetInstance.mockResolvedValue({
      runPca: mockRunPca,
      runKmeans: mockRunKmeans,
      computeClusterStats: mockComputeClusterStats,
      estimateKElbow: mockEstimateKElbow,
    })
  })

  it('TC-1604-01: initial state is correct', () => {
    const state = useClusterStore.getState()
    expect(state.pcaProjections).toBeNull()
    expect(state.clusterLabels).toBeNull()
    expect(state.clusterStats).toBeNull()
    expect(state.elbowResult).toBeNull()
    expect(state.isRunning).toBe(false)
    expect(state.clusterError).toBeNull()
    expect(state.clusterSpace).toBe('param')
    expect(state.k).toBe(3)
  })

  it('TC-1604-02: runClustering runs PCA → k-means → stats in order', async () => {
    mockRunPca.mockReturnValue(mockPcaResult)
    mockRunKmeans.mockReturnValue(mockKmeansResult)
    mockComputeClusterStats.mockReturnValue(mockClusterStatsResult)

    await act(async () => {
      await useClusterStore.getState().runClustering('param', 3)
    })

    expect(useClusterStore.getState().pcaProjections).toEqual(mockPcaResult.projections)
    expect(useClusterStore.getState().clusterLabels).toEqual(mockKmeansResult.labels)
    expect(useClusterStore.getState().clusterStats).toEqual(mockClusterStatsResult)
    expect(useClusterStore.getState().isRunning).toBe(false)
    expect(useClusterStore.getState().clusterError).toBeNull()
  })

  it('TC-1604-03: runClustering sets isRunning=true during execution', async () => {
    let resolveWasm!: (value: unknown) => void
    mockGetInstance.mockReturnValue(
      new Promise((resolve) => {
        resolveWasm = resolve
      }),
    )

    const promise = useClusterStore.getState().runClustering('param', 3)
    expect(useClusterStore.getState().isRunning).toBe(true)

    resolveWasm({
      runPca: () => mockPcaResult,
      runKmeans: () => mockKmeansResult,
      computeClusterStats: () => mockClusterStatsResult,
    })
    await act(async () => {
      await promise
    })
    expect(useClusterStore.getState().isRunning).toBe(false)
  })

  it('TC-1604-04: runClustering sets clusterError when PCA throws', async () => {
    mockRunPca.mockImplementation(() => {
      throw new Error('Insufficient data for PCA')
    })

    await act(async () => {
      await useClusterStore.getState().runClustering('all', 3)
    })

    expect(useClusterStore.getState().clusterError).toBe('Insufficient data for PCA')
    expect(useClusterStore.getState().isRunning).toBe(false)
    expect(useClusterStore.getState().pcaProjections).toBeNull()
  })

  it('TC-1604-05: estimateK runs PCA → elbow and sets elbowResult', async () => {
    mockRunPca.mockReturnValue(mockPcaResult)
    mockEstimateKElbow.mockReturnValue(mockElbowResult)

    await act(async () => {
      await useClusterStore.getState().estimateK('param')
    })

    expect(useClusterStore.getState().elbowResult).toEqual(mockElbowResult)
    expect(useClusterStore.getState().isRunning).toBe(false)
    expect(useClusterStore.getState().clusterError).toBeNull()
  })

  it('TC-1604-06: runClustering passes correct Float64Array to runKmeans', async () => {
    mockRunPca.mockReturnValue(mockPcaResult)
    mockRunKmeans.mockReturnValue(mockKmeansResult)
    mockComputeClusterStats.mockReturnValue(mockClusterStatsResult)

    await act(async () => {
      await useClusterStore.getState().runClustering('param', 3)
    })

    const callArgs = mockRunKmeans.mock.calls[0]
    expect(callArgs[0]).toBe(3) // k
    const flatData = callArgs[1] as Float64Array
    expect(flatData).toBeInstanceOf(Float64Array)
    expect(Array.from(flatData)).toEqual([1.0, 2.0, 3.0, 4.0, 5.0, 6.0])
    expect(callArgs[2]).toBe(2) // n_cols
  })

  it('TC-1604-07: study change resets all cluster state', () => {
    useClusterStore.setState({
      pcaProjections: mockPcaResult.projections,
      clusterLabels: mockKmeansResult.labels,
      clusterStats: mockClusterStatsResult,
      elbowResult: mockElbowResult,
      isRunning: false,
      clusterError: null,
    })

    expect(capturedStudySubscribers.length).toBeGreaterThan(0)
    const subscriber = capturedStudySubscribers[capturedStudySubscribers.length - 1]
    subscriber({ currentStudy: { name: 'new-study' } as unknown })

    expect(useClusterStore.getState().pcaProjections).toBeNull()
    expect(useClusterStore.getState().clusterLabels).toBeNull()
    expect(useClusterStore.getState().clusterStats).toBeNull()
    expect(useClusterStore.getState().elbowResult).toBeNull()
    expect(useClusterStore.getState().isRunning).toBe(false)
  })
})
