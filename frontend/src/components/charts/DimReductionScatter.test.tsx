import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, cleanup, act, waitFor } from '@testing-library/react'

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

vi.mock('echarts-for-react')

const { mockUseClusterStore, mockUseSelectionStore, mockRunPca, mockGetInstance, clusterState } =
  vi.hoisted(() => {
    const mockRunPca = vi.fn()
    const mockGetInstance = vi.fn().mockResolvedValue({ runPca: mockRunPca })
    const clusterState = { pcaProjections: null as number[][] | null, isRunning: false }
    // Apply selector when called with a function, otherwise return full state
    const mockUseClusterStore = vi
      .fn()
      .mockImplementation((selector?: (s: typeof clusterState) => unknown) =>
        typeof selector === 'function' ? selector(clusterState) : clusterState,
      )
    const mockUseSelectionStore = vi.fn().mockReturnValue({ colorMode: 'objective' })
    return { mockUseClusterStore, mockUseSelectionStore, mockRunPca, mockGetInstance, clusterState }
  })

vi.mock('../../stores/clusterStore', () => ({
  useClusterStore: mockUseClusterStore,
}))

vi.mock('../../stores/selectionStore', () => ({
  useSelectionStore: mockUseSelectionStore,
}))

vi.mock('../../wasm/wasmLoader', () => ({
  WasmLoader: {
    getInstance: mockGetInstance,
    reset: vi.fn(),
  },
}))

import { DimReductionScatter } from './DimReductionScatter'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const mockProjections = [
  [1.0, 2.0],
  [3.0, 4.0],
]
const mockPcaResult = {
  projections: mockProjections,
  explainedVariance: [0.6, 0.3],
  featureNames: ['x', 'y'],
  durationMs: 5,
}

function setupClusterStore(pcaProjections: number[][] | null = null, isRunning = false) {
  clusterState.pcaProjections = pcaProjections
  clusterState.isRunning = isRunning
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('DimReductionScatter', () => {
  beforeEach(() => {
    cleanup()
    vi.clearAllMocks()
    clusterState.pcaProjections = null
    clusterState.isRunning = false
    mockGetInstance.mockResolvedValue({ runPca: mockRunPca })
    mockUseSelectionStore.mockReturnValue({ colorMode: 'objective' })
  })

  it('TC-1607-01: reuses clusterStore.pcaProjections without calling WasmLoader', async () => {
    setupClusterStore(mockProjections)

    await act(async () => {
      render(<DimReductionScatter />)
    })

    expect(mockGetInstance).not.toHaveBeenCalled()
    expect(screen.getByTestId('echarts')).toBeDefined()
  })

  it('TC-1607-02: calls wasm.runPca(2, all) when pcaProjections is null', async () => {
    setupClusterStore(null)
    mockRunPca.mockReturnValue(mockPcaResult)

    await act(async () => {
      render(<DimReductionScatter />)
    })

    expect(mockRunPca).toHaveBeenCalledWith(2, 'all')
  })

  it('TC-1607-03: shows Loading while WasmLoader is pending', async () => {
    setupClusterStore(null)
    let resolveWasm!: (value: unknown) => void
    mockGetInstance.mockReturnValue(
      new Promise((resolve) => {
        resolveWasm = resolve
      }),
    )

    render(<DimReductionScatter />)

    expect(screen.getByText('Loading...')).toBeDefined()
    resolveWasm({ runPca: () => mockPcaResult })
  })

  it('TC-1607-04: shows EmptyState when runPca throws', async () => {
    setupClusterStore(null)
    mockRunPca.mockImplementation(() => {
      throw new Error('Insufficient data')
    })

    render(<DimReductionScatter />)

    await waitFor(() => {
      expect(screen.getByText('Insufficient data')).toBeDefined()
    })
  })

  it('TC-1607-05: UMAP option is disabled', async () => {
    setupClusterStore(mockProjections)

    await act(async () => {
      render(<DimReductionScatter />)
    })

    const umapOption = screen.getByText(/UMAP/)
    expect(umapOption.closest('option')?.disabled).toBe(true)
  })

  it('TC-1607-06: ECharts option contains Dimensionality Reduction title', async () => {
    setupClusterStore(mockProjections)

    await act(async () => {
      render(<DimReductionScatter />)
    })

    const chart = screen.getByTestId('echarts')
    const option = JSON.parse(chart.dataset.option ?? '{}')
    expect(JSON.stringify(option)).toContain('Dimensionality Reduction (PCA)')
  })

  it('TC-1607-07: shows Loading when clusterStore.isRunning is true', () => {
    setupClusterStore(null, true)

    render(<DimReductionScatter />)

    expect(screen.getByText('Loading...')).toBeDefined()
  })
})
