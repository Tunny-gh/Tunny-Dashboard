/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, vi } from 'vitest'

// -------------------------------------------------------------------------
// Documentation.
// Documentation.
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

async function flushMicrotasks() {
  await Promise.resolve()
  await Promise.resolve()
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
function makeFile(content: string = 'dummy journal') {
  return new File([content], 'journal.log', { type: 'text/plain' })
}

/**
 * Documentation.
 */
function resetStore() {
  useStudyStore.setState({
    currentStudy: null,
    allStudies: [],
    studyMode: 'single-objective',
    isLoading: false,
    loadError: null,
    gpuBuffer: null,
    trialRows: [],
  })
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
    mockGetInstance.mockResolvedValue({
      parseJournal: mockParseJournal,
      selectStudy: mockSelectStudy,
    })
  })

  // Documentation.
  test('TC-302-09', async () => {
    // Documentation.
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

    // Documentation.
    await useStudyStore.getState().loadJournal(makeFile())

    // Documentation.
    expect(mockParseJournal).toHaveBeenCalledOnce()
    // Documentation.
    expect(useStudyStore.getState().allStudies).toEqual(mockStudies)
    // Documentation.
    expect(useStudyStore.getState().isLoading).toBe(false)
  })

  // Documentation.
  test('TC-302-10', async () => {
    // Documentation.
    mockParseJournal.mockReturnValue({ studies: [], durationMs: 0 })

    // Documentation.
    const loadPromise = useStudyStore.getState().loadJournal(makeFile())

    // Documentation.
    expect(useStudyStore.getState().isLoading).toBe(true)

    // Documentation.
    await loadPromise

    // Documentation.
    expect(useStudyStore.getState().isLoading).toBe(false)
  })

  test('shows a dedicated message for an empty log file', async () => {
    useStudyStore.setState({
      currentStudy: {
        studyId: 9,
        name: 'previous-study',
        directions: ['minimize'],
        completedTrials: 1,
        totalTrials: 1,
        paramNames: ['x'],
        objectiveNames: ['y'],
        userAttrNames: [],
        hasConstraints: false,
      },
      allStudies: [
        {
          studyId: 9,
          name: 'previous-study',
          directions: ['minimize'],
          completedTrials: 1,
          totalTrials: 1,
          paramNames: ['x'],
          objectiveNames: ['y'],
          userAttrNames: [],
          hasConstraints: false,
        },
      ],
    })

    await useStudyStore.getState().loadJournal(makeFile(''))

    expect(mockGetInstance).not.toHaveBeenCalled()
    expect(useStudyStore.getState().currentStudy).toBeNull()
    expect(useStudyStore.getState().allStudies).toEqual([])
    expect(useStudyStore.getState().loadError).toBe('The selected log file is empty.')
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    resetStore()
    vi.clearAllMocks()
    mockGetInstance.mockResolvedValue({
      parseJournal: mockParseJournal,
      selectStudy: mockSelectStudy,
    })
  })

  // Documentation.
  test('TC-302-E01', async () => {
    // Documentation.
    mockParseJournal.mockImplementation(() => {
      throw new Error('parse failed')
    })

    // Documentation.
    await useStudyStore.getState().loadJournal(makeFile())

    // Documentation.
    expect(useStudyStore.getState().loadError).toContain('parse failed')
    // Documentation.
    expect(useStudyStore.getState().isLoading).toBe(false)
    // Documentation.
    expect(useStudyStore.getState().allStudies).toEqual([])
  })

  test('surfaces selectStudy failures after a successful parse', async () => {
    const mockStudies = [
      {
        studyId: 1,
        name: 'study-1',
        directions: ['minimize'],
        completedTrials: 1,
        totalTrials: 1,
        paramNames: ['x'],
        objectiveNames: ['y'],
        userAttrNames: [],
        hasConstraints: false,
      },
    ]
    mockParseJournal.mockReturnValue({ studies: mockStudies, durationMs: 10 })
    mockSelectStudy.mockImplementation(() => {
      throw new Error('select failed')
    })

    await useStudyStore.getState().loadJournal(makeFile())
    await flushMicrotasks()

    expect(useStudyStore.getState().loadError).toContain('select failed')
    expect(useStudyStore.getState().currentStudy).toBeNull()
    expect(useStudyStore.getState().trialRows).toEqual([])
  })
})
