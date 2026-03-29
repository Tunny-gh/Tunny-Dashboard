/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */

// -------------------------------------------------------------------------
// Type definitions
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 */
export interface GpuBufferInitData {
  positions: ArrayBuffer // Documentation.
  positions3d: ArrayBuffer // Documentation.
  sizes: ArrayBuffer // Documentation.
  trialCount: number
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */
export class GpuBuffer {
  /** Documentation. */
  readonly positions: Float32Array

  /** Documentation. */
  readonly positions3d: Float32Array

  /** Documentation. */
  readonly sizes: Float32Array

  /** Documentation. */
  readonly colors: Float32Array

  /** Documentation. */
  readonly trialCount: number

  /**
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   */
  constructor(data: GpuBufferInitData, defaultRgb: [number, number, number] = [1.0, 1.0, 1.0]) {
    this.trialCount = data.trialCount

    // Documentation.
    this.positions = new Float32Array(data.positions)
    this.positions3d = new Float32Array(data.positions3d)
    this.sizes = new Float32Array(data.sizes)

    // Documentation.
    this.colors = new Float32Array(data.trialCount * 4)
    for (let i = 0; i < data.trialCount; i++) {
      this.colors[i * 4 + 0] = defaultRgb[0] // R
      this.colors[i * 4 + 1] = defaultRgb[1] // G
      this.colors[i * 4 + 2] = defaultRgb[2] // B
      this.colors[i * 4 + 3] = 1.0 // Documentation.
    }
  }

  /**
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   */
  updateAlphas(
    selectedIndices: Uint32Array,
    selectedAlpha: number = 1.0,
    deselectedAlpha: number = 0.2,
  ): void {
    // Documentation.
    const c = this.colors
    const limit = this.trialCount * 4

    // Documentation.
    // Documentation.
    for (let i = 3; i < limit; i += 4) {
      c[i] = deselectedAlpha
    }

    // Documentation.
    const len = selectedIndices.length
    for (let i = 0; i < len; i++) {
      c[(selectedIndices[i] << 2) + 3] = selectedAlpha // Documentation.
    }
  }

  /**
   * Documentation.
   * Documentation.
   * Documentation.
   */
  resetAlphas(): void {
    // Documentation.
    const c = this.colors
    const limit = this.trialCount * 4
    for (let i = 3; i < limit; i += 4) {
      c[i] = 1.0
    }
  }
}
