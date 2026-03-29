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
 * Documentation.
 */

import { create } from 'zustand'
import type { ArtifactMeta, ArtifactType } from '../types'

// -------------------------------------------------------------------------
// Type definitions
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
interface ArtifactStoreState {
  // --- State ---
  /** selectedartifactdirectory（null = unselected）*/
  dirHandle: FileSystemDirectoryHandle | null
  /** artifactId → ObjectURL cache */
  urlCache: Map<string, string>
  /** directoryselection flag */
  isPickingDir: boolean
  /** error message */
  error: string | null

  // --- Actions ---
  /** Open the directory selection dialog 🟢 REQ-140 */
  pickDirectory: () => Promise<boolean>
  /**
   * Documentation.
   * Documentation.
   */
  loadArtifactUrl: (artifactId: string, filename: string) => Promise<string | null>
  /** Documentation. */
  releaseAll: () => void
  /** Documentation. */
  clearError: () => void
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 */
export function getMimeTypeCategory(filename: string): ArtifactType {
  const ext = filename.split('.').pop()?.toLowerCase() ?? ''
  if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'svg'].includes(ext)) return 'image'
  if (ext === 'csv') return 'csv'
  if (['txt', 'log', 'md'].includes(ext)) return 'text'
  if (ext === 'json') return 'json'
  if (['mp3', 'wav', 'ogg', 'flac'].includes(ext)) return 'audio'
  if (['mp4', 'webm', 'mov', 'avi'].includes(ext)) return 'video'
  return 'other'
}

/**
 * Documentation.
 */
export function buildArtifactMeta(
  artifactId: string,
  filename: string,
  trialId: number,
): ArtifactMeta {
  const type = getMimeTypeCategory(filename)
  const mimeMap: Record<ArtifactType, string> = {
    image: 'image/*',
    csv: 'text/csv',
    text: 'text/plain',
    json: 'application/json',
    audio: 'audio/*',
    video: 'video/*',
    other: 'application/octet-stream',
  }
  return {
    artifactId,
    filename,
    mimetype: mimeMap[type],
    trialId,
  }
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
export const useArtifactStore = create<ArtifactStoreState>()((set, get) => ({
  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------
  dirHandle: null,
  urlCache: new Map(),
  isPickingDir: false,
  error: null,

  // -------------------------------------------------------------------------
  // Documentation.
  // -------------------------------------------------------------------------

  /**
   * Documentation.
   * Documentation.
   */
  pickDirectory: async () => {
    if (!useArtifactStore.isSupported?.()) {
      set({ error: 'This browser does not support directory selection' })
      return false
    }

    set({ isPickingDir: true, error: null })

    try {
      const handle = await (
        window as unknown as { showDirectoryPicker: () => Promise<FileSystemDirectoryHandle> }
      ).showDirectoryPicker()

      // Documentation.
      get().releaseAll()

      set({ dirHandle: handle, isPickingDir: false })
      return true
    } catch (e) {
      // Documentation.
      if (
        (e instanceof Error && e.name === 'AbortError') ||
        (e instanceof DOMException && e.name === 'AbortError')
      ) {
        set({ isPickingDir: false })
        return false
      }
      set({
        error: e instanceof Error ? e.message : 'Failed to select directory',
        isPickingDir: false,
      })
      return false
    }
  },

  /**
   * Documentation.
   *
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   * Documentation.
   *
   * Documentation.
   */
  loadArtifactUrl: async (artifactId, filename) => {
    const { dirHandle, urlCache } = get()

    // Documentation.
    if (urlCache.has(artifactId)) {
      return urlCache.get(artifactId) ?? null
    }

    if (!dirHandle) {
      return null
    }

    try {
      // Documentation.
      const fileHandle = await dirHandle.getFileHandle(filename, { create: false })
      const file = await fileHandle.getFile()
      const url = URL.createObjectURL(file)

      // Documentation.
      const nextCache = new Map(get().urlCache)
      nextCache.set(artifactId, url)
      set({ urlCache: nextCache })

      return url
    } catch {
      // Documentation.
      return null
    }
  },

  /**
   * Documentation.
   * Documentation.
   */
  releaseAll: () => {
    const { urlCache } = get()
    urlCache.forEach((url) => URL.revokeObjectURL(url))
    set({ urlCache: new Map() })
  },

  /** Documentation. */
  clearError: () => set({ error: null }),
}))

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 */
useArtifactStore.isSupported = (): boolean =>
  typeof window !== 'undefined' && 'showDirectoryPicker' in window

// Documentation.
declare module 'zustand' {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  interface StoreApi<T> {
    isSupported?: () => boolean
  }
}
