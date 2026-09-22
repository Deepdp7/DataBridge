/**
 * Logs page — PRD §18.5
 */

import { useEffect, useState, useRef } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { useAppStore } from '../store/useAppStore';
import { invoke } from '@tauri-apps/api/core';

export function Logs() {
  const { logs, fetchLogs, logsLoading } = useAppStore();
  const [level, setLevel] = useState('ALL');
  const [module, setModule] = useState('');

  useEffect(() => {
    fetchLogs({ 
      level: level === 'ALL' ? undefined : level, 
      module: module || undefined 
    });
  }, [level, module]);

  const parentRef = useRef<HTMLDivElement>(null);
  const rowVirtualizer = useVirtualizer({
    count: logs.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 24, // log row height
    overscan: 20,
  });

  const handleClear = async () => {
    if (confirm('Clear old logs? (Keeps last 24h)')) {
      await invoke('clear_logs');
      fetchLogs();
    }
  };

  return (
    <div className="flex flex-col h-full p-4 gap-3">
      <div className="flex items-center justify-between">
        <h1 className="text-base font-semibold text-text-primary">System Logs</h1>
        <div className="flex gap-2">
          <button className="btn btn-ghost" onClick={() => fetchLogs()} disabled={logsLoading}>
            {logsLoading ? '...' : 'Refresh'}
          </button>
          <button className="btn btn-danger" onClick={handleClear}>Purge Old</button>
        </div>
      </div>

      <div className="flex gap-2 bg-bg-surface p-2 border border-border rounded">
        <select className="db-select w-32" value={level} onChange={e => setLevel(e.target.value)}>
          <option value="ALL">All Levels</option>
          <option value="ERROR">Error</option>
          <option value="WARN">Warning</option>
          <option value="INFO">Info</option>
          <option value="DEBUG">Debug</option>
        </select>
        <input 
          type="text" 
          placeholder="Filter module (e.g. broker, amibroker)" 
          className="db-input flex-1" 
          value={module}
          onChange={e => setModule(e.target.value)}
        />
      </div>

      <div className="flex-1 panel overflow-auto bg-[#0a0a0c]" ref={parentRef}>
        <div
          style={{
            height: `${rowVirtualizer.getTotalSize()}px`,
            position: 'relative',
          }}
        >
          {rowVirtualizer.getVirtualItems().map((virtualRow) => {
            const l = logs[virtualRow.index];
            return (
              <div
                key={l.id}
                className="font-mono text-[11px] leading-6 px-3 border-b border-white/5 whitespace-nowrap hover:bg-white/5"
                style={{
                  position: 'absolute',
                  top: 0,
                  left: 0,
                  width: '100%',
                  height: `${virtualRow.size}px`,
                  transform: `translateY(${virtualRow.start}px)`,
                }}
              >
                <span className="text-text-muted mr-3">{new Date(l.timestamp).toLocaleTimeString()}</span>
                <span className={`inline-block w-12 font-bold log-level-${l.level}`}>{l.level}</span>
                <span className="text-text-secondary mr-3 w-24 inline-block truncate" title={l.module}>[{l.module}]</span>
                <span className="text-text-primary">{l.message}</span>
                {l.symbol && <span className="ml-2 text-accent bg-accent/10 px-1 rounded">{l.symbol}</span>}
              </div>
            );
          })}
        </div>
        {logs.length === 0 && !logsLoading && (
          <div className="p-8 text-center text-text-muted text-sm italic">No logs match criteria.</div>
        )}
      </div>
    </div>
  );
}
