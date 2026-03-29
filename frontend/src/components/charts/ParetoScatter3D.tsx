/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 * Documentation.
 */

import { useEffect, useRef } from 'react'
import { DeckGL, PointCloudLayer } from 'deck.gl'
import { useSelectionStore } from '../../stores/selectionStore'
import type { GpuBuffer } from '../../wasm/gpuBuffer'
import type { Study } from '../../types'
import { EmptyState } from '../common/EmptyState'

// -------------------------------------------------------------------------
// Props Type definitions
// -------------------------------------------------------------------------

export interface ParetoScatter3DProps {
  /** Documentation. */
  gpuBuffer: GpuBuffer | null
  /** Documentation. */
  currentStudy: Study | null
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 * Documentation.
 * Test Coverage: TC-501-01〜03, TC-501-B01, TC-501-E01
 */
export function ParetoScatter3D({ gpuBuffer }: ParetoScatter3DProps) {
  // Documentation.
  const unsubscribeRef = useRef<(() => void) | null>(null)

  useEffect(() => {
    // Documentation.
    // Documentation.
    if (gpuBuffer) {
      const unsubscribe = useSelectionStore.subscribe(
        (state) => state.selectedIndices,
        (indices) => gpuBuffer.updateAlphas(indices),
      )
      unsubscribeRef.current = unsubscribe
    }

    // Documentation.
    return () => {
      if (unsubscribeRef.current) {
        unsubscribeRef.current()
        unsubscribeRef.current = null
      }
    }
  }, [gpuBuffer])

  // Documentation.
  if (!gpuBuffer) {
    return <EmptyState />
  }

  // Documentation.
  const layer = new PointCloudLayer({
    id: 'pareto-3d',
    data: { length: gpuBuffer.trialCount },
    getPosition: (_: unknown, { index }: { index: number }) => [
      gpuBuffer.positions3d[index * 3],
      gpuBuffer.positions3d[index * 3 + 1],
      gpuBuffer.positions3d[index * 3 + 2],
    ],
    getColor: (_: unknown, { index }: { index: number }) => [
      gpuBuffer.colors[index * 4] * 255,
      gpuBuffer.colors[index * 4 + 1] * 255,
      gpuBuffer.colors[index * 4 + 2] * 255,
      gpuBuffer.colors[index * 4 + 3] * 255,
    ],
    getRadius: (_: unknown, { index }: { index: number }) => gpuBuffer.sizes[index],
    pickable: true,
  })

  // Documentation.
  return <DeckGL layers={[layer]} controller={true} style={{ width: '100%', height: '100%' }} />
}
