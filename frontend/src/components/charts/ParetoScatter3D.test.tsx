/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup } from '@testing-library/react'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

// Documentation.
vi.mock('deck.gl', () => ({
  DeckGL: vi.fn(({ children }: { children?: unknown }) => (
    <div data-testid="deck-gl">{children as never}</div>
  )),
  PointCloudLayer: vi
    .fn()
    .mockImplementation((props) => ({ id: props.id, type: 'PointCloudLayer' })),
}))

// -------------------------------------------------------------------------
// Documentation.
// Documentation.
// -------------------------------------------------------------------------

const { mockSubscribe, mockGetState } = vi.hoisted(() => {
  const mockUnsubscribe = vi.fn()
  const mockSubscribe = vi.fn().mockReturnValue(mockUnsubscribe)
  const mockGetState = vi.fn().mockReturnValue({ selectedIndices: new Uint32Array(0) })
  return { mockSubscribe, mockGetState }
})

vi.mock('../../stores/selectionStore', () => ({
  useSelectionStore: Object.assign(vi.fn().mockReturnValue({}), {
    subscribe: mockSubscribe,
    getState: mockGetState,
  }),
}))

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

vi.mock('../../wasm/gpuBuffer', () => ({
  GpuBuffer: vi.fn(),
}))

import { ParetoScatter3D } from './ParetoScatter3D'
import type { GpuBuffer } from '../../wasm/gpuBuffer'
import type { Study } from '../../types'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
function makeGpuBuffer(): GpuBuffer {
  return {
    trialCount: 5,
    positions: new Float32Array([0, 0, 0.1, 0.1, 0.2, 0.2, 0.3, 0.3, 0.4, 0.4]),
    positions3d: new Float32Array(5 * 3),
    colors: new Float32Array(5 * 4),
    sizes: new Float32Array([1, 2, 1, 1, 2]),
    updateAlphas: vi.fn(),
    resetAlphas: vi.fn(),
  } as unknown as GpuBuffer
}

/**
 * Documentation.
 */
function makeStudy(): Study {
  return {
    studyId: 1,
    name: 'test-study',
    directions: ['minimize', 'minimize'],
    completedTrials: 5,
    totalTrials: 5,
    paramNames: ['x1', 'x2'],
    objectiveNames: ['obj1', 'obj2'],
    userAttrNames: [],
    hasConstraints: false,
  }
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockSubscribe.mockReturnValue(vi.fn()) // Documentation.
  })

  afterEach(() => {
    cleanup()
  })

  // Documentation.
  test('TC-501-01', () => {
    // Documentation.

    // Documentation.
    expect(() => render(<ParetoScatter3D gpuBuffer={null} currentStudy={null} />)).not.toThrow()
  })

  // Documentation.
  test('TC-501-02', () => {
    // Documentation.
    const gpuBuffer = makeGpuBuffer()
    const study = makeStudy()

    // Documentation.
    render(<ParetoScatter3D gpuBuffer={gpuBuffer} currentStudy={study} />)

    // Documentation.
    expect(screen.getByTestId('deck-gl')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-501-03', () => {
    // Documentation.
    const gpuBuffer = makeGpuBuffer()

    // Documentation.
    render(<ParetoScatter3D gpuBuffer={gpuBuffer} currentStudy={makeStudy()} />)

    // Documentation.
    expect(mockSubscribe).toHaveBeenCalledOnce()
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockSubscribe.mockReturnValue(vi.fn())
  })

  afterEach(() => {
    cleanup()
  })

  // Documentation.
  test('TC-501-E01', () => {
    // Documentation.

    // Documentation.
    render(<ParetoScatter3D gpuBuffer={null} currentStudy={null} />)

    // Documentation.
    expect(screen.getByText('No data available')).toBeInTheDocument()
  })
})

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
  test('TC-501-B01', () => {
    // Documentation.
    const mockUnsubscribe = vi.fn()
    mockSubscribe.mockReturnValue(mockUnsubscribe)

    const gpuBuffer = makeGpuBuffer()

    // Documentation.
    const { unmount } = render(<ParetoScatter3D gpuBuffer={gpuBuffer} currentStudy={makeStudy()} />)

    // Documentation.
    unmount()

    // Documentation.
    expect(mockUnsubscribe).toHaveBeenCalledOnce()
  })
})
