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

  // 【Store接続】: studyStore から currentStudy と trialRows を取得 🟢
  const currentStudy = useStudyStore((s) => s.currentStudy)
  const trialRows = useStudyStore((s) => s.trialRows)

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
              {/* 【trial_id セル】: trialRows から実際の trial_id を表示 */}
              <td style={{ padding: '3px 8px', borderBottom: '1px solid #f3f4f6' }}>
                {trialRows[idx]?.trialId ?? idx}
              </td>
              {/* 【パラメータセル】: trialRows の params から実データを表示 🟢 */}
              {currentStudy.paramNames.map((name) => {
                const trial = trialRows[idx]
                const raw = trial?.params[name]
                const value = raw !== undefined ? Number(raw) : undefined
                return (
                  <td key={name} style={{ padding: '3px 8px', borderBottom: '1px solid #f3f4f6' }}>
                    {value !== undefined && Number.isFinite(value) ? value.toPrecision(6) : '—'}
                  </td>
                )
              })}
              {/* 【目的関数セル】: trialRows の values から実データを表示 🟢 */}
              {currentStudy.objectiveNames.map((name, objIdx) => {
                const trial = trialRows[idx]
                const value = trial?.values[objIdx]
                return (
                  <td key={name} style={{ padding: '3px 8px', borderBottom: '1px solid #f3f4f6' }}>
                    {value !== undefined && Number.isFinite(value) ? value.toFixed(4) : '—'}
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
