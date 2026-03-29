/**
 * ScatterMatrix — 散布図行列 UIシェル (TASK-702)
 *
 * 【役割】: ScatterMatrixEngine を使って N×N 散布図グリッドを表示する
 * 【設計方針】:
 *   - Mode 1（変数×変数）/ Mode 2（変数×目的）/ Mode 3（全変数）の 3 モード
 *   - 軸ソート: アルファベット順 / 相関順 / 重要度順（後者 2 つは WASM 実装後に有効化）
 *   - ScatterCell は useEffect で engine.renderCell() を呼び出して非同期描画
 *   - Worker 失敗時は当該セルに「❌」を表示して他セルへの影響を防ぐ
 * 🟢 REQ-060〜REQ-066 に準拠
 */

import { useState, useEffect } from 'react'
import type { ScatterMatrixEngine, ScatterCellSize } from '../../wasm/workers/ScatterMatrixEngine'
import type { Study } from '../../types'

// -------------------------------------------------------------------------
// 定数・型定義
// -------------------------------------------------------------------------

/** 【表示モード型】: 散布図行列の表示モード */
export type ScatterMode = 'mode1' | 'mode2' | 'mode3'

/** 【軸ソート型】: 軸の並び順 */
export type SortOrder = 'alphabetical' | 'correlation' | 'importance'

/** 【モードラベル】: UI 表示用のモード名 */
const MODE_LABELS: Record<ScatterMode, string> = {
  mode1: 'Params×Params', // 🟢 変数間の散布図行列
  mode2: 'Params×Objectives', // 🟢 変数と目的関数間の散布図
  mode3: 'All', // 🟢 全変数（変数+目的）の散布図行列
}

/** 【ソートラベル】: UI 表示用のソート順名 */
const SORT_LABELS: Record<SortOrder, string> = {
  alphabetical: 'Alphabetical', // 🟢 常に利用可能
  correlation: 'By Correlation', // 🟡 WASM 実装後に有効化
  importance: 'By Importance', // 🟡 WASM 実装後に有効化
}

// -------------------------------------------------------------------------
// Props 型定義
// -------------------------------------------------------------------------

/**
 * 【Props】: ScatterMatrix コンポーネントのプロパティ
 */
export interface ScatterMatrixProps {
  /** 🟢 WebWorker エンジン — null のときセルはローディングプレースホルダーを表示 */
  engine: ScatterMatrixEngine | null
  /** 🟢 現在の Study — 変数名・目的名の取得用 */
  currentStudy: Study | null
}

// -------------------------------------------------------------------------
// 軸計算ヘルパー
// -------------------------------------------------------------------------

/**
 * 【軸計算】: モードとソート順に基づいて行・列軸名の配列を返す
 * @param study - 現在の Study
 * @param mode - 表示モード
 * @param sortOrder - 軸ソート順
 * @returns rowAxes（行軸名配列）と colAxes（列軸名配列）
 */
export function getAxesForMode(
  study: Study,
  mode: ScatterMode,
  sortOrder: SortOrder,
): { rowAxes: string[]; colAxes: string[] } {
  // 【ソート適用】: アルファベット順のみ実装（相関順・重要度順は WASM 依存のためプレースホルダー）
  const applySort = (axes: string[]): string[] => {
    if (sortOrder === 'alphabetical') {
      return [...axes].sort()
    }
    // 🟡 correlation / importance: TASK-801 (WASM 感度分析) 完成後に実データソートを実装予定
    return [...axes]
  }

  const params = applySort(study.paramNames)
  const objectives = applySort(study.objectiveNames)
  const all = applySort([...study.paramNames, ...study.objectiveNames])

  switch (mode) {
    case 'mode1':
      // 【変数×変数】: 変数間の正方行列
      return { rowAxes: params, colAxes: params }
    case 'mode2':
      // 【変数×目的】: 変数行・目的列の矩形行列
      return { rowAxes: params, colAxes: objectives }
    case 'mode3':
    default:
      // 【全変数】: 変数+目的の正方行列
      return { rowAxes: all, colAxes: all }
  }
}

// -------------------------------------------------------------------------
// ScatterCell サブコンポーネント
// -------------------------------------------------------------------------

/**
 * 【ScatterCell Props】: 散布図行列の 1 セルのプロパティ
 */
interface ScatterCellProps {
  row: number
  col: number
  xAxis: string
  yAxis: string
  engine: ScatterMatrixEngine | null
  size?: ScatterCellSize
}

/**
 * 【ScatterCell】: 散布図行列の 1 セルコンポーネント
 * 【レンダリング状態】:
 *   - engine=null: グレーローディングプレースホルダー
 *   - engine あり・描画中: グレーローディングプレースホルダー
 *   - 描画完了: ImageData を canvas 経由で img 表示
 *   - エラー: 「❌」テキストを表示
 * 🟢 REQ-064 (Worker 失敗時は ❌ 表示) に準拠
 */
function ScatterCell({ row, col, xAxis, yAxis, engine, size = 'thumbnail' }: ScatterCellProps) {
  const [imageUrl, setImageUrl] = useState<string | null>(null)
  const [error, setError] = useState(false)

  useEffect(() => {
    // 【エンジン未初期化】: engine がなければ何もしない（ローディング状態を維持）
    if (!engine) return

    let cancelled = false

    // 【非同期描画】: engine.renderCell() でサムネイルを取得する 🟢
    engine
      .renderCell(row, col, size)
      .then((imageData) => {
        if (cancelled || !imageData) return

        // 【Canvas 変換】: ImageData → ObjectURL 経由で img タグに表示
        const canvas = document.createElement('canvas')
        canvas.width = imageData.width
        canvas.height = imageData.height
        const ctx = canvas.getContext('2d')
        ctx?.putImageData(imageData, 0, 0)
        setImageUrl(canvas.toDataURL())
      })
      .catch(() => {
        // 【エラー処理】: Worker 失敗時はエラーフラグを立てて ❌ を表示する 🟢
        if (!cancelled) setError(true)
      })

    // 【クリーンアップ】: アンマウント時にキャンセルしてメモリリークを防ぐ
    return () => {
      cancelled = true
    }
  }, [engine, row, col, size])

  const pixelSize = size === 'thumbnail' ? 80 : size === 'preview' ? 300 : 600

  return (
    <div
      data-testid={`scatter-cell-${row}-${col}`}
      title={`${xAxis} vs ${yAxis} — Click to expand`}
      style={{
        width: `${pixelSize}px`,
        height: `${pixelSize}px`,
        position: 'relative',
        overflow: 'hidden',
        cursor: 'pointer',
        border: '1px solid #e5e7eb',
      }}
    >
      {error ? (
        // 【エラー表示】: Worker 失敗時は ❌ を表示する 🟢
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            width: '100%',
            height: '100%',
            fontSize: '24px',
          }}
        >
          ❌
        </div>
      ) : imageUrl ? (
        // 【描画完了】: サムネイル画像を表示する
        <img
          src={imageUrl}
          alt={`${xAxis} vs ${yAxis}`}
          style={{ width: '100%', height: '100%', objectFit: 'fill' }}
        />
      ) : (
        // 【ローディング】: 描画待ちはグレーのプレースホルダーを表示する 🟢
        <div
          style={{
            width: '100%',
            height: '100%',
            background: '#e5e7eb',
          }}
        />
      )}
    </div>
  )
}

// -------------------------------------------------------------------------
// ScatterMatrix メインコンポーネント
// -------------------------------------------------------------------------

/**
 * 【機能概要】: 散布図行列の UIシェルコンポーネント
 * 【モード切り替え】: mode1/mode2/mode3 ボタンで行・列軸を切り替える
 * 【軸ソート】: sort-select で軸の並び順を切り替える
 * 【グリッド描画】: N×M CSS Grid で ScatterCell を並べる
 * 【テスト対応】: TC-702-01〜05, TC-702-E01〜E02
 */
export function ScatterMatrix({ engine, currentStudy }: ScatterMatrixProps) {
  // 【表示モード状態】: デフォルトは Mode 1（変数×変数）
  const [mode, setMode] = useState<ScatterMode>('mode1')

  // 【ソート順状態】: デフォルトはアルファベット順
  const [sortOrder, setSortOrder] = useState<SortOrder>('alphabetical')

  // 【空状態UI】: Study がない場合はメッセージを表示 🟢
  if (!currentStudy) {
    return (
      <div style={{ padding: '12px' }}>
        <span>Data not loaded</span>
      </div>
    )
  }

  // 【軸計算】: モードとソート順から行・列軸名を取得する
  const { rowAxes, colAxes } = getAxesForMode(currentStudy, mode, sortOrder)

  // -------------------------------------------------------------------------
  // レンダリング
  // -------------------------------------------------------------------------

  return (
    <div
      data-testid="scatter-matrix"
      style={{ display: 'flex', flexDirection: 'column', height: '100%' }}
    >
      {/* 【コントロールバー】: モード切り替え + ソート選択 */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: '8px',
          padding: '8px',
          borderBottom: '1px solid #e5e7eb',
          flexShrink: 0,
        }}
      >
        {/* 【モードボタン群】: 3 表示モードを切り替えるボタン 🟢 */}
        <div style={{ display: 'flex', gap: '4px' }}>
          {(['mode1', 'mode2', 'mode3'] as ScatterMode[]).map((m) => (
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

        {/* 【ソートセレクタ】: 軸の並び順を選択する 🟢 */}
        <select
          data-testid="sort-select"
          value={sortOrder}
          onChange={(e) => setSortOrder(e.target.value as SortOrder)}
          style={{
            fontSize: '12px',
            padding: '4px',
            border: '1px solid #d1d5db',
            borderRadius: '4px',
          }}
        >
          {(['alphabetical', 'correlation', 'importance'] as SortOrder[]).map((s) => (
            <option key={s} value={s}>
              {SORT_LABELS[s]}
            </option>
          ))}
        </select>
      </div>

      {/* 【グリッドコンテナ】: 散布図セルを N×M に並べる 🟢 */}
      <div
        data-testid="scatter-grid"
        style={{
          display: 'grid',
          gridTemplateColumns: `repeat(${colAxes.length}, 80px)`,
          gridAutoRows: '80px',
          overflow: 'auto',
          flex: 1,
        }}
      >
        {/* 【セル生成】: rowAxes × colAxes の全組み合わせでセルを生成する */}
        {rowAxes.flatMap((yAxis, row) =>
          colAxes.map((xAxis, col) => (
            <ScatterCell
              key={`${row}-${col}`}
              row={row}
              col={col}
              xAxis={xAxis}
              yAxis={yAxis}
              engine={engine}
            />
          )),
        )}
      </div>
    </div>
  )
}
