/**
 * PDPChart — Partial Dependence Plot (PDP) component (TASK-804)
 *
 * Visualizes a Ridge-based simplified PDP using ECharts.
 *
 * Design:
 *   - ECharts Line shows the PDP curve (thick line)
 *   - ICE lines (semi-transparent gray) overlaid to show individual variation
 *   - Brushing integration: ICE lines in highlightedIndices are highlighted in orange
 *   - Shows "Displaying linear approximation" warning banner when useOnnx=false
 *   - Shows "Caution interpreting PDP" warning badge when R² < 0.8
 *   - Two-variable PDP shown as an interaction heatmap when data2d is provided
 *
 * Conforms to REQ-103–REQ-106.
 */

import ReactECharts from 'echarts-for-react';

// -------------------------------------------------------------------------
// Types
// -------------------------------------------------------------------------

/**
 * 1D PDP data: result of WASM compute_pdp()
 */
export interface PdpData1d {
  /** Target parameter name */
  paramName: string;
  /** Target objective name */
  objectiveName: string;
  /** Parameter values at grid points (n_grid points) */
  grid: number[];
  /** PDP values at each grid point (n_grid points) */
  values: number[];
  /** Ridge model coefficient of determination R² ∈ [0, 1] */
  rSquared: number;
  /**
   * ICE lines: conditional expectation curves per sample (optional)
   * iceLines[sample_idx][grid_idx]
   */
  iceLines?: number[][];
}

/**
 * 2D PDP data: result of WASM compute_pdp_2d() for interaction PDP
 */
export interface PdpData2d {
  /** First parameter name */
  param1Name: string;
  /** Second parameter name */
  param2Name: string;
  /** Target objective name */
  objectiveName: string;
  /** Grid points for the first parameter */
  grid1: number[];
  /** Grid points for the second parameter */
  grid2: number[];
  /** PDP value matrix: values[i][j] = f̄(grid1[i], grid2[j]) */
  values: number[][];
  /** Ridge model coefficient of determination R² */
  rSquared: number;
}

// -------------------------------------------------------------------------
// Constants
// -------------------------------------------------------------------------

/** R² threshold above which quality is considered "good" */
const R2_GOOD_THRESHOLD = 0.8;

/** R² threshold above which quality is considered "caution" (below R2_GOOD) */
const R2_WARN_THRESHOLD = 0.5;

/** Color for the PDP curve */
const PDP_LINE_COLOR = '#4f46e5';

/** Color for normal ICE lines */
const ICE_NORMAL_COLOR = 'rgba(107, 114, 128, 0.3)';

/** Color for highlighted ICE lines */
const ICE_HIGHLIGHT_COLOR = '#f59e0b';

// -------------------------------------------------------------------------
// Model quality assessment
// -------------------------------------------------------------------------

/** Model quality label based on R² */
export type ModelQuality = '良好' | '要注意' | '推奨外';

/**
 * Returns a quality label based on R²:
 * R² >= 0.8 → good / R² >= 0.5 → caution / R² < 0.5 → not recommended
 */
export function getModelQuality(rSquared: number): ModelQuality {
  if (rSquared >= R2_GOOD_THRESHOLD) return '良好';
  if (rSquared >= R2_WARN_THRESHOLD) return '要注意';
  return '推奨外';
}

// -------------------------------------------------------------------------
// ECharts option builders
// -------------------------------------------------------------------------

/**
 * Build ECharts option for 1D PDP: PDP curve + ICE lines.
 *
 * ICE line styling:
 *   - Normal: semi-transparent gray (opacity=0.3)
 *   - Highlighted: thick orange line (Brushing integration)
 *
 * Conforms to REQ-104: highlight ICE lines for selected samples after Brushing.
 */
function buildPdpOption(data: PdpData1d, highlightedSet: Set<number>): object {
  const iceLines = data.iceLines ?? [];

  // ICE line series: draw each sample's conditional expectation as a thin line
  const iceSeries = iceLines.map((iceLine, idx) => ({
    type: 'line',
    name: `ICE-${idx}`,
    data: data.grid.map((x, i) => [x, iceLine[i]]),
    lineStyle: {
      width: highlightedSet.has(idx) ? 2 : 0.5,
      color: highlightedSet.has(idx) ? ICE_HIGHLIGHT_COLOR : ICE_NORMAL_COLOR,
    },
    symbolSize: 0,
    // Hide from legend; ICE lines don't need individual entries
    showInLegend: false,
  }));

  // PDP curve series: thick purple line drawn on top of ICE lines
  const pdpSeries = {
    type: 'line',
    name: 'PDP',
    data: data.grid.map((x, i) => [x, data.values[i]]),
    lineStyle: { width: 3, color: PDP_LINE_COLOR },
    symbolSize: 0,
    z: 10, // Render in front of ICE lines
  };

  return {
    tooltip: {
      trigger: 'axis',
      formatter: (params: { seriesName: string; data: [number, number] }[]) => {
        const pdp = params.find((p) => p.seriesName === 'PDP');
        if (!pdp) return '';
        return `${data.paramName}: ${pdp.data[0].toFixed(3)}<br/>PDP: ${pdp.data[1].toFixed(4)}`;
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
    // Show only the PDP entry in the legend
    legend: { data: ['PDP'] },
  };
}

/**
 * Build ECharts option for 2D PDP interaction heatmap.
 * Conforms to REQ-106: 2-variable PDP interaction heatmap.
 */
function buildPdp2dOption(data: PdpData2d): object {
  // Convert to [grid1_idx, grid2_idx, value] format for ECharts heatmap
  const heatmapData: [number, number, number][] = [];
  for (let i = 0; i < data.grid1.length; i++) {
    for (let j = 0; j < data.grid2.length; j++) {
      heatmapData.push([i, j, data.values[i]?.[j] ?? 0]);
    }
  }

  const allValues = heatmapData.map((d) => d[2]);
  const minVal = Math.min(...allValues);
  const maxVal = Math.max(...allValues);

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
      // Blue = low, red = high
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
  };
}

// -------------------------------------------------------------------------
// Props types
// -------------------------------------------------------------------------

export interface PDPChartProps {
  /** 1D PDP data — shows empty state or loading when null */
  data1d: PdpData1d | null;
  /** 2D PDP data (optional) */
  data2d?: PdpData2d | null;
  /** PDP computation in progress */
  isLoading?: boolean;
  /** Whether ONNX high-precision mode is enabled */
  useOnnx?: boolean;
  /**
   * ICE line indices to highlight (Brushing integration).
   * ICE lines at these indices are highlighted in orange.
   */
  highlightedIndices?: number[];
  /** Callback to request ONNX model loading */
  onOnnxRequest?: () => void;
}

// -------------------------------------------------------------------------
// Main component
// -------------------------------------------------------------------------

/**
 * Partial Dependence Plot (PDP) component.
 * - PDP curve: Ridge-based simplified PDP shown as a thick line (switches to high-precision after .onnx load)
 * - ICE lines: individual variation shown as thin overlaid lines; Brushing-linked highlighting
 * - Quality display: R² and assessment (good/caution/not recommended)
 *
 * Conforms to REQ-103–REQ-106.
 */
export function PDPChart({
  data1d,
  data2d,
  isLoading = false,
  useOnnx = false,
  highlightedIndices = [],
  onOnnxRequest,
}: PDPChartProps) {
  // -------------------------------------------------------------------------
  // Loading state
  // -------------------------------------------------------------------------

  if (isLoading) {
    return (
      <div
        data-testid="pdp-chart"
        style={{ padding: '24px', display: 'flex', alignItems: 'center', gap: '12px' }}
      >
        {/* Loading spinner while PDP is being computed */}
        <div
          style={{
            width: '16px',
            height: '16px',
            borderRadius: '50%',
            border: '2px solid #4f46e5',
            borderTopColor: 'transparent',
            animation: 'spin 1s linear infinite',
          }}
        />
        <span style={{ fontSize: '13px', color: '#6b7280' }}>PDP計算中...</span>
      </div>
    );
  }

  // -------------------------------------------------------------------------
  // Empty state
  // -------------------------------------------------------------------------

  if (!data1d && !data2d) {
    return (
      <div data-testid="pdp-chart" style={{ padding: '12px' }}>
        <span style={{ fontSize: '13px', color: '#6b7280' }}>データが読み込まれていません</span>
      </div>
    );
  }

  // -------------------------------------------------------------------------
  // R² and quality assessment
  // -------------------------------------------------------------------------

  const rSquared = data1d?.rSquared ?? data2d?.rSquared ?? 0;
  const quality = getModelQuality(rSquared);
  const qualityLabel =
    quality === '良好' ? '✓良好' : quality === '要注意' ? '△要注意' : '✕推奨外';
  const qualityColor =
    quality === '良好' ? '#16a34a' : quality === '要注意' ? '#d97706' : '#dc2626';

  // -------------------------------------------------------------------------
  // Highlighted index set
  // -------------------------------------------------------------------------

  const highlightedSet = new Set(highlightedIndices);

  // -------------------------------------------------------------------------
  // Render
  // -------------------------------------------------------------------------

  return (
    <div
      data-testid="pdp-chart"
      style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}
    >
      {/* Linear approximation warning banner: shown when ONNX is not in use (REQ-105) */}
      {!useOnnx && (
        <div
          data-testid="linear-approx-banner"
          style={{
            padding: '8px 12px',
            background: '#fef3c7',
            border: '1px solid #f59e0b',
            borderRadius: '4px',
            fontSize: '12px',
            color: '#92400e',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
          }}
        >
          <span>
            線形近似で表示中 / より精度の高いPDPには .onnx ファイルを読み込んでください
          </span>
          {/* Button to request ONNX model loading */}
          {onOnnxRequest && (
            <button
              data-testid="onnx-request-btn"
              onClick={onOnnxRequest}
              style={{
                padding: '2px 8px',
                fontSize: '11px',
                background: '#f59e0b',
                color: '#fff',
                border: 'none',
                borderRadius: '3px',
                cursor: 'pointer',
              }}
            >
              .onnx を読み込む
            </button>
          )}
        </div>
      )}

      {/* R² warning badge: shown when R² < 0.8 (REQ-103) */}
      {rSquared < R2_GOOD_THRESHOLD && (
        <div
          data-testid="r2-warning-badge"
          style={{
            padding: '4px 10px',
            background: '#fef2f2',
            border: '1px solid #fca5a5',
            borderRadius: '4px',
            fontSize: '12px',
            color: '#dc2626',
          }}
        >
          PDPの解釈に注意が必要です（R²={rSquared.toFixed(2)}）
        </div>
      )}

      {/* Model quality panel: shows R² and assessment (REQ-103) */}
      <div
        data-testid="model-quality-panel"
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '12px',
          padding: '6px 10px',
          background: '#f9fafb',
          border: '1px solid #e5e7eb',
          borderRadius: '4px',
          fontSize: '12px',
        }}
      >
        <span style={{ color: '#6b7280' }}>R²:</span>
        <span data-testid="r2-value" style={{ fontWeight: 600 }}>
          {rSquared.toFixed(3)}
        </span>
        <span data-testid="quality-label" style={{ color: qualityColor, fontWeight: 600 }}>
          {qualityLabel}
        </span>
      </div>

      {/* 1D PDP chart: ECharts with PDP curve and ICE lines */}
      {data1d && !data2d && (
        <ReactECharts
          option={buildPdpOption(data1d, highlightedSet)}
          style={{ height: '300px' }}
        />
      )}

      {/* 2D PDP heatmap: ECharts interaction heatmap */}
      {data2d && (
        <ReactECharts
          option={buildPdp2dOption(data2d)}
          style={{ height: '300px' }}
        />
      )}
    </div>
  );
}
