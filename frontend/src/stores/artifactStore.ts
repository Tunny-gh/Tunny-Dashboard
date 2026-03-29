/**
 * ArtifactStore — アーティファクト管理 Zustand Store (TASK-1301)
 *
 * 【役割】: Directory Picker API でアーティファクトディレクトリを管理し、
 *          trial_id / artifact_id をキーにファイル URL を提供する
 * 【設計方針】:
 *   - `showDirectoryPicker()` でユーザーがディレクトリを選択
 *   - ObjectURL キャッシュ: artifactId → ObjectURL（解放は releaseAll() で一括）
 *   - MIME タイプは拡張子から推定（ArtifactType に変換）
 *   - 非対応ブラウザ（Firefox 等）では isSupported()=false 🟢 REQ-140
 * 🟢 REQ-140〜REQ-144 に準拠
 */

import { create } from 'zustand'
import type { ArtifactMeta, ArtifactType } from '../types'

// -------------------------------------------------------------------------
// 型定義
// -------------------------------------------------------------------------

/**
 * 【ArtifactStore 状態型】
 */
interface ArtifactStoreState {
  // --- 状態 ---
  /** 選択済みアーティファクトディレクトリ（null = 未選択）*/
  dirHandle: FileSystemDirectoryHandle | null
  /** artifactId → ObjectURL キャッシュ */
  urlCache: Map<string, string>
  /** ディレクトリ選択中フラグ */
  isPickingDir: boolean
  /** エラーメッセージ */
  error: string | null

  // --- アクション ---
  /** ディレクトリ選択ダイアログを開く 🟢 REQ-140 */
  pickDirectory: () => Promise<boolean>
  /**
   * アーティファクトファイルの ObjectURL を返す
   * キャッシュ済みなら即返却、未ロードならディレクトリから読み込む
   */
  loadArtifactUrl: (artifactId: string, filename: string) => Promise<string | null>
  /** 全 ObjectURL を解放してメモリを回収する */
  releaseAll: () => void
  /** エラーをクリアする */
  clearError: () => void
}

// -------------------------------------------------------------------------
// ユーティリティ
// -------------------------------------------------------------------------

/**
 * 【MIME タイプ推定】: ファイル名の拡張子から ArtifactType を返す
 * 🟡 major な拡張子のみ対応（その他は 'other'）
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
 * 【ArtifactMeta からメタ情報を生成】: filename と artifactId から ArtifactMeta を構築する
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
// Store 実装
// -------------------------------------------------------------------------

/**
 * 【ArtifactStore 作成】
 */
export const useArtifactStore = create<ArtifactStoreState>()((set, get) => ({
  // -------------------------------------------------------------------------
  // 初期状態
  // -------------------------------------------------------------------------
  dirHandle: null,
  urlCache: new Map(),
  isPickingDir: false,
  error: null,

  // -------------------------------------------------------------------------
  // アクション実装
  // -------------------------------------------------------------------------

  /**
   * 【ディレクトリ選択】: showDirectoryPicker でユーザーにディレクトリを選択させる
   * 成功時 true、キャンセル / エラー時 false を返す 🟢 REQ-140
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

      // 【既存キャッシュ解放】: 新しいディレクトリを選択したら前の URL をクリア
      get().releaseAll()

      set({ dirHandle: handle, isPickingDir: false })
      return true
    } catch (e) {
      // 【キャンセル処理】: AbortError はエラー通知しない（DOMException も含む）
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
   * 【アーティファクト URL 取得】: artifactId に対応するファイルの ObjectURL を返す
   *
   * 【処理フロー】:
   *   1. キャッシュにあれば即返却
   *   2. dirHandle からファイルを取得
   *   3. ObjectURL を生成してキャッシュに保存
   *   4. ファイルが見つからなければ null を返す
   *
   * 🟢 REQ-141: ファイルが存在しない場合は null を返す
   */
  loadArtifactUrl: async (artifactId, filename) => {
    const { dirHandle, urlCache } = get()

    // 【キャッシュ確認】: 既にロード済みなら即返却
    if (urlCache.has(artifactId)) {
      return urlCache.get(artifactId) ?? null
    }

    if (!dirHandle) {
      return null
    }

    try {
      // 【ファイル取得】: filename でまずアクセスを試みる
      const fileHandle = await dirHandle.getFileHandle(filename, { create: false })
      const file = await fileHandle.getFile()
      const url = URL.createObjectURL(file)

      // 【キャッシュ保存】: 新しいマップを作成して immutable 更新
      const nextCache = new Map(get().urlCache)
      nextCache.set(artifactId, url)
      set({ urlCache: nextCache })

      return url
    } catch {
      // 【ファイルなし】: 見つからない場合は null を返す（エラーは設定しない）
      return null
    }
  },

  /**
   * 【全 URL 解放】: キャッシュ済みの ObjectURL を全て解放する
   * 🟡 コンポーネントのアンマウント時またはディレクトリ切り替え時に呼ぶ
   */
  releaseAll: () => {
    const { urlCache } = get()
    urlCache.forEach((url) => URL.revokeObjectURL(url))
    set({ urlCache: new Map() })
  },

  /** 【エラークリア】 */
  clearError: () => set({ error: null }),
}))

// -------------------------------------------------------------------------
// 静的メソッド（Store 外）
// -------------------------------------------------------------------------

/**
 * 【Directory Picker 対応チェック】: showDirectoryPicker がブラウザで利用可能か判定する
 * Firefox / Safari では false になる 🟢 REQ-140
 */
useArtifactStore.isSupported = (): boolean =>
  typeof window !== 'undefined' && 'showDirectoryPicker' in window

// TypeScript 型拡張: isSupported 静的メソッド
declare module 'zustand' {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  interface StoreApi<T> {
    isSupported?: () => boolean
  }
}
