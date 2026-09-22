/**
 * Badge component
 */

type BadgeColor = 'green' | 'red' | 'yellow' | 'blue' | 'gray';

interface Props {
  color: BadgeColor;
  children: React.ReactNode;
}

export function Badge({ color, children }: Props) {
  return (
    <span className={`badge badge-${color}`}>{children}</span>
  );
}

/** Derive badge color from a status string */
export function statusBadge(status: string): BadgeColor {
  switch (status.toLowerCase()) {
    case 'enabled':
    case 'completed':
    case 'connected':
    case 'active':
      return 'green';
    case 'disabled':
    case 'failed':
    case 'error':
    case 'expired':
      return 'red';
    case 'queued':
    case 'retrying':
    case 'warning':
    case 'unknown':
      return 'yellow';
    case 'downloading':
    case 'processing':
    case 'connecting':
      return 'blue';
    default:
      return 'gray';
  }
}
