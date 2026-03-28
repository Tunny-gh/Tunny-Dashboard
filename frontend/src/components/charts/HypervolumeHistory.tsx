/**
 * HypervolumeHistory — ECharts折れ線グラフ（Hypervolume推移）(TASK-501)
 *
 * 【役割】: trial 番号ごとのHypervolume推移を折れ線グラフで表示
 * 【設計方針】: echarts-for-react でシンプルに実装
 * 🟢 EChartsOption の series に [trial, hypervolume] の配列を渡す
 */

import ReactECharts from 'echarts-for-react';

// -------------------------------------------------------------------------
// Props 型定義
// -------------------------------------------------------------------------

export interface HypervolumeDataPoint {
  trial: number;
  hypervolume: number;
}

export interface HypervolumeHistoryProps {
  /** 🟢 Hypervolume推移データ — 空配列のとき空状態UIを表示 */
  data: HypervolumeDataPoint[];
}

// -------------------------------------------------------------------------
// コンポーネント実装
// -------------------------------------------------------------------------

/**
 * 【機能概要】: ECharts を使った Hypervolume 推移折れ線グラフ
 * 【テスト対応】: TC-501-05, TC-501-06, TC-501-E03
 */
export function HypervolumeHistory({ data }: HypervolumeHistoryProps) {
  // 【空状態UI】: データがない場合はメッセージを表示 🟢
  if (data.length === 0) {
    return (
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
        <span>データがありません</span>
      </div>
    );
  }

  // 【ECharts option 構築】: series に [trial, hypervolume] の配列を渡す 🟢
  const option = {
    xAxis: {
      type: 'value',
      name: 'Trial',
    },
    yAxis: {
      type: 'value',
      name: 'Hypervolume',
    },
    series: [
      {
        type: 'line',
        // 【データ変換】: {trial, hypervolume} → [trial, hypervolume] の形式に変換
        data: data.map((d) => [d.trial, d.hypervolume]),
      },
    ],
    tooltip: {
      trigger: 'axis',
    },
  };

  // 【ECharts レンダリング】: ReactECharts に option を渡す 🟢
  return (
    <ReactECharts
      option={option}
      style={{ width: '100%', height: '100%' }}
    />
  );
}
