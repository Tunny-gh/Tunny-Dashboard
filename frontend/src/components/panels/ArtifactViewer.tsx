/**
 * ArtifactViewer — per-trial artifact detail viewer (TASK-1301)
 *
 * Renders artifacts for the selected trial by type:
 * images as thumbnails with lightbox, CSV as a table (first 20 rows),
 * other files as download links. Shows a grey placeholder while loading.
 * Hidden when no directory is selected or the trial has no artifacts.
 */

import React, { useEffect, useState } from 'react'
import { useArtifactStore, getMimeTypeCategory } from '../../stores/artifactStore'
import type { Trial, ArtifactType } from '../../types'

// -------------------------------------------------------------------------
// Constants
// -------------------------------------------------------------------------

const MAX_CSV_ROWS = 20

// -------------------------------------------------------------------------
// Types
// -------------------------------------------------------------------------

interface ArtifactViewerProps {
  /** Trial to display */
  trial: Trial
}

/** Internal state for a single artifact item */
interface ArtifactItem {
  artifactId: string
  filename: string
  type: ArtifactType
  url: string | null
  isLoading: boolean
}

// -------------------------------------------------------------------------
// CSV preview component
// -------------------------------------------------------------------------
const CsvPreview: React.FC<{ url: string }> = ({ url }) => {
  const [rows, setRows] = useState<string[][]>([])
  const [isLoading, setIsLoading] = useState(true)

  useEffect(() => {
    const fetchCsv = async () => {
      try {
        const response = await fetch(url)
        const text = await response.text()
        const parsed = text
          .trim()
          .split('\n')
          .slice(0, MAX_CSV_ROWS + 1) // ヘッダ + MAX_CSV_ROWS 行
          .map((line) => line.split(','))
        setRows(parsed)
      } catch {
        setRows([])
      } finally {
        setIsLoading(false)
      }
    }
    fetchCsv()
  }, [url])

  if (isLoading) {
    return (
      <div data-testid="csv-loading" className="text-gray-400 text-sm">
        読み込み中...
      </div>
    )
  }

  if (rows.length === 0) {
    return <p className="text-gray-400 text-sm">CSV データを読み込めませんでした</p>
  }

  const [header, ...dataRows] = rows

  return (
    <div data-testid="csv-table" className="overflow-x-auto max-h-64">
      <table className="text-xs border-collapse w-full">
        <thead>
          <tr>
            {header.map((cell, i) => (
              <th key={i} className="border border-gray-300 bg-gray-100 px-2 py-1 text-left">
                {cell.trim()}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {dataRows.map((row, ri) => (
            <tr key={ri} className={ri % 2 === 0 ? 'bg-white' : 'bg-gray-50'}>
              {row.map((cell, ci) => (
                <td key={ci} className="border border-gray-300 px-2 py-0.5">
                  {cell.trim()}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      {rows.length > MAX_CSV_ROWS && (
        <p className="text-xs text-gray-400 mt-1">先頭 {MAX_CSV_ROWS} 行を表示</p>
      )}
    </div>
  )
}

// -------------------------------------------------------------------------
// ライトボックスコンポーネント
// -------------------------------------------------------------------------

const Lightbox: React.FC<{ url: string; onClose: () => void }> = ({ url, onClose }) => (
  <div
    data-testid="lightbox"
    className="fixed inset-0 bg-black/80 z-50 flex items-center justify-center"
    onClick={onClose}
  >
    <img
      src={url}
      alt="拡大表示"
      className="max-w-[90vw] max-h-[90vh] object-contain"
      onClick={(e) => e.stopPropagation()}
    />
    <button
      className="absolute top-4 right-4 text-white text-2xl"
      onClick={onClose}
      aria-label="閉じる"
    >
      ✕
    </button>
  </div>
)

// -------------------------------------------------------------------------
// ArtifactViewer コンポーネント
// -------------------------------------------------------------------------

/**
 * 【機能概要】: 単一 trial のアーティファクト一覧を表示するビューア
 */
export const ArtifactViewer: React.FC<ArtifactViewerProps> = ({ trial }) => {
  const { dirHandle, loadArtifactUrl } = useArtifactStore()
  const [items, setItems] = useState<ArtifactItem[]>([])
  const [lightboxUrl, setLightboxUrl] = useState<string | null>(null)

  // 【アーティファクトロード】: trial の artifactIds からファイルをロードする
  useEffect(() => {
    if (!dirHandle || !trial.artifactIds || trial.artifactIds.length === 0) {
      setItems([])
      return
    }

    // 【初期状態】: ロード中プレースホルダーを設定
    const initial: ArtifactItem[] = trial.artifactIds.map((aid) => ({
      artifactId: aid,
      filename: aid, // 🟡 ファイル名はデフォルトで artifactId と同じ
      type: 'other' as ArtifactType,
      url: null,
      isLoading: true,
    }))
    setItems(initial)

    // 【非同期ロード】: 各アーティファクトの URL を取得
    trial.artifactIds.forEach(async (artifactId, index) => {
      const filename = artifactId // 🟡 ファイル名は artifactId と同じと仮定
      const url = await loadArtifactUrl(artifactId, filename)
      const type = getMimeTypeCategory(filename)

      setItems((prev) => {
        const next = [...prev]
        next[index] = { artifactId, filename, type, url, isLoading: false }
        return next
      })
    })
  }, [dirHandle, trial.artifactIds, loadArtifactUrl])

  // 【非表示条件】: ディレクトリ未選択またはアーティファクトなし
  if (!dirHandle || !trial.artifactIds || trial.artifactIds.length === 0) {
    return null
  }

  return (
    <div data-testid="artifact-viewer" className="space-y-3 p-3">
      <h4 className="text-sm font-semibold text-gray-700">
        Trial {trial.trialId} のアーティファクト
      </h4>

      {items.map((item) => (
        <div
          key={item.artifactId}
          data-testid={`artifact-item-${item.artifactId}`}
          className="border border-gray-200 rounded p-2"
        >
          <p className="text-xs text-gray-500 mb-1">{item.filename}</p>

          {/* 【ローディング中】: グレープレースホルダー */}
          {item.isLoading && (
            <div
              data-testid={`artifact-loading-${item.artifactId}`}
              className="w-full h-20 bg-gray-200 rounded animate-pulse"
            />
          )}

          {/* 【ファイルなし】 */}
          {!item.isLoading && item.url === null && (
            <p data-testid={`artifact-missing-${item.artifactId}`} className="text-xs text-red-500">
              ファイルが見つかりません
            </p>
          )}

          {/* 【画像】: サムネイル + ライトボックス 🟢 REQ-141 */}
          {!item.isLoading && item.url && item.type === 'image' && (
            <button
              data-testid={`artifact-thumbnail-${item.artifactId}`}
              onClick={() => setLightboxUrl(item.url)}
              className="block"
            >
              <img
                src={item.url}
                alt={item.filename}
                className="max-h-32 object-contain rounded cursor-pointer hover:opacity-80"
              />
            </button>
          )}

          {/* 【CSV】: テーブル表示 🟢 REQ-142 */}
          {!item.isLoading && item.url && item.type === 'csv' && <CsvPreview url={item.url} />}

          {/* 【その他】: ダウンロードリンク 🟢 REQ-143 */}
          {!item.isLoading && item.url && item.type !== 'image' && item.type !== 'csv' && (
            <a
              data-testid={`artifact-download-${item.artifactId}`}
              href={item.url}
              download={item.filename}
              className="text-xs text-blue-600 underline"
            >
              {item.filename} をダウンロード
            </a>
          )}
        </div>
      ))}

      {/* 【ライトボックス】: 画像拡大表示 */}
      {lightboxUrl && <Lightbox url={lightboxUrl} onClose={() => setLightboxUrl(null)} />}
    </div>
  )
}

export default ArtifactViewer
