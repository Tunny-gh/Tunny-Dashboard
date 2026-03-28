/**
 * ParetoScatter2D — deck.gl ScatterplotLayer 2D散布図 (TASK-501)
 *
 * 【役割】: GpuBuffer の positions/colors/sizes を使った2D点群描画
 * 【設計方針】: selectionStore.subscribe() で直接GPUバッファを更新（React再レンダリングなし）
 * 🟢 ParetoScatter3D と同じ Brushing パターンを2D用に適用
 */

import { useEffect, useRef } from 'react';
import { DeckGL, ScatterplotLayer } from 'deck.gl';
import { useSelectionStore } from '../../stores/selectionStore';
import type { GpuBuffer } from '../../wasm/gpuBuffer';
import type { Study } from '../../types';

// -------------------------------------------------------------------------
// Props 型定義
// -------------------------------------------------------------------------

export interface ParetoScatter2DProps {
  /** 🟢 GPU バッファ — null のとき空状態UIを表示 */
  gpuBuffer: GpuBuffer | null;
  /** 🟢 現在の Study — 軸名取得用 */
  currentStudy: Study | null;
}

// -------------------------------------------------------------------------
// コンポーネント実装
// -------------------------------------------------------------------------

/**
 * 【機能概要】: deck.gl ScatterplotLayer を使った2D Pareto散布図
 * 【Brushing連携】: selectionStore.subscribe() で selectedIndices 変化時に alpha を更新
 * 【テスト対応】: TC-501-04, TC-501-E02
 */
export function ParetoScatter2D({ gpuBuffer }: ParetoScatter2DProps) {
  // 【購読参照】: unsubscribe 関数を保持するための ref
  const unsubscribeRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    // 【Brushing購読設定】: selectionStore の selectedIndices 変化を購読して alpha を更新 🟢
    if (gpuBuffer) {
      const unsubscribe = useSelectionStore.subscribe(
        (state) => state.selectedIndices,
        (indices) => gpuBuffer.updateAlphas(indices),
      );
      unsubscribeRef.current = unsubscribe;
    }

    // 【クリーンアップ】: アンマウント時に購読解除してメモリリークを防ぐ
    return () => {
      if (unsubscribeRef.current) {
        unsubscribeRef.current();
        unsubscribeRef.current = null;
      }
    };
  }, [gpuBuffer]);

  // 【空状態UI】: データがない場合はメッセージを表示 🟢
  if (!gpuBuffer) {
    return (
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
        <span>データがありません</span>
      </div>
    );
  }

  // 【deck.gl レイヤー定義】: ScatterplotLayer に GpuBuffer データを渡す 🟢
  const layer = new ScatterplotLayer({
    id: 'pareto-2d',
    data: { length: gpuBuffer.trialCount },
    getPosition: (_: unknown, { index }: { index: number }) => [
      gpuBuffer.positions[index * 2],
      gpuBuffer.positions[index * 2 + 1],
      0,
    ],
    getColor: (_: unknown, { index }: { index: number }) => [
      gpuBuffer.colors[index * 4] * 255,
      gpuBuffer.colors[index * 4 + 1] * 255,
      gpuBuffer.colors[index * 4 + 2] * 255,
      gpuBuffer.colors[index * 4 + 3] * 255,
    ],
    getRadius: (_: unknown, { index }: { index: number }) => gpuBuffer.sizes[index],
    pickable: true,
  });

  // 【DeckGL レンダリング】: ScatterplotLayer を描画する 🟢
  return (
    <DeckGL
      layers={[layer]}
      controller={true}
      style={{ width: '100%', height: '100%' }}
    />
  );
}
