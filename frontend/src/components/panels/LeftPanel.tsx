/**
 * LeftPanel — Study情報カウンタ・フィルタスライダー・カラーリング選択 (TASK-402)
 *
 * 【役割】: selected件数表示 / 変数フィルタスライダー / カラーモード選択
 * 【設計方針】: selectionStore / studyStore に直接接続
 * 🟢 スライダー変更 → addAxisFilter(axis, 0, value) でリアルタイムフィルタ
 */

import { useSelectionStore } from '../../stores/selectionStore';
import { useStudyStore } from '../../stores/studyStore';
import type { ColorMode } from '../../types';
import type { ChangeEvent } from 'react';

// -------------------------------------------------------------------------
// カラーモード定義
// -------------------------------------------------------------------------

/** 🟢 選択可能なカラーモード一覧 */
const COLOR_MODES: ColorMode[] = ['objective', 'cluster', 'rank', 'generation'];

// -------------------------------------------------------------------------
// コンポーネント実装
// -------------------------------------------------------------------------

/**
 * 【機能概要】: 左側パネル — カウンタ / フィルタスライダー / カラーモード選択
 * 【テスト対応】: TC-402-01〜04, TC-402-E01
 */
export function LeftPanel() {
  // 【Store接続】: selectionStore から selectedIndices, colorMode, addAxisFilter を取得 🟢
  const selectedIndices = useSelectionStore((s) => s.selectedIndices);
  const colorMode = useSelectionStore((s) => s.colorMode);
  const addAxisFilter = useSelectionStore((s) => s.addAxisFilter);
  const setColorMode = useSelectionStore((s) => s.setColorMode);

  // 【Store接続】: studyStore から currentStudy を取得 🟢
  const currentStudy = useStudyStore((s) => s.currentStudy);

  // 【空状態UI】: Study がない場合はメッセージを表示 🟢
  if (!currentStudy) {
    return (
      <div style={{ padding: '12px' }}>
        <span>データが読み込まれていません</span>
      </div>
    );
  }

  /**
   * 【スライダーハンドラ】: 軸フィルタを 0〜value の範囲で適用
   * 【設計方針】: 0 を min 固定とし、スライダー値を max として addAxisFilter を呼ぶ
   */
  const handleSliderChange = (axisName: string) => (e: ChangeEvent<HTMLInputElement>) => {
    addAxisFilter(axisName, 0, parseFloat(e.target.value));
  };

  // 【レンダリング】: カウンタ + スライダー + カラーモード 🟢
  return (
    <div style={{ padding: '12px', display: 'flex', flexDirection: 'column', gap: '16px' }}>
      {/* 【選択件数カウンタ】: selectedIndices.length をリアルタイム表示 🟢 */}
      <div>
        <div style={{ fontSize: '12px', color: '#6b7280' }}>Selected</div>
        <div data-testid="selected-count" style={{ fontSize: '20px', fontWeight: 'bold' }}>
          {selectedIndices.length}
        </div>
      </div>

      {/* 【パラメータフィルタスライダー群】: 各変数のフィルタスライダー 🟢 */}
      <div>
        <div style={{ fontSize: '12px', color: '#6b7280', marginBottom: '4px' }}>Parameters</div>
        {currentStudy.paramNames.map((name) => (
          <div key={name} style={{ marginBottom: '8px' }}>
            <label style={{ fontSize: '13px' }}>{name}</label>
            <input
              data-testid={`slider-${name}`}
              type="range"
              min="0"
              max="1"
              step="0.01"
              defaultValue="1"
              onChange={handleSliderChange(name)}
              style={{ width: '100%' }}
            />
          </div>
        ))}
      </div>

      {/* 【カラーモード選択】: ラジオボタンで4モードを切り替え 🟢 */}
      <div>
        <div style={{ fontSize: '12px', color: '#6b7280', marginBottom: '4px' }}>Color Mode</div>
        {COLOR_MODES.map((mode) => (
          <label key={mode} style={{ display: 'flex', alignItems: 'center', gap: '4px', fontSize: '13px' }}>
            <input
              data-testid={`color-mode-${mode}`}
              type="radio"
              name="color-mode"
              value={mode}
              checked={colorMode === mode}
              onChange={() => setColorMode(mode)}
            />
            {mode}
          </label>
        ))}
      </div>
    </div>
  );
}
