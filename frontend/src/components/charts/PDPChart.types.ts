export interface PdpData1d {
  paramName: string
  objectiveName: string
  grid: number[]
  values: number[]
  rSquared: number
  iceLines?: number[][]
}

export interface PdpData2d {
  param1Name: string
  param2Name: string
  objectiveName: string
  grid1: number[]
  grid2: number[]
  values: number[][]
  rSquared: number
}

export interface PDPChartProps {
  data1d: PdpData1d | null
  data2d?: PdpData2d | null
  isLoading?: boolean
  useOnnx?: boolean
  highlightedIndices?: number[]
  onOnnxRequest?: () => void
}
