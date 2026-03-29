/**
 * Colormap definitions — named palettes used by ScatterMatrix and ParallelCoordinates.
 *
 * Each entry provides both:
 *  - an array of hex stops for ECharts `visualMap.inRange.color`
 *  - an `interpolate(t)` function (t ∈ [0, 1]) for canvas rendering
 */

// -------------------------------------------------------------------------
// Types
// -------------------------------------------------------------------------

export type ColormapName =
  | 'Viridis'
  | 'Plasma'
  | 'Inferno'
  | 'Magma'
  | 'Cividis'
  | 'Turbo'
  | 'Coolwarm'
  | 'Spectral'
  | 'Rainbow'
  | 'Jet'

export interface Colormap {
  /** Human-readable label */
  label: string
  /** Hex colour stops (for ECharts visualMap) */
  stops: string[]
  /** Interpolate t ∈ [0,1] → CSS colour string */
  interpolate: (t: number) => string
}

// -------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------

/** Parse "#rrggbb" into [r, g, b]. */
function hexToRgb(hex: string): [number, number, number] {
  const n = parseInt(hex.slice(1), 16)
  return [(n >> 16) & 0xff, (n >> 8) & 0xff, n & 0xff]
}

/** Build an `interpolate` function from a list of hex stops. */
function makeInterpolate(stops: string[]): (t: number) => string {
  const rgbs = stops.map(hexToRgb)
  return (t: number) => {
    const clamped = Math.max(0, Math.min(1, t))
    const pos = clamped * (rgbs.length - 1)
    const lo = Math.floor(pos)
    const hi = Math.min(lo + 1, rgbs.length - 1)
    const frac = pos - lo
    const r = Math.round(rgbs[lo][0] + (rgbs[hi][0] - rgbs[lo][0]) * frac)
    const g = Math.round(rgbs[lo][1] + (rgbs[hi][1] - rgbs[lo][1]) * frac)
    const b = Math.round(rgbs[lo][2] + (rgbs[hi][2] - rgbs[lo][2]) * frac)
    return `rgba(${r},${g},${b},0.7)`
  }
}

function defineColormap(label: string, stops: string[]): Colormap {
  return { label, stops, interpolate: makeInterpolate(stops) }
}

// -------------------------------------------------------------------------
// Palettes (sampled from Matplotlib / Plotly standard colormaps)
// -------------------------------------------------------------------------

export const COLORMAPS: Record<ColormapName, Colormap> = {
  Viridis: defineColormap('Viridis', [
    '#440154',
    '#482777',
    '#3e4a89',
    '#31688e',
    '#26838e',
    '#1f9e89',
    '#6cce5a',
    '#b6de2b',
    '#fee825',
  ]),
  Plasma: defineColormap('Plasma', [
    '#0d0887',
    '#5b02a3',
    '#9a179b',
    '#cb4679',
    '#eb7852',
    '#fbb32b',
    '#f0f921',
  ]),
  Inferno: defineColormap('Inferno', [
    '#000004',
    '#1b0c41',
    '#4a0c6b',
    '#781c6d',
    '#a52c60',
    '#cf4446',
    '#ed6925',
    '#fb9b06',
    '#f7d13d',
    '#fcffa4',
  ]),
  Magma: defineColormap('Magma', [
    '#000004',
    '#180f3d',
    '#440f76',
    '#721f81',
    '#9e2f7f',
    '#cd4071',
    '#f1605d',
    '#fd9668',
    '#feca8d',
    '#fcfdbf',
  ]),
  Cividis: defineColormap('Cividis', [
    '#002051',
    '#0a326a',
    '#2b446e',
    '#4d566d',
    '#6b6b6f',
    '#898173',
    '#a89a6e',
    '#c8b85e',
    '#ebd644',
    '#fdea45',
  ]),
  Turbo: defineColormap('Turbo', [
    '#30123b',
    '#4662d7',
    '#36aaf9',
    '#1ae4b6',
    '#72fe5e',
    '#c8ef34',
    '#faba39',
    '#f66b19',
    '#ca2a04',
    '#7a0403',
  ]),
  Coolwarm: defineColormap('Coolwarm', [
    '#3b4cc0',
    '#6788ee',
    '#9abbff',
    '#c9d7ef',
    '#eddbd5',
    '#f7a889',
    '#e26952',
    '#b40426',
  ]),
  Spectral: defineColormap('Spectral', [
    '#9e0142',
    '#d53e4f',
    '#f46d43',
    '#fdae61',
    '#fee08b',
    '#e6f598',
    '#abdda4',
    '#66c2a5',
    '#3288bd',
    '#5e4fa2',
  ]),
  Rainbow: defineColormap('Rainbow', [
    '#6e00ff',
    '#0060ff',
    '#00c8aa',
    '#40d040',
    '#d0d000',
    '#ff8c00',
    '#ff0000',
  ]),
  Jet: defineColormap('Jet', [
    '#00007f',
    '#0000ff',
    '#007fff',
    '#00ffff',
    '#7fff7f',
    '#ffff00',
    '#ff7f00',
    '#ff0000',
    '#7f0000',
  ]),
}

export const COLORMAP_NAMES: ColormapName[] = Object.keys(COLORMAPS) as ColormapName[]

export const DEFAULT_COLORMAP: ColormapName = 'Jet'
