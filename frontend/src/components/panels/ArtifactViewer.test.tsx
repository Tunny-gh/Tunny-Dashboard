/**
 * ArtifactViewer テスト (TASK-1301)
 *
 * 【テスト対象】: ArtifactViewer コンポーネント
 * 【テスト方針】: artifactStore を vi.hoisted + vi.mock でスタブ
 */

import { describe, test, expect, beforeEach, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import '@testing-library/jest-dom'

// -------------------------------------------------------------------------
// artifactStore モック
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
// ヘルパー
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
// テスト
// -------------------------------------------------------------------------

describe('ArtifactViewer', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockStoreState.dirHandle = { name: 'artifacts' } as unknown as FileSystemDirectoryHandle
    mockStoreState.error = null
  })

  // TC-1301-V01: ディレクトリ未選択時は表示されない
  test('TC-1301-V01: dirHandle=null のとき ArtifactViewer は非表示', () => {
    // 【テスト目的】: ディレクトリ未選択時にコンポーネントが非表示になることを確認 🟢 REQ-140
    mockStoreState.dirHandle = null as unknown as FileSystemDirectoryHandle
    render(<ArtifactViewer trial={makeTrial(['artifact1'])} />)
    // 【確認内容】: artifact-viewer が存在しないこと
    expect(screen.queryByTestId('artifact-viewer')).not.toBeInTheDocument()
  })

  // TC-1301-V02: アーティファクトなし時は表示されない
  test('TC-1301-V02: artifactIds が空のとき ArtifactViewer は非表示', () => {
    // 【テスト目的】: アーティファクトがない trial では非表示になることを確認 🟢
    render(<ArtifactViewer trial={makeTrial([])} />)
    expect(screen.queryByTestId('artifact-viewer')).not.toBeInTheDocument()
  })

  // TC-1301-V03: ローディング中はプレースホルダーが表示される
  test('TC-1301-V03: ローディング中にグレープレースホルダーが表示される', () => {
    // 【テスト目的】: ロード前にプレースホルダーが表示されることを確認 🟢 REQ-144
    mockLoadArtifactUrl.mockReturnValue(new Promise(() => {})) // pending
    render(<ArtifactViewer trial={makeTrial(['artifact1.png'])} />)
    // 【確認内容】: ローディングプレースホルダーが表示されること
    expect(screen.getByTestId('artifact-loading-artifact1.png')).toBeInTheDocument()
  })

  // TC-1301-V04: 画像アーティファクトはサムネイルが表示される
  test('TC-1301-V04: 画像ファイルはサムネイルが表示される', async () => {
    // 【テスト目的】: 画像アーティファクトがサムネイルとして表示されることを確認 🟢 REQ-141
    mockLoadArtifactUrl.mockResolvedValue('blob:image-url')
    render(<ArtifactViewer trial={makeTrial(['photo.png'])} />)
    // 【確認内容】: サムネイルボタンが表示されること
    await waitFor(() => {
      expect(screen.getByTestId('artifact-thumbnail-photo.png')).toBeInTheDocument()
    })
  })

  // TC-1301-V05: ファイルなし時は「ファイルが見つかりません」が表示される
  test('TC-1301-V05: ファイルが見つからない場合エラーメッセージが表示される', async () => {
    // 【テスト目的】: ファイルが見つからない場合にエラー表示されることを確認 🟢 REQ-141
    mockLoadArtifactUrl.mockResolvedValue(null)
    render(<ArtifactViewer trial={makeTrial(['missing.png'])} />)
    await waitFor(() => {
      expect(screen.getByTestId('artifact-missing-missing.png')).toBeInTheDocument()
    })
  })

  // TC-1301-V06: その他ファイルはダウンロードリンクが表示される
  test('TC-1301-V06: その他ファイルはダウンロードリンクが表示される', async () => {
    // 【テスト目的】: 非画像/CSV ファイルにダウンロードリンクが表示されることを確認 🟢 REQ-143
    mockLoadArtifactUrl.mockResolvedValue('blob:file-url')
    render(<ArtifactViewer trial={makeTrial(['data.pkl'])} />)
    await waitFor(() => {
      expect(screen.getByTestId('artifact-download-data.pkl')).toBeInTheDocument()
    })
  })
})
