/**
 * ArtifactGallery tests (TASK-1301)
 *
 * Stubs artifactStore with vi.mock.
 */

import { describe, test, expect, beforeEach, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import '@testing-library/jest-dom'

// -------------------------------------------------------------------------
// artifactStore mock
// -------------------------------------------------------------------------

vi.mock('../../stores/artifactStore', () => ({
  useArtifactStore: vi.fn(() => ({
    dirHandle: { name: 'artifacts' } as unknown as FileSystemDirectoryHandle,
    loadArtifactUrl: vi.fn().mockResolvedValue(null),
    urlCache: new Map(),
  })),
  getMimeTypeCategory: (filename: string) => {
    const ext = filename.split('.').pop()?.toLowerCase() ?? ''
    if (['png', 'jpg'].includes(ext)) return 'image'
    return 'other'
  },
}))

import { ArtifactGallery } from './ArtifactGallery'
import type { Trial } from '../../types'
import { useArtifactStore } from '../../stores/artifactStore'

// -------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------

function makeTrial(trialId: number, hasArtifact = true): Trial {
  return {
    trialId,
    state: 'COMPLETE',
    params: {},
    values: [1.0],
    clusterId: null,
    isFeasible: true,
    userAttrs: {},
    artifactIds: hasArtifact ? [`artifact-${trialId}.png`] : [],
  }
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

describe('ArtifactGallery', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    // Reset mock to default state (TC-1301-G01 sets dirHandle=null)
    ;(useArtifactStore as ReturnType<typeof vi.fn>).mockReturnValue({
      dirHandle: { name: 'artifacts' } as unknown as FileSystemDirectoryHandle,
      loadArtifactUrl: vi.fn().mockResolvedValue(null),
      urlCache: new Map(),
    })
  })

  // TC-1301-G01: hidden when no directory is selected
  test('TC-1301-G01', () => {
    ;(useArtifactStore as ReturnType<typeof vi.fn>).mockReturnValue({
      dirHandle: null,
      loadArtifactUrl: vi.fn(),
    })
    render(<ArtifactGallery trials={[makeTrial(1)]} />)
    expect(screen.queryByTestId('artifact-gallery')).not.toBeInTheDocument()
  })

  // TC-1301-G02: trials with artifacts are shown as cards
  test('TC-1301-G02', () => {
    const trials = [makeTrial(1), makeTrial(2), makeTrial(3)]
    render(<ArtifactGallery trials={trials} />)
    expect(screen.getByTestId('gallery-card-1')).toBeInTheDocument()
    expect(screen.getByTestId('gallery-card-2')).toBeInTheDocument()
    expect(screen.getByTestId('gallery-card-3')).toBeInTheDocument()
  })

  // TC-1301-G03: trials without artifacts are filtered out
  test('TC-1301-G03', () => {
    const trials = [makeTrial(1, true), makeTrial(2, false)]
    render(<ArtifactGallery trials={trials} />)
    expect(screen.getByTestId('gallery-card-1')).toBeInTheDocument()
    expect(screen.queryByTestId('gallery-card-2')).not.toBeInTheDocument()
  })

  // TC-1301-G04: card size toggle buttons are shown
  test('TC-1301-G04', () => {
    render(<ArtifactGallery trials={[makeTrial(1)]} />)
    expect(screen.getByTestId('card-size-small')).toBeInTheDocument()
    expect(screen.getByTestId('card-size-medium')).toBeInTheDocument()
    expect(screen.getByTestId('card-size-large')).toBeInTheDocument()
  })

  // TC-1301-G05: "load more" button appears when items exceed page size
  test('TC-1301-G05', () => {
    const trials = Array.from({ length: 49 }, (_, i) => makeTrial(i + 1))
    render(<ArtifactGallery trials={trials} />)
    expect(screen.getByTestId('load-more-btn')).toBeInTheDocument()
  })

  // TC-1301-G06: clicking "load more" increases visible count
  test('TC-1301-G06', () => {
    const trials = Array.from({ length: 49 }, (_, i) => makeTrial(i + 1))
    render(<ArtifactGallery trials={trials} />)

    // Before clicking: 48 items shown
    expect(screen.getByTestId('gallery-card-48')).toBeInTheDocument()
    expect(screen.queryByTestId('gallery-card-49')).not.toBeInTheDocument()

    fireEvent.click(screen.getByTestId('load-more-btn'))

    // After clicking: all 49 items shown
    expect(screen.getByTestId('gallery-card-49')).toBeInTheDocument()
  })

  // TC-1301-G07: empty message shown when no artifacts
  test('TC-1301-G07', () => {
    render(<ArtifactGallery trials={[makeTrial(1, false)]} />)
    expect(screen.getByTestId('gallery-empty')).toBeInTheDocument()
  })
})
