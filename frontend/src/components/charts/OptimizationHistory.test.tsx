/**
 * OptimizationHistory tests (TASK-1001)
 *
 * Target: OptimizationHistory — convergence history chart for single-objective optimization
 * Strategy: mock echarts-for-react with vi.mock; verify mode switching and
 *           phase detection boundary values
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'

// -------------------------------------------------------------------------
// echarts-for-react mock (uses __mocks__/echarts-for-react.tsx automatically)
// -------------------------------------------------------------------------

vi.mock('echarts-for-react')

import { OptimizationHistory, detectPhase } from './OptimizationHistory'

// -------------------------------------------------------------------------
// Test helpers
// -------------------------------------------------------------------------

/** Generates test data with monotonically decreasing best values */
function makeConvergingData(count: number): { trial: number; value: number }[] {
  return Array.from({ length: count }, (_, i) => ({
    trial: i + 1,
    value: 100 - i,
  }))
}

// -------------------------------------------------------------------------
// Happy path
// -------------------------------------------------------------------------

describe('OptimizationHistory — 正常系', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    cleanup()
  })

  // TC-1001-01: renders without error
  test('TC-1001-01: OptimizationHistory がエラーなくレンダリングされる', () => {
    expect(() => render(<OptimizationHistory data={[]} direction="minimize" />)).not.toThrow()
  })

  // TC-1001-02: all four mode buttons exist
  test('TC-1001-02: 4つの表示モードボタン（best/all/moving-avg/improvement）が表示される', () => {
    render(<OptimizationHistory data={makeConvergingData(10)} direction="minimize" />)

    expect(screen.getByTestId('mode-btn-best')).toBeInTheDocument()
    expect(screen.getByTestId('mode-btn-all')).toBeInTheDocument()
    expect(screen.getByTestId('mode-btn-moving-avg')).toBeInTheDocument()
    expect(screen.getByTestId('mode-btn-improvement')).toBeInTheDocument()
  })

  // TC-1001-03: clicking a mode button updates aria-pressed
  test('TC-1001-03: moving-avg ボタンクリックで moving-avg モードがアクティブになる', () => {
    render(<OptimizationHistory data={makeConvergingData(10)} direction="minimize" />)

    // Default mode is 'best'
    expect(screen.getByTestId('mode-btn-best')).toHaveAttribute('aria-pressed', 'true')

    fireEvent.click(screen.getByTestId('mode-btn-moving-avg'))

    expect(screen.getByTestId('mode-btn-moving-avg')).toHaveAttribute('aria-pressed', 'true')
    expect(screen.getByTestId('mode-btn-best')).toHaveAttribute('aria-pressed', 'false')
  })
})

// -------------------------------------------------------------------------
// Phase detection — boundary value tests
// -------------------------------------------------------------------------

describe('detectPhase — フェーズ自動検出 境界値テスト', () => {
  // TC-1001-04: exploration phase (progress < 0.3)
  test('TC-1001-04: progress=0.1 のとき exploration（探索期）が返される', () => {
    expect(detectPhase(10, 100)).toBe('exploration') // 10/100 = 0.1
  })

  // TC-1001-05: exploitation phase (0.3 <= progress < 0.7)
  test('TC-1001-05: progress=0.5 のとき exploitation（精緻化期）が返される', () => {
    expect(detectPhase(50, 100)).toBe('exploitation') // 50/100 = 0.5
  })

  // TC-1001-06: convergence phase (progress >= 0.7)
  test('TC-1001-06: progress=0.8 のとき convergence（収束期）が返される', () => {
    expect(detectPhase(80, 100)).toBe('convergence') // 80/100 = 0.8
  })
})
