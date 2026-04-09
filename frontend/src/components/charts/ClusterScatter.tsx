import ReactECharts from 'echarts-for-react'
import { useClusterStore } from '../../stores/clusterStore'
import { useDownsampleStore } from '../../stores/downsampleStore'
import { getClusterColor } from '../panels/ClusterList'
import { EmptyState } from '../common/EmptyState'

export function ClusterScatter() {
  const { pcaProjections, clusterLabels, isRunning, clusterError } = useClusterStore()
  const clusterIndices = useDownsampleStore((s) => s.getIndices('cluster'))

  if (clusterError) {
    return <EmptyState message={clusterError} />
  }
  if (isRunning) {
    return (
      <div
        style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100%' }}
      >
        Loading...
      </div>
    )
  }
  if (!pcaProjections) {
    return <EmptyState message="Run clustering in the left panel first" />
  }

  // Filter projections using cluster indices
  const visibleProjections =
    clusterIndices.length > 0
      ? Array.from(clusterIndices)
          .map((i) => pcaProjections[i])
          .filter((p): p is number[] => p !== undefined)
      : pcaProjections
  const visibleLabels =
    clusterLabels && clusterIndices.length > 0
      ? Array.from(clusterIndices)
          .map((i) => clusterLabels[i])
          .filter((l): l is number => l !== undefined)
      : clusterLabels

  const k = visibleLabels ? Math.max(...visibleLabels) + 1 : 1
  const series = visibleLabels
    ? Array.from({ length: k }, (_, ci) => ({
        name: `Cluster ${ci}`,
        type: 'scatter' as const,
        data: visibleProjections.filter((_, i) => visibleLabels[i] === ci).map(([x, y]) => [x, y]),
        itemStyle: { color: getClusterColor(ci) },
      }))
    : [
        {
          name: 'Data',
          type: 'scatter' as const,
          data: visibleProjections.map(([x, y]) => [x, y]),
        },
      ]

  const option = {
    xAxis: { name: 'PC1', type: 'value' },
    yAxis: { name: 'PC2', type: 'value' },
    legend: { show: !!clusterLabels },
    series,
    tooltip: { trigger: 'item' },
  }

  return <ReactECharts option={option} style={{ width: '100%', height: '100%' }} />
}
