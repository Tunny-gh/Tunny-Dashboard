/**
 * ParallelCoordinates — ECharts parallel座標 30軸コンポーネント (TASK-601)
 *
 * 【役割】: 全変数（最大30軸）＋全目的（最大4軸）を平行座標で表示
 * 【設計方針】: ECharts `parallel` チャートで軸ブラシ → selectionStore.addAxisFilter() を連携
 * 🟢 Brushing & Linking: axisareaselected イベント → addAxisFilter → WASM → GPUバッファ更新
 */

import ReactECharts from 'echarts-for-react'
import { useSelectionStore } from '../../stores/selectionStore'
import type { GpuBuffer } from '../../wasm/gpuBuffer'
import type { Study } from '../../types'
import { EmptyState } from '../common/EmptyState'

// -------------------------------------------------------------------------
// Props 型定義
// -------------------------------------------------------------------------

export interface ParallelCoordinatesProps {
  /** 🟢 GPU バッファ — null のとき空状態UIを表示 */
  gpuBuffer: GpuBuffer | null
  /** 🟢 現在の Study — 軸名（paramNames + objectiveNames）取得用 */
  currentStudy: Study | null
}

// -------------------------------------------------------------------------
// axisareaselected イベントの型定義
// 【設計】: ECharts v5 parallel座標の axisareaselected イベントデータ形式
// -------------------------------------------------------------------------

interface AxisAreaSelectedEvent {
  axesInfo: Array<{
    axisIndex: number
    /** [[min, max], ...] — 空配列のときフィルタ解除 */
    intervals: number[][]
  }>
}

// -------------------------------------------------------------------------
// コンポーネント実装
// -------------------------------------------------------------------------

/**
 * 【機能概要】: ECharts parallel座標で30変数＋4目的を表示し、軸ブラシで addAxisFilter を呼ぶ
 * 【テスト対応】: TC-601-01〜04, TC-601-E01〜E02, TC-601-B01
 */
export function ParallelCoordinates({ gpuBuffer, currentStudy }: ParallelCoordinatesProps) {
  // 【Store接続】: addAxisFilter / removeAxisFilter を取得 🟢
  const { addAxisFilter, removeAxisFilter } = useSelectionStore()

  // 【空状態UI】: データがない場合はメッセージを表示 🟢
  if (!gpuBuffer || !currentStudy) {
    return <EmptyState message="Data not loaded" />
  }

  // 【軸名配列構築】: paramNames + objectiveNames で全軸名を定義 🟢
  const axisNames = [...currentStudy.paramNames, ...currentStudy.objectiveNames]

  // 【parallelAxis 定義】: ECharts に軸名と軸インデックスを渡す 🟢
  const parallelAxis = axisNames.map((name, dim) => ({
    dim,
    name,
    type: 'value' as const,
  }))

  // 【series データ構築】: GpuBuffer から各トライアルのパラメータ値を取得 🟢
  // ※ GpuBuffer には positions しかないので N点分のプレースホルダーデータを使う
  // （実際のパラメータ値は WASM の get_column() で取得する予定 — TASK-102 完成後）
  const seriesData: number[][] = Array.from({ length: gpuBuffer.trialCount }, (_, i) => {
    // 【プレースホルダー】: positions配列から x,y 座標を繰り返し埋める
    const row = new Array(axisNames.length).fill(0)
    if (gpuBuffer.positions.length >= (i + 1) * 2) {
      row[0] = gpuBuffer.positions[i * 2]
      row[1] = gpuBuffer.positions[i * 2 + 1]
    }
    return row
  })

  // 【ECharts option 構築】: parallel チャートの設定を組み立てる 🟢
  const option = {
    parallel: {
      left: '5%',
      right: '5%',
      top: '10%',
      bottom: '10%',
    },
    parallelAxis,
    series: [
      {
        type: 'parallel',
        data: seriesData,
        lineStyle: { width: 1, opacity: 0.3 },
      },
    ],
  }

  /**
   * 【axisareaselected イベントハンドラ】: 軸ブラシの範囲を addAxisFilter に連携する
   * 【Brushing実装】: intervals が空のときはフィルタを解除する
   * 🟢 REQ-041 対応: 軸ブラシ → addAxisFilter → WASM filterByRanges
   */
  const handleAxisAreaSelected = (params: unknown) => {
    const event = params as AxisAreaSelectedEvent
    if (!event?.axesInfo) return

    event.axesInfo.forEach(({ axisIndex, intervals }) => {
      const axisName = axisNames[axisIndex]
      if (!axisName) return

      if (intervals.length === 0) {
        // 【フィルタ解除】: ブラシが削除された場合は removeAxisFilter を呼ぶ
        removeAxisFilter(axisName)
      } else {
        // 【フィルタ適用】: ブラシ範囲の min/max を addAxisFilter に渡す
        const [min, max] = intervals[0] // 単一区間（最初の interval を使用）
        addAxisFilter(axisName, min, max)
      }
    })
  }

  // 【ECharts レンダリング】: parallel チャートを描画する 🟢
  return (
    <ReactECharts
      option={option}
      style={{ width: '100%', height: '100%' }}
      onEvents={{
        // 【イベント登録】: 軸ブラシのイベントを登録する
        axisareaselected: handleAxisAreaSelected,
      }}
    />
  )
}
