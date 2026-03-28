/**
 * ContourPlot — 2次元パラメータ相関散布図（コンタープロット簡易版）
 *
 * 【役割】: 2つのパラメータを軸として目的関数値で色付けした散布図を表示する
 *
 * ⚠️【注意】: optuna-dashboard の contour plot は Gradient Boosting Tree
 *   (scikit-learn) でサーフェスを補間しコンター線を描画します（Python 必須）。
 *   このコンポーネントは実トライアル点の散布のみ表示します（補間なし）。
 *
 * 【設計方針】:
 *   - X軸・Y軸パラメータ、目的関数を選択可能なドロップダウン
 *   - visualMap で目的関数値をカラースケールにマッピング
 *   - 数値パラメータのみ対象（文字列パラメータは散布できないため除外）
 * 🟢 Python 不要の範囲で optuna-dashboard の contour plot に相当する機能を提供
 */

import { useState } from 'react';
import ReactECharts from 'echarts-for-react';
import { EmptyState } from '../common/EmptyState';

// -------------------------------------------------------------------------
// 型定義
// -------------------------------------------------------------------------

/** 【試行データ型】: ContourPlot が受け取る1試行分のデータ */
export interface ContourTrial {
  /** パラメータ値マップ（数値・文字列混在可） */
  params: Record<string, number | string>;
  /** 目的関数値リスト（null = 未完了試行） */
  values: number[] | null;
}

/** 【Props 型】 */
export interface ContourPlotProps {
  /** 表示する試行一覧 */
  trials: ContourTrial[];
  /** パラメータ名リスト */
  paramNames: string[];
  /** 目的関数名リスト */
  objectiveNames: string[];
}

// -------------------------------------------------------------------------
// コンポーネント実装
// -------------------------------------------------------------------------

/**
 * 【機能概要】: 2パラメータ × 目的関数値の散布図（コンタープロット簡易版）
 * 【データフロー】: trials → X/Y/目的関数フィルタ → ECharts scatter + visualMap
 * 【インタラクション】: X/Yパラメータ・目的関数のドロップダウン選択でリアルタイム更新
 */
export function ContourPlot({ trials, paramNames, objectiveNames }: ContourPlotProps) {
  // 【状態管理】: X軸・Y軸パラメータインデックス、目的関数インデックス
  const [xParamIdx, setXParamIdx] = useState(0);
  const [yParamIdx, setYParamIdx] = useState(Math.min(1, paramNames.length - 1));
  const [objIdx, setObjIdx] = useState(0);

  // 【空状態チェック】: データなし or パラメータ数不足（2つ必要）
  if (trials.length === 0 || paramNames.length < 2) {
    return <EmptyState message="データがありません（パラメータが2つ以上必要です）" />;
  }

  const xParam = paramNames[xParamIdx];
  const yParam = paramNames[yParamIdx];

  // 【有効トライアルフィルタ】: X/Y パラメータが数値かつ目的関数値が存在するものだけ使用
  const validTrials = trials.filter(
    (t) =>
      t.values !== null &&
      t.values[objIdx] != null &&
      typeof t.params[xParam] === 'number' &&
      typeof t.params[yParam] === 'number',
  );

  // 【目的関数値の範囲】: visualMap の min/max 計算
  const objValues = validTrials.map((t) => t.values![objIdx]);
  const minObj = objValues.length > 0 ? Math.min(...objValues) : 0;
  const maxObj = objValues.length > 0 ? Math.max(...objValues) : 1;

  // 【ECharts オプション構築】: scatter + visualMap（目的関数値でカラースケール）
  const option = {
    tooltip: {
      trigger: 'item',
      formatter: (params: { value: [number, number, number] }) =>
        `${xParam}: ${params.value[0].toFixed(4)}<br/>` +
        `${yParam}: ${params.value[1].toFixed(4)}<br/>` +
        `${objectiveNames[objIdx]}: ${params.value[2].toFixed(4)}`,
    },
    xAxis: {
      type: 'value',
      name: xParam,
      nameLocation: 'center',
      nameGap: 24,
    },
    yAxis: {
      type: 'value',
      name: yParam,
      nameLocation: 'center',
      nameGap: 40,
    },
    // 【カラースケール】: 青（低値）→ 赤（高値）で目的関数値を表現
    visualMap: {
      min: minObj,
      max: maxObj,
      dimension: 2,
      inRange: {
        color: [
          '#313695', '#4575b4', '#74add1', '#abd9e9',
          '#e0f3f8', '#ffffbf', '#fee090', '#fdae61',
          '#f46d43', '#d73027', '#a50026',
        ],
      },
      calculable: true,
      orient: 'vertical',
      right: 10,
      top: 'center',
    },
    series: [
      {
        type: 'scatter',
        // 【データ形式】: [x値, y値, 目的関数値（visualMap の dimension=2 で参照）]
        data: validTrials.map((t) => [
          t.params[xParam] as number,
          t.params[yParam] as number,
          t.values![objIdx],
        ]),
        symbolSize: 8,
      },
    ],
    grid: { containLabel: true, right: 80 },
  };

  return (
    <div
      data-testid="contour-plot"
      style={{ height: '100%', display: 'flex', flexDirection: 'column' }}
    >
      {/* 【注意バナー】: Python/scikit-learn が必要な機能との差異を明示 */}
      <div
        data-testid="contour-note"
        style={{
          padding: '2px 8px',
          background: '#fffbeb',
          borderBottom: '1px solid #fde68a',
          fontSize: 11,
          color: '#92400e',
          flexShrink: 0,
        }}
      >
        ⚠️ optuna-dashboard のコンター補間（Python / scikit-learn 必須）は非対応。実トライアル点のみ表示。
      </div>

      {/* 【コントロールバー】: X/Y パラメータ・目的関数選択 */}
      <div
        style={{
          display: 'flex',
          gap: 8,
          padding: '4px 8px',
          flexShrink: 0,
          fontSize: 12,
          alignItems: 'center',
          flexWrap: 'wrap',
        }}
      >
        <label>
          X:{' '}
          <select
            data-testid="contour-x-select"
            value={xParamIdx}
            onChange={(e) => setXParamIdx(Number(e.target.value))}
            style={{ fontSize: 12 }}
          >
            {paramNames.map((p, i) => (
              <option key={p} value={i}>
                {p}
              </option>
            ))}
          </select>
        </label>

        <label>
          Y:{' '}
          <select
            data-testid="contour-y-select"
            value={yParamIdx}
            onChange={(e) => setYParamIdx(Number(e.target.value))}
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
              data-testid="contour-obj-select"
              value={objIdx}
              onChange={(e) => setObjIdx(Number(e.target.value))}
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

      {/* 【チャート本体】: ECharts scatter + visualMap */}
      <ReactECharts option={option} style={{ flex: 1 }} />
    </div>
  );
}

export default ContourPlot;
