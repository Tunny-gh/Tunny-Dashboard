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
// Documentation.
// -------------------------------------------------------------------------

vi.mock('../../stores/selectionStore', () => ({
  useSelectionStore: vi
    .fn()
    .mockImplementation((selector: (s: { selectedIndices: Uint32Array }) => unknown) =>
      selector({ selectedIndices: new Uint32Array([0, 1, 2]) }),
    ),
}))

// -------------------------------------------------------------------------
// Documentation.
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
  test('TC-1101-18', () => {
    // Documentation.
    expect(() => render(<ExportPanel />)).not.toThrow()
  })

  // Documentation.
  test('TC-1101-19', () => {
    // Documentation.
    render(<ExportPanel />)
    expect(screen.getByTestId('csv-target-all')).toBeInTheDocument()
    expect(screen.getByTestId('csv-target-selected')).toBeInTheDocument()
    expect(screen.getByTestId('csv-target-pareto')).toBeInTheDocument()
    expect(screen.getByTestId('csv-target-cluster')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-1101-20', () => {
    // Documentation.
    render(<ExportPanel />)
    fireEvent.click(screen.getByTestId('csv-target-selected'))
    expect(mockSetCsvTarget).toHaveBeenCalledWith('selected')
  })

  // Documentation.
  test('TC-1101-21', () => {
    // Documentation.
    render(<ExportPanel />)
    fireEvent.click(screen.getByTestId('export-csv-btn'))
    expect(mockExportCsv).toHaveBeenCalledOnce()
  })

  // Documentation.
  test('TC-1101-22', () => {
    // Documentation.
    render(<ExportPanel />)
    expect(screen.getByText('No pins yet')).toBeInTheDocument()
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
  test('TC-1101-L01', () => {
    // Documentation.
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
          isExporting: true, // Documentation.
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
    // Documentation.
    expect(screen.getByTestId('export-csv-btn')).toBeDisabled()
    // Documentation.
    expect(screen.getByTestId('export-spinner')).toBeInTheDocument()
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
  test('TC-1101-E03', () => {
    // Documentation.
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
          exportError: 'No data to export', // Documentation.
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
    // Documentation.
    expect(screen.getByTestId('export-error')).toHaveTextContent('No data to export')
  })

  // Documentation.
  test('TC-1101-E04', () => {
    // Documentation.
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
          pinError: 'Limit is 20. Please remove an old pin first.', // Documentation.
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
    // Documentation.
    expect(screen.getByTestId('pin-error')).toHaveTextContent('Limit is 20')
  })
})
