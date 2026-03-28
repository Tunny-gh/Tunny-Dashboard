/**
 * EmptyState — チャート共通の空状態プレースホルダー
 *
 * 【役割】: データなし時に「データがありません」を中央に表示する
 * 【使用箇所】: 全チャートコンポーネントの空状態 UI
 */

// -------------------------------------------------------------------------
// Props 型定義
// -------------------------------------------------------------------------

export interface EmptyStateProps {
  /** 表示メッセージ（省略時: 「データがありません」） */
  message?: string;
}

// -------------------------------------------------------------------------
// コンポーネント実装
// -------------------------------------------------------------------------

/**
 * 【機能概要】: 親コンテナ全体に中央揃えで空状態メッセージを表示する
 */
export function EmptyState({ message = 'データがありません' }: EmptyStateProps) {
  return (
    <div
      data-testid="empty-state"
      style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}
    >
      <span>{message}</span>
    </div>
  );
}

export default EmptyState;
