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

vi.mock('./pkg/tunny_core', () => ({
  // Documentation.
  default: vi.fn().mockResolvedValue({}),
  // English comment.
  computePdp2d: vi.fn().mockReturnValue({
    param1Name: 'x',
    param2Name: 'y',
    objectiveName: 'obj0',
    grid1: [1.0, 2.0, 3.0],
    grid2: [1.0, 2.0, 3.0],
    values: [
      [0.1, 0.2, 0.3],
      [0.4, 0.5, 0.6],
      [0.7, 0.8, 0.9],
    ],
    rSquared: 0.95,
  }),
  // Documentation.
  getTrials: vi.fn().mockReturnValue([]),
  // Documentation.
  filterByRanges: vi.fn().mockReturnValue(new Uint32Array([0, 1, 2])),
  // Documentation.
  serializeCsv: vi.fn().mockReturnValue('trial_id,x\n0,1.5\n'),
  // Documentation.
  computeHvHistory: vi.fn().mockReturnValue({
    trialIds: new Uint32Array([0, 1]),
    hvValues: new Float64Array([0.5, 0.8]),
  }),
  // Documentation.
  appendJournalDiff: vi.fn().mockReturnValue({ new_completed: 2, consumed_bytes: 100 }),
  // Documentation.
  computeReportStats: vi.fn().mockReturnValue('{"x":{"min":1.0,"max":2.0}}'),
  // English comment.
  computeTopsis: vi.fn().mockReturnValue({
    scores: [0.8, 0.3],
    rankedIndices: [0, 1],
    positiveIdeal: [1.0, 2.0],
    negativeIdeal: [5.0, 6.0],
    durationMs: 0.5,
  }),
}))

import { WasmLoader } from './wasmLoader'
import initWasm from './pkg/tunny_core'

// Documentation.
const mockInit = vi.mocked(initWasm)

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    // Documentation.
    WasmLoader.reset()
    // Documentation.
    mockInit.mockClear()
    // Documentation.
    mockInit.mockResolvedValue({})
  })

  // Documentation.
  test('TC-301-01', async () => {
    // Documentation.

    // Documentation.
    const instanceA = await WasmLoader.getInstance()
    const instanceB = await WasmLoader.getInstance()

    // Documentation.
    expect(instanceA).toBe(instanceB)
    // Documentation.
    expect(mockInit).toHaveBeenCalledTimes(1)
  })

  // Documentation.
  test('TC-301-02', async () => {
    // Documentation.

    // Documentation.
    const loader = await WasmLoader.getInstance()

    // Documentation.
    expect(typeof loader.parseJournal).toBe('function') // Documentation.
    expect(typeof loader.selectStudy).toBe('function') // Documentation.
    expect(typeof loader.filterByRanges).toBe('function') // Documentation.
  })

  // Documentation.
  test('TC-301-05', async () => {
    // Documentation.
    const loader = await WasmLoader.getInstance()
    // Documentation.
    const result = loader.filterByRanges('{}')
    expect(result).toBeInstanceOf(Uint32Array)
    expect(Array.from(result)).toEqual([0, 1, 2])
  })

  // Documentation.
  test('TC-301-06', async () => {
    // Documentation.
    const loader = await WasmLoader.getInstance()
    const result = loader.serializeCsv([0, 1], '[]')
    expect(typeof result).toBe('string')
    expect(result).toContain('trial_id')
  })

  // Documentation.
  test('TC-301-07', async () => {
    // Documentation.
    const loader = await WasmLoader.getInstance()
    const result = loader.computeHvHistory([true, true])
    expect(result).toHaveProperty('trialIds')
    expect(result).toHaveProperty('hvValues')
    expect(result.trialIds).toBeInstanceOf(Uint32Array)
    expect(result.hvValues).toBeInstanceOf(Float64Array)
  })

  // Documentation.
  test('TC-301-08', async () => {
    // Documentation.
    const loader = await WasmLoader.getInstance()
    const result = loader.appendJournalDiff(new Uint8Array([1, 2, 3]))
    expect(result).toHaveProperty('new_completed')
    expect(result).toHaveProperty('consumed_bytes')
    expect(result.new_completed).toBe(2)
    expect(result.consumed_bytes).toBe(100)
  })

  // Documentation.
  test('TC-301-09', async () => {
    // Documentation.
    const loader = await WasmLoader.getInstance()
    const result = loader.computeReportStats()
    expect(typeof result).toBe('string')
    expect(() => JSON.parse(result)).not.toThrow()
  })

  // Documentation.
  test('TC-301-03', async () => {
    // Documentation.
    const loader = await WasmLoader.getInstance()
    expect(typeof loader.getTrials).toBe('function')
    // Documentation.
    const result = loader.getTrials()
    expect(Array.isArray(result)).toBe(true)
  })

  // English comment.
  test('TC-1619-01', async () => {
    // English comment.
    // English comment.
    // English comment.
    const loader = await WasmLoader.getInstance()

    // English comment.
    expect(typeof loader.computeTopsis).toBe('function')

    // English comment.
    const values = new Float64Array([1.0, 2.0, 5.0, 6.0])
    const weights = new Float64Array([0.5, 0.5])
    const result = loader.computeTopsis(values, 2, 2, weights, [true, true])

    // English comment.
    expect(result).toHaveProperty('scores')
    expect(result).toHaveProperty('rankedIndices')
    expect(result).toHaveProperty('positiveIdeal')
    expect(result).toHaveProperty('negativeIdeal')
    expect(result).toHaveProperty('durationMs')
    expect(Array.isArray(result.scores)).toBe(true)
    expect(Array.isArray(result.rankedIndices)).toBe(true)
  })

  // English comment.
  test('TC-1626-01', async () => {
    // English comment.
    // English comment.
    const loader = await WasmLoader.getInstance()

    // English comment.
    expect(typeof loader.computePdp2d).toBe('function')

    // English comment.
    const result = loader.computePdp2d('x', 'y', 'obj0', 10)

    // English comment.
    expect(result).toHaveProperty('grid1')
    expect(result).toHaveProperty('grid2')
    expect(result).toHaveProperty('values')
    expect(result).toHaveProperty('rSquared')
    expect(Array.isArray(result.grid1)).toBe(true)
    expect(Array.isArray(result.values)).toBe(true)
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    // Documentation.
    WasmLoader.reset()
    mockInit.mockClear()
  })

  // Documentation.
  test('TC-301-E01', async () => {
    // Documentation.

    // Documentation.
    mockInit.mockRejectedValueOnce(new Error('WASM load failed'))

    // Documentation.
    await expect(WasmLoader.getInstance()).rejects.toThrow('WASM load failed')

    // Documentation.
    // Documentation.
    await expect(WasmLoader.getInstance()).rejects.toThrow()

    // Documentation.
    expect(mockInit).toHaveBeenCalledTimes(1)
  })
})
