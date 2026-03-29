/**
 * ParetoScatter2D テスト (TASK-501)
 *
 * 【テスト対象】: ParetoScatter2D — deck.gl ScatterplotLayer 2D散布図コンポーネント
 * 【テスト方針】: deck.gl と selectionStore を vi.mock でモック
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup } from '@testing-library/react'

// -------------------------------------------------------------------------
// deck.gl モック
// -------------------------------------------------------------------------

// 【deck.gl モック】: WebGL不要のダミーコンポーネントを返す 🟢
vi.mock('deck.gl', () => ({
  DeckGL: vi.fn(({ children }: { children?: unknown }) => (
    <div data-testid="deck-gl">{children as never}</div>
  )),
  ScatterplotLayer: vi
    .fn()
    .mockImplementation((props) => ({ id: props.id, type: 'ScatterplotLayer' })),
}))

// -------------------------------------------------------------------------
// selectionStore モック
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

vi.mock('../../wasm/gpuBuffer', () => ({
  GpuBuffer: vi.fn(),
}))

import { ParetoScatter2D } from './ParetoScatter2D'
import type { GpuBuffer } from '../../wasm/gpuBuffer'
import type { Study } from '../../types'

// -------------------------------------------------------------------------
// テストヘルパー
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
// 正常系
// -------------------------------------------------------------------------

describe('ParetoScatter2D — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockSubscribe2D.mockReturnValue(vi.fn())
  })

  afterEach(() => {
    cleanup()
  })

  // TC-501-04: deck.glモックでエラーなくレンダリング
  test('TC-501-04: ParetoScatter2D がエラーなくレンダリングされる', () => {
    // 【テスト目的】: 2D散布図コンポーネントが安全にレンダリングできること 🟢
    expect(() => render(<ParetoScatter2D gpuBuffer={null} currentStudy={null} />)).not.toThrow()
  })

  test('TC-501-04b: gpuBuffer が渡されると DeckGL コンテナが表示される', () => {
    // 【テスト目的】: データありの場合に deck.gl ラッパー要素が表示されること 🟢
    render(<ParetoScatter2D gpuBuffer={makeGpuBuffer()} currentStudy={makeStudy()} />)
    expect(screen.getByTestId('deck-gl')).toBeInTheDocument()
  })
})

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('ParetoScatter2D — 異常系', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockSubscribe2D.mockReturnValue(vi.fn())
  })

  afterEach(() => {
    cleanup()
  })

  // TC-501-E02: gpuBuffer=null で空状態UIを表示
  test('TC-501-E02: gpuBuffer=null のとき「データがありません」を表示する', () => {
    // 【テスト目的】: データなし時に適切な空状態UIが表示されること 🟢
    render(<ParetoScatter2D gpuBuffer={null} currentStudy={null} />)
    expect(screen.getByText('No data available')).toBeInTheDocument()
  })
})
