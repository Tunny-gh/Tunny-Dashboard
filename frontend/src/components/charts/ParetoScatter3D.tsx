/**
 * ParetoScatter3D — deck.gl PointCloudLayer 3D散布図 (TASK-501)
 *
 * 【役割】: GpuBuffer の positions3d/colors/sizes を使った3D点群描画
 * 【設計方針】: selectionStore.subscribe() で直接GPUバッファを更新（React再レンダリングなし）
 * 🟢 Brushing & Linking は React サイクル外で GPU alpha を直接更新する
 */

import { useEffect, useRef } from 'react';
import { DeckGL, PointCloudLayer } from 'deck.gl';
import { useSelectionStore } from '../../stores/selectionStore';
import type { GpuBuffer } from '../../wasm/gpuBuffer';
import type { Study } from '../../types';

// -------------------------------------------------------------------------
// Props 型定義
// -------------------------------------------------------------------------

export interface ParetoScatter3DProps {
  /** 🟢 GPU バッファ — null のとき空状態UIを表示 */
  gpuBuffer: GpuBuffer | null;
  /** 🟢 現在の Study — 軸名取得用 */
  currentStudy: Study | null;
}

// -------------------------------------------------------------------------
// コンポーネント実装
// -------------------------------------------------------------------------

/**
 * 【機能概要】: deck.gl PointCloudLayer を使った3D Pareto散布図
 * 【Brushing連携】: selectionStore.subscribe() で selectedIndices 変化時に alpha を更新
 * 【テスト対応】: TC-501-01〜03, TC-501-B01, TC-501-E01
 */
export function ParetoScatter3D({ gpuBuffer }: ParetoScatter3DProps) {
  // 【購読参照】: unsubscribe 関数を保持するための ref
  const unsubscribeRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    // 【Brushing購読設定】: selectionStore の selectedIndices 変化を購読して alpha を更新 🟢
    // React再レンダリングなしに GPU バッファを直接更新することで 60fps を維持する
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

  // 【deck.gl レイヤー定義】: PointCloudLayer に GpuBuffer データを渡す 🟢
  const layer = new PointCloudLayer({
    id: 'pareto-3d',
    data: { length: gpuBuffer.trialCount },
    getPosition: (_: unknown, { index }: { index: number }) => [
      gpuBuffer.positions3d[index * 3],
      gpuBuffer.positions3d[index * 3 + 1],
      gpuBuffer.positions3d[index * 3 + 2],
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

  // 【DeckGL レンダリング】: PointCloudLayer を描画する 🟢
  return (
    <DeckGL
      layers={[layer]}
      controller={true}
      style={{ width: '100%', height: '100%' }}
    />
  );
}
