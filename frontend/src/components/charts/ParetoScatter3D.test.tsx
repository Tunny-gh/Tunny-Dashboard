/**
 * ParetoScatter3D テスト (TASK-501)
 *
 * 【テスト対象】: ParetoScatter3D — deck.gl PointCloudLayer 3D散布図コンポーネント
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
  PointCloudLayer: vi
    .fn()
    .mockImplementation((props) => ({ id: props.id, type: 'PointCloudLayer' })),
}))

// -------------------------------------------------------------------------
// selectionStore モック
// vi.hoisted でファクトリより前に変数を確保する
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
// GpuBuffer モック
// -------------------------------------------------------------------------

vi.mock('../../wasm/gpuBuffer', () => ({
  GpuBuffer: vi.fn(),
}))

import { ParetoScatter3D } from './ParetoScatter3D'
import type { GpuBuffer } from '../../wasm/gpuBuffer'
import type { Study } from '../../types'

// -------------------------------------------------------------------------
// テストヘルパー
// -------------------------------------------------------------------------

/**
 * 【ヘルパー】: テスト用 GpuBuffer モックを生成する
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
 * 【ヘルパー】: テスト用 Study を生成する
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
// 正常系
// -------------------------------------------------------------------------

describe('ParetoScatter3D — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockSubscribe.mockReturnValue(vi.fn()) // unsubscribe 関数を返す
  })

  afterEach(() => {
    cleanup()
  })

  // TC-501-01: deck.glモックでエラーなくレンダリング
  test('TC-501-01: gpuBuffer=null でもコンポーネントがエラーなくレンダリングされる', () => {
    // 【テスト目的】: コンポーネントが null データで安全にレンダリングできること 🟢

    // 【処理実行】
    expect(() => render(<ParetoScatter3D gpuBuffer={null} currentStudy={null} />)).not.toThrow()
  })

  // TC-501-02: gpuBufferありでDeckGLが表示される
  test('TC-501-02: gpuBuffer が渡されると DeckGL コンテナが表示される', () => {
    // 【テスト目的】: データありの場合に deck.gl ラッパー要素が表示されること 🟢
    const gpuBuffer = makeGpuBuffer()
    const study = makeStudy()

    // 【処理実行】
    render(<ParetoScatter3D gpuBuffer={gpuBuffer} currentStudy={study} />)

    // 【確認内容】: deck.gl コンテナが存在する
    expect(screen.getByTestId('deck-gl')).toBeInTheDocument()
  })

  // TC-501-03: selectionStore購読が mount で設定される
  test('TC-501-03: コンポーネントが mount したときに selectionStore.subscribe が呼ばれる', () => {
    // 【テスト目的】: Brushing & Linking のための subscribe が設定されること 🟢
    const gpuBuffer = makeGpuBuffer()

    // 【処理実行】
    render(<ParetoScatter3D gpuBuffer={gpuBuffer} currentStudy={makeStudy()} />)

    // 【確認内容】: subscribe が呼ばれた
    expect(mockSubscribe).toHaveBeenCalledOnce()
  })
})

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('ParetoScatter3D — 異常系', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockSubscribe.mockReturnValue(vi.fn())
  })

  afterEach(() => {
    cleanup()
  })

  // TC-501-E01: gpuBuffer=null で空状態UIを表示
  test('TC-501-E01: gpuBuffer=null のとき「データがありません」を表示する', () => {
    // 【テスト目的】: データなし時に適切な空状態UIが表示されること 🟢

    // 【処理実行】
    render(<ParetoScatter3D gpuBuffer={null} currentStudy={null} />)

    // 【確認内容】: 空状態メッセージが表示されている
    expect(screen.getByText('データがありません')).toBeInTheDocument()
  })
})

// -------------------------------------------------------------------------
// 境界値
// -------------------------------------------------------------------------

describe('ParetoScatter3D — 境界値', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    cleanup()
  })

  // TC-501-B01: アンマウント時に unsubscribe が呼ばれる
  test('TC-501-B01: コンポーネントがアンマウントされると unsubscribe が呼ばれる', () => {
    // 【テスト目的】: メモリリークを防ぐためにアンマウント時に購読解除されること 🟢
    const mockUnsubscribe = vi.fn()
    mockSubscribe.mockReturnValue(mockUnsubscribe)

    const gpuBuffer = makeGpuBuffer()

    // 【処理実行】: マウント
    const { unmount } = render(<ParetoScatter3D gpuBuffer={gpuBuffer} currentStudy={makeStudy()} />)

    // 【処理実行】: アンマウント
    unmount()

    // 【確認内容】: unsubscribe が呼ばれた
    expect(mockUnsubscribe).toHaveBeenCalledOnce()
  })
})
