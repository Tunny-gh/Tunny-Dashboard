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
  ScatterplotLayer: vi
    .fn()
    .mockImplementation((props) => ({ id: props.id, type: 'ScatterplotLayer' })),
}))

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

const { mockSubscribe2D } = vi.hoisted(() => {
  const mockUnsubscribe = vi.fn()
  const mockSubscribe2D = vi.fn().mockReturnValue(mockUnsubscribe)
  return { mockSubscribe2D }
})

vi.mock('../../stores/selectionStore', () => ({
  useSelectionStore: Object.assign(vi.fn().mockReturnValue({}), {
    subscribe: mockSubscribe2D,
    getState: vi.fn().mockReturnValue({ selectedIndices: new Uint32Array(0) }),
  }),
}))

const { mockGetIndices } = vi.hoisted(() => {
  const mockGetIndices = vi.fn().mockReturnValue(new Uint32Array(0))
  return { mockGetIndices }
})

vi.mock('../../stores/downsampleStore', () => ({
  useDownsampleStore: vi.fn((selector: (s: { getIndices: typeof mockGetIndices }) => unknown) =>
    selector({ getIndices: mockGetIndices }),
  ),
}))

vi.mock('../../wasm/gpuBuffer', () => ({
  GpuBuffer: vi.fn(),
}))

import { ScatterplotLayer } from 'deck.gl'
import { ParetoScatter2D } from './ParetoScatter2D'
import type { GpuBuffer } from '../../wasm/gpuBuffer'
import type { Study } from '../../types'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

function makeGpuBuffer(): GpuBuffer {
  return {
    trialCount: 5,
    positions: new Float32Array(5 * 2),
    positions3d: new Float32Array(5 * 3),
    colors: new Float32Array(5 * 4),
    sizes: new Float32Array(5),
    updateAlphas: vi.fn(),
    resetAlphas: vi.fn(),
  } as unknown as GpuBuffer
}

function makeStudy(): Study {
  return {
    studyId: 1,
    name: 'test-study',
    directions: ['minimize', 'minimize'],
    completedTrials: 5,
    totalTrials: 5,
    paramNames: ['x1'],
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
    mockSubscribe2D.mockReturnValue(vi.fn())
  })

  afterEach(() => {
    cleanup()
  })

  // Documentation.
  test('TC-501-04', () => {
    // Documentation.
    expect(() => render(<ParetoScatter2D gpuBuffer={null} currentStudy={null} />)).not.toThrow()
  })

  test('TC-501-04', () => {
    // Documentation.
    render(<ParetoScatter2D gpuBuffer={makeGpuBuffer()} currentStudy={makeStudy()} />)
    expect(screen.getByTestId('deck-gl')).toBeInTheDocument()
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockSubscribe2D.mockReturnValue(vi.fn())
  })

  afterEach(() => {
    cleanup()
  })

  // Documentation.
  test('TC-501-E02', () => {
    // Documentation.
    render(<ParetoScatter2D gpuBuffer={null} currentStudy={null} />)
    expect(screen.getByText('No data available')).toBeInTheDocument()
  })
})

// -------------------------------------------------------------------------
// TASK-1664: scatter index integration
// -------------------------------------------------------------------------

describe('ParetoScatter2D - scatter index integration (TASK-1664)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockSubscribe2D.mockReturnValue(vi.fn())
  })

  afterEach(() => {
    cleanup()
  })

  test('TC-1664-01: uses full data when renderIndices is empty', () => {
    mockGetIndices.mockReturnValue(new Uint32Array(0))
    const gpuBuffer = makeGpuBuffer() // trialCount=5
    render(<ParetoScatter2D gpuBuffer={gpuBuffer} currentStudy={makeStudy()} />)

    const calls = (ScatterplotLayer as unknown as ReturnType<typeof vi.fn>).mock.calls
    const lastCall = calls[calls.length - 1][0]
    expect(lastCall.data.length).toBe(5)
  })

  test('TC-1664-02: uses renderIndices length when indices are provided', () => {
    const indices = new Uint32Array([0, 2, 4])
    mockGetIndices.mockReturnValue(indices)
    const gpuBuffer = makeGpuBuffer() // trialCount=5
    render(<ParetoScatter2D gpuBuffer={gpuBuffer} currentStudy={makeStudy()} />)

    const calls = (ScatterplotLayer as unknown as ReturnType<typeof vi.fn>).mock.calls
    const lastCall = calls[calls.length - 1][0]
    expect(lastCall.data.length).toBe(3)
  })

  test('TC-1664-03: getIndices is called with "scatter"', () => {
    mockGetIndices.mockReturnValue(new Uint32Array(0))
    render(<ParetoScatter2D gpuBuffer={makeGpuBuffer()} currentStudy={makeStudy()} />)
    expect(mockGetIndices).toHaveBeenCalledWith('scatter')
  })
})
