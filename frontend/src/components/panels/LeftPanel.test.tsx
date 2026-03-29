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

const { mockAddAxisFilter, mockSetColorMode } = vi.hoisted(() => {
  const mockAddAxisFilter = vi.fn()
  const mockSetColorMode = vi.fn()
  return { mockAddAxisFilter, mockSetColorMode }
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
          setColorMode: typeof mockSetColorMode
        }) => unknown,
      ) =>
        selector({
          selectedIndices: new Uint32Array([0, 1, 2]),
          colorMode: 'objective',
          addAxisFilter: mockAddAxisFilter,
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
      }) => unknown,
    ) =>
      selector({
        currentStudy: {
          paramNames: ['x1', 'x2'],
          objectiveNames: ['obj1'],
          completedTrials: 10,
        },
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
  test('TC-402-03: パラメータスライダーの変更で addAxisFilter が呼ばれる', () => {
    // 【テスト目的】: スライダー操作が addAxisFilter に連携されること 🟢
    render(<LeftPanel />)

    // 【処理実行】: x1 スライダーの change イベントをシミュレート
    const slider = screen.getByTestId('slider-x1')
    fireEvent.change(slider, { target: { value: '0.5' } })

    // 【確認内容】: addAxisFilter が呼ばれた
    expect(mockAddAxisFilter).toHaveBeenCalled()
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
      (selector: (s: { currentStudy: null }) => unknown) => selector({ currentStudy: null }),
    )

    render(<LeftPanel />)
    expect(screen.getByText('データが読み込まれていません')).toBeInTheDocument()
  })
})
