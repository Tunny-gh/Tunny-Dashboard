/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 * Design:
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
interface RenderMessage {
  type: 'render'
  /** Documentation. */
  row: number
  /** Documentation. */
  col: number
  /** Documentation. */
  size: number
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */
self.onmessage = (e: MessageEvent<RenderMessage>) => {
  const { type, row, col, size } = e.data

  // Documentation.
  if (type !== 'render') return

  try {
    // Documentation.
    const canvas = new OffscreenCanvas(size, size)
    const ctx = canvas.getContext('2d')

    if (!ctx) {
      // Documentation.
      self.postMessage({ type: 'done', row, col, imageData: null })
      return
    }

    // Documentation.
    ctx.fillStyle = '#f9fafb'
    ctx.fillRect(0, 0, size, size)

    // Documentation.
    // Documentation.
    // Documentation.

    // Documentation.
    const imageData = ctx.getImageData(0, 0, size, size)

    // Documentation.
    // Documentation.
    self.postMessage({ type: 'done', row, col, imageData }, { transfer: [imageData.data.buffer] })
  } catch (_err) {
    // Documentation.
    // Documentation.
    self.postMessage({ type: 'done', row, col, imageData: null })
  }
}
