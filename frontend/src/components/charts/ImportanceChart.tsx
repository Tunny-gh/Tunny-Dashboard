import { useEffect, useState } from 'react'
import ReactECharts from 'echarts-for-react'
import { useAnalysisStore } from '../../stores/analysisStore'
import { useStudyStore } from '../../stores/studyStore'
import { EmptyState } from '../common/EmptyState'

type ImportanceMetric = 'spearman' | 'beta' | 'sobol_first' | 'sobol_total'

export function ImportanceChart() {
  const [metric, setMetric] = useState<ImportanceMetric>('spearman')
  const currentStudy = useStudyStore((s) => s.currentStudy)

  const {
    sensitivityResult,
    isComputingSensitivity,
    sensitivityError,
    computeSensitivity,
    sobolResult,
    isComputingSobol,
    sobolError,
    computeSobol,
  } = useAnalysisStore()

  const isSobolMetric = metric === 'sobol_first' || metric === 'sobol_total'

  useEffect(() => {
    if (!currentStudy) return
    if (isSobolMetric) {
      if (!sobolResult && !isComputingSobol && !sobolError) {
        computeSobol()
      }
    } else {
      if (!sensitivityResult && !isComputingSensitivity && !sensitivityError) {
        computeSensitivity()
      }
    }
  }, [
    currentStudy,
    metric,
    isSobolMetric,
    sensitivityResult,
    isComputingSensitivity,
    sensitivityError,
    computeSensitivity,
    sobolResult,
    isComputingSobol,
    sobolError,
    computeSobol,
  ])

  if (!currentStudy) {
    return <EmptyState message="Please load data" />
  }

  const activeError = isSobolMetric ? sobolError : sensitivityError
  const activeLoading = isSobolMetric ? isComputingSobol : isComputingSensitivity
  const activeResult = isSobolMetric ? sobolResult : sensitivityResult

  if (activeError) {
    return <EmptyState message={activeError} />
  }
  if (activeLoading || !activeResult) {
    return (
      <div
        style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100%' }}
      >
        Loading...
      </div>
    )
  }
  if (activeResult.paramNames.length === 0) {
    return <EmptyState message="No parameters" />
  }

  const nObj = activeResult.objectiveNames.length
  const importances = activeResult.paramNames.map((name, pi) => {
    let score = 0
    if (metric === 'spearman' && sensitivityResult) {
      score = sensitivityResult.spearman[pi].reduce((sum, v) => sum + Math.abs(v), 0) / nObj
    } else if (metric === 'beta' && sensitivityResult) {
      score = sensitivityResult.ridge.reduce((sum, r) => sum + Math.abs(r.beta[pi]), 0) / nObj
    } else if (metric === 'sobol_first' && sobolResult) {
      score = sobolResult.firstOrder[pi].reduce((sum, v) => sum + v, 0) / nObj
    } else if (metric === 'sobol_total' && sobolResult) {
      score = sobolResult.totalEffect[pi].reduce((sum, v) => sum + v, 0) / nObj
    }
    return { name, score }
  })

  importances.sort((a, b) => a.score - b.score)

  const metricLabel =
    metric === 'spearman'
      ? 'Spearman |ρ|'
      : metric === 'beta'
        ? 'Ridge |β|'
        : metric === 'sobol_first'
          ? 'Sobol S_i (first-order)'
          : 'Sobol ST_i (total-effect)'

  const option = {
    title: { text: `Parameter Importance (${metricLabel})` },
    tooltip: {},
    xAxis: { type: 'value', min: 0, max: metric.startsWith('sobol') ? 1 : undefined },
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
          <option value="sobol_first">Sobol S_i (first-order)</option>
          <option value="sobol_total">Sobol ST_i (total-effect)</option>
        </select>
      </div>
      <ReactECharts option={option} style={{ flex: 1 }} />
    </div>
  )
}
