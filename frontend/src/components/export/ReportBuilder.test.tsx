/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import '@testing-library/jest-dom'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

const { mockSetReportSections, mockGenerateHtmlReport, mockClearReportError } = vi.hoisted(() => ({
  mockSetReportSections: vi.fn(),
  mockGenerateHtmlReport: vi.fn().mockResolvedValue(undefined),
  mockClearReportError: vi.fn(),
}))

const mockStoreState = {
  reportSections: ['summary', 'pareto', 'pinned', 'history', 'cluster'] as const,
  isGeneratingReport: false,
  reportError: null as string | null,
  pinnedTrials: [],
  setReportSections: mockSetReportSections,
  generateHtmlReport: mockGenerateHtmlReport,
  clearReportError: mockClearReportError,
}

vi.mock('../../stores/exportStore', () => ({
  useExportStore: vi.fn(() => mockStoreState),
  ReportSection: {},
}))

import { ReportBuilder } from './ReportBuilder'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

function renderReportBuilder(paretoIndices = new Uint32Array([0, 1, 2])) {
  return render(<ReportBuilder paretoIndices={paretoIndices} />)
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockStoreState.isGeneratingReport = false
    mockStoreState.reportError = null
    mockStoreState.reportSections = ['summary', 'pareto', 'pinned', 'history', 'cluster']
  })

  // Documentation.
  test('TC-1102-R01', () => {
    // Documentation.
    renderReportBuilder()

    // Documentation.
    expect(screen.getByTestId('section-checkbox-summary')).toBeInTheDocument()
    expect(screen.getByTestId('section-checkbox-pareto')).toBeInTheDocument()
    expect(screen.getByTestId('section-checkbox-pinned')).toBeInTheDocument()
    expect(screen.getByTestId('section-checkbox-history')).toBeInTheDocument()
    expect(screen.getByTestId('section-checkbox-cluster')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-1102-R02', () => {
    // Documentation.
    renderReportBuilder()

    const checkbox = screen.getByTestId('section-checkbox-summary')
    expect(checkbox).toBeChecked()

    fireEvent.click(checkbox)

    // Documentation.
    expect(checkbox).not.toBeChecked()
  })

  // Documentation.
  test('TC-1102-R03', () => {
    // Documentation.
    renderReportBuilder()
    expect(screen.getByTestId('report-builder')).toBeInTheDocument()
  })
})

describe('translated test case', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockStoreState.isGeneratingReport = false
    mockStoreState.reportError = null
    mockStoreState.reportSections = ['summary', 'pareto', 'pinned', 'history', 'cluster']
  })

  // Documentation.
  test('TC-1102-R04', () => {
    // Documentation.
    renderReportBuilder()
    expect(screen.getByTestId('generate-report-btn')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-1102-R05', () => {
    // Documentation.
    const paretoIndices = new Uint32Array([0, 1, 2])
    renderReportBuilder(paretoIndices)

    fireEvent.click(screen.getByTestId('generate-report-btn'))

    // Documentation.
    expect(mockGenerateHtmlReport).toHaveBeenCalledOnce()
  })

  // Documentation.
  test('TC-1102-R06', () => {
    // Documentation.
    mockStoreState.isGeneratingReport = true
    renderReportBuilder()

    // Documentation.
    expect(screen.getByTestId('report-spinner')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-1102-R07', () => {
    // Documentation.
    mockStoreState.reportError = 'translatederror message'
    renderReportBuilder()

    // Documentation.
    const errorEl = screen.getByTestId('report-error')
    expect(errorEl).toBeInTheDocument()
    expect(errorEl).toHaveTextContent('translatederror message')
  })
})
