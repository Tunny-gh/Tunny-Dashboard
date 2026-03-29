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

const { mockAppendDiff } = vi.hoisted(() => {
  const mockAppendDiff = vi.fn()
  return { mockAppendDiff }
})

vi.mock('./wasmLoader', () => ({
  WasmLoader: {
    getInstance: vi.fn().mockResolvedValue({
      appendJournalDiff: mockAppendDiff,
    }),
  },
}))

import { FsapiPoller } from './fsapiPoller'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/** Documentation. */
function makeFileHandle(fileSize: number): FileSystemFileHandle {
  const mockFile = {
    size: fileSize,
    slice: vi.fn().mockReturnValue({
      arrayBuffer: vi.fn().mockResolvedValue(new ArrayBuffer(fileSize)),
    }),
  } as unknown as File

  return {
    getFile: vi.fn().mockResolvedValue(mockFile),
    kind: 'file',
    name: 'test.log',
  } as unknown as FileSystemFileHandle
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  // Documentation.
  test('TC-1201-F01', () => {
    // Documentation.
    const orig = (window as Record<string, unknown>).showOpenFilePicker
    ;(window as Record<string, unknown>).showOpenFilePicker = vi.fn()
    // Documentation.
    expect(FsapiPoller.isSupported()).toBe(true)
    // Documentation.
    if (orig === undefined) {
      delete (window as Record<string, unknown>).showOpenFilePicker
    } else {
      ;(window as Record<string, unknown>).showOpenFilePicker = orig
    }
  })

  // Documentation.
  test('TC-1201-F02', () => {
    // Documentation.
    const orig = (window as Record<string, unknown>).showOpenFilePicker
    delete (window as Record<string, unknown>).showOpenFilePicker
    // Documentation.
    expect(FsapiPoller.isSupported()).toBe(false)
    // Documentation.
    if (orig !== undefined) {
      ;(window as Record<string, unknown>).showOpenFilePicker = orig
    }
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockAppendDiff.mockReturnValue({ new_completed: 0, consumed_bytes: 0 })
  })

  // Documentation.
  test('TC-1201-P01', async () => {
    // Documentation.
    mockAppendDiff.mockReturnValue({ new_completed: 3, consumed_bytes: 100 })

    const onNewTrials = vi.fn()
    const poller = new FsapiPoller({
      onNewTrials,
      onError: vi.fn(),
    })

    // Documentation.
    ;(poller as unknown as Record<string, unknown>)['fileHandle'] = makeFileHandle(200)

    await poller.poll()

    // Documentation.
    expect(onNewTrials).toHaveBeenCalledWith(3)
  })

  // Documentation.
  test('TC-1201-P02', async () => {
    // Documentation.
    const poller = new FsapiPoller({ onNewTrials: vi.fn(), onError: vi.fn() })
    ;(poller as unknown as Record<string, unknown>)['fileHandle'] = makeFileHandle(0)
    ;(poller as unknown as Record<string, unknown>)['byteOffset'] = 0

    await poller.poll()

    // Documentation.
    expect(mockAppendDiff).not.toHaveBeenCalled()
  })

  // Documentation.
  test('TC-1201-P03', async () => {
    // Documentation.
    mockAppendDiff.mockReturnValue({ new_completed: 0, consumed_bytes: 50 })

    const poller = new FsapiPoller({ onNewTrials: vi.fn(), onError: vi.fn() })
    ;(poller as unknown as Record<string, unknown>)['fileHandle'] = makeFileHandle(100)

    await poller.poll()

    // Documentation.
    expect(poller.offset).toBe(50)
  })

  // Documentation.
  test('TC-1201-P04', async () => {
    // Documentation.
    const badHandle = {
      getFile: vi.fn().mockRejectedValue(new Error('File not found')),
    } as unknown as FileSystemFileHandle

    const onAutoStop = vi.fn()
    const poller = new FsapiPoller({
      onNewTrials: vi.fn(),
      onError: vi.fn(),
      onAutoStop,
    })
    ;(poller as unknown as Record<string, unknown>)['fileHandle'] = badHandle
    ;(poller as unknown as Record<string, unknown>)['isRunning'] = true

    // Documentation.
    await poller.poll()
    await poller.poll()
    await poller.poll()

    // Documentation.
    expect(onAutoStop).toHaveBeenCalledOnce()
    // Documentation.
    expect(poller.running).toBe(false)
  })
})
