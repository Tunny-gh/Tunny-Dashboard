/**
 * ArtifactGallery — アーティファクトギャラリービュー (TASK-1301)
 *
 * 【役割】: 複数 trial のアーティファクトをギャラリー形式で表示する
 * 【設計方針】:
 *   - グループ別表示: Brushing 選択 / Pareto / クラスタ
 *   - 最大 4 列の並列比較レイアウト 🟢 REQ-141
 *   - カードサイズ切替（小/中/大）
 *   - 「さらに読み込む」ボタン: 48 件ずつ追加表示 🟡
 *   - ディレクトリ未選択時は完全に非表示 🟢 REQ-140
 * 🟢 REQ-140〜REQ-144 に準拠
 */

import React, { useState, useEffect } from 'react';
import { useArtifactStore, getMimeTypeCategory } from '../../stores/artifactStore';
import type { Trial } from '../../types';

// -------------------------------------------------------------------------
// 定数
// -------------------------------------------------------------------------

/** 【1ページあたりの表示件数】 */
const PAGE_SIZE = 48;

/** 【カードサイズ設定】: small/medium/large → CSS クラス */
const CARD_SIZE_CLASSES: Record<CardSize, string> = {
  small: 'h-20',
  medium: 'h-36',
  large: 'h-56',
};

// -------------------------------------------------------------------------
// 型定義
// -------------------------------------------------------------------------

export type CardSize = 'small' | 'medium' | 'large';
export type GalleryGroup = 'selection' | 'pareto' | 'cluster' | 'all';

interface ArtifactGalleryProps {
  /** 表示する試行リスト */
  trials: Trial[];
  /** Pareto 解インデックス */
  paretoIndices?: Uint32Array;
  /** 現在のグループフィルタ */
  group?: GalleryGroup;
}

// -------------------------------------------------------------------------
// ArtifactCard — 個別カード
// -------------------------------------------------------------------------

const ArtifactCard: React.FC<{
  trial: Trial;
  cardSize: CardSize;
}> = ({ trial, cardSize }) => {
  const { dirHandle, loadArtifactUrl } = useArtifactStore();
  const [url, setUrl] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [type, setType] = useState<string>('other');

  const firstArtifactId = trial.artifactIds?.[0] ?? null;

  useEffect(() => {
    if (!dirHandle || !firstArtifactId) {
      setIsLoading(false);
      return;
    }
    const filename = firstArtifactId;
    loadArtifactUrl(firstArtifactId, filename).then((u) => {
      setUrl(u);
      setType(getMimeTypeCategory(filename));
      setIsLoading(false);
    });
  }, [dirHandle, firstArtifactId, loadArtifactUrl]);

  const sizeClass = CARD_SIZE_CLASSES[cardSize];

  return (
    <div
      data-testid={`gallery-card-${trial.trialId}`}
      className="border border-gray-200 rounded overflow-hidden bg-white"
    >
      {/* サムネイル領域 */}
      <div className={`${sizeClass} bg-gray-100 flex items-center justify-center`}>
        {isLoading && (
          <div
            data-testid={`gallery-loading-${trial.trialId}`}
            className="w-full h-full bg-gray-200 animate-pulse"
          />
        )}
        {!isLoading && url === null && (
          <span className="text-xs text-gray-400">
            {firstArtifactId ? 'ファイルが見つかりません' : 'なし'}
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

      {/* フッター情報 */}
      <div className="px-2 py-1 text-xs text-gray-600">
        Trial {trial.trialId}
      </div>
    </div>
  );
};

// -------------------------------------------------------------------------
// ArtifactGallery コンポーネント
// -------------------------------------------------------------------------

/**
 * 【機能概要】: 複数 trial のアーティファクトをグリッドギャラリーで表示する
 */
export const ArtifactGallery: React.FC<ArtifactGalleryProps> = ({
  trials,
  paretoIndices,
  group = 'all',
}) => {
  const { dirHandle } = useArtifactStore();
  const [cardSize, setCardSize] = useState<CardSize>('medium');
  const [visibleCount, setVisibleCount] = useState(PAGE_SIZE);

  // 【ディレクトリ未選択時は非表示】: REQ-140
  if (!dirHandle) return null;

  // 【グループフィルタ】: Pareto / 選択 / 全件
  const filteredTrials = React.useMemo(() => {
    if (group === 'pareto' && paretoIndices) {
      const set = new Set(Array.from(paretoIndices));
      return trials.filter((t) => set.has(t.trialId));
    }
    return trials.filter((t) => t.artifactIds && t.artifactIds.length > 0);
  }, [trials, group, paretoIndices]);

  const visibleTrials = filteredTrials.slice(0, visibleCount);
  const hasMore = visibleCount < filteredTrials.length;

  // 【列数設定】: カードサイズに応じて列数を変える（最大 4 列）
  const colClass =
    cardSize === 'small' ? 'grid-cols-4' : cardSize === 'medium' ? 'grid-cols-3' : 'grid-cols-2';

  return (
    <div data-testid="artifact-gallery" className="flex flex-col gap-3 p-3">
      {/* ツールバー */}
      <div className="flex items-center gap-2">
        {/* カードサイズ切替 */}
        <div className="flex gap-1" data-testid="card-size-controls">
          {(['small', 'medium', 'large'] as CardSize[]).map((size) => (
            <button
              key={size}
              data-testid={`card-size-${size}`}
              onClick={() => setCardSize(size)}
              className={`px-2 py-0.5 text-xs rounded border transition-colors
                ${cardSize === size ? 'bg-blue-600 text-white border-blue-600' : 'border-gray-300 text-gray-600 hover:bg-gray-50'}`}
            >
              {size === 'small' ? '小' : size === 'medium' ? '中' : '大'}
            </button>
          ))}
        </div>
        <span className="text-xs text-gray-400">{filteredTrials.length} 件</span>
      </div>

      {/* グリッド */}
      {visibleTrials.length === 0 ? (
        <p data-testid="gallery-empty" className="text-sm text-gray-400">
          表示するアーティファクトがありません
        </p>
      ) : (
        <div className={`grid ${colClass} gap-2`} data-testid="gallery-grid">
          {visibleTrials.map((trial) => (
            <ArtifactCard key={trial.trialId} trial={trial} cardSize={cardSize} />
          ))}
        </div>
      )}

      {/* さらに読み込むボタン */}
      {hasMore && (
        <button
          data-testid="load-more-btn"
          onClick={() => setVisibleCount((n) => n + PAGE_SIZE)}
          className="self-center px-4 py-1.5 text-sm border border-gray-300 rounded
                     text-gray-600 hover:bg-gray-50 transition-colors"
        >
          さらに読み込む ({filteredTrials.length - visibleCount} 件)
        </button>
      )}
    </div>
  );
};

export default ArtifactGallery;
