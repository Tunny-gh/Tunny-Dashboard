/**
 * ObjectivePairMatrix — N×N目的ペア行列 (TASK-502)
 *
 * 【役割】: 目的関数間の散布図行列を表示する
 *         - 対角セル: 1D分布ヒストグラム（目的名ラベル）
 *         - 下三角セル: 2D散布図（deck.gl ScatterplotLayer）
 * 【設計方針】: gpuBuffer と currentStudy を props で受け取り、直接描画する
 * 🟢 REQ-070, REQ-075 に準拠
 */

import { DeckGL, ScatterplotLayer } from 'deck.gl';
import type { GpuBuffer } from '../../wasm/gpuBuffer';
import type { Study } from '../../types';

// -------------------------------------------------------------------------
// Props 型定義
// -------------------------------------------------------------------------

/**
 * 【Props】: ObjectivePairMatrix コンポーネントのプロパティ
 */
export interface ObjectivePairMatrixProps {
  /** 🟢 GPU バッファ — null のとき散布図セルは空表示 */
  gpuBuffer: GpuBuffer | null;
  /** 🟢 現在の Study — 目的名・目的数の取得用 */
  currentStudy: Study | null;
  /**
   * 【セルクリックコールバック】: セル選択時に xAxisName, yAxisName を通知する
   * 🟢 呼び出し元（AppShell 等）が 3D ビューの軸割り当てに使用する
   */
  onCellClick?: (xAxisName: string, yAxisName: string) => void;
}

// -------------------------------------------------------------------------
// コンポーネント実装
// -------------------------------------------------------------------------

/**
 * 【機能概要】: 目的ペアごとの散布図行列コンポーネント
 * 【グリッド構造】:
 *   - 対角 (row === col): 目的名ラベル / ヒストグラムプレースホルダー
 *   - 下三角 (row > col): deck.gl ScatterplotLayer による 2D 散布図
 *   - 上三角 (row < col): 空セル（将来的に統計情報を表示予定）
 * 【表示制御】:
 *   - 1目的以下: null を返しコンポーネント自体を非表示にする
 *   - currentStudy=null: 「データが読み込まれていません」を表示
 * 【テスト対応】: TC-502-01〜04, TC-502-E01〜E02
 */
export function ObjectivePairMatrix({ gpuBuffer, currentStudy, onCellClick }: ObjectivePairMatrixProps) {
  // 【空状態UI】: Study がない場合はメッセージを表示 🟢
  if (!currentStudy) {
    return (
      <div style={{ padding: '12px' }}>
        <span>データが読み込まれていません</span>
      </div>
    );
  }

  const { objectiveNames } = currentStudy;
  const n = objectiveNames.length;

  // 【1目的以下】: ペア行列が意味をなさないため非表示にする 🟢
  if (n <= 1) return null;

  // -------------------------------------------------------------------------
  // セル生成
  // -------------------------------------------------------------------------

  /**
   * 【セル配列生成】: n×n の全セルをフラット配列として生成する
   * インデックス変換: row = Math.floor(idx / n), col = idx % n
   */
  const cells = Array.from({ length: n * n }, (_, idx) => {
    const row = Math.floor(idx / n);
    const col = idx % n;
    const xAxis = objectiveNames[col]; // 【x軸】: 列方向の目的名
    const yAxis = objectiveNames[row]; // 【y軸】: 行方向の目的名

    return (
      <div
        key={`${row}-${col}`}
        data-testid={`matrix-cell-${row}-${col}`}
        onClick={() => onCellClick?.(xAxis, yAxis)}
        style={{
          cursor: 'pointer',
          position: 'relative',
          border: '1px solid #e5e7eb',
          overflow: 'hidden',
          minHeight: '80px',
        }}
      >
        {row === col ? (
          // 【対角セル】: 目的名ラベル（将来的に 1D ヒストグラムを表示） 🟢
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              height: '100%',
              padding: '4px',
              fontSize: '11px',
              fontWeight: 'bold',
              background: '#f9fafb',
              color: '#374151',
            }}
          >
            {objectiveNames[row]}
          </div>
        ) : row > col ? (
          // 【下三角セル】: deck.gl ScatterplotLayer による 2D 散布図 🟢
          gpuBuffer ? (
            <DeckGL
              layers={[
                new ScatterplotLayer({
                  id: `scatter-${row}-${col}`,
                  data: { length: gpuBuffer.trialCount },
                  getPosition: (_: unknown, { index }: { index: number }) => [
                    gpuBuffer.positions[index * 2],
                    gpuBuffer.positions[index * 2 + 1],
                    0,
                  ],
                  getColor: [79, 70, 229, 180], // 【配色】: インジゴ系（半透明）
                  getRadius: 3,
                  pickable: false,
                }),
              ]}
              controller={false}
              style={{ width: '100%', height: '100%' }}
            />
          ) : (
            // 【データなし】: gpuBuffer 未ロード時はプレースホルダーを表示
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                height: '100%',
                color: '#9ca3af',
                fontSize: '10px',
              }}
            >
              —
            </div>
          )
        ) : (
          // 【上三角セル】: 現在は空（将来的に統計情報を表示予定）
          null
        )}
      </div>
    );
  });

  // -------------------------------------------------------------------------
  // レンダリング
  // -------------------------------------------------------------------------

  return (
    // 【グリッドコンテナ】: n×n CSS Grid レイアウト 🟢
    <div
      data-testid="objective-pair-matrix"
      style={{
        display: 'grid',
        gridTemplateColumns: `repeat(${n}, 1fr)`,
        gridTemplateRows: `repeat(${n}, 1fr)`,
        width: '100%',
        height: '100%',
      }}
    >
      {cells}
    </div>
  );
}
