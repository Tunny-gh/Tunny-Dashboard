/**
 * Documentation.
 *
 * Documentation.
 * Design:
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */

import { useState } from 'react'
import { useSelectionStore } from '../../stores/selectionStore'

// -------------------------------------------------------------------------
// Type definitions
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
export interface ClusterStatData {
  /** Documentation. */
  clusterId: number
  /** Documentation. */
  size: number
  /** Documentation. */
  centroid: number[]
  /** Documentation. */
  stdDev: number[]
  /** Documentation. */
  significantFeatures: boolean[]
}

/**
 * Documentation.
 */
export interface ClusterListProps {
  /** Documentation. */
  clusterStats: ClusterStatData[]
  /** Documentation. */
  featureNames: string[]
  /**
   * Documentation.
   * Documentation.
   */
  trialsByCluster: Uint32Array[]
}

// -------------------------------------------------------------------------
// Constants
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
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
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
export function getClusterColor(clusterId: number): string {
  return CLUSTER_COLORS[clusterId % CLUSTER_COLORS.length]
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 * Documentation.
 * Documentation.
 */
export function ClusterList({ clusterStats, featureNames, trialsByCluster }: ClusterListProps) {
  // Documentation.
  const brushSelect = useSelectionStore((s) => s.brushSelect)

  // Documentation.
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set())

  /**
   * Documentation.
   * Documentation.
   */
  const handleClusterClick = (clusterId: number, ctrlKey: boolean) => {
    let newSelected: Set<number>

    if (ctrlKey) {
      // Documentation.
      newSelected = new Set(selectedIds)
      if (newSelected.has(clusterId)) {
        newSelected.delete(clusterId)
      } else {
        newSelected.add(clusterId)
      }
    } else {
      // Documentation.
      newSelected = new Set([clusterId])
    }

    setSelectedIds(newSelected)

    // Documentation.
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
  // empty stateUI
  // -------------------------------------------------------------------------

  /** Documentation. */
  if (clusterStats.length === 0) {
    return (
      <div data-testid="cluster-list" style={{ padding: '12px' }}>
        <span style={{ fontSize: '13px', color: '#6b7280' }}>Clustering has not been run yet</span>
      </div>
    )
  }

  // -------------------------------------------------------------------------
  // Rendering
  // -------------------------------------------------------------------------

  return (
    <div data-testid="cluster-list" style={{ overflowX: 'auto' }}>
      {/* Documentation. */}
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
                {/* Documentation. */}
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

                {/* Documentation. */}
                <td style={{ padding: '4px 8px', textAlign: 'right' }}>{stat.size}</td>

                {/* Documentation. */}
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
                    {/* Documentation. */}
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
