/**
 * ClusterList — クラスタ一覧コンポーネント (TASK-902)
 *
 * 【役割】: クラスタ一覧（件数・特徴サマリー・行クリック選択）を表示する
 * 【設計方針】:
 *   - クラスタ行クリック → selectionStore.brushSelect() で選択更新 🟢 REQ-085
 *   - Ctrl+クリック → 複数クラスタの OR 選択
 *   - centroid ± std + 有意差★ を比較テーブルで表示 🟢 REQ-083
 * 🟢 REQ-083〜REQ-087 に準拠
 */

import { useState } from 'react'
import { useSelectionStore } from '../../stores/selectionStore'

// -------------------------------------------------------------------------
// 型定義
// -------------------------------------------------------------------------

/**
 * 【クラスタ統計データ型】: WASM compute_cluster_stats() の結果を UI 表示用に変換した型
 */
export interface ClusterStatData {
  /** クラスタ ID（0 始まり）*/
  clusterId: number
  /** クラスタに含まれるサンプル数 */
  size: number
  /** 各特徴の重心値（centroid[j] = 特徴 j の平均）*/
  centroid: number[]
  /** 各特徴の標準偏差 */
  stdDev: number[]
  /** 有意差フラグ: Welch's t 検定 |t|>3.0 のとき true 🟢 */
  significantFeatures: boolean[]
}

/**
 * 【ClusterList Props】: クラスタ一覧のプロパティ
 */
export interface ClusterListProps {
  /** クラスタ統計データ一覧 */
  clusterStats: ClusterStatData[]
  /** 特徴名リスト（centroid/std の列名に対応）*/
  featureNames: string[]
  /**
   * 各クラスタに含まれる試行インデックス: trialsByCluster[clusterId] = Uint32Array
   * クラスタ行クリック時に brushSelect() に渡すインデックス群
   */
  trialsByCluster: Uint32Array[]
}

// -------------------------------------------------------------------------
// 定数
// -------------------------------------------------------------------------

/**
 * 【クラスタ識別色】: 最大 8 クラスタまで対応
 * 【色選択方針】: 視認性・色覚バリアフリーを考慮した彩度高めの配色 🟡
 */
const CLUSTER_COLORS = [
  '#4f46e5', // indigo
  '#e11d48', // rose
  '#059669', // emerald
  '#d97706', // amber
  '#0891b2', // cyan
  '#7c3aed', // violet
  '#ea580c', // orange
  '#0d9488', // teal
]

// -------------------------------------------------------------------------
// ユーティリティ
// -------------------------------------------------------------------------

/**
 * 【クラスタ色取得】: clusterId から識別色を返す（8 色以上は循環）
 */
export function getClusterColor(clusterId: number): string {
  return CLUSTER_COLORS[clusterId % CLUSTER_COLORS.length]
}

// -------------------------------------------------------------------------
// メインコンポーネント
// -------------------------------------------------------------------------

/**
 * 【機能概要】: クラスタ一覧表示コンポーネント
 * 【クラスタ選択】: 行クリック → brushSelect()、Ctrl+クリック → 複数選択
 * 【特徴サマリー】: centroid ± std、有意差★
 * 🟢 REQ-083〜REQ-087 に準拠
 */
export function ClusterList({ clusterStats, featureNames, trialsByCluster }: ClusterListProps) {
  // 【Store 接続】: selectionStore から brushSelect を取得 🟢
  const brushSelect = useSelectionStore((s) => s.brushSelect)

  // 【内部状態】: 現在選択中のクラスタ ID セット
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set())

  /**
   * 【クラスタ行クリックハンドラ】: 通常クリックは単一選択、Ctrl+クリックは複数選択
   * 選択後に対象クラスタの全インデックスを brushSelect() に渡す 🟢 REQ-085
   */
  const handleClusterClick = (clusterId: number, ctrlKey: boolean) => {
    let newSelected: Set<number>

    if (ctrlKey) {
      // 【Ctrl+クリック】: 既存選択セットに追加 or 除外
      newSelected = new Set(selectedIds)
      if (newSelected.has(clusterId)) {
        newSelected.delete(clusterId)
      } else {
        newSelected.add(clusterId)
      }
    } else {
      // 【通常クリック】: 単一クラスタのみ選択
      newSelected = new Set([clusterId])
    }

    setSelectedIds(newSelected)

    // 【brushSelect 呼び出し】: 選択クラスタの全インデックスを結合してソート
    const allIndices: number[] = []
    newSelected.forEach((id) => {
      const cluster = trialsByCluster[id]
      if (cluster) {
        allIndices.push(...Array.from(cluster))
      }
    })
    brushSelect(new Uint32Array(allIndices.sort((a, b) => a - b)))
  }

  // -------------------------------------------------------------------------
  // 空状態UI
  // -------------------------------------------------------------------------

  /** 【空状態】: クラスタリング未実行時のメッセージ */
  if (clusterStats.length === 0) {
    return (
      <div data-testid="cluster-list" style={{ padding: '12px' }}>
        <span style={{ fontSize: '13px', color: '#6b7280' }}>Clustering has not been run yet</span>
      </div>
    )
  }

  // -------------------------------------------------------------------------
  // レンダリング
  // -------------------------------------------------------------------------

  return (
    <div data-testid="cluster-list" style={{ overflowX: 'auto' }}>
      {/* 【クラスタ比較テーブル】: centroid ± std + 有意差★ 🟢 REQ-083 */}
      <table
        style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px', minWidth: '400px' }}
      >
        <thead>
          <tr style={{ background: '#f9fafb' }}>
            <th
              style={{
                padding: '4px 8px',
                textAlign: 'left',
                borderBottom: '1px solid #e5e7eb',
                whiteSpace: 'nowrap',
              }}
            >
              Cluster
            </th>
            <th
              style={{
                padding: '4px 8px',
                textAlign: 'right',
                borderBottom: '1px solid #e5e7eb',
                whiteSpace: 'nowrap',
              }}
            >
              Count
            </th>
            {featureNames.map((name) => (
              <th
                key={name}
                style={{
                  padding: '4px 8px',
                  textAlign: 'right',
                  borderBottom: '1px solid #e5e7eb',
                  whiteSpace: 'nowrap',
                  maxWidth: '120px',
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                }}
              >
                {name}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {clusterStats.map((stat) => {
            const color = getClusterColor(stat.clusterId)
            const isSelected = selectedIds.has(stat.clusterId)

            return (
              <tr
                key={stat.clusterId}
                data-testid={`cluster-row-${stat.clusterId}`}
                onClick={(e) => handleClusterClick(stat.clusterId, e.ctrlKey)}
                style={{
                  cursor: 'pointer',
                  background: isSelected ? '#eff6ff' : undefined,
                  borderBottom: '1px solid #f3f4f6',
                }}
              >
                {/* 【クラスタバッジ】: 識別色のカラードバッジ 🟢 REQ-086 */}
                <td style={{ padding: '4px 8px' }}>
                  <span
                    data-testid={`cluster-badge-${stat.clusterId}`}
                    style={{
                      display: 'inline-block',
                      padding: '1px 6px',
                      background: color,
                      color: '#fff',
                      borderRadius: '10px',
                      fontSize: '11px',
                      fontWeight: 600,
                    }}
                  >
                    C{stat.clusterId}
                  </span>
                </td>

                {/* 【件数セル】 */}
                <td style={{ padding: '4px 8px', textAlign: 'right' }}>{stat.size}</td>

                {/* 【特徴統計セル】: centroid ± std + 有意差★ */}
                {featureNames.map((_, j) => (
                  <td
                    key={j}
                    style={{
                      padding: '4px 8px',
                      textAlign: 'right',
                      fontVariantNumeric: 'tabular-nums',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    <span data-testid={`stat-${stat.clusterId}-${j}`}>
                      {stat.centroid[j] !== undefined ? stat.centroid[j].toFixed(3) : '—'}±
                      {stat.stdDev[j] !== undefined ? stat.stdDev[j].toFixed(3) : '—'}
                    </span>
                    {/* 【有意差マーク】: Welch's t |t|>3.0 のとき ★ 表示 🟢 */}
                    {stat.significantFeatures[j] && (
                      <span
                        data-testid={`sig-${stat.clusterId}-${j}`}
                        style={{ color: '#4f46e5', fontWeight: 700, marginLeft: '2px' }}
                      >
                        ★
                      </span>
                    )}
                  </td>
                ))}
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}
