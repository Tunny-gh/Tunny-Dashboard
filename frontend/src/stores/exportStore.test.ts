/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, vi } from 'vitest'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

const { mockSerializeCsv, mockComputeReportStats } = vi.hoisted(() => {
  const mockSerializeCsv = vi.fn()
  const mockComputeReportStats = vi
    .fn()
    .mockReturnValue('{"x1":{"min":1,"max":3,"mean":2,"std":1,"count":2}}')
  return { mockSerializeCsv, mockComputeReportStats }
})

vi.mock('../wasm/wasmLoader', () => ({
  WasmLoader: {
    getInstance: vi.fn().mockResolvedValue({
      serializeCsv: mockSerializeCsv,
      computeReportStats: mockComputeReportStats,
    }),
  },
}))

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

const mockBrushSelect = vi.fn()
const mockAddAxisFilter = vi.fn()
const mockClearSelection = vi.fn()
const mockSetColorMode = vi.fn()
const mockSetLayoutMode = vi.fn()
const mockLoadLayout = vi.fn()

vi.mock('./selectionStore', () => ({
  useSelectionStore: {
    getState: vi.fn(() => ({
      selectedIndices: new Uint32Array([0, 1, 2]),
      filterRanges: { x1: { min: 0, max: 1 } },
      colorMode: 'objective',
      brushSelect: mockBrushSelect,
      addAxisFilter: mockAddAxisFilter,
      clearSelection: mockClearSelection,
      setColorMode: mockSetColorMode,
    })),
  },
}))

vi.mock('./layoutStore', () => ({
  useLayoutStore: {
    getState: vi.fn(() => ({
      layoutMode: 'A',
      visibleCharts: new Set(['pareto-front', 'history']),
      freeModeLayout: null,
      setLayoutMode: mockSetLayoutMode,
      loadLayout: mockLoadLayout,
    })),
  },
}))

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

// Documentation.
Object.defineProperty(globalThis, 'URL', {
  value: {
    createObjectURL: vi.fn(() => 'blob:mock-url'),
    revokeObjectURL: vi.fn(),
  },
  writable: true,
})

// Documentation.
const mockClick = vi.fn()
const mockAppendChild = vi.fn()
const mockRemoveChild = vi.fn()
vi.spyOn(document.body, 'appendChild').mockImplementation(mockAppendChild)
vi.spyOn(document.body, 'removeChild').mockImplementation(mockRemoveChild)

// Documentation.
const origCreateElement = document.createElement.bind(document)
vi.spyOn(document, 'createElement').mockImplementation((tag: string) => {
  const el = origCreateElement(tag)
  if (tag === 'a') {
    Object.defineProperty(el, 'click', { value: mockClick, writable: true })
  }
  return el
})

import { useExportStore, MAX_PINS } from './exportStore'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    // Documentation.
    useExportStore.setState({
      csvTarget: 'all',
      selectedColumns: [],
      isExporting: false,
      exportError: null,
      pinnedTrials: [],
      pinError: null,
    })
    vi.clearAllMocks()
    mockSerializeCsv.mockReturnValue('trial_id,x1\n0,1.5\n')
  })

  // Documentation.
  test('TC-1101-11', () => {
    // Documentation.
    useExportStore.getState().setCsvTarget('selected')
    // Documentation.
    expect(useExportStore.getState().csvTarget).toBe('selected')
  })

  // Documentation.
  test('TC-1101-12', async () => {
    // Documentation.
    await useExportStore.getState().exportCsv(new Uint32Array([0, 1, 2]))
    // Documentation.
    expect(mockSerializeCsv).toHaveBeenCalledOnce()
  })

  // Documentation.
  test('TC-1101-13', async () => {
    // Documentation.
    await useExportStore.getState().exportCsv(new Uint32Array([0]))
    // Documentation.
    expect(useExportStore.getState().isExporting).toBe(false)
  })

  // Documentation.
  test('TC-1101-14', () => {
    // Documentation.
    useExportStore.getState().pinTrial(0, 100)
    // Documentation.
    const { pinnedTrials } = useExportStore.getState()
    expect(pinnedTrials).toHaveLength(1)
    expect(pinnedTrials[0].trialId).toBe(100)
  })

  // Documentation.
  test('TC-1101-15', () => {
    // Documentation.
    useExportStore.getState().pinTrial(0, 100)
    useExportStore.getState().unpinTrial(0)
    // Documentation.
    expect(useExportStore.getState().pinnedTrials).toHaveLength(0)
  })

  // Documentation.
  test('TC-1101-16', () => {
    // Documentation.
    useExportStore.getState().pinTrial(0, 100)
    useExportStore.getState().updatePinMemo(0, 'translated')
    // Documentation.
    const pin = useExportStore.getState().pinnedTrials[0]
    expect(pin.memo).toBe('translated')
  })

  // Documentation.
  test('TC-1101-17', () => {
    // Documentation.
    useExportStore.getState().pinTrial(0, 100)
    useExportStore.getState().pinTrial(0, 100)
    // Documentation.
    expect(useExportStore.getState().pinnedTrials).toHaveLength(1)
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    useExportStore.setState({
      csvTarget: 'all',
      selectedColumns: [],
      isExporting: false,
      exportError: null,
      pinnedTrials: [],
      pinError: null,
    })
    vi.clearAllMocks()
    mockSerializeCsv.mockReturnValue('trial_id\n')
  })

  // Documentation.
  test('TC-1101-E01', async () => {
    // Documentation.
    await useExportStore.getState().exportCsv(new Uint32Array([]))
    // Documentation.
    expect(useExportStore.getState().exportError).toBe('No data to export')
    // Documentation.
    expect(mockSerializeCsv).not.toHaveBeenCalled()
  })

  // Documentation.
  test(`TC-1101-E02: pinError is shown when the pin limit exceeds ${MAX_PINS}`, () => {
    // Documentation.
    // Documentation.
    for (let i = 0; i < MAX_PINS; i++) {
      useExportStore.getState().pinTrial(i, i)
    }
    // Documentation.
    useExportStore.getState().pinTrial(MAX_PINS, MAX_PINS)
    // Documentation.
    expect(useExportStore.getState().pinError).toMatch(/Limit/)
    // Documentation.
    expect(useExportStore.getState().pinnedTrials).toHaveLength(MAX_PINS)
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('ExportStore — generateHtmlReport (TASK-1102)', () => {
  beforeEach(() => {
    useExportStore.setState({
      csvTarget: 'all',
      selectedColumns: [],
      isExporting: false,
      exportError: null,
      pinnedTrials: [],
      pinError: null,
      reportSections: ['summary', 'pareto', 'pinned', 'history', 'cluster'],
      isGeneratingReport: false,
      reportError: null,
    })
    vi.clearAllMocks()
    mockComputeReportStats.mockReturnValue('{"x1":{"min":1,"max":3,"mean":2,"std":1,"count":2}}')
  })

  // Documentation.
  test('TC-1102-S01', async () => {
    // Documentation.
    await useExportStore.getState().generateHtmlReport(new Uint32Array([0, 1]))
    // Documentation.
    expect(useExportStore.getState().isGeneratingReport).toBe(false)
  })

  // Documentation.
  test('TC-1102-S02', async () => {
    // Documentation.
    await useExportStore.getState().generateHtmlReport(new Uint32Array([0, 1]))
    // Documentation.
    expect(mockComputeReportStats).toHaveBeenCalledOnce()
  })

  // Documentation.
  test('TC-1102-S03', async () => {
    // Documentation.
    await useExportStore.getState().generateHtmlReport(new Uint32Array([0, 1]))
    // Documentation.
    expect(URL.createObjectURL).toHaveBeenCalledOnce()
    // Documentation.
    expect(URL.revokeObjectURL).toHaveBeenCalledOnce()
  })

  // Documentation.
  test('TC-1102-S04', async () => {
    // Documentation.
    useExportStore.getState().pinTrial(0, 42)
    useExportStore.getState().updatePinMemo(0, 'Best solution')

    // Documentation.
    const capturedContent: string[] = []
    const origBlob = globalThis.Blob
    globalThis.Blob = class MockBlob {
      constructor(parts: BlobPart[], _opts?: BlobPropertyBag) {
        capturedContent.push(...(parts as string[]))
      }
    } as typeof Blob

    await useExportStore.getState().generateHtmlReport(new Uint32Array([0, 1]))

    globalThis.Blob = origBlob

    // Documentation.
    const html = capturedContent.join('')
    expect(html).toContain('42')
    // Documentation.
    expect(html).toContain('Best solution')
  })

  // Documentation.
  test('TC-1102-S05', () => {
    // Documentation.
    useExportStore.getState().setReportSections(['pinned', 'summary'])
    // Documentation.
    expect(useExportStore.getState().reportSections).toEqual(['pinned', 'summary'])
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

import { SESSION_VERSION } from './exportStore'

describe('translated test case', () => {
  beforeEach(() => {
    useExportStore.setState({
      csvTarget: 'all',
      selectedColumns: [],
      isExporting: false,
      exportError: null,
      pinnedTrials: [],
      pinError: null,
      reportSections: ['summary', 'pareto', 'pinned', 'history', 'cluster'],
      isGeneratingReport: false,
      reportError: null,
      sessionState: null,
      isSavingSession: false,
      sessionError: null,
      sessionWarning: null,
    })
    vi.clearAllMocks()
  })

  // Documentation.
  test('TC-1103-01', async () => {
    // Documentation.
    await useExportStore.getState().saveSession(0, 'test.log')
    // Documentation.
    expect(useExportStore.getState().isSavingSession).toBe(false)
  })

  // Documentation.
  test('TC-1103-02', async () => {
    // Documentation.
    await useExportStore.getState().saveSession(1, '/path/to/journal.log')
    const state = useExportStore.getState().sessionState
    // Documentation.
    expect(state).not.toBeNull()
    // Documentation.
    expect(state?.selectedStudyId).toBe(1)
  })

  // Documentation.
  test('TC-1103-03', async () => {
    // Documentation.
    await useExportStore.getState().saveSession(0, 'test.log')
    // Documentation.
    expect(URL.createObjectURL).toHaveBeenCalledOnce()
  })

  // Documentation.
  test('TC-1103-04', async () => {
    // Documentation.
    await useExportStore.getState().saveSession(0, 'journal.log')
    expect(useExportStore.getState().sessionState?.version).toBe(SESSION_VERSION)
  })
})

describe('translated test case', () => {
  beforeEach(() => {
    useExportStore.setState({
      csvTarget: 'all',
      selectedColumns: [],
      isExporting: false,
      exportError: null,
      pinnedTrials: [],
      pinError: null,
      reportSections: ['summary', 'pareto', 'pinned', 'history', 'cluster'],
      isGeneratingReport: false,
      reportError: null,
      sessionState: null,
      isSavingSession: false,
      sessionError: null,
      sessionWarning: null,
    })
    vi.clearAllMocks()
  })

  // Documentation.
  test('TC-1103-05', () => {
    // Documentation.
    const sessionJson = JSON.stringify({
      version: SESSION_VERSION,
      journalPath: 'test.log',
      selectedStudyId: 2,
      filterRanges: { x1: { min: 0.5, max: 1.5 } },
      selectedIndices: [0, 1, 2],
      colorMode: 'rank',
      clusterConfig: null,
      layoutMode: 'B',
      visibleCharts: ['pareto-front'],
      pinnedTrials: [{ trialId: 42, note: 'translated', pinnedAt: 1234567890000 }],
      freeModeLayout: null,
      savedAt: new Date().toISOString(),
    })

    useExportStore.getState().loadSessionFromJson(sessionJson)

    const state = useExportStore.getState().sessionState
    // Documentation.
    expect(state).not.toBeNull()
    expect(state?.selectedStudyId).toBe(2)
    // Documentation.
    expect(useExportStore.getState().sessionError).toBeNull()
  })

  // Documentation.
  test('TC-1103-06', () => {
    // Documentation.
    useExportStore.getState().loadSessionFromJson('not valid json {{{')
    // Documentation.
    expect(useExportStore.getState().sessionError).toContain('Invalid session file format')
  })

  // Documentation.
  test('TC-1103-07', () => {
    // Documentation.
    const oldSessionJson = JSON.stringify({
      version: '0.9',
      journalPath: 'old.log',
      selectedStudyId: 0,
      filterRanges: {},
      selectedIndices: [],
      colorMode: 'objective',
      clusterConfig: null,
      layoutMode: 'A',
      visibleCharts: [],
      pinnedTrials: [],
      freeModeLayout: null,
      savedAt: new Date().toISOString(),
    })

    useExportStore.getState().loadSessionFromJson(oldSessionJson)

    // Documentation.
    expect(useExportStore.getState().sessionWarning).toContain('Old session version')
    // Documentation.
    expect(useExportStore.getState().sessionState).not.toBeNull()
  })

  // Documentation.
  test('TC-1103-08', () => {
    // Documentation.
    const sessionJson = JSON.stringify({
      version: SESSION_VERSION,
      journalPath: 'test.log',
      selectedStudyId: 0,
      filterRanges: {},
      selectedIndices: [],
      colorMode: 'objective',
      clusterConfig: null,
      layoutMode: 'A',
      visibleCharts: [],
      pinnedTrials: [
        { trialId: 10, note: 'translated1', pinnedAt: 1000000000000 },
        { trialId: 20, note: 'translated2', pinnedAt: 1000000001000 },
      ],
      freeModeLayout: null,
      savedAt: new Date().toISOString(),
    })

    useExportStore.getState().loadSessionFromJson(sessionJson)

    const { pinnedTrials } = useExportStore.getState()
    // Documentation.
    expect(pinnedTrials).toHaveLength(2)
    expect(pinnedTrials[0].trialId).toBe(10)
    expect(pinnedTrials[1].trialId).toBe(20)
  })

  // Documentation.
  test('TC-1103-09', () => {
    // Documentation.
    useExportStore.setState({ sessionError: 'translated', sessionWarning: 'translated' })
    useExportStore.getState().clearSessionMessages()
    expect(useExportStore.getState().sessionError).toBeNull()
    expect(useExportStore.getState().sessionWarning).toBeNull()
  })
})
