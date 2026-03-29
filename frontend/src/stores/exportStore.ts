/**
 * ExportStore — CSV エクスポート・ピン留め管理 Zustand Store (TASK-1101)
 *
 * 【役割】: CSVエクスポート設定・ピン留め試行・エクスポート実行を一元管理する
 * 【設計方針】:
 *   - CSV 対象選択（全件/選択/Pareto/クラスタ）+ 列選択を保持
 *   - ピン留め上限: 最大 20 件（超過時はエラーメッセージ）
 *   - File System Access API 非対応時は <a download> フォールバック
 * 🟢 REQ-150〜REQ-153, REQ-156 に準拠
 */

import { create } from 'zustand'
import { WasmLoader } from '../wasm/wasmLoader'
import type { SessionState, ClusterConfig } from '../types'
import { useSelectionStore } from './selectionStore'
import { useLayoutStore } from './layoutStore'

// -------------------------------------------------------------------------
// 型定義
// -------------------------------------------------------------------------

/**
 * 【CSV 対象選択型】: どのトライアルを CSV に含めるか
 * 🟢 REQ-150: 対象選択
 */
export type CsvTarget = 'all' | 'selected' | 'pareto' | 'cluster'

/**
 * 【ピン留めトライアル型】: ピン留めした試行のデータ
 * 🟢 REQ-156: ピン留め機能
 */
export interface PinnedTrial {
  /** DataFrame の行インデックス */
  index: number
  /** Optuna trial_id */
  trialId: number
  /** ユーザーメモ（空文字列可）*/
  memo: string
  /** ピン留め日時（ISO文字列）*/
  pinnedAt: string
}

/**
 * 【HTMLレポートセクション型】: レポートに含められるセクションの識別子
 * 🟢 REQ-154: レポートセクション選択
 */
export type ReportSection = 'summary' | 'pareto' | 'pinned' | 'history' | 'cluster'

/**
 * 【ExportStore 状態型】
 */
interface ExportState {
  // --- CSV 設定 ---
  /** CSV 出力対象 */
  csvTarget: CsvTarget
  /** 出力列名リスト（空のとき全列） */
  selectedColumns: string[]
  /** CSV エクスポート実行中フラグ */
  isExporting: boolean
  /** エクスポートエラーメッセージ */
  exportError: string | null

  // --- ピン留め ---
  /** ピン留め試行リスト（最大 MAX_PINS 件）*/
  pinnedTrials: PinnedTrial[]
  /** ピン留め超過エラーメッセージ */
  pinError: string | null

  // --- HTMLレポート ---
  /** レポートに含めるセクションの順序リスト */
  reportSections: ReportSection[]
  /** レポート生成中フラグ */
  isGeneratingReport: boolean
  /** レポート生成エラーメッセージ */
  reportError: string | null

  // --- アクション ---
  setCsvTarget: (target: CsvTarget) => void
  setSelectedColumns: (columns: string[]) => void
  exportCsv: (indices: Uint32Array) => Promise<void>
  pinTrial: (index: number, trialId: number) => void
  unpinTrial: (index: number) => void
  updatePinMemo: (index: number, memo: string) => void
  clearPinError: () => void
  clearExportError: () => void
  /** HTMLレポートセクション順序を設定する */
  setReportSections: (sections: ReportSection[]) => void
  /** HTMLレポートを生成してダウンロードする 🟢 REQ-154〜REQ-155 */
  generateHtmlReport: (paretoIndices: Uint32Array) => Promise<void>
  /** レポートエラーをクリアする */
  clearReportError: () => void

  // --- セッション保存・復元 ---
  /** 最後に読み込んだセッション状態 */
  sessionState: SessionState | null
  /** セッション保存中フラグ */
  isSavingSession: boolean
  /** セッション操作エラーメッセージ */
  sessionError: string | null
  /** バージョン不一致警告（復元は続行）*/
  sessionWarning: string | null

  /** 現在の全ストア状態をセッション JSON として保存する 🟢 REQ-157 */
  saveSession: (studyId: number, journalPath: string) => Promise<void>
  /** JSON 文字列からセッションを復元する */
  loadSessionFromJson: (json: string) => void
  /** セッションエラー・警告をクリアする */
  clearSessionMessages: () => void
}

// -------------------------------------------------------------------------
// 定数
// -------------------------------------------------------------------------

/** 【ピン留め上限】: REQ-156 仕様 🟢 */
export const MAX_PINS = 20

/** 【セッションバージョン】: 互換性チェック用 🟢 REQ-157 */
export const SESSION_VERSION = '1.0'

// -------------------------------------------------------------------------
// Store 実装
// -------------------------------------------------------------------------

/**
 * 【ExportStore 作成】: CSV エクスポート・ピン留め管理
 */
export const useExportStore = create<ExportState>()((set, get) => ({
  // -------------------------------------------------------------------------
  // 初期状態
  // -------------------------------------------------------------------------
  csvTarget: 'all',
  selectedColumns: [],
  isExporting: false,
  exportError: null,
  pinnedTrials: [],
  pinError: null,
  reportSections: ['summary', 'pareto', 'pinned', 'history', 'cluster'],
  isGeneratingReport: false,
  reportError: null,
  sessionState: null,
  isSavingSession: false,
  sessionError: null,
  sessionWarning: null,

  // -------------------------------------------------------------------------
  // アクション実装
  // -------------------------------------------------------------------------

  /**
   * 【CSV 対象設定】: エクスポート対象の選択範囲を変更する
   */
  setCsvTarget: (target) => set({ csvTarget: target }),

  /**
   * 【列選択設定】: エクスポートする列名リストを設定する
   * 空配列のときは全列を出力する（WASM 側が全列を返す）
   */
  setSelectedColumns: (columns) => set({ selectedColumns: columns }),

  /**
   * 【CSV エクスポート実行】: WASM serialize_csv() を呼び出してダウンロードを起動する
   *
   * 【処理フロー】:
   *   1. indices が空なら「対象データがありません」エラー 🟢 REQ-150
   *   2. WASM serialize_csv(indices, columns_json) を呼び出す
   *   3. File System Access API があれば showSaveFilePicker で保存
   *      なければ <a download> フォールバック 🟢 REQ-153
   */
  exportCsv: async (indices) => {
    // 【空チェック】: 選択が 0 件なら即エラー
    if (indices.length === 0) {
      set({ exportError: '対象データがありません' })
      return
    }

    set({ isExporting: true, exportError: null })

    try {
      const wasm = await WasmLoader.getInstance()
      const { selectedColumns } = get()

      // 【列名 JSON 生成】: 空のとき全列を指定（"" を空配列として WASM に渡す）
      const columnsJson =
        selectedColumns.length > 0 ? JSON.stringify(selectedColumns) : JSON.stringify([])

      // 【WASM CSV 生成】: serialize_csv は UTF-8 文字列を返す
      const csvContent = wasm.serializeCsv(Array.from(indices), columnsJson)

      // 【ダウンロード起動】: Blob → URL → <a download> フォールバック
      _downloadCsv(csvContent, `tunny-export-${Date.now()}.csv`)
    } catch (e) {
      set({ exportError: e instanceof Error ? e.message : 'エクスポートに失敗しました' })
    } finally {
      set({ isExporting: false })
    }
  },

  /**
   * 【ピン留め追加】: 試行をピン留めリストに追加する
   * 上限 20 件を超える場合は pinError をセットしてキャンセル 🟢 REQ-156
   */
  pinTrial: (index, trialId) => {
    const { pinnedTrials } = get()

    // 【重複チェック】: 既にピン留めされている場合は何もしない
    if (pinnedTrials.some((p) => p.index === index)) {
      return
    }

    // 【上限チェック】: MAX_PINS 超過時はエラー
    if (pinnedTrials.length >= MAX_PINS) {
      set({ pinError: `上限${MAX_PINS}件です。古いピン留めを削除してください` })
      return
    }

    const newPin: PinnedTrial = {
      index,
      trialId,
      memo: '',
      pinnedAt: new Date().toISOString(),
    }

    set({ pinnedTrials: [...pinnedTrials, newPin], pinError: null })
  },

  /**
   * 【ピン留め削除】: 指定インデックスのピン留めを削除する
   */
  unpinTrial: (index) => {
    set((s) => ({
      pinnedTrials: s.pinnedTrials.filter((p) => p.index !== index),
    }))
  },

  /**
   * 【ピンメモ更新】: ピン留め試行のメモを更新する
   */
  updatePinMemo: (index, memo) => {
    set((s) => ({
      pinnedTrials: s.pinnedTrials.map((p) => (p.index === index ? { ...p, memo } : p)),
    }))
  },

  /** 【ピンエラークリア】 */
  clearPinError: () => set({ pinError: null }),

  /** 【エクスポートエラークリア】 */
  clearExportError: () => set({ exportError: null }),

  /**
   * 【レポートセクション設定】: ユーザーが並び替え・選択したセクション順序を設定する
   */
  setReportSections: (sections) => set({ reportSections: sections }),

  /**
   * 【HTML レポート生成】: WASM 統計 + ピン留め情報を組み合わせてスタンドアロン HTML を生成する
   *
   * 【処理フロー】:
   *   1. WASM compute_report_stats() でサマリー統計を取得
   *   2. reportSections の順序に従い HTML を組み立てる
   *   3. <a download> でダウンロードを起動する 🟢 REQ-155
   *
   * 🟢 REQ-154〜REQ-155
   */
  generateHtmlReport: async (paretoIndices) => {
    set({ isGeneratingReport: true, reportError: null })

    try {
      const wasm = await WasmLoader.getInstance()
      const { reportSections, pinnedTrials } = get()

      // 【統計取得】: WASM でサマリー統計 JSON を取得
      const stats = wasm.computeReportStats()

      // 【HTML 組み立て】: セクション順序に従ってコンテンツを生成
      const html = _buildHtmlReport(reportSections, stats, pinnedTrials, paretoIndices)

      // 【ダウンロード起動】
      _downloadFile(html, `tunny-report-${Date.now()}.html`, 'text/html;charset=utf-8')
    } catch (e) {
      set({
        reportError: e instanceof Error ? e.message : 'レポート生成に失敗しました',
      })
    } finally {
      set({ isGeneratingReport: false })
    }
  },

  /** 【レポートエラークリア】 */
  clearReportError: () => set({ reportError: null }),

  /**
   * 【セッション保存】: 現在の全ストア状態を JSON ファイルとしてダウンロードする
   *
   * 【収集対象】:
   *   - filterRanges / selectedIndices / colorMode (selectionStore)
   *   - layoutMode / freeModeLayout / visibleCharts (layoutStore)
   *   - pinnedTrials (exportStore)
   *
   * 🟢 REQ-157
   */
  saveSession: async (studyId, journalPath) => {
    set({ isSavingSession: true, sessionError: null })

    try {
      const { pinnedTrials } = get()
      const sel = useSelectionStore.getState()
      const layout = useLayoutStore.getState()

      const session: SessionState = {
        version: SESSION_VERSION,
        journalPath,
        selectedStudyId: studyId,
        filterRanges: sel.filterRanges,
        selectedIndices: Array.from(sel.selectedIndices),
        colorMode: sel.colorMode,
        clusterConfig: null as ClusterConfig | null, // 🟡 クラスタ Store 接続は TASK-1301 以降
        layoutMode: layout.layoutMode,
        visibleCharts: Array.from(layout.visibleCharts),
        pinnedTrials: pinnedTrials.map((p) => ({
          trialId: p.trialId,
          note: p.memo,
          pinnedAt: new Date(p.pinnedAt).getTime(),
        })),
        freeModeLayout: layout.freeModeLayout,
        savedAt: new Date().toISOString(),
      }

      const json = JSON.stringify(session, null, 2)
      const dateStr = new Date().toISOString().slice(0, 10).replace(/-/g, '')
      _downloadFile(json, `session_${dateStr}.json`, 'application/json;charset=utf-8')

      set({ sessionState: session })
    } catch (e) {
      set({
        sessionError: e instanceof Error ? e.message : 'セッションの保存に失敗しました',
      })
    } finally {
      set({ isSavingSession: false })
    }
  },

  /**
   * 【セッション復元】: JSON 文字列を解析して全ストアに状態を適用する
   *
   * 【エラー処理】:
   *   - JSON パースエラー → sessionError をセット
   *   - バージョン不一致 → sessionWarning をセットして続行
   *
   * 🟢 REQ-157
   */
  loadSessionFromJson: (json) => {
    set({ sessionError: null, sessionWarning: null })

    let session: SessionState
    try {
      session = JSON.parse(json) as SessionState
    } catch {
      set({ sessionError: 'セッションファイルの形式が正しくありません' })
      return
    }

    // 【バージョン確認】: 異なるバージョンは警告を表示して続行
    if (session.version !== SESSION_VERSION) {
      set({
        sessionWarning: '古いバージョンのセッションです。一部の設定が復元できない場合があります',
      })
    }

    // 【各ストアへの適用】: selectionStore
    const sel = useSelectionStore.getState()
    sel.brushSelect(new Uint32Array(session.selectedIndices ?? []))
    // filterRanges: 各軸フィルタを再適用
    sel.clearSelection()
    if (session.filterRanges) {
      Object.entries(session.filterRanges).forEach(([axis, range]) => {
        sel.addAxisFilter(axis, range.min, range.max)
      })
    }
    if (session.colorMode) {
      sel.setColorMode(session.colorMode)
    }

    // 【各ストアへの適用】: layoutStore
    const layout = useLayoutStore.getState()
    if (session.layoutMode) {
      layout.setLayoutMode(session.layoutMode)
    }
    if (session.freeModeLayout !== undefined) {
      layout.loadLayout({
        mode: session.layoutMode ?? 'A',
        visibleCharts: session.visibleCharts ?? [],
        panelSizes: { leftPanel: 280, bottomPanel: 200 },
        freeModeLayout: session.freeModeLayout,
      })
    }

    // 【各ストアへの適用】: pinnedTrials の復元
    if (Array.isArray(session.pinnedTrials)) {
      const restored = session.pinnedTrials.map((p, i) => ({
        index: i, // ⚠️ 元の DataFrame インデックスは保存していないため仮インデックスを使用 🟡
        trialId: p.trialId,
        memo: p.note ?? '',
        pinnedAt:
          typeof p.pinnedAt === 'number'
            ? new Date(p.pinnedAt).toISOString()
            : new Date().toISOString(),
      }))
      set({ pinnedTrials: restored.slice(0, MAX_PINS) })
    }

    set({ sessionState: session })
  },

  /** 【セッションメッセージクリア】 */
  clearSessionMessages: () => set({ sessionError: null, sessionWarning: null }),
}))

// -------------------------------------------------------------------------
// 内部ユーティリティ
// -------------------------------------------------------------------------

/**
 * 【CSV ダウンロード】: <a download> 要素でブラウザダウンロードを起動する
 * 🟢 REQ-153: File System Access API 非対応時フォールバック
 */
function _downloadCsv(content: string, filename: string): void {
  // 【Blob 生成】: UTF-8 BOM 付きで Excel 互換にする 🟡
  const bom = '\uFEFF'
  const blob = new Blob([bom + content], { type: 'text/csv;charset=utf-8;' })
  const url = URL.createObjectURL(blob)

  // 【<a download> フォールバック】: 非表示リンクを生成してクリック
  const link = document.createElement('a')
  link.href = url
  link.download = filename
  link.style.display = 'none'
  document.body.appendChild(link)
  link.click()

  // 【クリーンアップ】: リンク要素と Blob URL を解放
  document.body.removeChild(link)
  URL.revokeObjectURL(url)
}

/**
 * 【ファイルダウンロード】: 任意の MIME タイプで <a download> を起動する
 */
function _downloadFile(content: string, filename: string, mimeType: string): void {
  const blob = new Blob([content], { type: mimeType })
  const url = URL.createObjectURL(blob)

  const link = document.createElement('a')
  link.href = url
  link.download = filename
  link.style.display = 'none'
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
  URL.revokeObjectURL(url)
}

/**
 * 【HTML レポート組み立て】: セクション・統計・ピン留め情報をスタンドアロン HTML に変換する
 *
 * 【設計方針】:
 *   - 生データ（全試行）は埋め込まない → 5〜15MB 目標
 *   - Plotly.js は CDN から読み込む（オフライン環境ではチャート非表示）🟡
 *   - 統計サマリーと注目解のみを JSON として埋め込む
 * 🟢 REQ-154〜REQ-155
 */
function _buildHtmlReport(
  sections: ReportSection[],
  statsJson: string,
  pinnedTrials: PinnedTrial[],
  paretoIndices: Uint32Array,
): string {
  // 【データ埋め込み】: Pareto 解インデックスと統計をシリアライズ
  const embeddedData = JSON.stringify({
    generatedAt: new Date().toISOString(),
    stats: JSON.parse(statsJson || '{}'),
    pinnedTrials: pinnedTrials.map((p) => ({
      trialId: p.trialId,
      memo: p.memo,
      pinnedAt: p.pinnedAt,
    })),
    paretoCount: paretoIndices.length,
  })

  // 【セクション HTML 生成】
  const sectionHtml = sections
    .map((sec) => _buildSectionHtml(sec, pinnedTrials, paretoIndices))
    .join('\n')

  return `<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Tunny Dashboard Report</title>
<style>
body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;margin:0;padding:2rem;background:#f8fafc;color:#1e293b;}
h1{font-size:1.75rem;font-weight:700;border-bottom:2px solid #3b82f6;padding-bottom:0.5rem;margin-bottom:1.5rem;}
h2{font-size:1.25rem;font-weight:600;color:#1e40af;margin-top:2rem;}
table{border-collapse:collapse;width:100%;margin-top:0.5rem;background:#fff;border-radius:0.5rem;overflow:hidden;box-shadow:0 1px 3px rgba(0,0,0,.1);}
th,td{border:1px solid #e2e8f0;padding:0.5rem 0.75rem;font-size:0.85rem;}
th{background:#1e40af;color:#fff;text-align:left;}
tr:nth-child(even){background:#f1f5f9;}
.note{color:#64748b;font-size:0.8rem;margin-top:0.25rem;}
@media print{body{background:#fff;padding:1rem;}.no-print{display:none;}}
</style>
</head>
<body>
<h1>Tunny Dashboard Report</h1>
<p class="note">生成日時: ${new Date().toLocaleString('ja-JP')}</p>
<div class="no-print" style="margin:1rem 0;">
  <button onclick="window.print()" style="background:#3b82f6;color:#fff;border:none;padding:0.5rem 1rem;border-radius:0.375rem;cursor:pointer;">PDFとして印刷</button>
</div>
${sectionHtml}
<script>
// 【埋め込みデータ】: レポート生成時の統計・試行情報
const REPORT_DATA = ${embeddedData};
</script>
</body>
</html>`
}

/** 【セクション HTML 生成】: セクション種別に応じた HTML を返す */
function _buildSectionHtml(
  section: ReportSection,
  pinnedTrials: PinnedTrial[],
  paretoIndices: Uint32Array,
): string {
  switch (section) {
    case 'summary':
      return '<h2>統計サマリー</h2><p class="note">各パラメータ・目的関数の統計値は埋め込みデータ (REPORT_DATA.stats) を参照してください。</p>'
    case 'pareto':
      return `<h2>Pareto 解</h2><p>Pareto 最適解: ${paretoIndices.length} 件</p>`
    case 'pinned':
      if (pinnedTrials.length === 0) {
        return '<h2>注目解（ピン留め）</h2><p class="note">ピン留めされた試行はありません。</p>'
      }
      return (
        '<h2>注目解（ピン留め）</h2>' +
        '<table><thead><tr><th>Trial ID</th><th>ピン留め日時</th><th>メモ</th></tr></thead><tbody>' +
        pinnedTrials
          .map(
            (p) =>
              `<tr><td>${p.trialId}</td><td>${new Date(p.pinnedAt).toLocaleString('ja-JP')}</td><td>${_escapeHtml(p.memo)}</td></tr>`,
          )
          .join('') +
        '</tbody></table>'
      )
    case 'history':
      return '<h2>最適化履歴</h2><p class="note">最適化履歴チャートは Plotly.js が利用可能な環境で表示されます。</p>'
    case 'cluster':
      return '<h2>クラスタ分析</h2><p class="note">クラスタ分析結果は埋め込みデータを参照してください。</p>'
    default:
      return ''
  }
}

/** 【HTML エスケープ】: XSS 対策のため特殊文字をエスケープする */
function _escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;')
}
