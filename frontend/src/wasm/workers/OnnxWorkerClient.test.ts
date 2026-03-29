/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest'
import { OnnxWorkerClient } from './OnnxWorkerClient'
import type { OnnxWorkerResponse } from './onnxWorker'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 *
 * Documentation.
 */
class MockWorker {
  onmessage: ((event: MessageEvent<OnnxWorkerResponse>) => void) | null = null
  onerror: ((event: ErrorEvent) => void) | null = null

  readonly messages: unknown[] = []

  private responseMap: Map<string, OnnxWorkerResponse> = new Map([
    // Documentation.
    [
      'load',
      {
        type: 'loaded',
        success: false,
        error: 'ONNX Runtime is not implemented. Please use the Ridge simplified PDP instead.',
      } as OnnxWorkerResponse,
    ],
    // Documentation.
    ['infer', { type: 'result', outputData: new Float32Array(0) } as OnnxWorkerResponse],
  ])

  postMessage(data: { type: string }): void {
    // Documentation.
    this.messages.push(data)

    // Documentation.
    const response = this.responseMap.get(data.type)
    if (response && this.onmessage) {
      // Documentation.
      Promise.resolve().then(() => {
        this.onmessage?.({ data: response } as MessageEvent<OnnxWorkerResponse>)
      })
    }
  }

  terminate(): void {
    // Documentation.
  }
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

let mockWorker: MockWorker
let client: OnnxWorkerClient

beforeEach(() => {
  // Documentation.
  mockWorker = new MockWorker()
  client = new OnnxWorkerClient(() => mockWorker as unknown as Worker)
})

afterEach(() => {
  // Documentation.
  client.terminate()
  vi.clearAllMocks()
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  // Documentation.
  test('TC-803-W01', async () => {
    // Documentation.
    // Documentation.
    const modelData = new ArrayBuffer(8)

    // Documentation.
    const result = await client.load(modelData)

    // Documentation.
    expect(result.success).toBe(false) // Documentation.
    expect(result.error).toBeDefined() // Documentation.
    expect(result.error).toContain('Ridge') // Documentation.
  })

  // Documentation.
  test('TC-803-W02', async () => {
    // Documentation.
    const modelData = new ArrayBuffer(16)

    await client.load(modelData)

    // Documentation.
    expect(mockWorker.messages).toHaveLength(1)
    expect((mockWorker.messages[0] as { type: string }).type).toBe('load') // Documentation.
  })
})

describe('translated test case', () => {
  // Documentation.
  test('TC-803-W03', async () => {
    // Documentation.
    const inputData = new Float32Array([1.0, 2.0, 3.0])
    const inputShape: [number, number] = [1, 3]

    // Documentation.
    const result = await client.infer(inputData, inputShape)

    // Documentation.
    expect(result.outputData).toBeInstanceOf(Float32Array) // Documentation.
    // Documentation.
    expect(result.outputData.length).toBe(0) // Documentation.
  })

  // Documentation.
  test('TC-803-W04', async () => {
    // Documentation.
    const inputData = new Float32Array([0.5, 1.5])
    const inputShape: [number, number] = [1, 2]

    await client.infer(inputData, inputShape)

    // Documentation.
    expect(mockWorker.messages).toHaveLength(1)
    expect((mockWorker.messages[0] as { type: string }).type).toBe('infer') // Documentation.
  })
})

describe('translated test case', () => {
  // Documentation.
  test('TC-803-W05', async () => {
    // Documentation.
    const modelData = new ArrayBuffer(4)
    await client.load(modelData)

    const msg = mockWorker.messages[0] as { type: string; modelData: ArrayBuffer }

    // Documentation.
    expect(msg.type).toBe('load') // Documentation.
    expect(msg.modelData).toBeInstanceOf(ArrayBuffer) // Documentation.
  })

  test('TC-803-W06', async () => {
    // Documentation.
    const inputData = new Float32Array([1.0])
    const inputShape: [number, number] = [1, 1]

    await client.infer(inputData, inputShape)

    const msg = mockWorker.messages[0] as {
      type: string
      inputData: Float32Array
      inputShape: [number, number]
    }

    // Documentation.
    expect(msg.type).toBe('infer') // Documentation.
    expect(msg.inputData).toBeInstanceOf(Float32Array) // Documentation.
    expect(Array.isArray(msg.inputShape)).toBe(true) // Documentation.
    expect(msg.inputShape).toHaveLength(2) // Documentation.
  })
})
