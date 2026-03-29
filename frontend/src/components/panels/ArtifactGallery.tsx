/**
 * ArtifactGallery — artifact gallery view (TASK-1301)
 *
 * Displays artifacts for multiple trials in a grid gallery.
 * Supports group filtering (selection / Pareto / cluster), card size switching,
 * paginated loading (48 per page), and hides entirely when no directory is selected.
 */

import React, { useState, useEffect } from 'react'
import { useArtifactStore, getMimeTypeCategory } from '../../stores/artifactStore'
import type { Trial } from '../../types'

// -------------------------------------------------------------------------
// Constants
// -------------------------------------------------------------------------

const PAGE_SIZE = 48

/** Card height CSS classes keyed by size */
const CARD_SIZE_CLASSES: Record<CardSize, string> = {
  small: 'h-20',
  medium: 'h-36',
  large: 'h-56',
}

// -------------------------------------------------------------------------
// Types
// -------------------------------------------------------------------------

export type CardSize = 'small' | 'medium' | 'large'
export type GalleryGroup = 'selection' | 'pareto' | 'cluster' | 'all'

interface ArtifactGalleryProps {
  /** Trials to display */
  trials: Trial[]
  /** Pareto solution indices */
  paretoIndices?: Uint32Array
  /** Active group filter */
  group?: GalleryGroup
}

// -------------------------------------------------------------------------
// ArtifactCard — individual card
// -------------------------------------------------------------------------

const ArtifactCard: React.FC<{
  trial: Trial
  cardSize: CardSize
}> = ({ trial, cardSize }) => {
  const { dirHandle, loadArtifactUrl } = useArtifactStore()
  const [url, setUrl] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [type, setType] = useState<string>('other')

  const firstArtifactId = trial.artifactIds?.[0] ?? null

  useEffect(() => {
    if (!dirHandle || !firstArtifactId) {
      setIsLoading(false)
      return
    }
    const filename = firstArtifactId
    loadArtifactUrl(firstArtifactId, filename).then((u) => {
      setUrl(u)
      setType(getMimeTypeCategory(filename))
      setIsLoading(false)
    })
  }, [dirHandle, firstArtifactId, loadArtifactUrl])

  const sizeClass = CARD_SIZE_CLASSES[cardSize]

  return (
    <div
      data-testid={`gallery-card-${trial.trialId}`}
      className="border border-gray-200 rounded overflow-hidden bg-white"
    >
      {/* Thumbnail area */}
      <div className={`${sizeClass} bg-gray-100 flex items-center justify-center`}>
        {isLoading && (
          <div
            data-testid={`gallery-loading-${trial.trialId}`}
            className="w-full h-full bg-gray-200 animate-pulse"
          />
        )}
        {!isLoading && url === null && (
          <span className="text-xs text-gray-400">
            {firstArtifactId ? 'File not found' : 'None'}
          </span>
        )}
        {!isLoading && url && type === 'image' && (
          <img
            data-testid={`gallery-image-${trial.trialId}`}
            src={url}
            alt={`Trial ${trial.trialId}`}
            className="w-full h-full object-contain"
          />
        )}
        {!isLoading && url && type !== 'image' && (
          <span className="text-xs text-gray-500">{type.toUpperCase()}</span>
        )}
      </div>

      {/* Footer */}
      <div className="px-2 py-1 text-xs text-gray-600">Trial {trial.trialId}</div>
    </div>
  )
}

// -------------------------------------------------------------------------
// ArtifactGallery component
// -------------------------------------------------------------------------
export const ArtifactGallery: React.FC<ArtifactGalleryProps> = ({
  trials,
  paretoIndices,
  group = 'all',
}) => {
  const { dirHandle } = useArtifactStore()
  const [cardSize, setCardSize] = useState<CardSize>('medium')
  const [visibleCount, setVisibleCount] = useState(PAGE_SIZE)

  // Hide when no directory is selected
  if (!dirHandle) return null

  // Filter by group: Pareto / selection / all
  const filteredTrials = React.useMemo(() => {
    if (group === 'pareto' && paretoIndices) {
      const set = new Set(Array.from(paretoIndices))
      return trials.filter((t) => set.has(t.trialId))
    }
    return trials.filter((t) => t.artifactIds && t.artifactIds.length > 0)
  }, [trials, group, paretoIndices])

  const visibleTrials = filteredTrials.slice(0, visibleCount)
  const hasMore = visibleCount < filteredTrials.length

  // Column count based on card size (max 4 columns)
  const colClass =
    cardSize === 'small' ? 'grid-cols-4' : cardSize === 'medium' ? 'grid-cols-3' : 'grid-cols-2'

  return (
    <div data-testid="artifact-gallery" className="flex flex-col gap-3 p-3">
      {/* Toolbar */}
      <div className="flex items-center gap-2">
        {/* Card size toggle */}
        <div className="flex gap-1" data-testid="card-size-controls">
          {(['small', 'medium', 'large'] as CardSize[]).map((size) => (
            <button
              key={size}
              data-testid={`card-size-${size}`}
              onClick={() => setCardSize(size)}
              className={`px-2 py-0.5 text-xs rounded border transition-colors
                ${cardSize === size ? 'bg-blue-600 text-white border-blue-600' : 'border-gray-300 text-gray-600 hover:bg-gray-50'}`}
            >
              {size === 'small' ? 'S' : size === 'medium' ? 'M' : 'L'}
            </button>
          ))}
        </div>
        <span className="text-xs text-gray-400">{filteredTrials.length} items</span>
      </div>

      {/* Grid */}
      {visibleTrials.length === 0 ? (
        <p data-testid="gallery-empty" className="text-sm text-gray-400">
          No artifacts to display
        </p>
      ) : (
        <div className={`grid ${colClass} gap-2`} data-testid="gallery-grid">
          {visibleTrials.map((trial) => (
            <ArtifactCard key={trial.trialId} trial={trial} cardSize={cardSize} />
          ))}
        </div>
      )}

      {/* Load more button */}
      {hasMore && (
        <button
          data-testid="load-more-btn"
          onClick={() => setVisibleCount((n) => n + PAGE_SIZE)}
          className="self-center px-4 py-1.5 text-sm border border-gray-300 rounded
                     text-gray-600 hover:bg-gray-50 transition-colors"
        >
          Load more ({filteredTrials.length - visibleCount} remaining)
        </button>
      )}
    </div>
  )
}

export default ArtifactGallery
