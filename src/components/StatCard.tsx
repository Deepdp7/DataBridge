/**
 * StatCard — dashboard metric widget
 */

interface Props {
  label: string;
  value: string | number;
  sub?: string;
  accent?: 'green' | 'yellow' | 'red' | 'blue' | 'default';
  mono?: boolean;
}

export function StatCard({ label, value, sub, accent = 'default', mono = false }: Props) {
  const accentClass = {
    green:   'text-status-green',
    yellow:  'text-status-yellow',
    red:     'text-status-red',
    blue:    'text-accent',
    default: 'text-text-primary',
  }[accent];

  return (
    <div className="panel p-4 flex flex-col gap-1">
      <div className="text-text-muted text-xs uppercase tracking-wide font-medium">{label}</div>
      <div className={`text-2xl font-semibold ${accentClass} ${mono ? 'font-mono' : ''}`}>
        {value}
      </div>
      {sub && <div className="text-text-muted text-xs">{sub}</div>}
    </div>
  );
}
