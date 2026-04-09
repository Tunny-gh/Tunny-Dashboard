/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 * Documentation.
 */

import { useEffect, useRef } from 'react'
import { DeckGL, ScatterplotLayer } from 'deck.gl'
import { useSelectionStore } from '../../stores/selectionStore'
import { useDownsampleStore } from '../../stores/downsampleStore'
import type { GpuBuffer } from '../../wasm/gpuBuffer'
import type { Study } from '../../types'
import { EmptyState } from '../common/EmptyState'

// -------------------------------------------------------------------------
// Props Type definitions
// -------------------------------------------------------------------------

export interface ParetoScatter2DProps {
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
 * Test Coverage: TC-501-04, TC-501-E02
 */
export function ParetoScatter2D({ gpuBuffer }: ParetoScatter2DProps) {
  // Documentation.
  const unsubscribeRef = useRef<(() => void) | null>(null)

  useEffect(() => {
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
  const renderIndices = useDownsampleStore((s) => s.getIndices('scatter'))

  // Documentation.
  if (!gpuBuffer) {
    return <EmptyState />
  }

  // Documentation.
  const dataLength = renderIndices.length > 0 ? renderIndices.length : gpuBuffer.trialCount
  const getTrialIdx = (index: number) => (renderIndices.length > 0 ? renderIndices[index] : index)

  // Documentation.
  const layer = new ScatterplotLayer({
    id: 'pareto-2d',
    data: { length: dataLength },
    getPosition: (_: unknown, { index }: { index: number }) => {
      const trialIdx = getTrialIdx(index)
      return [gpuBuffer.positions[trialIdx * 2], gpuBuffer.positions[trialIdx * 2 + 1], 0]
    },
    getColor: (_: unknown, { index }: { index: number }) => {
      const trialIdx = getTrialIdx(index)
      return [
        gpuBuffer.colors[trialIdx * 4] * 255,
        gpuBuffer.colors[trialIdx * 4 + 1] * 255,
        gpuBuffer.colors[trialIdx * 4 + 2] * 255,
        gpuBuffer.colors[trialIdx * 4 + 3] * 255,
      ]
    },
    getRadius: (_: unknown, { index }: { index: number }) => gpuBuffer.sizes[getTrialIdx(index)],
    pickable: true,
  })

  // Documentation.
  return <DeckGL layers={[layer]} controller={true} style={{ width: '100%', height: '100%' }} />
}
