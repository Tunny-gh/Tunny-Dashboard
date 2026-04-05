import { useEffect } from 'react'
import ReactECharts from 'echarts-for-react'
import { useStudyStore } from '../../stores/studyStore'
import { useMcdmStore } from '../../stores/mcdmStore'
import { useSelectionStore } from '../../stores/selectionStore'
import { EmptyState } from '../common/EmptyState'

export function TopsisRankingChart() {
  const currentStudy = useStudyStore((s) => s.currentStudy)
  const {
    topsisResult,
    topsisWeights,
    isComputing,
    topsisError,
    topN,
    computeTopsis,
    setTopsisWeights,
    setTopN,
  } = useMcdmStore()
  const setHighlight = useSelectionStore((s) => s.setHighlight)

  useEffect(() => {
    if (!currentStudy || currentStudy.objectiveNames.length < 2) return
    if (!topsisResult && !isComputing && !topsisError) {
      computeTopsis()
    }
  }, [currentStudy, topsisResult, isComputing, topsisError, computeTopsis])

  if (!currentStudy) {
    return <EmptyState message="Please load data" />
  }

  if (currentStudy.objectiveNames.length < 2) {
    return <EmptyState message="At least 2 objectives required" />
  }

  if (topsisError) {
    return <EmptyState message={`Ranking computation error: ${topsisError}`} />
  }

  if (isComputing || !topsisResult) {
    return (
      <div
        style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100%' }}
      >
        Computing...
      </div>
    )
  }

  const objectiveNames = currentStudy.objectiveNames
  const weights =
    topsisWeights.length === objectiveNames.length
      ? topsisWeights
      : objectiveNames.map(() => 1 / objectiveNames.length)

  const displayN = Math.min(topN, topsisResult.rankedIndices.length)
  const topIndices = topsisResult.rankedIndices.slice(0, displayN)
  const topScores = topIndices.map((i) => topsisResult.scores[i])
  const topLabels = topIndices.map((i) => `Trial ${i}`)

  const chartOption = {
    tooltip: {
      trigger: 'axis',
      axisPointer: { type: 'shadow' },
      formatter: (params: { name: string; value: number }[]) => {
        const p = params[0]
        return `${p.name}: ${p.value.toFixed(4)}`
      },
    },
    xAxis: { type: 'value', min: 0, max: 1, name: 'TOPSIS Score' },
    yAxis: {
      type: 'category',
      data: topLabels,
      inverse: true,
    },
    series: [
      {
        type: 'bar',
        data: topScores,
        itemStyle: {
          color: (params: { dataIndex: number }) =>
            params.dataIndex === 0 ? '#e74c3c' : '#3498db',
        },
      },
    ],
    grid: { left: 80, right: 20, top: 10, bottom: 40 },
  }

  const handleBarClick = (params: { dataIndex: number }) => {
    const trialIndex = topIndices[params.dataIndex]
    setHighlight(trialIndex)
  }

  const updateWeight = (idx: number, value: number) => {
    const newWeights = [...weights]
    newWeights[idx] = value
    setTopsisWeights(newWeights)
    computeTopsis()
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', padding: '8px' }}>
      {/* Weight sliders */}
      <div style={{ marginBottom: '8px' }}>
        <div style={{ fontSize: '12px', fontWeight: 'bold', marginBottom: '4px' }}>Objective weights</div>
        {objectiveNames.map((name, i) => (
          <div
            key={i}
            style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '2px' }}
          >
            <label
              style={{
                minWidth: '80px',
                fontSize: '11px',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
              }}
            >
              {name}: {weights[i].toFixed(2)}
            </label>
            <input
              type="range"
              min={0}
              max={1}
              step={0.01}
              value={weights[i]}
              onChange={(e) => updateWeight(i, parseFloat(e.target.value))}
              style={{ flex: 1 }}
            />
          </div>
        ))}
      </div>

      {/* Top-N selector */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '8px',
          marginBottom: '8px',
          fontSize: '12px',
        }}
      >
        <label>Show:</label>
        <select
          value={topN}
          onChange={(e) => setTopN(parseInt(e.target.value))}
          style={{ fontSize: '12px' }}
        >
          <option value={5}>Top 5</option>
          <option value={10}>Top 10</option>
          <option value={20}>Top 20</option>
        </select>
      </div>

      {/* ECharts bar chart */}
      <div style={{ flex: 1, minHeight: 0 }}>
        <ReactECharts
          option={chartOption}
          style={{ height: '100%', minHeight: '200px' }}
          onEvents={{ click: handleBarClick }}
        />
      </div>
    </div>
  )
}
