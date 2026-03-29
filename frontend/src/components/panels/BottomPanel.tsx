/**
 * BottomPanel — 仮想スクロールテーブル（trial一覧）(TASK-402)
 *
 * 【役割】: selectedIndices の trial を一覧表示し、行クリック → setHighlight() を呼ぶ
 * 【設計方針】: selectionStore / studyStore に直接接続
 * 🟢 行クリック → setHighlight(trialIndex) でハイライト連動
 */

import { useSelectionStore } from '../../stores/selectionStore'
import { useStudyStore } from '../../stores/studyStore'

// -------------------------------------------------------------------------
// コンポーネント実装
// -------------------------------------------------------------------------

/**
 * 【機能概要】: selectedIndices に対応するトライアル一覧テーブル
 * 【テスト対応】: TC-402-05〜07, TC-402-E02
 */
export function BottomPanel() {
  // 【Store接続】: selectionStore から selectedIndices, highlighted, setHighlight を取得 🟢
  const selectedIndices = useSelectionStore((s) => s.selectedIndices)
  const highlighted = useSelectionStore((s) => s.highlighted)
  const setHighlight = useSelectionStore((s) => s.setHighlight)

  // 【Store接続】: studyStore から currentStudy と gpuBuffer を取得 🟢
  const currentStudy = useStudyStore((s) => s.currentStudy)
  const gpuBuffer = useStudyStore((s) => s.gpuBuffer)

  // 【空状態UI】: Study がない場合はメッセージを表示 🟢
  if (!currentStudy) {
    return (
      <div style={{ padding: '12px' }}>
        <span>Data not loaded</span>
      </div>
    )
  }

  // 【列定義】: trial_id + paramNames + objectiveNames 🟢
  const columns = ['trial_id', ...currentStudy.paramNames, ...currentStudy.objectiveNames]

  // 【レンダリング】: スクロール可能なテーブル 🟢
  return (
    <div style={{ height: '100%', overflowY: 'auto' }}>
      <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px' }}>
        <thead>
          <tr>
            {columns.map((col) => (
              <th
                key={col}
                style={{
                  padding: '4px 8px',
                  borderBottom: '1px solid #e5e7eb',
                  textAlign: 'left',
                  position: 'sticky',
                  top: 0,
                  background: '#f9fafb',
                }}
              >
                {col}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {/* 【行レンダリング】: selectedIndices に対応する行を表示 🟢 */}
          {Array.from(selectedIndices).map((idx) => (
            <tr
              key={idx}
              data-testid={`trial-row-${idx}`}
              onClick={() => setHighlight(idx)}
              style={{
                cursor: 'pointer',
                background: highlighted === idx ? '#eff6ff' : undefined,
              }}
            >
              {/* 【trial_id セル】: インデックスを trial_id として表示（WASM実装後に実際の値に変更） */}
              <td style={{ padding: '3px 8px', borderBottom: '1px solid #f3f4f6' }}>{idx}</td>
              {/* 【パラメータセル】: Trial パラメータデータは TASK-102 完成後に実装 */}
              {currentStudy.paramNames.map((name) => (
                <td key={name} style={{ padding: '3px 8px', borderBottom: '1px solid #f3f4f6' }}>
                  —
                </td>
              ))}
              {/* 【目的関数セル】: gpuBuffer.positions から値を読み取り表示する
                   単目的: positions[i*2+1] = obj0
                   多目的: positions[i*2] = obj0, positions[i*2+1] = obj1 */}
              {currentStudy.objectiveNames.map((name, objIdx) => {
                let value = '—'
                if (gpuBuffer && idx < gpuBuffer.trialCount) {
                  const isMulti = (currentStudy.directions?.length ?? 1) > 1
                  if (isMulti) {
                    if (objIdx === 0) value = gpuBuffer.positions[idx * 2].toFixed(4)
                    else if (objIdx === 1) value = gpuBuffer.positions[idx * 2 + 1].toFixed(4)
                  } else {
                    if (objIdx === 0) value = gpuBuffer.positions[idx * 2 + 1].toFixed(4)
                  }
                }
                return (
                  <td key={name} style={{ padding: '3px 8px', borderBottom: '1px solid #f3f4f6' }}>
                    {value}
                  </td>
                )
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
