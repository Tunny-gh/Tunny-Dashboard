/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

const { mockBrushSelect } = vi.hoisted(() => {
  const mockBrushSelect = vi.fn()
  return { mockBrushSelect }
})

vi.mock('../../stores/selectionStore', () => ({
  useSelectionStore: vi
    .fn()
    .mockImplementation((selector: (s: { brushSelect: typeof mockBrushSelect }) => unknown) =>
      selector({ brushSelect: mockBrushSelect }),
    ),
}))

import { ClusterList, getClusterColor } from './ClusterList'
import type { ClusterStatData } from './ClusterList'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/** Documentation. */
const TEST_STATS: ClusterStatData[] = [
  {
    clusterId: 0,
    size: 30,
    centroid: [1.234, 5.678],
    stdDev: [0.111, 0.222],
    significantFeatures: [true, false],
  },
  {
    clusterId: 1,
    size: 20,
    centroid: [3.456, 2.345],
    stdDev: [0.333, 0.444],
    significantFeatures: [false, true],
  },
]

/** Documentation. */
const TEST_FEATURES = ['x1', 'x2']

/** Documentation. */
const TEST_TRIALS_BY_CLUSTER = [
  new Uint32Array([0, 1, 2]), // cluster 0
  new Uint32Array([3, 4]), // cluster 1
]

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    cleanup()
  })

  // Documentation.
  test('TC-902-09', () => {
    // Documentation.
    render(
      <ClusterList
        clusterStats={TEST_STATS}
        featureNames={TEST_FEATURES}
        trialsByCluster={TEST_TRIALS_BY_CLUSTER}
      />,
    )
    // Documentation.
    expect(screen.getByTestId('cluster-badge-0')).toBeInTheDocument()
    expect(screen.getByTestId('cluster-badge-1')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-902-10', () => {
    // Documentation.
    render(
      <ClusterList
        clusterStats={TEST_STATS}
        featureNames={TEST_FEATURES}
        trialsByCluster={TEST_TRIALS_BY_CLUSTER}
      />,
    )

    // Documentation.
    fireEvent.click(screen.getByTestId('cluster-row-0'))

    // Documentation.
    expect(mockBrushSelect).toHaveBeenCalledOnce()
    const arg = mockBrushSelect.mock.calls[0][0] as Uint32Array
    expect(Array.from(arg)).toEqual([0, 1, 2])
  })

  // Documentation.
  test('TC-902-11', () => {
    // Documentation.
    render(
      <ClusterList
        clusterStats={TEST_STATS}
        featureNames={TEST_FEATURES}
        trialsByCluster={TEST_TRIALS_BY_CLUSTER}
      />,
    )
    const row = screen.getByTestId('cluster-row-0')

    // Documentation.
    expect(row.style.background).not.toBe('rgb(239, 246, 255)')

    // Documentation.
    fireEvent.click(row)

    // Documentation.
    expect(row.style.background).toBe('rgb(239, 246, 255)')
  })

  // Documentation.
  test('TC-902-12', () => {
    // Documentation.
    render(
      <ClusterList
        clusterStats={TEST_STATS}
        featureNames={TEST_FEATURES}
        trialsByCluster={TEST_TRIALS_BY_CLUSTER}
      />,
    )
    // Documentation.
    expect(screen.getByTestId('sig-0-0')).toBeInTheDocument()
    // Documentation.
    expect(screen.queryByTestId('sig-0-1')).toBeNull()
  })

  // Documentation.
  test('TC-902-13', () => {
    // Documentation.
    render(
      <ClusterList
        clusterStats={TEST_STATS}
        featureNames={TEST_FEATURES}
        trialsByCluster={TEST_TRIALS_BY_CLUSTER}
      />,
    )
    // Documentation.
    const statCell = screen.getByTestId('stat-0-0')
    expect(statCell).toHaveTextContent('1.234')
    expect(statCell).toHaveTextContent('0.111')
  })

  // Documentation.
  test('TC-902-14', () => {
    // Documentation.
    render(
      <ClusterList
        clusterStats={TEST_STATS}
        featureNames={TEST_FEATURES}
        trialsByCluster={TEST_TRIALS_BY_CLUSTER}
      />,
    )

    // Documentation.
    fireEvent.click(screen.getByTestId('cluster-row-0'))
    // Documentation.
    fireEvent.click(screen.getByTestId('cluster-row-1'), { ctrlKey: true })

    // Documentation.
    const lastArg = mockBrushSelect.mock.calls[1][0] as Uint32Array
    expect(Array.from(lastArg)).toEqual([0, 1, 2, 3, 4])
  })

  // Documentation.
  test('TC-902-15', () => {
    // Documentation.
    expect(getClusterColor(0)).toBe('#4f46e5')
    // Documentation.
    expect(getClusterColor(8)).toBe(getClusterColor(0))
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  afterEach(() => {
    cleanup()
  })

  // Documentation.
  test('TC-902-E01', () => {
    // Documentation.
    render(<ClusterList clusterStats={[]} featureNames={[]} trialsByCluster={[]} />)
    // Documentation.
    expect(screen.getByText('Clustering has not been run yet')).toBeInTheDocument()
  })
})
