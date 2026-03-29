/**
 * SensitivityHeatmap — 感度分析ヒートマップ (TASK-802)
 *
 * 【役割】: Spearman 相関係数または Ridge β 係数をヒートマップで可視化する
 * 【設計方針】:
 *   - ECharts HeatMap で変数×目的の感度行列を表示
 *   - しきい値スライダーで |相関| < threshold の行をグレーアウト
 *   - 指標切り替え: spearman / beta
 *   - isLoading=true のとき「WASM計算中...」を表示
 * 🟢 REQ-096〜REQ-098 に準拠
 */

import { useState } from 'react'
import ReactECharts from 'echarts-for-react'

// -------------------------------------------------------------------------
// 型定義
// -------------------------------------------------------------------------

/**
 * 【感度データ型】: WASM/JS から受け取る感度分析結果
 */
export interface SensitivityData {
  /** パラメータ名リスト */
  paramNames: string[]
  /** 目的名リスト */
  objectiveNames: string[]
  /** Spearman 相関行列: spearman[param_idx][obj_idx] ∈ [-1.0, 1.0] */
  spearman: number[][]
  /** Ridge 回帰結果: ridge[obj_idx].beta[param_idx] */
  ridge: { beta: number[]; rSquared: number }[]
}

/** 【感度指標型】: 表示する感度の種類 */
export type SensitivityMetric = 'spearman' | 'beta'

// -------------------------------------------------------------------------
// 定数定義
// -------------------------------------------------------------------------

/** 【指標ラベル】: UI 表示用の指標名 */
const METRIC_LABELS: Record<SensitivityMetric, string> = {
  spearman: 'Spearman',
  beta: 'Ridge β',
}

// -------------------------------------------------------------------------
// Props 型定義
// -------------------------------------------------------------------------

/**
 * 【Props】: SensitivityHeatmap コンポーネントのプロパティ
 */
export interface SensitivityHeatmapProps {
  /** 🟢 感度分析データ — null のときは空状態 or ローディングを表示 */
  data: SensitivityData | null
  /** 🟢 表示する感度指標 */
  metric: SensitivityMetric
  /** 🟢 しきい値: |感度| がこの値未満の行をグレーアウト */
  threshold: number
  /** 🟢 WASM 計算中フラグ */
  isLoading?: boolean
  /** 🟢 しきい値変更コールバック */
  onThresholdChange?: (threshold: number) => void
  /** 🟢 セルクリックコールバック — Pareto 図カラーモード切り替え用 */
  onCellClick?: (paramName: string, objectiveName: string) => void
}

// -------------------------------------------------------------------------
// ECharts オプション生成
// -------------------------------------------------------------------------

/**
 * 【オプション生成】: 感度行列から ECharts HeatMap オプションを生成する
 * @param data - 感度分析データ
 * @param metric - 表示指標
 * @param threshold - しきい値
 * @returns ECharts HeatMap オプション
 */
function buildHeatmapOption(
  data: SensitivityData,
  metric: SensitivityMetric,
  threshold: number,
): object {
  const { paramNames, objectiveNames } = data

  // 【感度行列抽出】: 指標に応じて spearman または beta を取得する
  const matrix: number[][] =
    metric === 'spearman'
      ? data.spearman
      : // beta[obj_idx][param_idx] → [param_idx][obj_idx] に転置
        paramNames.map((_, pIdx) =>
          objectiveNames.map((_, oIdx) => data.ridge[oIdx]?.beta[pIdx] ?? 0),
        )

  // 【ヒートマップデータ変換】: [obj_idx, param_idx, value] 形式に変換
  const heatmapData: [number, number, number][] = []
  for (let pIdx = 0; pIdx < paramNames.length; pIdx++) {
    // 【しきい値フィルタ】: パラメータの最大|感度|がしきい値未満はグレーアウト値を使用
    const maxAbs = Math.max(...(matrix[pIdx] ?? [0]).map(Math.abs))
    const isFiltered = maxAbs < threshold

    for (let oIdx = 0; oIdx < objectiveNames.length; oIdx++) {
      const value = matrix[pIdx]?.[oIdx] ?? 0
      // 【フィルタ処理】: しきい値以下のパラメータは 0 として表示（無相関色）
      heatmapData.push([oIdx, pIdx, isFiltered ? 0 : value])
    }
  }

  return {
    tooltip: {
      trigger: 'item',
      formatter: (params: { data: [number, number, number] }) => {
        const [oIdx, pIdx, val] = params.data
        return `${paramNames[pIdx]} × ${objectiveNames[oIdx]}: ${val.toFixed(3)}`
      },
    },
    xAxis: {
      type: 'category',
      data: objectiveNames,
      splitArea: { show: true },
    },
    yAxis: {
      type: 'category',
      data: paramNames,
      splitArea: { show: true },
    },
    visualMap: {
      min: -1,
      max: 1,
      calculable: true,
      orient: 'horizontal',
      left: 'center',
      bottom: '0%',
      // 【色設定】: 青=負の相関・赤=正の相関・白=無相関 🟢
      inRange: {
        color: ['#2563eb', '#ffffff', '#dc2626'],
      },
    },
    series: [
      {
        name: METRIC_LABELS[metric],
        type: 'heatmap',
        data: heatmapData,
        label: {
          show: true,
          formatter: (params: { data: [number, number, number] }) => params.data[2].toFixed(2),
        },
        emphasis: {
          itemStyle: { shadowBlur: 10, shadowColor: 'rgba(0, 0, 0, 0.5)' },
        },
      },
    ],
  }
}

// -------------------------------------------------------------------------
// メインコンポーネント
// -------------------------------------------------------------------------

/**
 * 【機能概要】: 感度分析ヒートマップコンポーネント
 * 【指標切り替え】: Spearman 相関 / Ridge β 係数
 * 【しきい値フィルタ】: |感度| < threshold の行をグレーアウト
 * 【ローディング】: isLoading=true のとき「WASM計算中...」を表示
 * 🟢 REQ-096〜REQ-098 に準拠
 */
export function SensitivityHeatmap({
  data,
  metric,
  threshold,
  isLoading = false,
  onThresholdChange,
  onCellClick,
}: SensitivityHeatmapProps) {
  // 【内部メトリクス状態】: 外部 metric prop を初期値として持つ内部状態
  const [activeMetric, setActiveMetric] = useState<SensitivityMetric>(metric)

  // -------------------------------------------------------------------------
  // ローディング状態の表示
  // -------------------------------------------------------------------------

  if (isLoading) {
    return (
      <div
        data-testid="sensitivity-heatmap"
        style={{ padding: '24px', display: 'flex', alignItems: 'center', gap: '12px' }}
      >
        {/* 【ローディングインジケーター】: WASM 計算中のメッセージ 🟢 */}
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
        <span style={{ fontSize: '13px', color: '#6b7280' }}>Computing (WASM)...</span>
      </div>
    )
  }

  // -------------------------------------------------------------------------
  // 空状態の表示
  // -------------------------------------------------------------------------

  if (!data) {
    return (
      <div data-testid="sensitivity-heatmap" style={{ padding: '12px' }}>
        <span style={{ fontSize: '13px', color: '#6b7280' }}>Data not loaded</span>
      </div>
    )
  }

  // -------------------------------------------------------------------------
  // ECharts オプション生成
  // -------------------------------------------------------------------------

  const option = buildHeatmapOption(data, activeMetric, threshold)

  // -------------------------------------------------------------------------
  // レンダリング
  // -------------------------------------------------------------------------

  return (
    <div
      data-testid="sensitivity-heatmap"
      style={{ display: 'flex', flexDirection: 'column', height: '100%' }}
    >
      {/* 【コントロールバー】: 指標切り替え + しきい値スライダー */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '12px',
          padding: '8px',
          borderBottom: '1px solid #e5e7eb',
          flexShrink: 0,
        }}
      >
        {/* 【指標切り替えボタン群】: Spearman / Ridge β 🟢 */}
        <div style={{ display: 'flex', gap: '4px' }}>
          {(['spearman', 'beta'] as SensitivityMetric[]).map((m) => (
            <button
              key={m}
              data-testid={`metric-btn-${m}`}
              aria-pressed={activeMetric === m}
              onClick={() => setActiveMetric(m)}
              style={{
                padding: '4px 10px',
                fontSize: '12px',
                background: activeMetric === m ? '#4f46e5' : '#f3f4f6',
                color: activeMetric === m ? '#fff' : '#374151',
                border: '1px solid',
                borderColor: activeMetric === m ? '#4f46e5' : '#d1d5db',
                borderRadius: '4px',
                cursor: 'pointer',
              }}
            >
              {METRIC_LABELS[m]}
            </button>
          ))}
        </div>

        {/* 【しきい値スライダー】: 表示フィルタのしきい値を設定する 🟢 */}
        <label style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '12px' }}>
          <span style={{ color: '#6b7280' }}>Threshold:</span>
          <input
            data-testid="threshold-slider"
            type="range"
            min="0"
            max="1"
            step="0.05"
            value={threshold}
            onChange={(e) => onThresholdChange?.(parseFloat(e.target.value))}
            style={{ width: '80px' }}
          />
          <span style={{ color: '#374151', minWidth: '32px' }}>{threshold.toFixed(2)}</span>
        </label>
      </div>

      {/* 【ヒートマップエリア】: ECharts で感度行列を描画する 🟢 */}
      <div
        style={{ flex: 1 }}
        onClick={(e) => {
          // 【セルクリック処理】: ECharts のクリックイベントは onEvents で処理
          // ここでは DOM クリックイベントをパス (onCellClick は ECharts コールバック経由)
          void e
        }}
      >
        <ReactECharts
          option={option}
          style={{ height: '100%' }}
          onEvents={
            onCellClick
              ? {
                  click: (params: { data?: [number, number, number] }) => {
                    if (params.data) {
                      const [oIdx, pIdx] = params.data
                      const paramName = data.paramNames[pIdx]
                      const objectiveName = data.objectiveNames[oIdx]
                      if (paramName && objectiveName) {
                        onCellClick(paramName, objectiveName)
                      }
                    }
                  },
                }
              : undefined
          }
        />
      </div>
    </div>
  )
}
