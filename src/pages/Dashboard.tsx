/**
 * Dashboard page — PRD §18.1
 * System overview: connection status, symbol counts, backfill progress,
 * tick throughput, error count, IPC port.
 */

import { useEffect, useState } from 'react';
import { useAppStore } from '../store/useAppStore';
import { StatCard } from '../components/StatCard';
import { StatusIndicator } from '../components/StatusIndicator';
import { ProgressBar } from '../components/ProgressBar';
import { Badge, statusBadge } from '../components/Badge';

export function Dashboard() {
  const { status, backfillStatus, tickStats, fetchStatus, fetchBackfillStatus, fetchTickStats } =
    useAppStore();

  useEffect(() => {
    fetchStatus();
    fetchBackfillStatus();
    fetchTickStats();
    const interval = setInterval(() => {
      fetchStatus();
      fetchBackfillStatus();
      fetchTickStats();
    }, 3000);
    return () => clearInterval(interval);
  }, []);

  const brokerState  = status?.broker_connected      ? 'connected' : 'disconnected';
  const wsState      = status?.websocket_connected    ? 'connected' : 'disconnected';
  const abState      = status?.amibroker_connected    ? 'connected' : 'disconnected';
  const sessionState = status?.session_status === 'active' ? 'connected' :
                       status?.session_status === 'expired' ? 'error' : 'disconnected';

  const totalJobs = backfillStatus
    ? backfillStatus.queued + backfillStatus.downloading + backfillStatus.completed + backfillStatus.failed
    : 0;
  const completedJobs = backfillStatus?.completed ?? 0;
  const backfillPct = totalJobs > 0 ? Math.round((completedJobs / totalJobs) * 100) : 0;

  return (
    <div className="flex flex-col h-full overflow-auto p-4 gap-4">
      {/* Page header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-base font-semibold text-text-primary">Dashboard</h1>
          <p className="text-xs text-text-muted">System overview and connection status</p>
        </div>
        <div className="text-xs text-text-muted font-mono">
          IPC Port: {status?.ipc_port ?? '—'}
        </div>
      </div>

      {/* Connection status row */}
      <div className="panel p-3">
        <div className="text-xs font-medium text-text-muted uppercase tracking-wide mb-3">Connection Status</div>
        <div className="grid grid-cols-4 gap-4">
          {[
            { label: 'Broker REST',    state: brokerState  },
            { label: 'WebSocket',      state: wsState      },
            { label: 'AmiBroker IPC',  state: abState      },
            { label: 'Auth Session',   state: sessionState },
          ].map(({ label, state }) => (
            <div key={label} className="flex flex-col gap-1">
              <div className="text-xs text-text-muted">{label}</div>
              <StatusIndicator
                state={state as any}
                label={state.charAt(0).toUpperCase() + state.slice(1)}
              />
            </div>
          ))}
        </div>
      </div>

      {/* Metric cards */}
      <div className="grid grid-cols-4 gap-3">
        <StatCard
          label="Total Symbols"
          value={status?.symbols_total ?? '—'}
          sub={`${status?.symbols_active ?? 0} active`}
        />
        <StatCard
          label="Ticks / sec"
          value={(tickStats?.ticks_per_sec ?? 0).toFixed(1)}
          accent={wsState === 'connected' ? 'green' : 'default'}
          mono
        />
        <StatCard
          label="Errors (last hour)"
          value={status?.errors_last_hour ?? 0}
          accent={(status?.errors_last_hour ?? 0) > 0 ? 'red' : 'default'}
        />
        <StatCard
          label="Active Subscriptions"
          value={tickStats?.active_subscriptions ?? 0}
          sub={`${tickStats?.total_ticks ?? 0} total ticks`}
          mono
        />
      </div>

      {/* Backfill progress */}
      <div className="panel p-4">
        <div className="flex items-center justify-between mb-3">
          <div className="text-xs font-medium text-text-muted uppercase tracking-wide">Backfill Progress</div>
          <div className="flex items-center gap-3 text-xs text-text-muted">
            {backfillStatus && (
              <>
                <span><span className="text-accent font-medium">{backfillStatus.queued}</span> queued</span>
                <span><span className="text-status-yellow font-medium">{backfillStatus.downloading}</span> downloading</span>
                <span><span className="text-status-green font-medium">{backfillStatus.completed}</span> completed</span>
                <span><span className="text-status-red font-medium">{backfillStatus.failed}</span> failed</span>
              </>
            )}
          </div>
        </div>
        <ProgressBar value={backfillPct} color={backfillPct === 100 ? 'green' : 'blue'} />
      </div>

      {/* Tick stats */}
      {tickStats && (
        <div className="panel p-4">
          <div className="text-xs font-medium text-text-muted uppercase tracking-wide mb-3">Live Tick Engine</div>
          <div className="grid grid-cols-3 gap-4 text-sm">
            <div>
              <div className="text-text-muted text-xs mb-1">Duplicates</div>
              <div className="num text-text-secondary">{tickStats.duplicate_ticks.toLocaleString()}</div>
            </div>
            <div>
              <div className="text-text-muted text-xs mb-1">Dropped</div>
              <div className={`num ${tickStats.dropped_ticks > 0 ? 'text-status-red' : 'text-text-secondary'}`}>
                {tickStats.dropped_ticks.toLocaleString()}
              </div>
            </div>
            <div>
              <div className="text-text-muted text-xs mb-1">WebSocket</div>
              <Badge color={statusBadge(tickStats.ws_state)}>{tickStats.ws_state}</Badge>
            </div>
          </div>
        </div>
      )}

      {/* Active broker */}
      {status && (
        <div className="text-xs text-text-muted">
          Active broker: <span className="text-text-secondary font-medium">{status.active_broker}</span>
        </div>
      )}
    </div>
  );
}
