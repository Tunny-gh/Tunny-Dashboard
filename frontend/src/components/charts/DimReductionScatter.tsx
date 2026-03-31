import { useEffect, useState } from 'react'
import ReactECharts from 'echarts-for-react'
import { useClusterStore } from '../../stores/clusterStore'
import { useSelectionStore } from '../../stores/selectionStore'
import { WasmLoader } from '../../wasm/wasmLoader'
import { EmptyState } from '../common/EmptyState'

export function DimReductionScatter() {
  const [localProjections, setLocalProjections] = useState<number[][] | null>(null)
  const [isLocalLoading, setIsLocalLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const pcaProjections = useClusterStore((s) => s.pcaProjections)
  const isRunning = useClusterStore((s) => s.isRunning)
  // colorMode is read for future use
  useSelectionStore((s) => s.colorMode)

  const projections = pcaProjections ?? localProjections

  useEffect(() => {
    if (!pcaProjections && !isLocalLoading && !localProjections) {
      setIsLocalLoading(true)
      WasmLoader.getInstance()
        .then((wasm) => {
          const result = wasm.runPca(2, 'all')
          setLocalProjections(result.projections)
        })
        .catch((e) => setError(e instanceof Error ? e.message : String(e)))
        .finally(() => setIsLocalLoading(false))
    }
  }, [pcaProjections, isLocalLoading, localProjections])

  if (error) return <EmptyState message={error} />
  if (isRunning || isLocalLoading) {
    return (
      <div
        style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100%' }}
      >
        Loading...
      </div>
    )
  }
  if (!projections) return null

  const option = {
    title: { text: 'Dimensionality Reduction (PCA)' },
    xAxis: { type: 'value', name: 'PC1' },
    yAxis: { type: 'value', name: 'PC2' },
    tooltip: { trigger: 'item' },
    series: [
      {
        type: 'scatter',
        data: projections.map(([x, y]) => [x, y]),
      },
    ],
  }

  return (
    <div style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column' }}>
      <div style={{ padding: '8px' }}>
        <select style={{ padding: '4px 8px', borderRadius: '4px' }} defaultValue="pca">
          <option value="pca">PCA</option>
          <option value="umap" disabled>
            UMAP (Coming Soon)
          </option>
        </select>
      </div>
      <ReactECharts option={option} style={{ flex: 1 }} />
    </div>
  )
}
