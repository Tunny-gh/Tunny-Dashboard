/**
 * Documentation.
 *
 * Documentation.
 * Documentation.
 */

// -------------------------------------------------------------------------
// Props Type definitions
// -------------------------------------------------------------------------

export interface EmptyStateProps {
  /** Documentation. */
  message?: string
}

// -------------------------------------------------------------------------
// Documentation.
// -------------------------------------------------------------------------

/**
 * Documentation.
 */
export function EmptyState({ message = 'No data available' }: EmptyStateProps) {
  return (
    <div
      data-testid="empty-state"
      style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}
    >
      <span>{message}</span>
    </div>
  )
}

export default EmptyState
