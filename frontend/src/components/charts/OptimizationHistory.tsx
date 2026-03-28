/**
 * OptimizationHistory — 単目的最適化の収束履歴チャート (TASK-1001)
 *
 * 【役割】: 最適化の収束過程を ECharts で可視化する
 * 【設計方針】:
 *   - best/all/moving-avg/improvement の 4 表示モード切り替え
 *   - detectPhase() で試行進捗からフェーズを自動検出
 *   - echarts-for-react でレンダリング（jsdom テスト用にモック対応）
 * 🟢 REQ-1001〜REQ-1006 に準拠
 */

import { useState } from 'react';
import ReactECharts from 'echarts-for-react';

// -------------------------------------------------------------------------
// 型定義
// -------------------------------------------------------------------------

/** 【試行データ型】: 1 試行あたりの結果データ */
export interface TrialData {
  /** 試行番号（1始まり） */
  trial: number;
  /** 目的関数値 */
  value: number;
}

/** 【表示モード型】: 収束履歴の表示モード */
export type HistoryMode = 'best' | 'all' | 'moving-avg' | 'improvement';

/** 【最適化方向型】: 最小化 or 最大化 */
export type OptimizationDirection = 'minimize' | 'maximize';

/** 【フェーズ型】: 最適化の進捗フェーズ */
export type OptimizationPhase = 'exploration' | 'exploitation' | 'convergence';

// -------------------------------------------------------------------------
// 定数定義
// -------------------------------------------------------------------------

/** 【モードラベル】: UI 表示用のモード名 */
const MODE_LABELS: Record<HistoryMode, string> = {
  best: 'Best値推移',        // 🟢 各試行時点のBest値を折れ線表示
  all: '全試行値',           // 🟢 全試行の目的値を散布図表示
  'moving-avg': '移動平均',  // 🟢 移動平均を折れ線表示
  improvement: '改善率',     // 🟢 Best値の改善率を棒グラフ表示
};

/** 【移動平均ウィンドウ】: 移動平均計算の窓幅 */
const MOVING_AVG_WINDOW = 5;

// -------------------------------------------------------------------------
// 純粋関数
// -------------------------------------------------------------------------

/**
 * 【フェーズ自動検出】: 試行進捗（progress）からフェーズを判定する
 * 【判定基準】:
 *   - exploration : progress < 0.3  （探索期: 全試行の先頭 30%）
 *   - exploitation: 0.3 <= progress < 0.7（精緻化期: 中間 40%）
 *   - convergence : progress >= 0.7 （収束期: 末尾 30%）
 * 🟢 REQ-1004〜REQ-1006 に準拠
 * @param trialIndex - 現在の試行インデックス（1始まり）
 * @param totalTrials - 試行総数
 * @returns フェーズ文字列
 */
export function detectPhase(trialIndex: number, totalTrials: number): OptimizationPhase {
  // 【進捗計算】: 全試行数に対する現在試行の割合を算出
  const progress = trialIndex / totalTrials;

  // 【フェーズ判定】: progress の境界値で 3 フェーズに分類する 🟢
  if (progress < 0.3) {
    return 'exploration';   // 【探索期】: 試行空間を広く探索している段階
  }
  if (progress < 0.7) {
    return 'exploitation';  // 【精緻化期】: 有望領域を集中的に探索している段階
  }
  return 'convergence';     // 【収束期】: 解が収束しつつある段階
}

/**
 * 【Best値系列計算】: 各試行時点でのBest値配列を計算する
 * @param data - 試行データ配列
 * @param direction - 最適化方向
 * @returns Best値配列
 */
function computeBestSeries(data: TrialData[], direction: OptimizationDirection): number[] {
  // 【累積Best計算】: 各試行時点における最良値を積み上げる 🟢
  let best = direction === 'minimize' ? Infinity : -Infinity;
  return data.map(({ value }) => {
    if (direction === 'minimize') {
      best = Math.min(best, value);
    } else {
      best = Math.max(best, value);
    }
    return best;
  });
}

/**
 * 【移動平均計算】: 指定ウィンドウ幅の移動平均を計算する
 * @param values - 入力値配列
 * @param window - ウィンドウ幅
 * @returns 移動平均配列
 */
function computeMovingAverage(values: number[], window: number): number[] {
  // 【移動平均】: 各点について直前 window 件の平均を計算する 🟢
  return values.map((_, i) => {
    const start = Math.max(0, i - window + 1);
    const slice = values.slice(start, i + 1);
    return slice.reduce((sum, v) => sum + v, 0) / slice.length;
  });
}

/**
 * 【改善率計算】: 前試行からのBest値改善率を計算する
 * @param bestSeries - Best値配列
 * @returns 改善率配列（%）
 */
function computeImprovementRate(bestSeries: number[]): number[] {
  // 【改善率】: |prev - curr| / |prev| * 100 で変化率を算出する 🟡
  return bestSeries.map((curr, i) => {
    if (i === 0) return 0;
    const prev = bestSeries[i - 1];
    if (prev === 0) return 0;
    return Math.abs((prev - curr) / prev) * 100;
  });
}

// -------------------------------------------------------------------------
// ECharts オプション生成
// -------------------------------------------------------------------------

/**
 * 【オプション生成】: 表示モードに応じた ECharts option を生成する
 * @param data - 試行データ配列
 * @param mode - 表示モード
 * @param direction - 最適化方向
 * @returns ECharts option オブジェクト
 */
function buildChartOption(
  data: TrialData[],
  mode: HistoryMode,
  direction: OptimizationDirection,
): object {
  // 【空データ処理】: データがない場合は空のオプションを返す 🟢
  if (data.length === 0) {
    return { xAxis: { type: 'value' }, yAxis: { type: 'value' }, series: [] };
  }

  const trials = data.map((d) => d.trial);
  const values = data.map((d) => d.value);
  const bestSeries = computeBestSeries(data, direction);

  switch (mode) {
    case 'best':
      // 【Best値推移】: 累積Best値の折れ線グラフ
      return {
        xAxis: { type: 'category', data: trials },
        yAxis: { type: 'value' },
        series: [{ type: 'line', data: bestSeries, name: 'Best値' }],
      };

    case 'all':
      // 【全試行値】: 全試行の目的値散布図
      return {
        xAxis: { type: 'value' },
        yAxis: { type: 'value' },
        series: [{ type: 'scatter', data: trials.map((t, i) => [t, values[i]]), name: '全試行値' }],
      };

    case 'moving-avg': {
      // 【移動平均】: Best値の移動平均折れ線グラフ
      const movingAvg = computeMovingAverage(values, MOVING_AVG_WINDOW);
      return {
        xAxis: { type: 'category', data: trials },
        yAxis: { type: 'value' },
        series: [
          { type: 'line', data: values, name: '全試行値', opacity: 0.4 },
          { type: 'line', data: movingAvg, name: `移動平均(${MOVING_AVG_WINDOW})` },
        ],
      };
    }

    case 'improvement': {
      // 【改善率】: Best値改善率の棒グラフ
      const improvementRate = computeImprovementRate(bestSeries);
      return {
        xAxis: { type: 'category', data: trials },
        yAxis: { type: 'value' },
        series: [{ type: 'bar', data: improvementRate, name: '改善率(%)' }],
      };
    }

    default:
      return { xAxis: { type: 'value' }, yAxis: { type: 'value' }, series: [] };
  }
}

// -------------------------------------------------------------------------
// Props 型定義
// -------------------------------------------------------------------------

/**
 * 【Props】: OptimizationHistory コンポーネントのプロパティ
 */
export interface OptimizationHistoryProps {
  /** 🟢 試行データ配列 */
  data: TrialData[];
  /** 🟢 最適化方向 */
  direction: OptimizationDirection;
}

// -------------------------------------------------------------------------
// メインコンポーネント
// -------------------------------------------------------------------------

/**
 * 【機能概要】: 単目的最適化の収束履歴チャートコンポーネント
 * 【表示モード】: best/all/moving-avg/improvement の 4 モード切り替え
 * 【フェーズ検出】: detectPhase() で試行進捗からフェーズを自動判定
 * 🟢 REQ-1001〜REQ-1006 に準拠
 */
export function OptimizationHistory({ data, direction }: OptimizationHistoryProps) {
  // 【表示モード状態】: デフォルトは 'best'（Best値推移）
  const [mode, setMode] = useState<HistoryMode>('best');

  // 【ECharts オプション生成】: 現在のモードとデータに応じたオプションを生成
  const option = buildChartOption(data, mode, direction);

  return (
    <div
      data-testid="optimization-history"
      style={{ display: 'flex', flexDirection: 'column', height: '100%' }}
    >
      {/* 【コントロールバー】: 表示モード切り替えボタン群 */}
      <div
        style={{
          display: 'flex',
          gap: '4px',
          padding: '8px',
          borderBottom: '1px solid #e5e7eb',
          flexShrink: 0,
        }}
      >
        {/* 【モードボタン群】: 4 表示モードを切り替えるボタン 🟢 */}
        {(['best', 'all', 'moving-avg', 'improvement'] as HistoryMode[]).map((m) => (
          <button
            key={m}
            data-testid={`mode-btn-${m}`}
            aria-pressed={mode === m}
            onClick={() => setMode(m)}
            style={{
              padding: '4px 10px',
              fontSize: '12px',
              background: mode === m ? '#4f46e5' : '#f3f4f6',
              color: mode === m ? '#fff' : '#374151',
              border: '1px solid',
              borderColor: mode === m ? '#4f46e5' : '#d1d5db',
              borderRadius: '4px',
              cursor: 'pointer',
            }}
          >
            {MODE_LABELS[m]}
          </button>
        ))}
      </div>

      {/* 【チャートエリア】: ECharts で収束履歴を描画する 🟢 */}
      <div style={{ flex: 1 }}>
        <ReactECharts option={option} style={{ height: '100%' }} />
      </div>
    </div>
  );
}
