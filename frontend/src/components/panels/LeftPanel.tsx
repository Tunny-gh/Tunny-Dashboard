/**
 * LeftPanel — study info counter, filter sliders, and color mode selection (TASK-402)
 *
 * Documentation.
 * Documentation.
 * Documentation.
 */

import { useMemo, useState, useCallback, useRef, useEffect } from 'react'
import { useSelectionStore } from '../../stores/selectionStore'
import { useStudyStore } from '../../stores/studyStore'
import type { ColorMode } from '../../types'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/** 🟢 Available color modes */
const COLOR_MODES: ColorMode[] = ['objective', 'cluster', 'rank', 'generation']

// -------------------------------------------------------------------------
// Dual-handle Range Slider
// -------------------------------------------------------------------------

interface DualRangeSliderProps {
  name: string
  dataMin: number
  dataMax: number
  onRangeChange: (name: string, min: number, max: number) => void
}

function DualRangeSlider({ name, dataMin, dataMax, onRangeChange }: DualRangeSliderProps) {
  const [lo, setLo] = useState(dataMin)
  const [hi, setHi] = useState(dataMax)
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Cleanup debounce timer on unmount
  useEffect(
    () => () => {
      if (debounceRef.current) clearTimeout(debounceRef.current)
    },
    [],
  )

  const range = dataMax - dataMin || 1
  const step = range / 200

  const debouncedRangeChange = useCallback(
    (min: number, max: number) => {
      if (debounceRef.current) clearTimeout(debounceRef.current)
      debounceRef.current = setTimeout(() => {
        onRangeChange(name, min, max)
      }, 150)
    },
    [name, onRangeChange],
  )

  const handleLo = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const v = Math.min(parseFloat(e.target.value), hi)
      setLo(v)
      debouncedRangeChange(v, hi)
    },
    [hi, debouncedRangeChange],
  )

  const handleHi = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const v = Math.max(parseFloat(e.target.value), lo)
      setHi(v)
      debouncedRangeChange(lo, v)
    },
    [lo, debouncedRangeChange],
  )

  const loPercent = ((lo - dataMin) / range) * 100
  const hiPercent = ((hi - dataMin) / range) * 100

  return (
    <div style={{ marginBottom: '10px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}>
        <label style={{ fontSize: '13px' }}>{name}</label>
        <span style={{ fontSize: '11px', color: '#6b7280' }}>
          {lo.toPrecision(4)} – {hi.toPrecision(4)}
        </span>
      </div>
      <div
        style={{ position: 'relative', height: '20px', marginTop: '2px' }}
        data-testid={`range-slider-${name}`}
      >
        {/* Track background + active range highlight */}
        <div
          style={{
            position: 'absolute',
            top: '8px',
            left: 0,
            right: 0,
            height: '4px',
            borderRadius: '2px',
            background: '#e5e7eb',
          }}
        />
        <div
          style={{
            position: 'absolute',
            top: '8px',
            left: `${loPercent}%`,
            width: `${hiPercent - loPercent}%`,
            height: '4px',
            borderRadius: '2px',
            background: '#3b82f6',
          }}
        />
        {/* Low handle */}
        <input
          data-testid={`slider-lo-${name}`}
          type="range"
          min={dataMin}
          max={dataMax}
          step={step}
          value={lo}
          onChange={handleLo}
          style={{
            position: 'absolute',
            width: '100%',
            top: 0,
            height: '20px',
            WebkitAppearance: 'none',
            appearance: 'none',
            background: 'transparent',
            pointerEvents: 'none',
            zIndex: 2,
          }}
          className="dual-range-thumb"
        />
        {/* High handle */}
        <input
          data-testid={`slider-hi-${name}`}
          type="range"
          min={dataMin}
          max={dataMax}
          step={step}
          value={hi}
          onChange={handleHi}
          style={{
            position: 'absolute',
            width: '100%',
            top: 0,
            height: '20px',
            WebkitAppearance: 'none',
            appearance: 'none',
            background: 'transparent',
            pointerEvents: 'none',
            zIndex: 3,
          }}
          className="dual-range-thumb"
        />
      </div>
    </div>
  )
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Test Coverage: TC-402-01〜04, TC-402-E01
 */
export function LeftPanel() {
  // Documentation.
  const selectedIndices = useSelectionStore((s) => s.selectedIndices)
  const colorMode = useSelectionStore((s) => s.colorMode)
  const addAxisFilter = useSelectionStore((s) => s.addAxisFilter)
  const removeAxisFilter = useSelectionStore((s) => s.removeAxisFilter)
  const setColorMode = useSelectionStore((s) => s.setColorMode)

  // Documentation.
  const currentStudy = useStudyStore((s) => s.currentStudy)
  const trialRows = useStudyStore((s) => s.trialRows)

  // Documentation.
  const paramRanges = useMemo(() => {
    const ranges: Record<string, { min: number; max: number }> = {}
    if (!currentStudy) return ranges
    for (const name of currentStudy.paramNames) {
      let min = Number.POSITIVE_INFINITY
      let max = Number.NEGATIVE_INFINITY
      for (const row of trialRows) {
        const v = row.params[name]
        if (v !== undefined && Number.isFinite(v)) {
          if (v < min) min = v
          if (v > max) max = v
        }
      }
      if (Number.isFinite(min) && Number.isFinite(max)) {
        ranges[name] = { min, max }
      }
    }
    return ranges
  }, [currentStudy, trialRows])

  // Documentation.
  const handleRangeChange = useCallback(
    (name: string, min: number, max: number) => {
      const dataRange = paramRanges[name]
      if (
        dataRange &&
        Math.abs(min - dataRange.min) < 1e-10 &&
        Math.abs(max - dataRange.max) < 1e-10
      ) {
        removeAxisFilter(name)
      } else {
        addAxisFilter(name, min, max)
      }
    },
    [addAxisFilter, removeAxisFilter, paramRanges],
  )

  // Documentation.
  if (!currentStudy) {
    return (
      <div style={{ padding: '12px' }}>
        <span>Data not loaded</span>
      </div>
    )
  }

  // Documentation.
  return (
    <div style={{ padding: '12px', display: 'flex', flexDirection: 'column', gap: '16px' }}>
      {/* Documentation. */}
      <div>
        <div style={{ fontSize: '12px', color: '#6b7280' }}>Selected</div>
        <div data-testid="selected-count" style={{ fontSize: '20px', fontWeight: 'bold' }}>
          {selectedIndices.length}
        </div>
      </div>

      {/* Documentation. */}
      <div>
        <div style={{ fontSize: '12px', color: '#6b7280', marginBottom: '4px' }}>Parameters</div>
        {currentStudy.paramNames.map((name) => {
          const r = paramRanges[name]
          if (!r) return null
          return (
            <DualRangeSlider
              key={name}
              name={name}
              dataMin={r.min}
              dataMax={r.max}
              onRangeChange={handleRangeChange}
            />
          )
        })}
      </div>

      {/* Documentation. */}
      <div>
        <div style={{ fontSize: '12px', color: '#6b7280', marginBottom: '4px' }}>Color Mode</div>
        {COLOR_MODES.map((mode) => (
          <label
            key={mode}
            style={{ display: 'flex', alignItems: 'center', gap: '4px', fontSize: '13px' }}
          >
            <input
              data-testid={`color-mode-${mode}`}
              type="radio"
              name="color-mode"
              value={mode}
              checked={colorMode === mode}
              onChange={() => setColorMode(mode)}
            />
            {mode}
          </label>
        ))}
      </div>
    </div>
  )
}
