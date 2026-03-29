/**
 * ConvergenceDiagnosis — 収束診断パネル (TASK-1001)
 *
 * 【役割】: 最適化の収束状態を診断してバッジで表示する
 * 【設計方針】:
 *   - diagnoseConvergence() で末尾 20% のBest値改善率から収束を判定
 *   - 試行数 < MINIMUM_TRIALS の場合は 'insufficient'（判定不可）
 *   - converged（緑）/ converging（黄）/ not-converged（赤）/ insufficient のバッジ
 * 🟢 REQ-111〜REQ-113 に準拠
 */

import type { TrialData, OptimizationDirection } from '../charts/OptimizationHistory'

// -------------------------------------------------------------------------
// 定数定義
// -------------------------------------------------------------------------

/** 【最小試行数】: 収束判定に必要な最低試行数 */
const MINIMUM_TRIALS = 10

/** 【末尾観測ウィンドウ率】: 末尾何%を収束判定に使うか */
const TAIL_WINDOW_RATE = 0.2

/** 【収束判定閾値】: 末尾ウィンドウ内の改善率がこれ未満なら収束済み */
const CONVERGED_THRESHOLD = 0.001 // 0.1%

/** 【収束中判定閾値】: 末尾ウィンドウ内の改善率がこれ未満なら収束中 */
const CONVERGING_THRESHOLD = 0.01 // 1%

// -------------------------------------------------------------------------
// 型定義
// -------------------------------------------------------------------------

/** 【収束状態型】: 収束診断の結果 */
export type ConvergenceStatus = 'converged' | 'converging' | 'not-converged' | 'insufficient'

// -------------------------------------------------------------------------
// 純粋関数
// -------------------------------------------------------------------------

/**
 * 【収束診断】: 試行データから収束状態を診断する
 * 【判定アルゴリズム】:
 *   1. 試行数 < MINIMUM_TRIALS → insufficient
 *   2. 末尾 20% のBest値の最大変化率を計算
 *   3. 変化率 < 0.1%  → converged
 *   4. 変化率 < 1%    → converging
 *   5. それ以外        → not-converged
 * 🟢 REQ-112〜REQ-113 に準拠
 * @param data - 試行データ配列
 * @param direction - 最適化方向
 * @returns 収束状態
 */
export function diagnoseConvergence(
  data: TrialData[],
  direction: OptimizationDirection,
): ConvergenceStatus {
  // 【試行数チェック】: 最低試行数未満の場合は判定不可を返す 🟢
  if (data.length < MINIMUM_TRIALS) {
    return 'insufficient'
  }

  // 【Best値系列計算】: 各試行時点でのBest値を累積計算する 🟢
  let best = direction === 'minimize' ? Infinity : -Infinity
  const bestSeries = data.map(({ value }) => {
    if (direction === 'minimize') {
      best = Math.min(best, value)
    } else {
      best = Math.max(best, value)
    }
    return best
  })

  // 【末尾ウィンドウ抽出】: 末尾 20% のBest値を取得する 🟢
  const tailStart = Math.floor(data.length * (1 - TAIL_WINDOW_RATE))
  const tailBest = bestSeries.slice(tailStart)

  // 【改善率計算】: 末尾ウィンドウの先頭と末尾のBest値から改善率を算出する 🟢
  const firstBest = tailBest[0]
  const lastBest = tailBest[tailBest.length - 1]

  // 【ゼロ除算ガード】: firstBest が 0 の場合は絶対差を使う
  const improvementRate =
    firstBest !== 0 ? Math.abs((firstBest - lastBest) / firstBest) : Math.abs(firstBest - lastBest)

  // 【収束判定】: 改善率の大きさで 3 段階に分類する 🟢
  if (improvementRate < CONVERGED_THRESHOLD) {
    return 'converged'
  }
  if (improvementRate < CONVERGING_THRESHOLD) {
    return 'converging'
  }
  return 'not-converged'
}

// -------------------------------------------------------------------------
// バッジ設定
// -------------------------------------------------------------------------

/** 【バッジ設定型】: バッジの表示内容 */
interface BadgeConfig {
  testId: string
  label: string
  color: string
  background: string
}

/** 【バッジ設定マップ】: 収束状態ごとのバッジ設定 */
const BADGE_CONFIG: Record<ConvergenceStatus, BadgeConfig> = {
  converged: {
    testId: 'badge-converged',
    label: '収束済み',
    color: '#fff',
    background: '#16a34a', // 🟢 緑: 収束済み
  },
  converging: {
    testId: 'badge-converging',
    label: '収束中',
    color: '#92400e',
    background: '#fbbf24', // 🟡 黄: 収束中
  },
  'not-converged': {
    testId: 'badge-not-converged',
    label: '未収束',
    color: '#fff',
    background: '#dc2626', // 🔴 赤: 未収束
  },
  insufficient: {
    testId: 'badge-insufficient',
    label: '判定不可',
    color: '#374151',
    background: '#e5e7eb', // ⚪ グレー: 判定不可
  },
}

// -------------------------------------------------------------------------
// Props 型定義
// -------------------------------------------------------------------------

/**
 * 【Props】: ConvergenceDiagnosis コンポーネントのプロパティ
 */
export interface ConvergenceDiagnosisProps {
  /** 🟢 試行データ配列 */
  data: TrialData[]
  /** 🟢 最適化方向 */
  direction: OptimizationDirection
}

// -------------------------------------------------------------------------
// メインコンポーネント
// -------------------------------------------------------------------------

/**
 * 【機能概要】: 収束診断パネルコンポーネント
 * 【バッジ表示】: 収束状態に応じて色分けされたバッジを表示する
 * 【判定不可】: 試行数が少ない場合は「判定不可（試行数不足）」を表示
 * 🟢 REQ-111〜REQ-113 に準拠
 */
export function ConvergenceDiagnosis({ data, direction }: ConvergenceDiagnosisProps) {
  // 【収束診断実行】: 試行データから収束状態を判定する
  const status = diagnoseConvergence(data, direction)

  // 【判定不可処理】: 試行数不足の場合はメッセージのみ表示する 🟢
  if (status === 'insufficient') {
    return (
      <div
        data-testid="convergence-diagnosis"
        style={{ padding: '12px', display: 'flex', alignItems: 'center', gap: '8px' }}
      >
        <span style={{ fontSize: '13px', color: '#6b7280' }}>収束診断:</span>
        <span
          data-testid="badge-insufficient"
          style={{
            fontSize: '12px',
            padding: '2px 8px',
            borderRadius: '9999px',
            background: BADGE_CONFIG.insufficient.background,
            color: BADGE_CONFIG.insufficient.color,
          }}
        >
          判定不可（試行数不足）
        </span>
      </div>
    )
  }

  const config = BADGE_CONFIG[status]

  return (
    <div
      data-testid="convergence-diagnosis"
      style={{ padding: '12px', display: 'flex', alignItems: 'center', gap: '8px' }}
    >
      {/* 【ラベル】: パネルタイトル */}
      <span style={{ fontSize: '13px', color: '#6b7280' }}>収束診断:</span>

      {/* 【バッジ】: 収束状態を色付きバッジで表示する 🟢 */}
      <span
        data-testid={config.testId}
        style={{
          fontSize: '12px',
          fontWeight: 600,
          padding: '2px 8px',
          borderRadius: '9999px',
          background: config.background,
          color: config.color,
        }}
      >
        {config.label}
      </span>
    </div>
  )
}
