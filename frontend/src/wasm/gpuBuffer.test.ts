/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach } from 'vitest'
import { GpuBuffer } from './gpuBuffer'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 */
function makeGpuBufferData(n: number) {
  const positions = new Float32Array(n * 2)
  const positions3d = new Float32Array(n * 3)
  const sizes = new Float32Array(n * 1)

  // Documentation.
  for (let i = 0; i < n * 2; i++) positions[i] = i * 0.01 + 0.001
  for (let i = 0; i < n * 3; i++) positions3d[i] = i * 0.01 + 0.001
  for (let i = 0; i < n; i++) sizes[i] = i * 0.1 + 1.0

  return {
    positions: positions.buffer,
    positions3d: positions3d.buffer,
    sizes: sizes.buffer,
    trialCount: n,
  }
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  // Documentation.
  test('TC-301-03', () => {
    // Documentation.
    const data = makeGpuBufferData(5)
    const buf = new GpuBuffer(data)

    // Documentation.
    expect(buf.trialCount).toBe(5)
    // Documentation.
    expect(buf.positions.length).toBe(10)
    // Documentation.
    expect(buf.positions3d.length).toBe(15)
    // Documentation.
    expect(buf.sizes.length).toBe(5)
    // Documentation.
    expect(buf.colors.length).toBe(20)
  })

  // Documentation.
  test('TC-301-04', () => {
    // Documentation.
    const data = makeGpuBufferData(3)
    const buf = new GpuBuffer(data, [1.0, 0.5, 0.0])

    // Documentation.
    expect(buf.colors[0]).toBeCloseTo(1.0) // R
    expect(buf.colors[1]).toBeCloseTo(0.5) // G
    expect(buf.colors[2]).toBeCloseTo(0.0) // B
    expect(buf.colors[3]).toBeCloseTo(1.0) // Documentation.
    // Documentation.
    expect(buf.colors[4]).toBeCloseTo(1.0) // R
    expect(buf.colors[5]).toBeCloseTo(0.5) // G
    expect(buf.colors[6]).toBeCloseTo(0.0) // B
    expect(buf.colors[7]).toBeCloseTo(1.0) // A = 1.0
  })

  // Documentation.
  test('TC-301-05', () => {
    // Documentation.
    const data = makeGpuBufferData(10)
    const buf = new GpuBuffer(data)
    const selected = new Uint32Array([2, 5, 7])
    buf.updateAlphas(selected)

    // Documentation.
    expect(buf.colors[2 * 4 + 3]).toBeCloseTo(1.0) // Documentation.
    expect(buf.colors[5 * 4 + 3]).toBeCloseTo(1.0) // Documentation.
    expect(buf.colors[7 * 4 + 3]).toBeCloseTo(1.0) // Documentation.
    // Documentation.
    expect(buf.colors[0 * 4 + 3]).toBeCloseTo(0.2) // Documentation.
    expect(buf.colors[3 * 4 + 3]).toBeCloseTo(0.2) // Documentation.
    expect(buf.colors[9 * 4 + 3]).toBeCloseTo(0.2) // Documentation.
  })

  // Documentation.
  test('TC-301-06', () => {
    // Documentation.
    const data = makeGpuBufferData(5)
    const buf = new GpuBuffer(data)

    // Documentation.
    const positionsBefore = new Float32Array(buf.positions)
    buf.updateAlphas(new Uint32Array([1, 3]))

    // Documentation.
    for (let i = 0; i < positionsBefore.length; i++) {
      expect(buf.positions[i]).toBeCloseTo(positionsBefore[i])
    }
  })

  // Documentation.
  test('TC-301-07', () => {
    // Documentation.
    const data = makeGpuBufferData(5)
    const buf = new GpuBuffer(data)

    const sizesBefore = new Float32Array(buf.sizes)
    buf.updateAlphas(new Uint32Array([0, 2, 4]))

    // Documentation.
    for (let i = 0; i < sizesBefore.length; i++) {
      expect(buf.sizes[i]).toBeCloseTo(sizesBefore[i])
    }
  })

  // Documentation.
  test('TC-301-08', () => {
    // Documentation.
    const data = makeGpuBufferData(5)
    const buf = new GpuBuffer(data)

    buf.updateAlphas(new Uint32Array([1])) // Documentation.
    buf.resetAlphas()

    // Documentation.
    for (let i = 0; i < 5; i++) {
      expect(buf.colors[i * 4 + 3]).toBeCloseTo(1.0)
    }
  })

  // Documentation.
  test('TC-301-09', () => {
    // Documentation.
    const data = makeGpuBufferData(5)
    const buf = new GpuBuffer(data)
    buf.updateAlphas(new Uint32Array(0)) // Documentation.

    // Documentation.
    for (let i = 0; i < 5; i++) {
      expect(buf.colors[i * 4 + 3]).toBeCloseTo(0.2)
    }
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  // Documentation.
  test('TC-301-E02', () => {
    // Documentation.
    const data = makeGpuBufferData(0)
    const buf = new GpuBuffer(data)

    // Documentation.
    expect(buf.trialCount).toBe(0)
    expect(buf.positions.length).toBe(0)
    expect(buf.colors.length).toBe(0)
    // Documentation.
    expect(() => buf.updateAlphas(new Uint32Array(0))).not.toThrow()
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  // Documentation.
  test('TC-301-B01', () => {
    // Documentation.
    const data = makeGpuBufferData(5)
    const buf = new GpuBuffer(data)
    buf.updateAlphas(new Uint32Array([0, 1, 2, 3, 4]))

    // Documentation.
    for (let i = 0; i < 5; i++) {
      expect(buf.colors[i * 4 + 3]).toBeCloseTo(1.0)
    }
  })

  // Documentation.
  test('TC-301-B02', () => {
    // Documentation.
    const data = makeGpuBufferData(5)
    const buf = new GpuBuffer(data)
    buf.updateAlphas(new Uint32Array([4]))

    // Documentation.
    expect(buf.colors[4 * 4 + 3]).toBeCloseTo(1.0)
    // Documentation.
    expect(buf.colors[0 * 4 + 3]).toBeCloseTo(0.2)
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  // Documentation.
  test('TC-301-P01', () => {
    // Documentation.
    const n = 50_000
    const data = makeGpuBufferData(n)
    const buf = new GpuBuffer(data)

    // Documentation.
    const selectedCount = n / 2
    const selected = new Uint32Array(selectedCount)
    for (let i = 0; i < selectedCount; i++) {
      selected[i] = i * 2 // Documentation.
    }

    const start = performance.now()
    buf.updateAlphas(selected)
    const elapsed = performance.now() - start

    // Documentation.
    expect(elapsed).toBeLessThan(1)
    expect(buf.trialCount).toBe(n)
  })
})
