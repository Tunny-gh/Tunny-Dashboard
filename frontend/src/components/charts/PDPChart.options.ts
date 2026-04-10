import type { PdpData1d, PdpData2d } from './PDPChart.types'

const PDP_LINE_COLOR = '#4f46e5'
const ICE_NORMAL_COLOR = 'rgba(107, 114, 128, 0.3)'
const ICE_HIGHLIGHT_COLOR = '#f59e0b'

export function buildPdpOption(data: PdpData1d, highlightedSet: Set<number>): object {
  const iceLines = data.iceLines ?? []

  const iceSeries = iceLines.map((iceLine, idx) => ({
    type: 'line',
    name: `ICE-${idx}`,
    data: data.grid.map((x, i) => [x, iceLine[i]]),
    lineStyle: {
      width: highlightedSet.has(idx) ? 2 : 0.5,
      color: highlightedSet.has(idx) ? ICE_HIGHLIGHT_COLOR : ICE_NORMAL_COLOR,
    },
    symbolSize: 0,
    showInLegend: false,
  }))

  const pdpSeries = {
    type: 'line',
    name: 'PDP',
    data: data.grid.map((x, i) => [x, data.values[i]]),
    lineStyle: { width: 3, color: PDP_LINE_COLOR },
    symbolSize: 0,
    z: 10,
  }

  return {
    tooltip: {
      trigger: 'axis',
      formatter: (params: { seriesName: string; data: [number, number] }[]) => {
        const pdp = params.find((p) => p.seriesName === 'PDP')
        if (!pdp) return ''
        return `${data.paramName}: ${pdp.data[0].toFixed(3)}<br/>PDP: ${pdp.data[1].toFixed(4)}`
      },
    },
    xAxis: {
      type: 'value',
      name: data.paramName,
      nameLocation: 'middle',
      nameGap: 25,
    },
    yAxis: {
      type: 'value',
      name: data.objectiveName,
    },
    series: [...iceSeries, pdpSeries],
    legend: { data: ['PDP'] },
  }
}

export function buildPdp2dOption(data: PdpData2d): object {
  const heatmapData: [number, number, number][] = []
  for (let i = 0; i < data.grid1.length; i++) {
    for (let j = 0; j < data.grid2.length; j++) {
      heatmapData.push([i, j, data.values[i]?.[j] ?? 0])
    }
  }

  const allValues = heatmapData.map((d) => d[2])
  const minVal = Math.min(...allValues)
  const maxVal = Math.max(...allValues)

  return {
    xAxis: {
      type: 'category',
      data: data.grid1.map((v) => v.toFixed(2)),
      name: data.param1Name,
    },
    yAxis: {
      type: 'category',
      data: data.grid2.map((v) => v.toFixed(2)),
      name: data.param2Name,
    },
    visualMap: {
      min: minVal,
      max: maxVal,
      calculable: true,
      orient: 'horizontal',
      left: 'center',
      bottom: '0%',
      inRange: { color: ['#2563eb', '#ffffff', '#dc2626'] },
    },
    series: [
      {
        name: `PDP (${data.param1Name} × ${data.param2Name})`,
        type: 'heatmap',
        data: heatmapData,
        label: { show: false },
      },
    ],
  }
}
