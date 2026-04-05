import { useEffect, useState } from 'react'
import ReactECharts from 'echarts-for-react'
import { useStudyStore } from '../../stores/studyStore'
import { useAnalysisStore } from '../../stores/analysisStore'
import { EmptyState } from '../common/EmptyState'
import type { SurrogateModelType } from '../../types'

// Surrogate model options (kriging is not yet implemented)
const MODEL_OPTIONS: { value: SurrogateModelType; label: string; disabled?: boolean }[] = [
  { value: 'ridge', label: 'Ridge Regression' },
  { value: 'random_forest', label: 'Random Forest' },
  { value: 'kriging', label: 'Kriging (coming soon)', disabled: true },
]

export function SurfacePlot3D() {
  const currentStudy = useStudyStore((s) => s.currentStudy)
  const {
    surrogateModelType,
    surface3dCache,
    isComputingSurface,
    surface3dError,
    setSurrogateModelType,
    computeSurface3d,
  } = useAnalysisStore()

  const paramNames = currentStudy?.paramNames ?? []
  const objectiveNames = currentStudy?.objectiveNames ?? []

  const [param1, setParam1] = useState<string>('')
  const [param2, setParam2] = useState<string>('')
  const [objective, setObjective] = useState<string>('')

  // Initialize selections when study changes
  useEffect(() => {
    if (paramNames.length >= 2) {
      setParam1((prev) => (prev && paramNames.includes(prev) ? prev : paramNames[0]))
      setParam2((prev) => {
        const fallback = paramNames.find((p) => p !== paramNames[0]) ?? paramNames[1]
        return prev && paramNames.includes(prev) && prev !== param1 ? prev : fallback
      })
    }
    if (objectiveNames.length >= 1) {
      setObjective((prev) => (prev && objectiveNames.includes(prev) ? prev : objectiveNames[0]))
    }
  }, [currentStudy]) // eslint-disable-line react-hooks/exhaustive-deps

  // Trigger computation when selections change
  useEffect(() => {
    if (!currentStudy || !param1 || !param2 || !objective || param1 === param2) return
    computeSurface3d(param1, param2, objective, 50)
  }, [currentStudy, param1, param2, objective, surrogateModelType, computeSurface3d])

  // Guard: no study loaded
  if (!currentStudy) {
    return <EmptyState message="Please load data" />
  }

  if (paramNames.length < 2) {
    return <EmptyState message="At least 2 parameters required" />
  }

  if (isComputingSurface) {
    return <EmptyState message="Computing 3D surface..." />
  }

  if (surface3dError) {
    return <EmptyState message={`Surface computation error: ${surface3dError}`} />
  }

  // Look up cached result
  const cacheKey = `${surrogateModelType}_${param1}_${param2}_${objective}_50`
  const result = surface3dCache.get(cacheKey)

  // Build ECharts heatmap data: [[x_index, y_index, value], ...]
  // values[i][j] = f(grid1[i], grid2[j])
  let chartOption: object = {}
  if (result && result.grid1.length > 0 && result.grid2.length > 0) {
    const flatData: [number, number, number][] = []
    let minVal = Infinity
    let maxVal = -Infinity

    for (let i = 0; i < result.grid1.length; i++) {
      for (let j = 0; j < result.grid2.length; j++) {
        const v = result.values[i][j]
        if (isFinite(v)) {
          flatData.push([i, j, v])
          if (v < minVal) minVal = v
          if (v > maxVal) maxVal = v
        }
      }
    }

    const fmt = (v: number) => (Math.abs(v) >= 1000 || Math.abs(v) < 0.01 ? v.toExponential(2) : v.toFixed(3))
    const xLabels = result.grid1.map(fmt)
    const yLabels = result.grid2.map(fmt)

    chartOption = {
      tooltip: {
        position: 'top',
        formatter: (p: { value: [number, number, number] }) =>
          `${param1}: ${xLabels[p.value[0]]}<br/>${param2}: ${yLabels[p.value[1]]}<br/>${objective}: ${fmt(p.value[2])}`,
      },
      grid: { top: 40, right: 100, bottom: 60, left: 80 },
      xAxis: {
        type: 'category',
        data: xLabels,
        name: param1,
        nameLocation: 'middle',
        nameGap: 35,
        axisLabel: {
          interval: Math.floor(result.grid1.length / 6),
          rotate: 30,
          fontSize: 10,
        },
      },
      yAxis: {
        type: 'category',
        data: yLabels,
        name: param2,
        nameLocation: 'middle',
        nameGap: 55,
        axisLabel: {
          interval: Math.floor(result.grid2.length / 6),
          fontSize: 10,
        },
      },
      visualMap: {
        min: minVal,
        max: maxVal,
        calculable: true,
        orient: 'vertical',
        right: 5,
        top: 'center',
        inRange: { color: ['#313695', '#4575b4', '#74add1', '#abd9e9', '#fee090', '#fdae61', '#f46d43', '#d73027', '#a50026'] },
        textStyle: { fontSize: 10 },
        formatter: (v: number) => fmt(v),
      },
      series: [
        {
          name: objective,
          type: 'heatmap',
          data: flatData,
          emphasis: { itemStyle: { shadowBlur: 6 } },
          label: { show: false },
        },
      ],
    }
  }

  return (
    <div
      data-testid="surface-plot-3d"
      style={{ display: 'flex', flexDirection: 'column', height: '100%', padding: '8px' }}
    >
      {/* Axis and model selection controls */}
      <div
        style={{
          display: 'flex',
          flexWrap: 'wrap',
          gap: '8px',
          marginBottom: '8px',
          fontSize: '12px',
        }}
      >
        <label style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
          X Axis:
          <select
            value={param1}
            onChange={(e) => setParam1(e.target.value)}
            style={{ padding: '2px 4px', borderRadius: '4px', border: '1px solid #ccc' }}
          >
            {paramNames.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
        </label>

        <label style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
          Y Axis:
          <select
            value={param2}
            onChange={(e) => setParam2(e.target.value)}
            style={{ padding: '2px 4px', borderRadius: '4px', border: '1px solid #ccc' }}
          >
            {paramNames.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
        </label>

        <label style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
          Objective:
          <select
            value={objective}
            onChange={(e) => setObjective(e.target.value)}
            style={{ padding: '2px 4px', borderRadius: '4px', border: '1px solid #ccc' }}
          >
            {objectiveNames.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
        </label>

        <label style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
          Model:
          <select
            value={surrogateModelType}
            onChange={(e) => setSurrogateModelType(e.target.value as SurrogateModelType)}
            style={{ padding: '2px 4px', borderRadius: '4px', border: '1px solid #ccc' }}
          >
            {MODEL_OPTIONS.map(({ value, label, disabled }) => (
              <option key={value} value={value} disabled={disabled}>
                {label}
              </option>
            ))}
          </select>
        </label>

        {result && (
          <span style={{ color: result.rSquared >= 0.5 ? '#27ae60' : '#e67e22', fontSize: '11px', alignSelf: 'center' }}>
            R²={result.rSquared.toFixed(3)}
          </span>
        )}
      </div>

      {/* ECharts heatmap */}
      <div style={{ flex: 1, minHeight: 0 }}>
        {result ? (
          <ReactECharts
            option={chartOption}
            style={{ height: '100%', minHeight: '200px' }}
          />
        ) : (
          <EmptyState message="Select parameters to compute" />
        )}
      </div>
    </div>
  )
}
