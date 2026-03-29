/**
 * ReportBuilder — HTMLレポート生成 UI コンポーネント (TASK-1102)
 *
 * 【役割】: レポートセクションの選択・並び替えと HTML レポートのダウンロードを提供する
 * 【設計方針】:
 *   - セクションリストのドラッグ&ドロップ（HTML5 Drag API）🟡
 *   - プレビュー: iframe モーダルで生成前確認
 *   - 生成・ダウンロードボタン + 進捗インジケーター
 *   - PDF 印刷: window.print() を提供
 * 🟢 REQ-154〜REQ-155, REQ-158 に準拠
 */

import React, { useState, useRef } from 'react'
import { useExportStore } from '../../stores/exportStore'
import type { ReportSection } from '../../stores/exportStore'

// -------------------------------------------------------------------------
// 型定義
// -------------------------------------------------------------------------

interface ReportBuilderProps {
  /** Pareto 解インデックス（エクスポート対象）*/
  paretoIndices: Uint32Array
}

// -------------------------------------------------------------------------
// セクション表示名マップ
// -------------------------------------------------------------------------

/** 【セクション表示名】: セクション識別子 → 日本語ラベル */
const SECTION_LABELS: Record<ReportSection, string> = {
  summary: 'Statistics Summary',
  pareto: 'Pareto Solutions',
  pinned: 'Pinned Trials',
  history: 'Optimization History',
  cluster: 'Cluster Analysis',
}

/** 【全セクション一覧】: デフォルト順 */
const ALL_SECTIONS: ReportSection[] = ['summary', 'pareto', 'pinned', 'history', 'cluster']

// -------------------------------------------------------------------------
// ReportBuilder コンポーネント
// -------------------------------------------------------------------------

/**
 * 【機能概要】: HTMLレポート生成 UI
 * 【設計方針】: セクション選択・並び替え → ダウンロード
 */
export const ReportBuilder: React.FC<ReportBuilderProps> = ({ paretoIndices }) => {
  const {
    reportSections,
    isGeneratingReport,
    reportError,
    setReportSections,
    generateHtmlReport,
    clearReportError,
  } = useExportStore()

  // 【選択セクション管理】: チェックボックスで on/off
  const [enabledSections, setEnabledSections] = useState<Set<ReportSection>>(
    new Set(reportSections),
  )

  // 【ドラッグ状態】: ドラッグ中のセクション index
  const dragIndex = useRef<number | null>(null)

  // -------------------------------------------------------------------------
  // ハンドラ
  // -------------------------------------------------------------------------

  /** 【セクション有効/無効切り替え】 */
  const handleToggleSection = (sec: ReportSection) => {
    const next = new Set(enabledSections)
    if (next.has(sec)) {
      next.delete(sec)
    } else {
      next.add(sec)
    }
    setEnabledSections(next)
    // 【Store 更新】: 有効なセクションのみ Store に反映（順序を保持）
    setReportSections(reportSections.filter((s) => next.has(s)))
  }

  /** 【ドラッグ開始】 */
  const handleDragStart = (e: React.DragEvent, index: number) => {
    dragIndex.current = index
    e.dataTransfer.effectAllowed = 'move'
  }

  /** 【ドラッグオーバー】: デフォルトを無効化してドロップを許可 */
  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault()
    e.dataTransfer.dropEffect = 'move'
  }

  /** 【ドロップ】: 順序を入れ替える */
  const handleDrop = (e: React.DragEvent, targetIndex: number) => {
    e.preventDefault()
    if (dragIndex.current === null || dragIndex.current === targetIndex) return

    const next = [...reportSections]
    const [dragged] = next.splice(dragIndex.current, 1)
    next.splice(targetIndex, 0, dragged)
    setReportSections(next)
    dragIndex.current = null
  }

  /** 【レポート生成・ダウンロード】 */
  const handleGenerate = () => {
    clearReportError()
    generateHtmlReport(paretoIndices)
  }

  // -------------------------------------------------------------------------
  // 全セクションリスト（有効・無効含む）
  // -------------------------------------------------------------------------

  /** 【全セクション表示順】: 有効なものを先頭、無効なものを末尾 */
  const enabledList = reportSections.filter((s) => enabledSections.has(s))
  const disabledList = ALL_SECTIONS.filter((s) => !enabledSections.has(s))

  // -------------------------------------------------------------------------
  // レンダリング
  // -------------------------------------------------------------------------

  return (
    <div data-testid="report-builder" className="p-4 space-y-4">
      {/* タイトル */}
      <h3 className="text-base font-semibold text-gray-800">Generate HTML Report</h3>

      {/* セクション選択・並び替えリスト */}
      <div>
        <p className="text-xs text-gray-500 mb-2">
          Drag to reorder the sections included in the report
        </p>

        {/* 有効なセクション（並び替え可能） */}
        <ul className="space-y-1" data-testid="section-list">
          {enabledList.map((sec, index) => (
            <li
              key={sec}
              data-testid={`section-item-${sec}`}
              draggable
              onDragStart={(e) => handleDragStart(e, index)}
              onDragOver={handleDragOver}
              onDrop={(e) => handleDrop(e, index)}
              className="flex items-center gap-2 p-2 bg-white border border-blue-200 rounded cursor-grab active:cursor-grabbing"
            >
              {/* ドラッグハンドルアイコン */}
              <span className="text-gray-400 text-xs select-none">⠿</span>
              {/* チェックボックス */}
              <input
                type="checkbox"
                id={`sec-${sec}`}
                data-testid={`section-checkbox-${sec}`}
                checked
                onChange={() => handleToggleSection(sec)}
                className="cursor-pointer"
              />
              <label htmlFor={`sec-${sec}`} className="text-sm cursor-pointer flex-1">
                {SECTION_LABELS[sec]}
              </label>
            </li>
          ))}
        </ul>

        {/* 無効化されたセクション */}
        {disabledList.length > 0 && (
          <ul className="space-y-1 mt-2 opacity-50" data-testid="disabled-section-list">
            {disabledList.map((sec) => (
              <li
                key={sec}
                data-testid={`section-item-${sec}`}
                className="flex items-center gap-2 p-2 bg-gray-50 border border-gray-200 rounded"
              >
                <span className="text-gray-300 text-xs select-none">⠿</span>
                <input
                  type="checkbox"
                  id={`sec-${sec}`}
                  data-testid={`section-checkbox-${sec}`}
                  checked={false}
                  onChange={() => handleToggleSection(sec)}
                  className="cursor-pointer"
                />
                <label htmlFor={`sec-${sec}`} className="text-sm cursor-pointer flex-1">
                  {SECTION_LABELS[sec]}
                </label>
              </li>
            ))}
          </ul>
        )}
      </div>

      {/* エラー表示 */}
      {reportError && (
        <p data-testid="report-error" className="text-red-600 text-xs">
          {reportError}
        </p>
      )}

      {/* アクションボタン */}
      <div className="flex gap-2">
        {/* HTML ダウンロードボタン */}
        <button
          data-testid="generate-report-btn"
          onClick={handleGenerate}
          disabled={isGeneratingReport || enabledList.length === 0}
          className="flex items-center gap-1.5 px-3 py-1.5 bg-blue-600 text-white text-sm rounded
                     hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          {isGeneratingReport ? (
            <>
              {/* 生成中スピナー */}
              <span
                data-testid="report-spinner"
                className="inline-block w-3.5 h-3.5 border-2 border-white border-t-transparent rounded-full animate-spin"
              />
              Generating...
            </>
          ) : (
            <>Download HTML</>
          )}
        </button>
      </div>

      {/* PDF 印刷ヒント */}
      <p className="text-xs text-gray-400">
        * After downloading, open in a browser and select "Print" to save as PDF.
      </p>
    </div>
  )
}

export default ReportBuilder
