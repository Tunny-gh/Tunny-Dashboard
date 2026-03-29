/**
 * ClusterPanel — クラスタリング設定パネル (TASK-902)
 *
 * 【役割】: PCA + k-means のパラメータ設定・実行制御・Elbow チャート表示
 * 【設計方針】:
 *   - Props でコールバックを受け取る（テスト可能な純粋コンポーネント）
 *   - Elbow 曲線を ECharts で表示し、推薦 k を強調
 *   - k=1 の不正入力をクライアント側でブロック
 * 🟢 REQ-080〜REQ-087 に準拠
 */

import { useState } from 'react'
import type { ChangeEvent } from 'react'
import ReactECharts from 'echarts-for-react'

// -------------------------------------------------------------------------
// 型定義
// -------------------------------------------------------------------------

/**
 * 【PCA 対象空間】: クラスタリングに使用する列の種類
 * 🟢 REQ-080 に準拠
 */
export type ClusterSpace = 'param' | 'objective' | 'all'

/**
 * 【Elbow 結果データ型】: WASM estimate_k_elbow() の結果を UI 表示用に変換した型
 */
export interface ElbowResultData {
  /** k=2, 3, ... に対応する WCSS 値 */
  wcssPerK: number[]
  /** Elbow 法による推薦 k */
  recommendedK: number
}

/**
 * 【ClusterPanel Props】: クラスタリングパネルのプロパティ
 */
export interface ClusterPanelProps {
  /** クラスタリング実行コールバック: 実行ボタン押下時に呼ばれる 🟢 */
  onRunClustering: (space: ClusterSpace, k: number) => void
  /** 計算実行中フラグ: true のときプログレスバーを表示 */
  isRunning?: boolean
  /** 進捗 0〜100 */
  progress?: number
  /** Elbow 法の結果（クラスタリング実行後に提供）*/
  elbowResult?: ElbowResultData | null
  /** エラーメッセージ（計算失敗時）*/
  error?: string | null
}

// -------------------------------------------------------------------------
// 定数
// -------------------------------------------------------------------------

/** 【k 最小値】: k=1 は意味がないため 2 以上を強制 */
const K_MIN = 2

/** 【k デフォルト値】: 一般的な初期値として 4 を設定 */
const K_DEFAULT = 4

/** 【空間ラベル】: ClusterSpace → 日本語表示名 */
const SPACE_LABELS: Record<ClusterSpace, string> = {
  param: 'パラメータ',
  objective: '目的関数',
  all: '全て',
}

// -------------------------------------------------------------------------
// ECharts オプション生成
// -------------------------------------------------------------------------

/**
 * 【Elbow チャートオプション生成】: WCSS 折れ線グラフ + 推薦 k マークポイント
 *
 * 【設計方針】:
 *   - k=2 スタートで横軸を category 型で表示
 *   - 推薦 k をマークポイントで強調 (🟢 REQ-084)
 */
function buildElbowOption(elbow: ElbowResultData): object {
  const kStart = 2
  const kLabels = elbow.wcssPerK.map((_, i) => String(i + kStart))
  const recommendedIdx = elbow.recommendedK - kStart
  const recommendedWcss =
    recommendedIdx >= 0 && recommendedIdx < elbow.wcssPerK.length
      ? elbow.wcssPerK[recommendedIdx]
      : undefined

  return {
    tooltip: { trigger: 'axis' },
    xAxis: {
      type: 'category',
      data: kLabels,
      name: 'k',
      nameLocation: 'middle',
      nameGap: 20,
    },
    yAxis: {
      type: 'value',
      name: 'WCSS',
    },
    series: [
      {
        type: 'line',
        data: elbow.wcssPerK,
        lineStyle: { color: '#4f46e5', width: 2 },
        symbolSize: 5,
        ...(recommendedWcss !== undefined
          ? {
              markPoint: {
                data: [
                  {
                    coord: [String(elbow.recommendedK), recommendedWcss],
                    symbolSize: 14,
                    itemStyle: { color: '#e11d48' },
                    label: {
                      show: true,
                      formatter: `k=${elbow.recommendedK}`,
                      position: 'top',
                      fontSize: 11,
                      color: '#e11d48',
                    },
                  },
                ],
              },
            }
          : {}),
      },
    ],
  }
}

// -------------------------------------------------------------------------
// メインコンポーネント
// -------------------------------------------------------------------------

/**
 * 【機能概要】: クラスタリング設定・実行パネル
 * 【UI 構成】: 空間選択 → k 設定 → 実行ボタン → プログレスバー → Elbow チャート
 * 🟢 REQ-080〜REQ-087 に準拠
 */
export function ClusterPanel({
  onRunClustering,
  isRunning = false,
  progress = 0,
  elbowResult,
  error,
}: ClusterPanelProps) {
  // -------------------------------------------------------------------------
  // 内部状態
  // -------------------------------------------------------------------------

  /** 【空間選択状態】: 現在選択されている PCA 対象空間 */
  const [space, setSpace] = useState<ClusterSpace>('param')
  /** 【k 設定状態】: ユーザーが入力したクラスタ数 */
  const [k, setK] = useState<number>(K_DEFAULT)
  /** 【k バリデーションエラー】: k<2 のとき警告メッセージ */
  const [kError, setKError] = useState<string | null>(null)

  // -------------------------------------------------------------------------
  // イベントハンドラ
  // -------------------------------------------------------------------------

  /**
   * 【k 入力変更ハンドラ】: 数値に変換し、有効値なら kError をクリア
   */
  const handleKChange = (e: ChangeEvent<HTMLInputElement>) => {
    const val = parseInt(e.target.value, 10)
    const newK = isNaN(val) ? K_DEFAULT : val
    setK(newK)
    if (newK >= K_MIN) setKError(null)
  }

  /**
   * 【実行ハンドラ】: k バリデーション後に onRunClustering コールバックを呼ぶ
   * k=1 は「k=2以上を指定してください」警告を表示してキャンセル 🟢 REQ-087
   */
  const handleRun = () => {
    if (k < K_MIN) {
      setKError('k=2以上を指定してください')
      return
    }
    setKError(null)
    onRunClustering(space, k)
  }

  // -------------------------------------------------------------------------
  // レンダリング
  // -------------------------------------------------------------------------

  return (
    <div
      data-testid="cluster-panel"
      style={{ padding: '12px', display: 'flex', flexDirection: 'column', gap: '12px' }}
    >
      {/* 【対象空間選択】: パラメータ / 目的関数 / 全て 🟢 REQ-080 */}
      <div>
        <div style={{ fontSize: '12px', color: '#6b7280', marginBottom: '4px' }}>対象空間</div>
        {(['param', 'objective', 'all'] as ClusterSpace[]).map((s) => (
          <label
            key={s}
            style={{ display: 'flex', alignItems: 'center', gap: '4px', fontSize: '13px' }}
          >
            <input
              data-testid={`space-${s}`}
              type="radio"
              name="cluster-space"
              value={s}
              checked={space === s}
              onChange={() => setSpace(s)}
            />
            {SPACE_LABELS[s]}
          </label>
        ))}
      </div>

      {/* 【k 設定】: クラスタ数入力フィールド 🟢 REQ-081 */}
      <div>
        <label style={{ fontSize: '12px', color: '#6b7280' }}>クラスタ数 (k)</label>
        <input
          data-testid="k-input"
          type="number"
          min={1}
          value={k}
          onChange={handleKChange}
          style={{
            display: 'block',
            width: '80px',
            padding: '4px',
            fontSize: '14px',
            border: '1px solid #d1d5db',
            borderRadius: '3px',
            marginTop: '4px',
          }}
        />
        {/* 【k バリデーション警告】: k<2 指定時に表示 🟢 REQ-087 */}
        {kError && (
          <div
            data-testid="k-error"
            style={{ marginTop: '4px', fontSize: '12px', color: '#dc2626' }}
          >
            {kError}
          </div>
        )}
      </div>

      {/* 【実行ボタン】: 計算中は非活性 */}
      <button
        data-testid="run-clustering-btn"
        onClick={handleRun}
        disabled={isRunning}
        style={{
          padding: '6px 16px',
          fontSize: '13px',
          background: isRunning ? '#9ca3af' : '#4f46e5',
          color: '#fff',
          border: 'none',
          borderRadius: '4px',
          cursor: isRunning ? 'not-allowed' : 'pointer',
          alignSelf: 'flex-start',
        }}
      >
        {isRunning ? '計算中...' : '実行'}
      </button>

      {/* 【プログレスバー】: isRunning=true のみ表示 🟢 */}
      {isRunning && (
        <div data-testid="progress-container">
          <div
            style={{
              height: '6px',
              background: '#e5e7eb',
              borderRadius: '3px',
              overflow: 'hidden',
            }}
          >
            <div
              data-testid="progress-bar"
              style={{
                height: '100%',
                width: `${progress}%`,
                background: '#4f46e5',
                transition: 'width 0.2s',
              }}
            />
          </div>
          <div
            data-testid="progress-text"
            style={{ fontSize: '12px', color: '#6b7280', marginTop: '4px' }}
          >
            計算中...{progress}%
          </div>
        </div>
      )}

      {/* 【エラー表示】: 計算失敗時 */}
      {error && (
        <div
          data-testid="cluster-error"
          style={{
            padding: '8px',
            background: '#fef2f2',
            border: '1px solid #fca5a5',
            borderRadius: '4px',
            fontSize: '12px',
            color: '#dc2626',
          }}
        >
          {error}
        </div>
      )}

      {/* 【Elbow チャート】: 実行後に表示、推薦 k を強調 🟢 REQ-084 */}
      {elbowResult && (
        <div>
          <div
            data-testid="elbow-recommended"
            style={{ fontSize: '13px', fontWeight: 600, color: '#4f46e5', marginBottom: '4px' }}
          >
            推薦 k = {elbowResult.recommendedK}
          </div>
          <ReactECharts option={buildElbowOption(elbowResult)} style={{ height: '180px' }} />
        </div>
      )}
    </div>
  )
}
