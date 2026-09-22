/**
 * History page — PRD §18.4
 * Manage historical data, inspect gaps, trigger gap repair.
 */

import { useEffect, useState } from 'react';
import { useAppStore } from '../store/useAppStore';
import { invoke } from '@tauri-apps/api/core';

export function History() {
  const { symbols, fetchSymbols } = useAppStore();
  const [selectedSymbol, setSelectedSymbol] = useState<number | null>(null);
  
  const [stats, setStats] = useState<any>(null);
  const [gaps, setGaps] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    fetchSymbols({ status: 'enabled' });
  }, []);

  useEffect(() => {
    if (selectedSymbol) {
      loadSymbolHistory(selectedSymbol);
    } else {
      setStats(null);
      setGaps([]);
    }
  }, [selectedSymbol]);

  const loadSymbolHistory = async (id: number) => {
    setLoading(true);
    try {
      const historyStats: any = await invoke('get_history', { symbolId: id, interval: "1m" });
      const gapsList: any[] = await invoke('get_gaps', { symbolId: id });
      setStats({
        count: historyStats.record_count,
        first: historyStats.first_timestamp,
        last: historyStats.last_timestamp,
      });
      setGaps(gapsList);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  const handleRepair = async () => {
    if (!selectedSymbol) return;
    try {
      await invoke('repair_gaps', { symbolId: selectedSymbol });
      alert('Repair jobs queued');
      loadSymbolHistory(selectedSymbol);
    } catch (e) {
      alert(e);
    }
  };

  return (
    <div className="flex h-full p-4 gap-6">
      {/* Sidebar: enabled symbols */}
      <div className="w-64 flex flex-col gap-3">
        <h2 className="text-sm font-semibold text-text-primary border-b border-border pb-2">Active Symbols</h2>
        <div className="flex-1 overflow-y-auto pr-2 flex flex-col gap-1">
          {symbols.length === 0 ? (
            <div className="text-xs text-text-muted italic">No enabled symbols.</div>
          ) : (
            symbols.map(s => (
              <button
                key={s.id}
                onClick={() => setSelectedSymbol(s.id)}
                className={`text-left px-3 py-2 rounded text-sm transition-colors ${
                  selectedSymbol === s.id 
                    ? 'bg-accent/20 text-accent font-medium border border-accent/30' 
                    : 'hover:bg-bg-surface border border-transparent text-text-secondary'
                }`}
              >
                <div className="flex justify-between items-center">
                  <span>{s.symbol}</span>
                  <span className="text-2xs text-text-muted">{s.exchange}</span>
                </div>
              </button>
            ))
          )}
        </div>
      </div>

      {/* Main Panel: History details */}
      <div className="flex-1 panel flex flex-col">
        {selectedSymbol ? (
          loading ? (
            <div className="flex-1 flex items-center justify-center text-text-muted text-sm">Loading history...</div>
          ) : (
            <div className="p-6 flex flex-col gap-6 h-full">
              <div className="flex items-center justify-between border-b border-border pb-4">
                <h1 className="text-xl font-bold text-text-primary">
                  {symbols.find(s => s.id === selectedSymbol)?.symbol} History
                </h1>
                <div className="text-xs text-text-muted">1-Minute OHLCV</div>
              </div>
              
              <div className="grid grid-cols-3 gap-4">
                <div className="bg-bg-base p-4 rounded border border-border">
                  <div className="text-xs text-text-muted mb-1">Total Bars</div>
                  <div className="text-xl font-mono text-text-primary">{stats?.count?.toLocaleString() ?? 0}</div>
                </div>
                <div className="bg-bg-base p-4 rounded border border-border">
                  <div className="text-xs text-text-muted mb-1">First Record</div>
                  <div className="text-sm font-mono text-text-secondary">
                    {stats?.first ? new Date(stats.first).toLocaleString() : '—'}
                  </div>
                </div>
                <div className="bg-bg-base p-4 rounded border border-border">
                  <div className="text-xs text-text-muted mb-1">Last Record</div>
                  <div className="text-sm font-mono text-text-secondary">
                    {stats?.last ? new Date(stats.last).toLocaleString() : '—'}
                  </div>
                </div>
              </div>

              <div className="flex-1 flex flex-col gap-3 min-h-0">
                <div className="flex items-center justify-between">
                  <h3 className="text-sm font-semibold">Detected Gaps ({gaps.length})</h3>
                  {gaps.length > 0 && (
                    <button className="btn btn-primary" onClick={handleRepair}>
                      Repair All Gaps
                    </button>
                  )}
                </div>
                <div className="flex-1 border border-border rounded bg-bg-base overflow-auto">
                  <table className="db-table w-full">
                    <thead>
                      <tr>
                        <th>Gap Start</th>
                        <th>Gap End</th>
                        <th>Status</th>
                        <th>Detected At</th>
                      </tr>
                    </thead>
                    <tbody>
                      {gaps.length === 0 ? (
                        <tr>
                          <td colSpan={4} className="text-center text-text-muted p-4 italic">No gaps detected. Data is contiguous.</td>
                        </tr>
                      ) : (
                        gaps.map(g => (
                          <tr key={g.id}>
                            <td className="font-mono">{new Date(g.gap_start).toLocaleString()}</td>
                            <td className="font-mono">{new Date(g.gap_end).toLocaleString()}</td>
                            <td><span className="text-status-yellow">{g.status}</span></td>
                            <td className="text-text-muted">{new Date(g.detected_at).toLocaleString()}</td>
                          </tr>
                        ))
                      )}
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          )
        ) : (
          <div className="flex-1 flex items-center justify-center text-text-muted text-sm">
            Select a symbol to view its historical data
          </div>
        )}
      </div>
    </div>
  );
}
