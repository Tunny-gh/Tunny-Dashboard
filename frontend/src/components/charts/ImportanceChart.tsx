import { useEffect, useState } from 'react'
import ReactECharts from 'echarts-for-react'
import { useAnalysisStore } from '../../stores/analysisStore'
import { EmptyState } from '../common/EmptyState'

type ImportanceMetric = 'spearman' | 'beta'

export function ImportanceChart() {
  const [metric, setMetric] = useState<ImportanceMetric>('spearman')
  const { sensitivityResult, isComputingSensitivity, sensitivityError, computeSensitivity } =
    useAnalysisStore()

  useEffect(() => {
    if (!sensitivityResult && !isComputingSensitivity) {
      computeSensitivity()
    }
  }, [sensitivityResult, isComputingSensitivity, computeSensitivity])

  if (sensitivityError) {
    return <EmptyState message={sensitivityError} />
  }
  if (isComputingSensitivity || !sensitivityResult) {
    return (
      <div
        style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100%' }}
      >
        Loading...
      </div>
    )
  }
  if (sensitivityResult.paramNames.length === 0) {
    return <EmptyState message="No parameters" />
  }

  const nObj = sensitivityResult.objectiveNames.length
  const importances = sensitivityResult.paramNames.map((name, pi) => {
    let score = 0
    if (metric === 'spearman') {
      score = sensitivityResult.spearman[pi].reduce((sum, v) => sum + Math.abs(v), 0) / nObj
    } else {
      score = sensitivityResult.ridge.reduce((sum, r) => sum + Math.abs(r.beta[pi]), 0) / nObj
    }
    return { name, score }
  })

  // ECharts yAxis is bottom-to-top so ascending sort = descending display
  importances.sort((a, b) => a.score - b.score)

  const metricLabel = metric === 'spearman' ? 'Spearman |ρ|' : 'Ridge |β|'

  const option = {
    title: { text: `Parameter Importance (${metricLabel})` },
    tooltip: {},
    xAxis: { type: 'value', min: 0 },
    yAxis: { type: 'category', data: importances.map((i) => i.name) },
    series: [{ type: 'bar', data: importances.map((i) => i.score) }],
  }

  return (
    <div style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column' }}>
      <div style={{ padding: '8px' }}>
        <select
          value={metric}
          onChange={(e) => setMetric(e.target.value as ImportanceMetric)}
          style={{ padding: '4px 8px', borderRadius: '4px' }}
        >
          <option value="spearman">Spearman |ρ|</option>
          <option value="beta">Ridge |β|</option>
        </select>
      </div>
      <ReactECharts option={option} style={{ flex: 1 }} />
    </div>
  )
}
