/**
 * ObjectivePairMatrix tests (TASK-502)
 *
 * Target: ObjectivePairMatrix — N×N objective pair matrix (diagonal: histogram, lower triangle: 2D scatter)
 * Strategy: mock deck.gl with vi.mock; inject props directly
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'

// -------------------------------------------------------------------------
// deck.gl mock — dummy components that do not require WebGL
// -------------------------------------------------------------------------

vi.mock('deck.gl', () => ({
  DeckGL: vi.fn(({ children }: { children?: unknown }) => (
    <div data-testid="deck-gl">{children as never}</div>
  )),
  ScatterplotLayer: vi.fn().mockImplementation((props: { id: string }) => ({
    id: props.id,
    type: 'ScatterplotLayer',
  })),
}))

vi.mock('../../wasm/gpuBuffer', () => ({
  GpuBuffer: vi.fn(),
}))

import { ObjectivePairMatrix } from './ObjectivePairMatrix'
import type { GpuBuffer } from '../../wasm/gpuBuffer'
import type { Study } from '../../types'

// -------------------------------------------------------------------------
// Test helpers
// -------------------------------------------------------------------------

/** Creates a GpuBuffer stub for testing */
function makeGpuBuffer(): GpuBuffer {
  return {
    trialCount: 3,
    positions: new Float32Array(3 * 2),
    positions3d: new Float32Array(3 * 3),
    colors: new Float32Array(3 * 4),
    sizes: new Float32Array(3),
    updateAlphas: vi.fn(),
    resetAlphas: vi.fn(),
  } as unknown as GpuBuffer
}

/** Creates a test Study with 4 objectives */
function makeStudy4(): Study {
  return {
    studyId: 1,
    name: 'test-study',
    directions: ['minimize', 'minimize', 'minimize', 'minimize'],
    completedTrials: 3,
    totalTrials: 3,
    paramNames: ['x1'],
    objectiveNames: ['f1', 'f2', 'f3', 'f4'],
    userAttrNames: [],
    hasConstraints: false,
  }
}

/** Creates a test Study with 2 objectives */
function makeStudy2(): Study {
  return {
    studyId: 2,
    name: 'test-study-2',
    directions: ['minimize', 'minimize'],
    completedTrials: 3,
    totalTrials: 3,
    paramNames: ['x1'],
    objectiveNames: ['f1', 'f2'],
    userAttrNames: [],
    hasConstraints: false,
  }
}

/** Creates a test Study with 1 objective */
function makeStudy1(): Study {
  return {
    studyId: 3,
    name: 'test-study-1',
    directions: ['minimize'],
    completedTrials: 3,
    totalTrials: 3,
    paramNames: ['x1'],
    objectiveNames: ['f1'],
    userAttrNames: [],
    hasConstraints: false,
  }
}

// -------------------------------------------------------------------------
// Happy path
// -------------------------------------------------------------------------

describe('ObjectivePairMatrix — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    cleanup()
  })

  // TC-502-01: renders without error with 4 objectives
  test('TC-502-01: ObjectivePairMatrix が4目的でエラーなくレンダリングされる', () => {
    expect(() =>
      render(<ObjectivePairMatrix gpuBuffer={null} currentStudy={makeStudy4()} />),
    ).not.toThrow()
  })

  // TC-502-02: 4 objectives produce a 4×4 grid (16 cells)
  test('TC-502-02: 4目的のとき4×4グリッド（16セル）が表示される', () => {
    render(<ObjectivePairMatrix gpuBuffer={null} currentStudy={makeStudy4()} />)

    const cells = screen.getAllByTestId(/^matrix-cell-/)
    expect(cells).toHaveLength(16)
  })

  // TC-502-03: cell click fires onCellClick with correct axis names
  test('TC-502-03: セルクリックでonCellClickが正しい軸名で呼ばれる', () => {
    const onCellClick = vi.fn()
    render(
      <ObjectivePairMatrix
        gpuBuffer={makeGpuBuffer()}
        currentStudy={makeStudy4()}
        onCellClick={onCellClick}
      />,
    )

    // Click cell at row=1, col=0 → xAxis='f1', yAxis='f2'
    const cell = screen.getByTestId('matrix-cell-1-0')
    fireEvent.click(cell)

    expect(onCellClick).toHaveBeenCalledWith('f1', 'f2')
  })

  // TC-502-04: 2 objectives produce a 2×2 grid (4 cells)
  test('TC-502-04: 2目的のとき2×2グリッド（4セル）が表示される', () => {
    render(<ObjectivePairMatrix gpuBuffer={null} currentStudy={makeStudy2()} />)

    const cells = screen.getAllByTestId(/^matrix-cell-/)
    expect(cells).toHaveLength(4)
  })
})

// -------------------------------------------------------------------------
// Error cases
// -------------------------------------------------------------------------

describe('ObjectivePairMatrix — 異常系', () => {
  afterEach(() => {
    cleanup()
  })

  // TC-502-E01: 1 objective hides the component
  test('TC-502-E01: 1目的のときコンポーネントが非表示になる', () => {
    render(<ObjectivePairMatrix gpuBuffer={null} currentStudy={makeStudy1()} />)

    expect(screen.queryByTestId('objective-pair-matrix')).not.toBeInTheDocument()
  })

  // TC-502-E02: currentStudy=null shows empty state
  test('TC-502-E02: currentStudy=null のとき「データが読み込まれていません」を表示する', () => {
    render(<ObjectivePairMatrix gpuBuffer={null} currentStudy={null} />)

    expect(screen.getByText('Data not loaded')).toBeInTheDocument()
  })
})
