import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, cleanup, fireEvent, act } from '@testing-library/react'

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

vi.mock('echarts-for-react')

const { mockComputeSensitivity, mockUseAnalysisStore } = vi.hoisted(() => {
  const mockComputeSensitivity = vi.fn().mockResolvedValue(undefined)
  const mockUseAnalysisStore = vi.fn()
  return { mockComputeSensitivity, mockUseAnalysisStore }
})

vi.mock('../../stores/analysisStore', () => ({
  useAnalysisStore: mockUseAnalysisStore,
}))

import { ImportanceChart } from './ImportanceChart'

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const mockSensitivityResult = {
  spearman: [
    [0.8, -0.3],
    [0.1, 0.9],
  ],
  ridge: [
    { beta: [0.7, 0.1], rSquared: 0.85 },
    { beta: [-0.2, 0.8], rSquared: 0.92 },
  ],
  paramNames: ['x1', 'x2'],
  objectiveNames: ['obj0', 'obj1'],
  durationMs: 10,
}

function setupStore(
  overrides: Partial<{
    sensitivityResult: typeof mockSensitivityResult | null
    isComputingSensitivity: boolean
    sensitivityError: string | null
    computeSensitivity: () => Promise<void>
  }> = {},
) {
  mockUseAnalysisStore.mockReturnValue({
    sensitivityResult: null,
    isComputingSensitivity: false,
    sensitivityError: null,
    computeSensitivity: mockComputeSensitivity,
    ...overrides,
  })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('ImportanceChart', () => {
  beforeEach(() => {
    cleanup()
    vi.clearAllMocks()
    mockComputeSensitivity.mockResolvedValue(undefined)
  })

  it('TC-1605-01: calls computeSensitivity when sensitivityResult is null', async () => {
    setupStore({ sensitivityResult: null, isComputingSensitivity: false })

    await act(async () => {
      render(<ImportanceChart />)
    })

    expect(mockComputeSensitivity).toHaveBeenCalledOnce()
  })

  it('TC-1605-02: shows Loading when isComputingSensitivity is true', () => {
    setupStore({ isComputingSensitivity: true })

    render(<ImportanceChart />)

    expect(screen.getByText('Loading...')).toBeDefined()
  })

  it('TC-1605-03: shows error message when sensitivityError is set', () => {
    setupStore({ sensitivityError: 'No active study' })

    render(<ImportanceChart />)

    expect(screen.getByText('No active study')).toBeDefined()
  })

  it('TC-1605-04: shows EmptyState when paramNames is empty', () => {
    setupStore({
      sensitivityResult: { ...mockSensitivityResult, paramNames: [], spearman: [], ridge: [] },
    })

    render(<ImportanceChart />)

    expect(screen.getByText(/No parameters/)).toBeDefined()
  })

  it('TC-1605-05: ECharts option contains Spearman title by default', () => {
    setupStore({ sensitivityResult: mockSensitivityResult })

    render(<ImportanceChart />)

    const chart = screen.getByTestId('echarts')
    const option = JSON.parse(chart.dataset.option ?? '{}')
    expect(JSON.stringify(option)).toContain('Spearman')
  })

  it('TC-1605-06: switching to beta metric updates chart title', () => {
    setupStore({ sensitivityResult: mockSensitivityResult })

    render(<ImportanceChart />)

    const select = screen.getByRole('combobox')
    fireEvent.change(select, { target: { value: 'beta' } })

    const chart = screen.getByTestId('echarts')
    const option = JSON.parse(chart.dataset.option ?? '{}')
    expect(JSON.stringify(option)).toContain('Ridge')
  })
})
