/**
 * ExportPanel テスト (TASK-1101)
 *
 * 【テスト対象】: ExportPanel — CSV エクスポート・ピン留め UI
 * 【テスト方針】: exportStore / selectionStore / studyStore を vi.mock でモック
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'

// -------------------------------------------------------------------------
// exportStore モック
// -------------------------------------------------------------------------

const {
  mockSetCsvTarget,
  mockExportCsv,
  mockUnpinTrial,
  mockUpdatePinMemo,
  mockClearExportError,
  mockClearPinError,
} = vi.hoisted(() => ({
  mockSetCsvTarget: vi.fn(),
  mockExportCsv: vi.fn(),
  mockUnpinTrial: vi.fn(),
  mockUpdatePinMemo: vi.fn(),
  mockClearExportError: vi.fn(),
  mockClearPinError: vi.fn(),
}))

vi.mock('../../stores/exportStore', () => ({
  MAX_PINS: 20,
  useExportStore: vi
    .fn()
    .mockImplementation(
      (
        selector: (s: {
          csvTarget: string
          selectedColumns: string[]
          isExporting: boolean
          exportError: string | null
          pinnedTrials: Array<{ index: number; trialId: number; memo: string }>
          pinError: string | null
          setCsvTarget: typeof mockSetCsvTarget
          setSelectedColumns: () => void
          exportCsv: typeof mockExportCsv
          unpinTrial: typeof mockUnpinTrial
          updatePinMemo: typeof mockUpdatePinMemo
          clearExportError: typeof mockClearExportError
          clearPinError: typeof mockClearPinError
        }) => unknown,
      ) =>
        selector({
          csvTarget: 'all',
          selectedColumns: [],
          isExporting: false,
          exportError: null,
          pinnedTrials: [],
          pinError: null,
          setCsvTarget: mockSetCsvTarget,
          setSelectedColumns: vi.fn(),
          exportCsv: mockExportCsv,
          unpinTrial: mockUnpinTrial,
          updatePinMemo: mockUpdatePinMemo,
          clearExportError: mockClearExportError,
          clearPinError: mockClearPinError,
        }),
    ),
}))

// -------------------------------------------------------------------------
// selectionStore モック
// -------------------------------------------------------------------------

vi.mock('../../stores/selectionStore', () => ({
  useSelectionStore: vi
    .fn()
    .mockImplementation((selector: (s: { selectedIndices: Uint32Array }) => unknown) =>
      selector({ selectedIndices: new Uint32Array([0, 1, 2]) }),
    ),
}))

// -------------------------------------------------------------------------
// studyStore モック
// -------------------------------------------------------------------------

vi.mock('../../stores/studyStore', () => ({
  useStudyStore: vi
    .fn()
    .mockImplementation(
      (
        selector: (s: {
          currentStudy: { paramNames: string[]; objectiveNames: string[] } | null
        }) => unknown,
      ) =>
        selector({
          currentStudy: { paramNames: ['x1'], objectiveNames: ['obj0'] },
        }),
    ),
}))

import { ExportPanel } from './ExportPanel'
import { useExportStore } from '../../stores/exportStore'

// -------------------------------------------------------------------------
// 正常系
// -------------------------------------------------------------------------

describe('ExportPanel — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    cleanup()
  })

  // TC-1101-18: エクスポートパネルがレンダリングされる
  test('TC-1101-18: ExportPanel がエラーなくレンダリングされる', () => {
    // 【テスト目的】: ExportPanel が正常にレンダリングできることを確認 🟢
    expect(() => render(<ExportPanel />)).not.toThrow()
  })

  // TC-1101-19: 対象ラジオボタンが表示される
  test('TC-1101-19: 対象選択ラジオボタン（全件・選択中・Pareto解・クラスタ）が表示される', () => {
    // 【テスト目的】: CSV 対象選択のすべてのオプションが表示されることを確認 🟢
    render(<ExportPanel />)
    expect(screen.getByTestId('csv-target-all')).toBeInTheDocument()
    expect(screen.getByTestId('csv-target-selected')).toBeInTheDocument()
    expect(screen.getByTestId('csv-target-pareto')).toBeInTheDocument()
    expect(screen.getByTestId('csv-target-cluster')).toBeInTheDocument()
  })

  // TC-1101-20: 対象変更で setCsvTarget が呼ばれる
  test('TC-1101-20: 「選択中」ラジオ選択で setCsvTarget("selected") が呼ばれる', () => {
    // 【テスト目的】: ラジオ選択が Store に反映されることを確認 🟢
    render(<ExportPanel />)
    fireEvent.click(screen.getByTestId('csv-target-selected'))
    expect(mockSetCsvTarget).toHaveBeenCalledWith('selected')
  })

  // TC-1101-21: ダウンロードボタンクリックで exportCsv が呼ばれる
  test('TC-1101-21: ダウンロードボタンクリックで exportCsv が呼ばれる', () => {
    // 【テスト目的】: ダウンロードボタンがエクスポートアクションをトリガーすることを確認 🟢
    render(<ExportPanel />)
    fireEvent.click(screen.getByTestId('export-csv-btn'))
    expect(mockExportCsv).toHaveBeenCalledOnce()
  })

  // TC-1101-22: ピン留めなし時は「ピン留めはありません」を表示
  test('TC-1101-22: pinnedTrials=[] のとき「ピン留めはありません」が表示される', () => {
    // 【テスト目的】: ピン留め未登録時に空状態UIが表示されることを確認 🟢
    render(<ExportPanel />)
    expect(screen.getByText('No pins yet')).toBeInTheDocument()
  })
})

// -------------------------------------------------------------------------
// ローディング
// -------------------------------------------------------------------------

describe('ExportPanel — ローディング', () => {
  afterEach(() => {
    cleanup()
  })

  // TC-1101-L01: isExporting=true でボタンが disabled になる
  test('TC-1101-L01: isExporting=true のときダウンロードボタンが disabled になる', () => {
    // 【テスト目的】: エクスポート中は重複実行を防ぐことを確認 🟢
    ;(useExportStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (
        selector: (s: {
          csvTarget: string
          selectedColumns: string[]
          isExporting: boolean
          exportError: null
          pinnedTrials: []
          pinError: null
          setCsvTarget: () => void
          setSelectedColumns: () => void
          exportCsv: typeof mockExportCsv
          unpinTrial: () => void
          updatePinMemo: () => void
          clearExportError: () => void
          clearPinError: () => void
        }) => unknown,
      ) =>
        selector({
          csvTarget: 'all',
          selectedColumns: [],
          isExporting: true, // 【注目】: エクスポート中
          exportError: null,
          pinnedTrials: [],
          pinError: null,
          setCsvTarget: vi.fn(),
          setSelectedColumns: vi.fn(),
          exportCsv: mockExportCsv,
          unpinTrial: vi.fn(),
          updatePinMemo: vi.fn(),
          clearExportError: vi.fn(),
          clearPinError: vi.fn(),
        }),
    )

    render(<ExportPanel />)
    // 【確認内容】: ダウンロードボタンが disabled
    expect(screen.getByTestId('export-csv-btn')).toBeDisabled()
    // 【確認内容】: スピナーが表示されること
    expect(screen.getByTestId('export-spinner')).toBeInTheDocument()
  })
})

// -------------------------------------------------------------------------
// 異常系
// -------------------------------------------------------------------------

describe('ExportPanel — 異常系', () => {
  afterEach(() => {
    cleanup()
  })

  // TC-1101-E03: exportError あり時にエラーメッセージが表示される
  test('TC-1101-E03: exportError があれば export-error が表示される', () => {
    // 【テスト目的】: エクスポートエラーが UI に表示されることを確認 🟢
    ;(useExportStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (
        selector: (s: {
          csvTarget: string
          selectedColumns: string[]
          isExporting: boolean
          exportError: string
          pinnedTrials: []
          pinError: null
          setCsvTarget: () => void
          setSelectedColumns: () => void
          exportCsv: () => void
          unpinTrial: () => void
          updatePinMemo: () => void
          clearExportError: typeof mockClearExportError
          clearPinError: () => void
        }) => unknown,
      ) =>
        selector({
          csvTarget: 'all',
          selectedColumns: [],
          isExporting: false,
          exportError: 'No data to export', // 【注目】: エラーあり
          pinnedTrials: [],
          pinError: null,
          setCsvTarget: vi.fn(),
          setSelectedColumns: vi.fn(),
          exportCsv: vi.fn(),
          unpinTrial: vi.fn(),
          updatePinMemo: vi.fn(),
          clearExportError: mockClearExportError,
          clearPinError: vi.fn(),
        }),
    )

    render(<ExportPanel />)
    // 【確認内容】: export-error に「対象データがありません」が表示されること
    expect(screen.getByTestId('export-error')).toHaveTextContent('No data to export')
  })

  // TC-1101-E04: pinError あり時に pin-error が表示される
  test('TC-1101-E04: pinError があれば pin-error が表示される', () => {
    // 【テスト目的】: ピン留め上限エラーが UI に表示されることを確認 🟢
    ;(useExportStore as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (
        selector: (s: {
          csvTarget: string
          selectedColumns: string[]
          isExporting: boolean
          exportError: null
          pinnedTrials: []
          pinError: string
          setCsvTarget: () => void
          setSelectedColumns: () => void
          exportCsv: () => void
          unpinTrial: () => void
          updatePinMemo: () => void
          clearExportError: () => void
          clearPinError: typeof mockClearPinError
        }) => unknown,
      ) =>
        selector({
          csvTarget: 'all',
          selectedColumns: [],
          isExporting: false,
          exportError: null,
          pinnedTrials: [],
          pinError: 'Limit is 20. Please remove an old pin first.', // 【注目】: pinError あり
          setCsvTarget: vi.fn(),
          setSelectedColumns: vi.fn(),
          exportCsv: vi.fn(),
          unpinTrial: vi.fn(),
          updatePinMemo: vi.fn(),
          clearExportError: vi.fn(),
          clearPinError: mockClearPinError,
        }),
    )

    render(<ExportPanel />)
    // 【確認内容】: pin-error が表示されること
    expect(screen.getByTestId('pin-error')).toHaveTextContent('Limit is 20')
  })
})
