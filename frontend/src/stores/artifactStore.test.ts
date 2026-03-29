/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

import { describe, test, expect, beforeEach, vi } from 'vitest'
import { useArtifactStore, getMimeTypeCategory, buildArtifactMeta } from './artifactStore'

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

Object.defineProperty(globalThis, 'URL', {
  value: {
    createObjectURL: vi.fn(() => 'blob:mock-url'),
    revokeObjectURL: vi.fn(),
  },
  writable: true,
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/** Documentation. */
function makeFileHandle() {
  return {
    getFile: vi.fn().mockResolvedValue(new Blob(['test content'])),
  } as unknown as FileSystemFileHandle
}

/** Documentation. */
function makeDirHandle(files: Record<string, FileSystemFileHandle> = {}) {
  return {
    kind: 'directory',
    name: 'artifacts',
    getFileHandle: vi.fn().mockImplementation((name: string, _opts?: object) => {
      const handle = files[name]
      if (handle) return Promise.resolve(handle)
      return Promise.reject(new DOMException('File not found', 'NotFoundError'))
    }),
  } as unknown as FileSystemDirectoryHandle
}

/** Documentation. */
function resetStore() {
  useArtifactStore.setState({
    dirHandle: null,
    urlCache: new Map(),
    isPickingDir: false,
    error: null,
  })
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('getMimeTypeCategory', () => {
  // Documentation.
  test('TC-1301-U01', () => {
    // Documentation.
    expect(getMimeTypeCategory('photo.png')).toBe('image')
    expect(getMimeTypeCategory('photo.jpg')).toBe('image')
    expect(getMimeTypeCategory('photo.gif')).toBe('image')
  })

  // Documentation.
  test('TC-1301-U02', () => {
    // Documentation.
    expect(getMimeTypeCategory('data.csv')).toBe('csv')
  })

  // Documentation.
  test('TC-1301-U03', () => {
    // Documentation.
    expect(getMimeTypeCategory('result.json')).toBe('json')
  })

  // Documentation.
  test('TC-1301-U04', () => {
    // Documentation.
    expect(getMimeTypeCategory('file.xyz')).toBe('other')
    expect(getMimeTypeCategory('noextension')).toBe('other')
  })
})

describe('buildArtifactMeta', () => {
  // Documentation.
  test('TC-1301-U05', () => {
    // Documentation.
    const meta = buildArtifactMeta('abc123', 'chart.png', 42)
    expect(meta.artifactId).toBe('abc123')
    expect(meta.filename).toBe('chart.png')
    expect(meta.trialId).toBe(42)
    expect(meta.mimetype).toBe('image/*')
  })
})

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

describe('translated test case', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetStore()
  })

  // Documentation.
  test('TC-1301-D01', async () => {
    // Documentation.
    const mockHandle = makeDirHandle()
    ;(window as Record<string, unknown>).showDirectoryPicker = vi.fn().mockResolvedValue(mockHandle)

    const result = await useArtifactStore.getState().pickDirectory()

    // Documentation.
    expect(result).toBe(true)
    // Documentation.
    expect(useArtifactStore.getState().dirHandle).toBe(mockHandle)
  })

  // Documentation.
  test('TC-1301-D02', async () => {
    // Documentation.
    const abort = new DOMException('User cancelled', 'AbortError')
    ;(window as Record<string, unknown>).showDirectoryPicker = vi.fn().mockRejectedValue(abort)

    const result = await useArtifactStore.getState().pickDirectory()

    // Documentation.
    expect(result).toBe(false)
    // Documentation.
    expect(useArtifactStore.getState().error).toBeNull()
  })

  // Documentation.
  test('TC-1301-D03', async () => {
    // Documentation.
    const orig = (window as Record<string, unknown>).showDirectoryPicker
    delete (window as Record<string, unknown>).showDirectoryPicker

    const result = await useArtifactStore.getState().pickDirectory()

    // Documentation.
    expect(result).toBe(false)
    // Documentation.
    expect(useArtifactStore.getState().error).toContain('does not support')

    if (orig) (window as Record<string, unknown>).showDirectoryPicker = orig
  })
})

describe('translated test case', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetStore()
  })

  // Documentation.
  test('TC-1301-L01', async () => {
    // Documentation.
    const fileHandle = makeFileHandle()
    const dirHandle = makeDirHandle({ 'abc.png': fileHandle })
    useArtifactStore.setState({ dirHandle })

    const url = await useArtifactStore.getState().loadArtifactUrl('abc123', 'abc.png')

    // Documentation.
    expect(url).toBe('blob:mock-url')
    // Documentation.
    expect(useArtifactStore.getState().urlCache.get('abc123')).toBe('blob:mock-url')
  })

  // Documentation.
  test('TC-1301-L02', async () => {
    // Documentation.
    const dirHandle = makeDirHandle({}) // Documentation.
    useArtifactStore.setState({ dirHandle })

    const url = await useArtifactStore.getState().loadArtifactUrl('missing', 'missing.png')

    // Documentation.
    expect(url).toBeNull()
  })

  // Documentation.
  test('TC-1301-L03', async () => {
    // Documentation.
    const initialCache = new Map([['cached123', 'blob:cached-url']])
    useArtifactStore.setState({
      dirHandle: makeDirHandle({}),
      urlCache: initialCache,
    })

    const url = await useArtifactStore.getState().loadArtifactUrl('cached123', 'any.png')

    // Documentation.
    expect(url).toBe('blob:cached-url')
  })

  // Documentation.
  test('TC-1301-L04', async () => {
    // Documentation.
    useArtifactStore.setState({ dirHandle: null })
    const url = await useArtifactStore.getState().loadArtifactUrl('any', 'any.png')
    expect(url).toBeNull()
  })
})

describe('ArtifactStore — releaseAll', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    resetStore()
  })

  // Documentation.
  test('TC-1301-R01', () => {
    // Documentation.
    const cache = new Map([
      ['a', 'blob:url1'],
      ['b', 'blob:url2'],
    ])
    useArtifactStore.setState({ urlCache: cache })

    useArtifactStore.getState().releaseAll()

    // Documentation.
    expect(useArtifactStore.getState().urlCache.size).toBe(0)
    // Documentation.
    expect(URL.revokeObjectURL).toHaveBeenCalledTimes(2)
  })
})
