/**
 * EdfPlot — 経験累積分布関数チャート (Optuna-Dashboard EDF 相当)
 *
 * 【役割】: 目的関数値の経験的累積分布関数 (EDF / ECDF) を折れ線で可視化する
 * 【設計方針】:
 *   - computeEdf() で昇順ソート → 累積確率を計算（純粋関数・テスト容易）
 *   - 複数の目的関数を同時比較可能（多目的最適化での比較に有用）
 *   - ECharts の step: 'end' で CDF の階段グラフを表現
 * 🟢 optuna-dashboard の plot_edf と同等機能（Python 不要）
 */

import ReactECharts from 'echarts-for-react';

// -------------------------------------------------------------------------
// 型定義
// -------------------------------------------------------------------------

/** 【シリーズ型】: 1目的関数分の EDF データ */
export interface EdfSeries {
  /** 目的関数名（凡例ラベルとして表示） */
  name: string;
  /** 目的関数値リスト（未ソートでよい） */
  values: number[];
}

/** 【Props 型】 */
export interface EdfPlotProps {
  /** 表示するシリーズ一覧（多目的は複数渡す） */
  series: EdfSeries[];
}

// -------------------------------------------------------------------------
// 純粋関数
// -------------------------------------------------------------------------

/**
 * 【EDF 計算】: 値リストから経験累積分布関数の座標列を生成する
 * 【処理フロー】:
 *   1. 昇順ソート
 *   2. 各点の累積確率 = (順位 / 総数) を計算
 * @param values - 目的関数値リスト（順不同）
 * @returns [値, 累積確率] のペアリスト
 */
export function computeEdf(values: number[]): [number, number][] {
  // 【空チェック】: 空配列は空を返す
  if (values.length === 0) return [];

  // 【昇順ソート】: 元配列を破壊しないようコピーしてソート
  const sorted = [...values].sort((a, b) => a - b);
  const n = sorted.length;

  // 【CDF 計算】: i 番目（1始まり）の累積確率 = i / n
  return sorted.map((v, i) => [v, (i + 1) / n]);
}

// -------------------------------------------------------------------------
// コンポーネント実装
// -------------------------------------------------------------------------

/**
 * 【機能概要】: 目的関数値の経験累積分布関数を step 折れ線で描画
 * 【データフロー】: series → computeEdf() → ECharts line series (step: 'end')
 * 【多目的対応】: series を複数渡すことで複数ラインを同一チャートに表示
 */
export function EdfPlot({ series }: EdfPlotProps) {
  // 【空状態チェック】: シリーズなし or 全シリーズが空値の場合はプレースホルダー表示
  if (series.length === 0 || series.every((s) => s.values.length === 0)) {
    return (
      <div
        style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}
      >
        <span>データがありません</span>
      </div>
    );
  }

  // 【ECharts オプション構築】: step line で CDF を表現
  const option = {
    tooltip: {
      trigger: 'axis',
      formatter: (params: Array<{ seriesName: string; value: [number, number] }>) =>
        params.map((p) => `${p.seriesName}: ${p.value[0].toFixed(4)}`).join('<br/>'),
    },
    legend: {
      data: series.map((s) => s.name),
    },
    xAxis: {
      type: 'value',
      name: '目的関数値',
      nameLocation: 'center',
      nameGap: 24,
    },
    yAxis: {
      type: 'value',
      name: '累積割合',
      nameLocation: 'center',
      nameGap: 40,
      min: 0,
      max: 1,
    },
    // 【各シリーズ】: computeEdf で計算した CDF データを step 折れ線で表示
    series: series.map((s) => ({
      name: s.name,
      type: 'line',
      step: 'end',   // 【CDF らしい表現】: 各点を水平線でつなぐ
      data: computeEdf(s.values),
      symbolSize: 0, // 点を非表示にして折れ線のみ表示
    })),
  };

  return (
    <div
      data-testid="edf-plot"
      style={{ height: '100%' }}
    >
      <ReactECharts option={option} style={{ height: '100%' }} />
    </div>
  );
}

export default EdfPlot;
