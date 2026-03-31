import ReactECharts from 'echarts-for-react'
import { useClusterStore } from '../../stores/clusterStore'
import { getClusterColor } from '../panels/ClusterList'
import { EmptyState } from '../common/EmptyState'

export function ClusterScatter() {
  const { pcaProjections, clusterLabels, isRunning, clusterError } = useClusterStore()

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

  const k = clusterLabels ? Math.max(...clusterLabels) + 1 : 1
  const series = clusterLabels
    ? Array.from({ length: k }, (_, ci) => ({
        name: `Cluster ${ci}`,
        type: 'scatter' as const,
        data: pcaProjections
          .filter((_, i) => clusterLabels[i] === ci)
          .map(([x, y]) => [x, y]),
        itemStyle: { color: getClusterColor(ci) },
      }))
    : [
        {
          name: 'Data',
          type: 'scatter' as const,
          data: pcaProjections.map(([x, y]) => [x, y]),
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
