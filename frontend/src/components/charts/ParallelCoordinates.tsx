/**
 * ParallelCoordinates — ECharts parallel座標 30軸コンポーネント (TASK-601)
 *
 * 【役割】: 全変数（最大30軸）＋全目的（最大4軸）を平行座標で表示
 * 【設計方針】: ECharts `parallel` チャートで軸ブラシ → selectionStore.addAxisFilter() を連携
 * 🟢 Brushing & Linking: axisareaselected イベント → addAxisFilter → WASM → GPUバッファ更新
 */

import { useMemo } from 'react'
import ReactECharts from 'echarts-for-react'
import { useSelectionStore } from '../../stores/selectionStore'
import { useStudyStore } from '../../stores/studyStore'
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
  const addAxisFilter = useSelectionStore((s) => s.addAxisFilter)
  const removeAxisFilter = useSelectionStore((s) => s.removeAxisFilter)
  const selectedIndices = useSelectionStore((s) => s.selectedIndices)
  const trialRows = useStudyStore((s) => s.trialRows)

  // 【空状態UI】: データがない場合はメッセージを表示 🟢
  if (!gpuBuffer || !currentStudy) {
    return <EmptyState message="Data not loaded" />
  }

  // 【ECharts option 構築】: memoize heavy computation
  const { option, axisNames: memoAxisNames } = useMemo(() => {
    // 【軸名配列構築】: paramNames + objectiveNames で全軸名を定義 🟢
    const _axisNames = [...currentStudy.paramNames, ...currentStudy.objectiveNames]
    const _paramCount = currentStudy.paramNames.length

    // 【series データ構築】
    const _seriesData: number[][] =
      trialRows.length > 0
        ? trialRows.map((trial) =>
            _axisNames.map((axisName, dim) => {
              const raw = dim < _paramCount ? trial.params[axisName] : trial.values[dim - _paramCount]
              const value = Number(raw)
              return Number.isFinite(value) ? value : Number.NaN
            }),
          )
        : Array.from({ length: gpuBuffer.trialCount }, (_, i) => {
            const row = new Array(_axisNames.length).fill(Number.NaN)
            if (gpuBuffer.positions.length >= (i + 1) * 2) {
              row[0] = gpuBuffer.positions[i * 2]
              if (_axisNames.length > 1) {
                row[1] = gpuBuffer.positions[i * 2 + 1]
              }
            }
            return row
          })

    // 【parallelAxis 定義】
    const _parallelAxis = _axisNames.map((name, dim) => {
      let min = Number.POSITIVE_INFINITY
      let max = Number.NEGATIVE_INFINITY
      for (const row of _seriesData) {
        const value = row[dim]
        if (Number.isFinite(value)) {
          if (value < min) min = value
          if (value > max) max = value
        }
      }
      const hasValidRange = Number.isFinite(min) && Number.isFinite(max)
      if (!hasValidRange) return { dim, name, type: 'value' as const }
      if (min === max) {
        const delta = Math.abs(min) * 0.01 || 1
        return { dim, name, type: 'value' as const, min: min - delta, max: max + delta }
      }
      return { dim, name, type: 'value' as const, min, max }
    })

    // selectedIndices でフィルタリング
    const _selectedSet = new Set(selectedIndices)
    const _isFiltered = selectedIndices.length > 0 && selectedIndices.length < _seriesData.length

    const _selectedData: number[][] = []
    const _unselectedData: number[][] = []
    for (let i = 0; i < _seriesData.length; i++) {
      if (!_isFiltered || _selectedSet.has(i)) {
        _selectedData.push(_seriesData[i])
      } else {
        _unselectedData.push(_seriesData[i])
      }
    }

    return {
      axisNames: _axisNames,
      option: {
        parallel: { left: '5%', right: '5%', top: '10%', bottom: '10%' },
        parallelAxis: _parallelAxis,
        series: [
          ...(_unselectedData.length > 0
            ? [{
                type: 'parallel',
                data: _unselectedData,
                lineStyle: { width: 1, opacity: 0.08, color: '#94a3b8' },
                silent: true,
              }]
            : []),
          {
            type: 'parallel',
            data: _selectedData,
            lineStyle: { width: 1, opacity: 0.3 },
          },
        ],
      },
    }
  }, [currentStudy, trialRows, gpuBuffer, selectedIndices])

  /**
   * 【axisareaselected イベントハンドラ】: 軸ブラシの範囲を addAxisFilter に連携する
   * 【Brushing実装】: intervals が空のときはフィルタを解除する
   * 🟢 REQ-041 対応: 軸ブラシ → addAxisFilter → WASM filterByRanges
   */
  const handleAxisAreaSelected = (params: unknown) => {
    const event = params as AxisAreaSelectedEvent
    if (!event?.axesInfo) return

    event.axesInfo.forEach(({ axisIndex, intervals }) => {
      const axisName = memoAxisNames[axisIndex]
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
      lazyUpdate
      onEvents={{
        // 【イベント登録】: 軸ブラシのイベントを登録する
        axisareaselected: handleAxisAreaSelected,
      }}
    />
  )
}
