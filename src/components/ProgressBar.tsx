/**
 * ProgressBar component
 */

interface Props {
  value: number; // 0–100
  color?: 'blue' | 'green';
  showLabel?: boolean;
}

export function ProgressBar({ value, color = 'blue', showLabel = true }: Props) {
  const pct = Math.max(0, Math.min(100, value));
  return (
    <div className="flex items-center gap-3 w-full">
      <div className="progress-bar-track flex-1">
        <div
          className={`progress-bar-fill ${color === 'green' ? 'green' : ''}`}
          style={{ width: `${pct}%` }}
        />
      </div>
      {showLabel && (
        <span className="num text-text-secondary w-10 text-right">{pct}%</span>
      )}
    </div>
  );
}
