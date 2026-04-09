import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, cleanup } from '@testing-library/react'

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

vi.mock('echarts-for-react')

const { mockUseClusterStore } = vi.hoisted(() => {
  const mockUseClusterStore = vi.fn()
  return { mockUseClusterStore }
})

vi.mock('../../stores/clusterStore', () => ({
  useClusterStore: mockUseClusterStore,
}))

vi.mock('../panels/ClusterList', () => ({
  getClusterColor: (id: number) => `#color${id}`,
}))

const { mockGetIndicesCS } = vi.hoisted(() => {
  const mockGetIndicesCS = vi.fn().mockReturnValue(new Uint32Array(0))
  return { mockGetIndicesCS }
})

vi.mock('../../stores/downsampleStore', () => ({
  useDownsampleStore: vi.fn((selector: (s: { getIndices: typeof mockGetIndicesCS }) => unknown) =>
    selector({ getIndices: mockGetIndicesCS }),
  ),
}))

import { ClusterScatter } from './ClusterScatter'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function setupStore(
  overrides: Partial<{
    pcaProjections: number[][] | null
    clusterLabels: number[] | null
    isRunning: boolean
    clusterError: string | null
  }> = {},
) {
  mockUseClusterStore.mockReturnValue({
    pcaProjections: null,
    clusterLabels: null,
    isRunning: false,
    clusterError: null,
    ...overrides,
  })
}

const mockProjections = [
  [0, 1],
  [2, 3],
  [4, 5],
]
const mockLabels = [0, 1, 0]

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('ClusterScatter', () => {
  beforeEach(() => {
    cleanup()
    vi.clearAllMocks()
  })

  it('TC-1606-01: shows EmptyState when pcaProjections is null', () => {
    setupStore({ pcaProjections: null, isRunning: false })

    render(<ClusterScatter />)

    expect(screen.getByText(/Run clustering/)).toBeDefined()
  })

  it('TC-1606-02: shows Loading when isRunning is true', () => {
    setupStore({ isRunning: true })

    render(<ClusterScatter />)

    expect(screen.getByText('Loading...')).toBeDefined()
  })

  it('TC-1606-03: shows error when clusterError is set', () => {
    setupStore({ clusterError: 'Insufficient data' })

    render(<ClusterScatter />)

    expect(screen.getByText('Insufficient data')).toBeDefined()
  })

  it('TC-1606-04: renders chart without labels (single color)', () => {
    setupStore({ pcaProjections: mockProjections, clusterLabels: null })

    render(<ClusterScatter />)

    const chart = screen.getByTestId('echarts')
    expect(chart).toBeDefined()
  })

  it('TC-1606-05: ECharts series contains Cluster series names when labels provided', () => {
    setupStore({ pcaProjections: mockProjections, clusterLabels: mockLabels })

    render(<ClusterScatter />)

    const chart = screen.getByTestId('echarts')
    const option = JSON.parse(chart.dataset.option ?? '{}')
    const seriesNames = option.series.map((s: { name: string }) => s.name)
    expect(seriesNames).toContain('Cluster 0')
    expect(seriesNames).toContain('Cluster 1')
  })
})

// -------------------------------------------------------------------------
// TASK-1671: cluster index integration (ClusterScatter)
// -------------------------------------------------------------------------

describe('ClusterScatter - cluster index integration (TASK-1671)', () => {
  beforeEach(() => {
    cleanup()
    vi.clearAllMocks()
  })

  it('TC-1671-01: getIndices called with "cluster"', () => {
    mockGetIndicesCS.mockReturnValue(new Uint32Array(0))
    mockUseClusterStore.mockReturnValue({
      pcaProjections: mockProjections,
      clusterLabels: null,
      isRunning: false,
      clusterError: null,
    })
    render(<ClusterScatter />)
    expect(mockGetIndicesCS).toHaveBeenCalledWith('cluster')
  })

  it('TC-1671-02: cluster indices filter projections', () => {
    // Only show index 1
    mockGetIndicesCS.mockImplementation((key: string) => {
      if (key === 'cluster') return new Uint32Array([1])
      return new Uint32Array(0)
    })
    mockUseClusterStore.mockReturnValue({
      pcaProjections: mockProjections, // [[0,1],[2,3],[4,5]]
      clusterLabels: null,
      isRunning: false,
      clusterError: null,
    })
    render(<ClusterScatter />)

    const chart = screen.getByTestId('echarts')
    const option = JSON.parse(chart.dataset.option ?? '{}')
    const allData = option.series.flatMap((s: { data: number[][] }) => s.data)
    // Only index 1 → [2,3]
    expect(allData).toHaveLength(1)
    expect(allData[0]).toEqual([2, 3])
  })
})
