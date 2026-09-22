/**
 * StatusIndicator — green/yellow/red dot with optional label
 */

interface Props {
  state: 'connected' | 'warning' | 'error' | 'disconnected' | 'unknown';
  label?: string;
  animate?: boolean;
}

export function StatusIndicator({ state, label, animate = true }: Props) {
  const dotClass = {
    connected:    'status-dot status-dot-green',
    warning:      `status-dot status-dot-yellow${animate ? '' : ''}`,
    error:        'status-dot status-dot-red',
    disconnected: 'status-dot status-dot-gray',
    unknown:      'status-dot status-dot-gray',
  }[state];

  return (
    <span className="inline-flex items-center gap-2">
      <span className={dotClass} />
      {label && (
        <span className={
          state === 'connected'    ? 'text-status-green' :
          state === 'warning'      ? 'text-status-yellow' :
          state === 'error'        ? 'text-status-red' :
          'text-text-muted'
        }>{label}</span>
      )}
    </span>
  );
}
