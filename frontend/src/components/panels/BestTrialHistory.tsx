/**
 * BestTrialHistory — Best解遷移トラッキングテーブル (TASK-1001)
 *
 * 【役割】: 最適化過程でBest値が更新された試行のみをテーブルで表示する
 * 【設計方針】:
 *   - Best値が更新された試行のみを一覧表示（非更新行は除外）
 *   - 行クリックで onRowClick コールバックを呼び出す（全グラフハイライト連動用）
 *   - minimize/maximize 方向に対応
 * 🟢 REQ-110, REQ-113 に準拠
 */

import type { TrialData, OptimizationDirection } from '../charts/OptimizationHistory';

// -------------------------------------------------------------------------
// 純粋関数
// -------------------------------------------------------------------------

/**
 * 【Best更新試行抽出】: Best値が更新された試行のみを抽出する
 * 🟢 REQ-110 に準拠
 * @param data - 試行データ配列（試行番号順）
 * @param direction - 最適化方向
 * @returns Best値が更新された試行データの配列
 */
function extractBestTrials(data: TrialData[], direction: OptimizationDirection): TrialData[] {
  let best = direction === 'minimize' ? Infinity : -Infinity;
  const result: TrialData[] = [];

  for (const trial of data) {
    // 【Best判定】: 現試行がBest値を更新するか判定する 🟢
    const isBetter =
      direction === 'minimize' ? trial.value < best : trial.value > best;

    if (isBetter) {
      best = trial.value;
      result.push(trial);
    }
  }

  return result;
}

// -------------------------------------------------------------------------
// Props 型定義
// -------------------------------------------------------------------------

/**
 * 【Props】: BestTrialHistory コンポーネントのプロパティ
 */
export interface BestTrialHistoryProps {
  /** 🟢 試行データ配列 */
  data: TrialData[];
  /** 🟢 最適化方向 */
  direction: OptimizationDirection;
  /** 🟢 行クリックコールバック — 全グラフハイライト連動用 */
  onRowClick?: (trial: TrialData) => void;
}

// -------------------------------------------------------------------------
// メインコンポーネント
// -------------------------------------------------------------------------

/**
 * 【機能概要】: Best解遷移トラッキングテーブルコンポーネント
 * 【表示内容】: Best値が更新された試行番号と目的値を一覧表示する
 * 【行クリック】: クリックで onRowClick(trial) を呼び出して全グラフに連動させる
 * 🟢 REQ-110, REQ-113 に準拠
 */
export function BestTrialHistory({ data, direction, onRowClick }: BestTrialHistoryProps) {
  // 【Best更新試行抽出】: Best値が更新された試行のみを取得する 🟢
  const bestTrials = extractBestTrials(data, direction);

  return (
    <div
      data-testid="best-trial-table"
      style={{ overflow: 'auto', maxHeight: '300px' }}
    >
      {/* 【テーブルヘッダー】: 試行番号と目的値のカラム */}
      <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px' }}>
        <thead>
          <tr style={{ background: '#f3f4f6', borderBottom: '1px solid #e5e7eb' }}>
            <th style={{ padding: '6px 12px', textAlign: 'left', fontWeight: 600 }}>試行番号</th>
            <th style={{ padding: '6px 12px', textAlign: 'right', fontWeight: 600 }}>目的値</th>
          </tr>
        </thead>
        <tbody>
          {/* 【行生成】: Best更新試行ごとにテーブル行を生成する 🟢 */}
          {bestTrials.map((trial) => (
            <tr
              key={trial.trial}
              data-testid={`best-row-${trial.trial}`}
              onClick={() => onRowClick?.(trial)}
              style={{
                borderBottom: '1px solid #f3f4f6',
                cursor: onRowClick ? 'pointer' : 'default',
              }}
            >
              {/* 【試行番号セル】: 試行番号を左揃えで表示する */}
              <td style={{ padding: '5px 12px' }}>{trial.trial}</td>

              {/* 【目的値セル】: 目的値を右揃えで表示する */}
              <td style={{ padding: '5px 12px', textAlign: 'right', fontFamily: 'monospace' }}>
                {trial.value.toFixed(4)}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
