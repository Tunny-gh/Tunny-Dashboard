/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, afterEach, vi } from 'vitest'
import { render, screen, cleanup, fireEvent } from '@testing-library/react'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

vi.mock('echarts-for-react')

import { ClusterPanel } from './ClusterPanel'
import type { ClusterPanelProps } from './ClusterPanel'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/** Documentation. */
function makeProps(overrides: Partial<ClusterPanelProps> = {}): ClusterPanelProps {
  return {
    onRunClustering: vi.fn(),
    isRunning: false,
    progress: 0,
    elbowResult: null,
    error: null,
    ...overrides,
  }
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    cleanup()
  })

  // Documentation.
  test('TC-902-01', () => {
    // Documentation.
    const props = makeProps()
    render(<ClusterPanel {...props} />)

    // Documentation.
    const btn = screen.getByTestId('run-clustering-btn')
    fireEvent.click(btn)

    // Documentation.
    expect(props.onRunClustering).toHaveBeenCalledWith('param', 4)
  })

  // Documentation.
  test('TC-902-02', () => {
    // Documentation.
    render(<ClusterPanel {...makeProps({ isRunning: true, progress: 0 })} />)
    // Documentation.
    expect(screen.getByTestId('progress-container')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-902-03', () => {
    // Documentation.
    render(<ClusterPanel {...makeProps({ isRunning: true, progress: 68 })} />)
    // Documentation.
    expect(screen.getByTestId('progress-text')).toHaveTextContent('Computing...68%')
  })

  // Documentation.
  test('TC-902-05', () => {
    // Documentation.
    render(
      <ClusterPanel
        {...makeProps({
          elbowResult: { wcssPerK: [100, 50, 30, 20], recommendedK: 4 },
        })}
      />,
    )
    // Documentation.
    const el = screen.getByTestId('elbow-recommended')
    expect(el).toHaveTextContent('4')
  })

  // Documentation.
  test('TC-902-06', () => {
    // Documentation.
    render(<ClusterPanel {...makeProps({ isRunning: false })} />)
    // Documentation.
    expect(screen.queryByTestId('progress-container')).toBeNull()
  })

  // Documentation.
  test('TC-902-07', () => {
    // Documentation.
    const props = makeProps()
    render(<ClusterPanel {...props} />)

    // Documentation.
    fireEvent.click(screen.getByTestId('space-objective'))
    fireEvent.click(screen.getByTestId('run-clustering-btn'))

    // Documentation.
    expect(props.onRunClustering).toHaveBeenCalledWith('objective', expect.any(Number))
  })

  // Documentation.
  test('TC-902-08', () => {
    // Documentation.
    render(<ClusterPanel {...makeProps({ elbowResult: null })} />)
    // Documentation.
    expect(screen.queryByTestId('elbow-recommended')).toBeNull()
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  afterEach(() => {
    cleanup()
  })

  // Documentation.
  test('TC-902-L01', () => {
    // Documentation.
    render(<ClusterPanel {...makeProps({ isRunning: true })} />)
    const btn = screen.getByTestId('run-clustering-btn')
    // Documentation.
    expect(btn).toBeDisabled()
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  afterEach(() => {
    cleanup()
  })

  // Documentation.
  test('TC-902-E01', () => {
    // Documentation.
    const props = makeProps()
    render(<ClusterPanel {...props} />)

    // Documentation.
    const kInput = screen.getByTestId('k-input')
    fireEvent.change(kInput, { target: { value: '1' } })

    // Documentation.
    fireEvent.click(screen.getByTestId('run-clustering-btn'))

    // Documentation.
    expect(screen.getByTestId('k-error')).toHaveTextContent('Please specify k >= 2')
    expect(props.onRunClustering).not.toHaveBeenCalled()
  })

  // Documentation.
  test('TC-902-E02', () => {
    // Documentation.
    render(<ClusterPanel {...makeProps({ error: 'Clustering failed' })} />)
    // Documentation.
    expect(screen.getByTestId('cluster-error')).toHaveTextContent('Clustering failed')
  })
})
