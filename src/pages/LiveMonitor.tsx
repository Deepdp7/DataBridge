/**
 * LiveMonitor page — PRD §18.3
 * Real-time monitoring of tick ingress, deduplication, and downstream throughput.
 */

import { useEffect } from 'react';
import { useAppStore } from '../store/useAppStore';
import { StatCard } from '../components/StatCard';
import { Badge, statusBadge } from '../components/Badge';

export function LiveMonitor() {
  const { tickStats, fetchTickStats, startLiveFeed, stopLiveFeed } = useAppStore();

  useEffect(() => {
    fetchTickStats();
    const interval = setInterval(fetchTickStats, 1000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="flex flex-col h-full p-4 gap-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
        <h1 className="text-base font-semibold text-text-primary">Live Monitor</h1>
        <p className="text-xs text-text-muted">Real-time WebSocket connection and tick ingestion statistics</p>
      </div>
      <div className="flex gap-2">
        <button className="btn btn-primary" onClick={() => startLiveFeed()}>Start Feed</button>
        <button className="btn btn-danger" onClick={() => stopLiveFeed()}>Stop Feed</button>
      </div>
    </div>

      <div className="grid grid-cols-4 gap-3 mt-2">
        <StatCard
          label="Throughput"
          value={`${tickStats?.ticks_per_sec.toFixed(1) ?? '0.0'} /s`}
          accent="green"
          mono
        />
        <StatCard
          label="Active Subs"
          value={tickStats?.active_subscriptions ?? 0}
          mono
        />
        <StatCard
          label="Dropped Ticks"
          value={tickStats?.dropped_ticks ?? 0}
          accent={tickStats?.dropped_ticks ? 'red' : 'default'}
          mono
        />
        <StatCard
          label="Duplicates Ignored"
          value={tickStats?.duplicate_ticks ?? 0}
          mono
        />
      </div>

      <div className="panel p-4 flex flex-col gap-4 mt-2 flex-1">
        <div className="text-sm font-semibold border-b border-border pb-2">Connection Details</div>
        
        <div className="grid grid-cols-2 gap-y-4 text-sm">
          <div>
            <div className="text-xs text-text-muted mb-1">WebSocket State</div>
            <Badge color={statusBadge(tickStats?.ws_state ?? 'unknown')}>
              {tickStats?.ws_state ?? 'Unknown'}
            </Badge>
          </div>
          <div>
            <div className="text-xs text-text-muted mb-1">Total Ticks Processed</div>
            <div className="font-mono text-text-secondary">{tickStats?.total_ticks ?? 0}</div>
          </div>
          <div>
            <div className="text-xs text-text-muted mb-1">Last Tick Received</div>
            <div className="font-mono text-text-secondary">
              {tickStats?.last_tick_at ? new Date(tickStats.last_tick_at).toLocaleTimeString() : '—'}
            </div>
          </div>
        </div>

        <div className="mt-auto p-4 bg-bg-base rounded border border-border">
          <p className="text-xs text-text-muted">
            The Live Tick Engine manages the WebSocket connection to the broker. 
            Incoming ticks are deduplicated via a sliding window (using a 6-tuple fingerprint) 
            and normalized before being passed to the Aggregation Engine.
          </p>
        </div>
      </div>
    </div>
  );
}
