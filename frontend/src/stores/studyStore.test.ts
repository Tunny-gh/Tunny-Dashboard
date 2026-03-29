/**
 * StudyStore テスト (TASK-302)
 *
 * 【テスト対象】: useStudyStore — Journal 読み込み・Study 選択
 * 【テスト方針】: WasmLoader と File API を vi.mock でモック
 */

import { describe, test, expect, beforeEach, vi } from 'vitest'

// -------------------------------------------------------------------------
// WasmLoader モック
// vi.hoisted でファクトリより前に変数を確保する（hoisting 問題の回避）
// -------------------------------------------------------------------------

const { mockParseJournal, mockSelectStudy, mockGetInstance } = vi.hoisted(() => {
  const mockParseJournal = vi.fn()
  const mockSelectStudy = vi.fn()
  const mockGetInstance = vi.fn().mockResolvedValue({
    parseJournal: mockParseJournal,
    selectStudy: mockSelectStudy,
  })
  return { mockParseJournal, mockSelectStudy, mockGetInstance }
})

vi.mock('../wasm/wasmLoader', () => ({
  WasmLoader: {
    getInstance: mockGetInstance,
    reset: vi.fn(),
  },
}))

import { useStudyStore } from './studyStore'

// -------------------------------------------------------------------------
// テストヘルパー
// -------------------------------------------------------------------------

/**
 * 【ヘルパー】: テスト用の File モックを生成する
 */
function makeFile(content: string = 'dummy journal') {
  return new File([content], 'journal.log', { type: 'text/plain' })
}

/**
 * 【ヘルパー】: StudyStore を初期状態にリセットする
 */
function resetStore() {
  useStudyStore.setState({
    currentStudy: null,
    allStudies: [],
    studyMode: 'single-objective',
    isLoading: false,
    loadError: null,
  })
}

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('StudyStore — 正常系', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
    mockGetInstance.mockResolvedValue({
      parseJournal: mockParseJournal,
      selectStudy: mockSelectStudy,
    })
  })

  // TC-302-09: loadJournal が WASM parseJournal を呼び出す
  test('TC-302-09: loadJournal が parseJournal を呼び出し allStudies を更新する', async () => {
    // 【テスト目的】: loadJournal が WASM を呼び出して allStudies を設定すること 🟢
    const mockStudies = [
      {
        studyId: 1,
        name: 'study-1',
        directions: ['minimize'],
        completedTrials: 100,
        totalTrials: 100,
        paramNames: ['x'],
        objectiveNames: ['y'],
        userAttrNames: [],
        hasConstraints: false,
      },
    ]
    mockParseJournal.mockReturnValue({ studies: mockStudies, durationMs: 10 })

    // 【処理実行】
    await useStudyStore.getState().loadJournal(makeFile())

    // 【確認内容】: parseJournal が呼ばれた
    expect(mockParseJournal).toHaveBeenCalledOnce()
    // 【確認内容】: allStudies が更新された
    expect(useStudyStore.getState().allStudies).toEqual(mockStudies)
    // 【確認内容】: isLoading が false に戻った
    expect(useStudyStore.getState().isLoading).toBe(false)
  })

  // TC-302-10: loadJournal 中は isLoading=true になる
  test('TC-302-10: loadJournal 開始時に isLoading=true になり完了後 false になる', async () => {
    // 【テスト目的】: ローディング状態が正しく管理されること 🟢
    mockParseJournal.mockReturnValue({ studies: [], durationMs: 0 })

    // 【処理実行】: loadJournal を開始（await しない）
    const loadPromise = useStudyStore.getState().loadJournal(makeFile())

    // 【確認内容 — ロード中】: set({isLoading:true}) は同期実行のため即座に true
    expect(useStudyStore.getState().isLoading).toBe(true)

    // 【完了待機】
    await loadPromise

    // 【確認内容 — ロード完了】: isLoading = false
    expect(useStudyStore.getState().isLoading).toBe(false)
  })
})

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('StudyStore — 異常系', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
    mockGetInstance.mockResolvedValue({
      parseJournal: mockParseJournal,
      selectStudy: mockSelectStudy,
    })
  })

  // TC-302-E01: loadJournal 失敗時に loadError が設定される
  test('TC-302-E01: parseJournal がエラーを throw すると loadError が設定される', async () => {
    // 【テスト目的】: WASM 失敗時にエラー状態が適切に設定されること 🟢
    mockParseJournal.mockImplementation(() => {
      throw new Error('parse failed')
    })

    // 【処理実行】
    await useStudyStore.getState().loadJournal(makeFile())

    // 【確認内容】: loadError にエラーメッセージが設定された
    expect(useStudyStore.getState().loadError).toContain('parse failed')
    // 【確認内容】: isLoading = false
    expect(useStudyStore.getState().isLoading).toBe(false)
    // 【確認内容】: allStudies は空のまま
    expect(useStudyStore.getState().allStudies).toEqual([])
  })
})
