/**
 * ExportPanel — CSV エクスポート・ピン留め UI (TASK-1101)
 *
 * 【役割】: CSV エクスポート設定・ピン留み管理のUIパネル
 * 【設計方針】:
 *   - exportStore に直接接続
 *   - 対象選択ラジオ + 列選択チェックボックス + ダウンロードボタン
 *   - ピン留めリスト（trial_id・メモ欄・削除ボタン）
 * 🟢 REQ-150〜REQ-153, REQ-156 に準拠
 */

import { useExportStore, MAX_PINS } from '../../stores/exportStore'
import type { CsvTarget } from '../../stores/exportStore'
import { useSelectionStore } from '../../stores/selectionStore'
import { useStudyStore } from '../../stores/studyStore'
import type { ChangeEvent } from 'react'

// -------------------------------------------------------------------------
// 定数
// -------------------------------------------------------------------------

/** 【CSV 対象ラベル】: CsvTarget → 日本語表示名 */
const CSV_TARGET_LABELS: Record<CsvTarget, string> = {
  all: 'All',
  selected: 'Selected',
  pareto: 'Pareto Only',
  cluster: 'By Cluster',
}

// -------------------------------------------------------------------------
// メインコンポーネント
// -------------------------------------------------------------------------

/**
 * 【機能概要】: CSV エクスポートパネル
 * 【UI 構成】: 対象選択 → 列選択 → ダウンロードボタン → エラー表示 → ピン留めリスト
 * 🟢 REQ-150〜REQ-153, REQ-156 に準拠
 */
export function ExportPanel() {
  // 【Store 接続】: exportStore から状態・アクションを取得 🟢
  const csvTarget = useExportStore((s) => s.csvTarget)
  const selectedColumns = useExportStore((s) => s.selectedColumns)
  const isExporting = useExportStore((s) => s.isExporting)
  const exportError = useExportStore((s) => s.exportError)
  const pinnedTrials = useExportStore((s) => s.pinnedTrials)
  const pinError = useExportStore((s) => s.pinError)
  const setCsvTarget = useExportStore((s) => s.setCsvTarget)
  const setSelectedColumns = useExportStore((s) => s.setSelectedColumns)
  const exportCsv = useExportStore((s) => s.exportCsv)
  const unpinTrial = useExportStore((s) => s.unpinTrial)
  const updatePinMemo = useExportStore((s) => s.updatePinMemo)
  const clearExportError = useExportStore((s) => s.clearExportError)
  const clearPinError = useExportStore((s) => s.clearPinError)

  // 【Store 接続】: selectionStore から selectedIndices を取得 🟢
  const selectedIndices = useSelectionStore((s) => s.selectedIndices)

  // 【Store 接続】: studyStore から列名リストを取得 🟢
  const currentStudy = useStudyStore((s) => s.currentStudy)

  // 【エクスポート実行】: 対象に応じたインデックスを渡す
  const handleExport = () => {
    // 【対象インデックス決定】: selected 以外は全件（TASK-302/901 で絞り込み予定）
    const indices = csvTarget === 'selected' ? selectedIndices : selectedIndices
    exportCsv(indices)
  }

  // 【列選択トグル】: チェックボックスの on/off で selectedColumns を更新
  const handleColumnToggle = (colName: string) => (e: ChangeEvent<HTMLInputElement>) => {
    if (e.target.checked) {
      setSelectedColumns([...selectedColumns, colName])
    } else {
      setSelectedColumns(selectedColumns.filter((c) => c !== colName))
    }
  }

  // 【全列名リスト】: Study があれば trial_id + params + objectives を列挙
  const allColumns = currentStudy
    ? ['trial_id', ...currentStudy.paramNames, ...currentStudy.objectiveNames]
    : ['trial_id']

  return (
    <div
      data-testid="export-panel"
      style={{ padding: '12px', display: 'flex', flexDirection: 'column', gap: '12px' }}
    >
      {/* 【CSV 対象選択】: ラジオボタン 🟢 REQ-150 */}
      <div>
        <div style={{ fontSize: '12px', color: '#6b7280', marginBottom: '4px' }}>Target</div>
        {(Object.keys(CSV_TARGET_LABELS) as CsvTarget[]).map((t) => (
          <label
            key={t}
            style={{ display: 'flex', alignItems: 'center', gap: '4px', fontSize: '13px' }}
          >
            <input
              data-testid={`csv-target-${t}`}
              type="radio"
              name="csv-target"
              value={t}
              checked={csvTarget === t}
              onChange={() => setCsvTarget(t)}
            />
            {CSV_TARGET_LABELS[t]}
          </label>
        ))}
      </div>

      {/* 【列選択】: チェックボックス 🟢 REQ-152 */}
      <div>
        <div style={{ fontSize: '12px', color: '#6b7280', marginBottom: '4px' }}>
          Columns (empty=all)
        </div>
        {allColumns.map((col) => (
          <label
            key={col}
            style={{ display: 'flex', alignItems: 'center', gap: '4px', fontSize: '13px' }}
          >
            <input
              data-testid={`col-checkbox-${col}`}
              type="checkbox"
              checked={selectedColumns.includes(col)}
              onChange={handleColumnToggle(col)}
            />
            {col}
          </label>
        ))}
      </div>

      {/* 【ダウンロードボタン】: 実行中は disabled 🟢 */}
      <button
        data-testid="export-csv-btn"
        onClick={handleExport}
        disabled={isExporting}
        style={{
          padding: '6px 16px',
          fontSize: '13px',
          background: isExporting ? '#9ca3af' : '#059669',
          color: '#fff',
          border: 'none',
          borderRadius: '4px',
          cursor: isExporting ? 'not-allowed' : 'pointer',
          alignSelf: 'flex-start',
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
        }}
      >
        {/* 【スピナー】: エクスポート中のみ表示 */}
        {isExporting && (
          <span
            data-testid="export-spinner"
            style={{
              display: 'inline-block',
              width: '12px',
              height: '12px',
              border: '2px solid #fff',
              borderTopColor: 'transparent',
              borderRadius: '50%',
              animation: 'spin 1s linear infinite',
            }}
          />
        )}
        {isExporting ? 'Exporting...' : 'Download CSV'}
      </button>

      {/* 【エクスポートエラー】: 対象0件や WASM エラー 🟢 REQ-150 */}
      {exportError && (
        <div
          data-testid="export-error"
          style={{
            padding: '8px',
            background: '#fef2f2',
            border: '1px solid #fca5a5',
            borderRadius: '4px',
            fontSize: '12px',
            color: '#dc2626',
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
          }}
        >
          <span>{exportError}</span>
          <button
            onClick={clearExportError}
            style={{ background: 'none', border: 'none', cursor: 'pointer', fontSize: '14px' }}
          >
            ×
          </button>
        </div>
      )}

      {/* 【ピン留めリスト】: 最大 MAX_PINS 件 🟢 REQ-156 */}
      <div>
        <div
          style={{
            fontSize: '12px',
            color: '#6b7280',
            marginBottom: '4px',
            display: 'flex',
            justifyContent: 'space-between',
          }}
        >
          <span>
            Pins ({pinnedTrials.length}/{MAX_PINS})
          </span>
        </div>

        {/* 【ピン上限エラー】 */}
        {pinError && (
          <div
            data-testid="pin-error"
            style={{
              padding: '6px 8px',
              background: '#fef3c7',
              border: '1px solid #f59e0b',
              borderRadius: '4px',
              fontSize: '12px',
              color: '#92400e',
              marginBottom: '4px',
              display: 'flex',
              justifyContent: 'space-between',
            }}
          >
            <span>{pinError}</span>
            <button
              onClick={clearPinError}
              style={{ background: 'none', border: 'none', cursor: 'pointer', fontSize: '14px' }}
            >
              ×
            </button>
          </div>
        )}

        {/* 【ピン留めテーブル】: trial_id・メモ欄・削除ボタン */}
        {pinnedTrials.length === 0 ? (
          <div style={{ fontSize: '12px', color: '#9ca3af', padding: '8px 0' }}>No pins yet</div>
        ) : (
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px' }}>
            <thead>
              <tr style={{ background: '#f9fafb' }}>
                <th
                  style={{
                    padding: '3px 6px',
                    textAlign: 'left',
                    borderBottom: '1px solid #e5e7eb',
                  }}
                >
                  trial_id
                </th>
                <th
                  style={{
                    padding: '3px 6px',
                    textAlign: 'left',
                    borderBottom: '1px solid #e5e7eb',
                  }}
                >
                  Memo
                </th>
                <th style={{ borderBottom: '1px solid #e5e7eb', width: '24px' }} />
              </tr>
            </thead>
            <tbody>
              {pinnedTrials.map((pin) => (
                <tr
                  key={pin.index}
                  data-testid={`pinned-row-${pin.index}`}
                  style={{ borderBottom: '1px solid #f3f4f6' }}
                >
                  <td style={{ padding: '3px 6px' }}>{pin.trialId}</td>
                  <td style={{ padding: '3px 6px' }}>
                    <input
                      data-testid={`pin-memo-${pin.index}`}
                      type="text"
                      value={pin.memo}
                      onChange={(e) => updatePinMemo(pin.index, e.target.value)}
                      placeholder="Enter memo..."
                      style={{
                        width: '100%',
                        border: '1px solid #e5e7eb',
                        borderRadius: '3px',
                        padding: '2px 4px',
                        fontSize: '11px',
                      }}
                    />
                  </td>
                  <td style={{ padding: '3px 6px' }}>
                    <button
                      data-testid={`unpin-btn-${pin.index}`}
                      onClick={() => unpinTrial(pin.index)}
                      style={{
                        background: 'none',
                        border: 'none',
                        cursor: 'pointer',
                        color: '#9ca3af',
                        fontSize: '14px',
                        padding: '0 2px',
                      }}
                      title="Unpin"
                    >
                      ×
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  )
}
