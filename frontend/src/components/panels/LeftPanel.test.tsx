/**
 * LeftPanel テスト (TASK-402)
 *
 * 【テスト対象】: LeftPanel — Study情報カウンタ・フィルタスライダー・カラーリング選択
 * 【テスト方針】: selectionStore / studyStore / layoutStore を vi.mock でモック
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'

// -------------------------------------------------------------------------
// selectionStore モック
// -------------------------------------------------------------------------

const { mockAddAxisFilter, mockSetColorMode, mockRemoveAxisFilter } = vi.hoisted(() => {
  const mockAddAxisFilter = vi.fn()
  const mockSetColorMode = vi.fn()
  const mockRemoveAxisFilter = vi.fn()
  return { mockAddAxisFilter, mockSetColorMode, mockRemoveAxisFilter }
})

vi.mock('../../stores/selectionStore', () => ({
  useSelectionStore: vi
    .fn()
    .mockImplementation(
      (
        selector: (s: {
          selectedIndices: Uint32Array
          colorMode: string
          addAxisFilter: typeof mockAddAxisFilter
          removeAxisFilter: typeof mockRemoveAxisFilter
          setColorMode: typeof mockSetColorMode
        }) => unknown,
      ) =>
        selector({
          selectedIndices: new Uint32Array([0, 1, 2]),
          colorMode: 'objective',
          addAxisFilter: mockAddAxisFilter,
          removeAxisFilter: mockRemoveAxisFilter,
          setColorMode: mockSetColorMode,
        }),
    ),
}))

// -------------------------------------------------------------------------
// studyStore モック
// -------------------------------------------------------------------------

vi.mock('../../stores/studyStore', () => ({
  useStudyStore: vi.fn().mockImplementation(
    (
      selector: (s: {
        currentStudy: {
          paramNames: string[]
          objectiveNames: string[]
          completedTrials: number
        } | null
        trialRows: Array<{
          trialId: number
          params: Record<string, number>
          values: number[]
          paretoRank: number | null
        }>
      }) => unknown,
    ) =>
      selector({
        currentStudy: {
          paramNames: ['x1', 'x2'],
          objectiveNames: ['obj1'],
          completedTrials: 10,
        },
        trialRows: [
          { trialId: 0, params: { x1: 0.0, x2: -5.0 }, values: [1.0], paretoRank: null },
          { trialId: 1, params: { x1: 10.0, x2: 5.0 }, values: [2.0], paretoRank: null },
          { trialId: 2, params: { x1: 5.0, x2: 0.0 }, values: [3.0], paretoRank: null },
        ],
      }),
  ),
}))

import { LeftPanel } from './LeftPanel'
import { useStudyStore } from '../../stores/studyStore'

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('LeftPanel — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    cleanup()
  })

  // TC-402-01: LeftPanel がレンダリングされる
  test('TC-402-01: LeftPanel がエラーなくレンダリングされる', () => {
    // 【テスト目的】: LeftPanel が正常にレンダリングできること 🟢
    expect(() => render(<LeftPanel />)).not.toThrow()
  })

  // TC-402-02: selected カウンタが selectedIndices.length を表示する
  test('TC-402-02: selected カウンタが selectedIndices の件数 (3) を表示する', () => {
    // 【テスト目的】: 選択件数が正しく表示されること 🟢
    render(<LeftPanel />)

    // 【確認内容】: selected カウンタ要素が存在して値が 3
    expect(screen.getByTestId('selected-count')).toHaveTextContent('3')
  })

  // TC-402-03: スライダー変更で addAxisFilter が呼ばれる
  test('TC-402-03: パラメータスライダーの変更で addAxisFilter が呼ばれる', async () => {
    // 【テスト目的】: スライダー操作が addAxisFilter に連携されること 🟢
    vi.useFakeTimers()
    render(<LeftPanel />)

    // 【処理実行】: x1 の high スライダーの change イベントをシミュレート
    const slider = screen.getByTestId('slider-hi-x1')
    fireEvent.change(slider, { target: { value: '5' } })

    // 【確認内容】: debounce 後に addAxisFilter が呼ばれた
    vi.advanceTimersByTime(200)
    expect(mockAddAxisFilter).toHaveBeenCalled()
    vi.useRealTimers()
  })

  // TC-402-04: カラーモード変更で setColorMode が呼ばれる
  test("TC-402-04: カラーモード 'cluster' 選択で setColorMode が呼ばれる", () => {
    // 【テスト目的】: カラーモード選択が setColorMode に連携されること 🟢
    render(<LeftPanel />)

    // 【処理実行】: cluster ラジオボタンをクリック
    fireEvent.click(screen.getByTestId('color-mode-cluster'))

    // 【確認内容】: setColorMode が 'cluster' で呼ばれた
    expect(mockSetColorMode).toHaveBeenCalledWith('cluster')
  })
})

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('LeftPanel — 異常系', () => {
  afterEach(() => {
    cleanup()
  })

  // TC-402-E01: currentStudy=null で「データが読み込まれていません」を表示
  test('TC-402-E01: currentStudy=null のとき「データが読み込まれていません」を表示する', () => {
    // 【テスト目的】: Study なし時に適切な空状態UIが表示されること 🟢
    ;(useStudyStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (selector: (s: { currentStudy: null; trialRows: [] }) => unknown) =>
        selector({ currentStudy: null, trialRows: [] }),
    )

    render(<LeftPanel />)
    expect(screen.getByText('Data not loaded')).toBeInTheDocument()
  })
})
