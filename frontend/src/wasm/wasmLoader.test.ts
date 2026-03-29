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
