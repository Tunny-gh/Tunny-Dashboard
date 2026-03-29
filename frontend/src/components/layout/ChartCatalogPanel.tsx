/**
 * ChartCatalogPanel — 右側収納可能なチャートカタログパネル (chart-catalog TASK-003)
 *
 * 【役割】: 全 ChartId のカタログ一覧を表示し、ドラッグでキャンバスに追加できる
 * 【設計方針】:
 *   - isOpen はコンポーネントローカル状態（useState）で管理
 *   - freeModeLayout を購読して各 chartId のインスタンス数を表示
 *   - ドラッグ開始時に dataTransfer に { type: 'add-chart', chartId } をセット
 *   - Tailwind クラス不使用（インラインスタイルのみ）
 */

import { useState } from 'react'
import { useLayoutStore } from '../../stores/layoutStore'
import type { ChartId } from '../../types'

// -------------------------------------------------------------------------
// チャートカタログ定義
// -------------------------------------------------------------------------

/** カタログに表示するチャートの一覧（ChartId + 表示名） */
export const CHART_CATALOG: { chartId: ChartId; label: string }[] = [
  { chartId: 'pareto-front', label: 'Pareto Front' },
  { chartId: 'parallel-coords', label: 'Parallel Coordinates' },
  { chartId: 'scatter-matrix', label: 'Scatter Matrix' },
  { chartId: 'history', label: 'Optimization History' },
  { chartId: 'hypervolume', label: 'Hypervolume' },
  { chartId: 'objective-pair-matrix', label: 'Objective Pair Matrix' },
  { chartId: 'pdp', label: 'Partial Dependence Plot' },
  { chartId: 'importance', label: 'Parameter Importance' },
  { chartId: 'sensitivity-heatmap', label: 'Sensitivity Heatmap' },
  { chartId: 'cluster-view', label: 'Cluster View' },
  { chartId: 'umap', label: 'UMAP' },
  { chartId: 'slice', label: 'Slice Plot' },
  { chartId: 'edf', label: 'EDF' },
  { chartId: 'contour', label: 'Contour Plot' },
]

// -------------------------------------------------------------------------
// コンポーネント実装
// -------------------------------------------------------------------------

/**
 * 【機能概要】: 右側収納可能なチャートカタログパネル
 * 【テスト対応】: TASK-003 テストケース
 */
export function ChartCatalogPanel() {
  // 【開閉状態】: ローカル管理（永続化なし）
  const [isOpen, setIsOpen] = useState(false)

  // 【配置済み数の計算】: freeModeLayout の cells から chartId ごとにカウント
  const freeModeLayout = useLayoutStore((s) => s.freeModeLayout)
  const chartInstanceCount: Record<string, number> = {}
  if (freeModeLayout) {
    for (const cell of freeModeLayout.cells) {
      chartInstanceCount[cell.chartId] = (chartInstanceCount[cell.chartId] ?? 0) + 1
    }
  }

  return (
    <div
      data-testid="chart-catalog-panel"
      style={{
        display: 'flex',
        flexDirection: 'row',
        height: '100%',
        borderLeft: '1px solid var(--border)',
        background: 'var(--bg-header)',
        transition: 'width 200ms ease',
        overflow: 'hidden',
        width: isOpen ? '220px' : '28px',
        flexShrink: 0,
      }}
    >
      {/* ---------------------------------------------------------------- */}
      {/* トグルボタン                                                      */}
      {/* ---------------------------------------------------------------- */}
      <button
        data-testid="catalog-toggle-btn"
        onClick={() => setIsOpen((prev) => !prev)}
        title={isOpen ? 'Close Panel' : 'Add Chart'}
        style={{
          width: '28px',
          minWidth: '28px',
          height: '100%',
          background: 'transparent',
          border: 'none',
          borderRight: isOpen ? '1px solid var(--border)' : 'none',
          cursor: 'pointer',
          color: 'var(--accent)',
          fontSize: '12px',
          fontWeight: 700,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          flexShrink: 0,
        }}
      >
        {isOpen ? '◀' : '▶'}
      </button>

      {/* ---------------------------------------------------------------- */}
      {/* カタログリスト（isOpen のときのみ表示）                           */}
      {/* ---------------------------------------------------------------- */}
      <div
        data-testid="catalog-list"
        style={{
          display: isOpen ? 'flex' : 'none',
          flexDirection: 'column',
          flex: 1,
          overflowY: 'auto',
          padding: '8px 0',
        }}
      >
        <div
          style={{
            fontSize: '11px',
            fontWeight: 700,
            color: 'var(--text-muted)',
            padding: '0 10px 6px',
          }}
        >
          Add Chart
        </div>
        {CHART_CATALOG.map(({ chartId, label }) => {
          const count = chartInstanceCount[chartId] ?? 0
          return (
            <div
              key={chartId}
              data-testid={`catalog-item-${chartId}`}
              data-count={count}
              draggable
              onDragStart={(e) => {
                e.dataTransfer?.setData(
                  'text/plain',
                  JSON.stringify({ type: 'add-chart', chartId }),
                )
              }}
              style={{
                padding: '5px 10px',
                fontSize: '12px',
                cursor: 'grab',
                color: 'var(--text)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                userSelect: 'none',
                whiteSpace: 'nowrap',
                overflow: 'hidden',
              }}
            >
              <span style={{ overflow: 'hidden', textOverflow: 'ellipsis' }}>{label}</span>
              {count > 0 && (
                <span
                  style={{
                    fontSize: '11px',
                    color: 'var(--accent)',
                    fontWeight: 600,
                    marginLeft: '4px',
                    flexShrink: 0,
                  }}
                >
                  ({count})
                </span>
              )}
            </div>
          )
        })}
      </div>
    </div>
  )
}
