/**
 * SlicePlot — パラメータ vs 目的関数値 散布図 (Optuna-Dashboard Slice Plot 相当)
 *
 * 【役割】: 各パラメータと目的関数値の1対1関係を散布図で可視化する
 * 【設計方針】:
 *   - 数値パラメータのみ対象（typeof === 'number' でフィルタ）
 *   - パラメータ選択ドロップダウン + ECharts scatter
 *   - 多目的の場合は目的関数選択ドロップダウンも表示
 *   - 色: 試行番号（早期=青, 後期=赤）で傾向を視覚化
 * 🟢 optuna-dashboard の plot_slice と同等機能（Python 不要）
 */

import { useState } from 'react'
import ReactECharts from 'echarts-for-react'
import { EmptyState } from '../common/EmptyState'

// -------------------------------------------------------------------------
// 型定義
// -------------------------------------------------------------------------

/** 【試行データ型】: SlicePlot が受け取る1試行分のデータ */
export interface SliceTrial {
  /** 試行ID */
  trialId: number
  /** パラメータ値マップ（数値・文字列混在可） */
  params: Record<string, number | string>
  /** 目的関数値リスト（null = 未完了試行） */
  values: number[] | null
  /** Pareto ランク（色分け用） */
  paretoRank: number | null
}

/** 【Props 型】 */
export interface SlicePlotProps {
  /** 表示する試行一覧 */
  trials: SliceTrial[]
  /** パラメータ名リスト */
  paramNames: string[]
  /** 目的関数名リスト */
  objectiveNames: string[]
  /** 初期表示する目的関数インデックス（デフォルト 0） */
  objectiveIndex?: number
}

// -------------------------------------------------------------------------
// コンポーネント実装
// -------------------------------------------------------------------------

/**
 * 【機能概要】: パラメータと目的関数値の1対1散布図
 * 【データフロー】: trials → 数値パラメータフィルタ → ECharts scatter series
 * 【インタラクション】: パラメータ選択・目的関数選択でリアルタイム更新
 */
export function SlicePlot({
  trials,
  paramNames,
  objectiveNames,
  objectiveIndex: initialObjIdx = 0,
}: SlicePlotProps) {
  // 【状態管理】: 選択中パラメータインデックス・目的関数インデックス
  const [paramIndex, setParamIndex] = useState(0)
  const [objIndex, setObjIndex] = useState(initialObjIdx)

  // 【空状態チェック】: データまたはパラメータがない場合はプレースホルダー表示
  if (trials.length === 0 || paramNames.length === 0) {
    return <EmptyState />
  }

  // 【選択パラメータ・目的関数名】
  const selectedParam = paramNames[paramIndex] ?? paramNames[0]
  const selectedObj = objectiveNames[objIndex] ?? objectiveNames[0]

  // 【散布データ生成】: 数値パラメータのみ対象にフィルタして [param_val, obj_val, trial_idx] を生成
  const scatterData = trials
    .filter(
      (t) =>
        t.values !== null &&
        t.values[objIndex] != null &&
        typeof t.params[selectedParam] === 'number',
    )
    .map((t, i) => [t.params[selectedParam] as number, t.values![objIndex], i])

  // 【ECharts オプション構築】: scatter + visualMap (試行番号で色分け)
  const option = {
    tooltip: {
      trigger: 'item',
      formatter: (params: { value: [number, number, number] }) =>
        `${selectedParam}: ${params.value[0]}<br/>${selectedObj}: ${params.value[1]}`,
    },
    xAxis: {
      type: 'value',
      name: selectedParam,
      nameLocation: 'center',
      nameGap: 24,
    },
    yAxis: {
      type: 'value',
      name: selectedObj,
      nameLocation: 'center',
      nameGap: 40,
    },
    // 【色分け】: 試行番号が小さい（早期）ほど青、大きい（後期）ほど赤
    visualMap: {
      min: 0,
      max: Math.max(scatterData.length - 1, 1),
      dimension: 2,
      inRange: { color: ['#5470c6', '#91cc75', '#fac858', '#ee6666'] },
      show: false,
    },
    series: [
      {
        type: 'scatter',
        data: scatterData,
        symbolSize: 8,
      },
    ],
    grid: { containLabel: true },
  }

  return (
    <div
      data-testid="slice-plot"
      style={{ height: '100%', display: 'flex', flexDirection: 'column' }}
    >
      {/* 【コントロールバー】: パラメータ・目的関数選択ドロップダウン */}
      <div
        style={{
          display: 'flex',
          gap: 8,
          padding: '4px 8px',
          flexShrink: 0,
          fontSize: 12,
          alignItems: 'center',
        }}
      >
        <label>
          パラメータ:{' '}
          <select
            data-testid="slice-param-select"
            value={paramIndex}
            onChange={(e) => setParamIndex(Number(e.target.value))}
            style={{ fontSize: 12 }}
          >
            {paramNames.map((p, i) => (
              <option key={p} value={i}>
                {p}
              </option>
            ))}
          </select>
        </label>

        {/* 【多目的専用】: 目的関数が 2 つ以上のとき表示 */}
        {objectiveNames.length > 1 && (
          <label>
            目的関数:{' '}
            <select
              data-testid="slice-obj-select"
              value={objIndex}
              onChange={(e) => setObjIndex(Number(e.target.value))}
              style={{ fontSize: 12 }}
            >
              {objectiveNames.map((o, i) => (
                <option key={o} value={i}>
                  {o}
                </option>
              ))}
            </select>
          </label>
        )}
      </div>

      {/* 【チャート本体】: ECharts scatter */}
      <ReactECharts option={option} style={{ flex: 1 }} />
    </div>
  )
}

export default SlicePlot
