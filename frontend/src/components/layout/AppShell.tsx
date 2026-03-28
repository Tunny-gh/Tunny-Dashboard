/**
 * AppShell — 4エリア CSS Grid レイアウトのアプリ骨格 (TASK-401)
 *
 * 【役割】: ToolBar / LeftPanel / MainCanvas / BottomPanel の4エリアを管理
 * 【設計方針】: layoutStore のモードに応じてグリッドを切り替え
 * 🟢 ファイルドラッグ&ドロップ → studyStore.loadJournal() を呼び出す
 */

import { useStudyStore } from '../../stores/studyStore';
import { useLayoutStore } from '../../stores/layoutStore';
import { ToolBar } from './ToolBar';
import { LeftPanel } from '../panels/LeftPanel';
import { BottomPanel } from '../panels/BottomPanel';
import { FreeLayoutCanvas } from './FreeLayoutCanvas';
import { ChartCatalogPanel } from './ChartCatalogPanel';
import type { DragEvent } from 'react';

// -------------------------------------------------------------------------
// コンポーネント実装
// -------------------------------------------------------------------------

/**
 * 【機能概要】: アプリケーション全体の CSS Grid レイアウト骨格
 * 【Brushing連携】: なし（ToolBar・Panel は各自 Store に接続）
 * 【テスト対応】: TC-401-01, TC-401-02, TC-401-E01
 */
export function AppShell() {
  // 【Store接続】: layoutMode と loadJournal を取得 🟢
  const layoutMode = useLayoutStore((s) => s.layoutMode);
  const loadJournal = useStudyStore((s) => s.loadJournal);
  const isLoading = useStudyStore((s) => s.isLoading);

  /**
   * 【dragOver ハンドラ】: ファイルドロップを有効にするために preventDefault 🟢
   */
  const handleDragOver = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
  };

  /**
   * 【drop ハンドラ】: ドロップされたファイルを loadJournal に渡す
   * 【ファイル検証】: 最初のファイルのみを使用（複数ドロップは無視）
   */
  const handleDrop = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    const file = e.dataTransfer?.files?.[0];
    if (file) {
      // 【Journal読み込み開始】: fire-and-forget で非同期実行
      void loadJournal(file);
    }
  };

  // 【レンダリング】: 4エリア CSS Grid レイアウト 🟢
  return (
    <div
      data-testid="app-shell"
      data-layout={layoutMode}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
      style={{
        display: 'grid',
        gridTemplateRows: 'auto 1fr auto',
        gridTemplateColumns: 'auto 1fr auto',
        height: '100vh',
        width: '100vw',
      }}
    >
      {/* 【ToolBar エリア】: 全幅で最上部に配置 */}
      <div style={{ gridColumn: '1 / 4' }}>
        <ToolBar />
      </div>

      {/* 【LeftPanel エリア】: 左側パネル */}
      <div data-testid="left-panel" style={{ width: '260px', overflowY: 'auto', borderRight: '1px solid var(--border)', background: 'var(--bg-panel)' }}>
        <LeftPanel />
      </div>

      {/* 【MainCanvas エリア】: メインの描画エリア */}
      <div data-testid="main-canvas" style={{ overflow: 'hidden', position: 'relative', height: '100%' }}>
        <FreeLayoutCanvas />
      </div>

      {/* 【ChartCatalogPanel エリア】: 右側収納可能チャートカタログ */}
      <ChartCatalogPanel />

      {/* 【BottomPanel エリア】: 全幅で最下部に配置 */}
      <div data-testid="bottom-panel" style={{ gridColumn: '1 / 4', height: '220px', overflowY: 'auto', borderTop: '1px solid var(--border)' }}>
        <BottomPanel />
      </div>

      {/* 【ローディングインジケータ】: isLoading=true のとき表示 🟢 */}
      {isLoading && (
        <div
          data-testid="loading-indicator"
          style={{
            position: 'fixed',
            top: 0,
            left: 0,
            right: 0,
            height: '4px',
            background: 'var(--accent, #4f46e5)',
            zIndex: 9999,
          }}
        />
      )}
    </div>
  );
}
