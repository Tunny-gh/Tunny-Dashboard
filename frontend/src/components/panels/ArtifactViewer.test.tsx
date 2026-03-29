/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import '@testing-library/jest-dom'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

const { mockLoadArtifactUrl } = vi.hoisted(() => ({
  mockLoadArtifactUrl: vi.fn(),
}))

const mockStoreState = {
  dirHandle: { name: 'artifacts' } as unknown as FileSystemDirectoryHandle,
  urlCache: new Map<string, string>(),
  isPickingDir: false,
  error: null as string | null,
  pickDirectory: vi.fn(),
  loadArtifactUrl: mockLoadArtifactUrl,
  releaseAll: vi.fn(),
  clearError: vi.fn(),
}

vi.mock('../../stores/artifactStore', () => ({
  useArtifactStore: vi.fn(() => mockStoreState),
  getMimeTypeCategory: (filename: string) => {
    const ext = filename.split('.').pop()?.toLowerCase() ?? ''
    if (['png', 'jpg', 'jpeg', 'gif'].includes(ext)) return 'image'
    if (ext === 'csv') return 'csv'
    return 'other'
  },
  buildArtifactMeta: vi.fn(),
}))

import { ArtifactViewer } from './ArtifactViewer'
import type { Trial } from '../../types'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

function makeTrial(artifactIds: string[] = []): Trial {
  return {
    trialId: 42,
    state: 'COMPLETE',
    params: {},
    values: [1.0],
    clusterId: null,
    isFeasible: true,
    userAttrs: {},
    artifactIds,
  }
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('ArtifactViewer', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockStoreState.dirHandle = { name: 'artifacts' } as unknown as FileSystemDirectoryHandle
    mockStoreState.error = null
  })

  // Documentation.
  test('TC-1301-V01', () => {
    // Documentation.
    mockStoreState.dirHandle = null as unknown as FileSystemDirectoryHandle
    render(<ArtifactViewer trial={makeTrial(['artifact1'])} />)
    // Documentation.
    expect(screen.queryByTestId('artifact-viewer')).not.toBeInTheDocument()
  })

  // Documentation.
  test('TC-1301-V02', () => {
    // Documentation.
    render(<ArtifactViewer trial={makeTrial([])} />)
    expect(screen.queryByTestId('artifact-viewer')).not.toBeInTheDocument()
  })

  // Documentation.
  test('TC-1301-V03', () => {
    // Documentation.
    mockLoadArtifactUrl.mockReturnValue(new Promise(() => {})) // pending
    render(<ArtifactViewer trial={makeTrial(['artifact1.png'])} />)
    // Documentation.
    expect(screen.getByTestId('artifact-loading-artifact1.png')).toBeInTheDocument()
  })

  // Documentation.
  test('TC-1301-V04', async () => {
    // Documentation.
    mockLoadArtifactUrl.mockResolvedValue('blob:image-url')
    render(<ArtifactViewer trial={makeTrial(['photo.png'])} />)
    // Documentation.
    await waitFor(() => {
      expect(screen.getByTestId('artifact-thumbnail-photo.png')).toBeInTheDocument()
    })
  })

  // Documentation.
  test('TC-1301-V05', async () => {
    // Documentation.
    mockLoadArtifactUrl.mockResolvedValue(null)
    render(<ArtifactViewer trial={makeTrial(['missing.png'])} />)
    await waitFor(() => {
      expect(screen.getByTestId('artifact-missing-missing.png')).toBeInTheDocument()
    })
  })

  // Documentation.
  test('TC-1301-V06', async () => {
    // Documentation.
    mockLoadArtifactUrl.mockResolvedValue('blob:file-url')
    render(<ArtifactViewer trial={makeTrial(['data.pkl'])} />)
    await waitFor(() => {
      expect(screen.getByTestId('artifact-download-data.pkl')).toBeInTheDocument()
    })
  })
})
