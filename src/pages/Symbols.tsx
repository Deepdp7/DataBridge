/**
 * Symbols page — PRD §18.2
 * Allows managing symbols: listing, adding manually, importing via CSV, and triggering backfill.
 */

import { useEffect, useState, useMemo } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { useAppStore } from '../store/useAppStore';
import { Badge } from '../components/Badge';
import { Modal } from '../components/Modal';

export function Symbols() {
  const { symbols, symbolsLoading, fetchSymbols, enableSymbol, disableSymbol, removeSymbol, triggerBackfill, importCsv } = useAppStore();
  const [search, setSearch] = useState('');
  const [statusFilter, setStatusFilter] = useState<'all' | 'enabled' | 'disabled'>('all');

  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
  const [isImportModalOpen, setIsImportModalOpen] = useState(false);

  useEffect(() => {
    fetchSymbols();
  }, []);

  const filteredSymbols = useMemo(() => {
    return symbols.filter((s) => {
      const matchSearch = search ? `${s.symbol} ${s.exchange}`.toLowerCase().includes(search.toLowerCase()) : true;
      const matchStatus = statusFilter === 'all' ? true : s.status === statusFilter;
      return matchSearch && matchStatus;
    });
  }, [symbols, search, statusFilter]);

  // Virtualized table container
  const parentRef = React.useRef<HTMLDivElement>(null);
  const rowVirtualizer = useVirtualizer({
    count: filteredSymbols.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 36, // row height
    overscan: 10,
  });

  return (
    <div className="flex flex-col h-full p-4 gap-4">
      {/* Header & Controls */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-base font-semibold text-text-primary">Symbol Manager</h1>
          <p className="text-xs text-text-muted">{symbols.length} total symbols mapped</p>
        </div>
        <div className="flex gap-2">
          <button className="btn btn-ghost" onClick={() => setIsImportModalOpen(true)}>
            Import CSV
          </button>
          <button className="btn btn-primary" onClick={() => setIsAddModalOpen(true)}>
            + Add Symbol
          </button>
        </div>
      </div>

      <div className="flex items-center gap-3 bg-bg-surface p-2 rounded border border-border">
        <input
          type="text"
          placeholder="Search symbols..."
          className="db-input flex-1"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        <select
          className="db-select w-40"
          value={statusFilter}
          onChange={(e) => setStatusFilter(e.target.value as any)}
        >
          <option value="all">All Status</option>
          <option value="enabled">Enabled</option>
          <option value="disabled">Disabled</option>
        </select>
        <button className="btn btn-ghost" onClick={() => fetchSymbols()} disabled={symbolsLoading}>
          {symbolsLoading ? '...' : 'Refresh'}
        </button>
      </div>

      {/* Virtualized Table */}
      <div className="flex-1 panel overflow-auto" ref={parentRef}>
        <table className="db-table w-full">
          <thead>
            <tr>
              <th className="w-10">ID</th>
              <th className="w-40">Symbol</th>
              <th className="w-20">Exchange</th>
              <th className="w-24">Type</th>
              <th className="w-24">Status</th>
              <th className="text-right">Actions</th>
            </tr>
          </thead>
          <tbody
            style={{
              height: `${rowVirtualizer.getTotalSize()}px`,
              position: 'relative',
            }}
          >
            {rowVirtualizer.getVirtualItems().map((virtualRow) => {
              const s = filteredSymbols[virtualRow.index];
              return (
                <tr
                  key={s.id}
                  style={{
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    width: '100%',
                    height: `${virtualRow.size}px`,
                    transform: `translateY(${virtualRow.start}px)`,
                  }}
                >
                  <td className="text-text-muted font-mono">{s.id}</td>
                  <td className="font-semibold text-text-primary">{s.symbol}</td>
                  <td>{s.exchange}</td>
                  <td><Badge color="gray">{s.instrument_type}</Badge></td>
                  <td>
                    <Badge color={s.status === 'enabled' ? 'green' : 'red'}>
                      {s.status}
                    </Badge>
                  </td>
                  <td className="text-right flex items-center justify-end gap-2">
                    {s.status === 'enabled' ? (
                      <button className="btn btn-ghost !py-1 !text-[11px]" onClick={() => disableSymbol(s.id)}>Disable</button>
                    ) : (
                      <button className="btn btn-success !py-1 !text-[11px]" onClick={() => enableSymbol(s.id)}>Enable</button>
                    )}
                    <button className="btn btn-ghost !py-1 !text-[11px]" onClick={() => triggerBackfill(s.id)}>Backfill</button>
                    <button className="btn btn-danger !py-1 !text-[11px]" onClick={() => {
                      if (confirm(`Remove ${s.symbol}?`)) removeSymbol(s.id);
                    }}>Remove</button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
        {filteredSymbols.length === 0 && !symbolsLoading && (
          <div className="p-8 text-center text-text-muted text-sm">No symbols found.</div>
        )}
      </div>

      {isAddModalOpen && <AddSymbolModal onClose={() => setIsAddModalOpen(false)} />}
      {isImportModalOpen && <ImportCsvModal onClose={() => setIsImportModalOpen(false)} />}
    </div>
  );
}

// ── Modals ─────────────────────────────────────────────────────────────

function AddSymbolModal({ onClose }: { onClose: () => void }) {
  const { addSymbol } = useAppStore();
  const [sym, setSym] = useState('');
  const [exc, setExc] = useState('NSE');
  const [tok, setTok] = useState('');
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    await addSymbol(sym, exc, tok);
    setLoading(false);
    onClose();
  };

  return (
    <Modal title="Add Symbol Manually" onClose={onClose} width={400}>
      <form onSubmit={handleSubmit} className="flex flex-col gap-4">
        <div>
          <label className="block text-xs text-text-muted mb-1">Symbol</label>
          <input className="db-input" value={sym} onChange={e => setSym(e.target.value)} placeholder="e.g. RELIANCE" required autoFocus />
        </div>
        <div>
          <label className="block text-xs text-text-muted mb-1">Exchange</label>
          <input className="db-input" value={exc} onChange={e => setExc(e.target.value)} placeholder="e.g. NSE" required />
        </div>
        <div>
          <label className="block text-xs text-text-muted mb-1">Broker Token</label>
          <input className="db-input" value={tok} onChange={e => setTok(e.target.value)} placeholder="e.g. 2885" required />
        </div>
        <div className="flex justify-end gap-2 mt-2">
          <button type="button" className="btn btn-ghost" onClick={onClose}>Cancel</button>
          <button type="submit" className="btn btn-primary" disabled={loading}>
            {loading ? 'Adding...' : 'Add Symbol'}
          </button>
        </div>
      </form>
    </Modal>
  );
}

import React from 'react';

function ImportCsvModal({ onClose }: { onClose: () => void }) {
  const { importCsv } = useAppStore();
  const [csvText, setCsvText] = useState('');
  const [loading, setLoading] = useState(false);
  const [summary, setSummary] = useState<any>(null);

  const handleImport = async () => {
    if (!csvText.trim()) return;
    setLoading(true);
    try {
      const res = await importCsv(csvText);
      setSummary(res);
    } catch (e) {
      alert(String(e));
    } finally {
      setLoading(false);
    }
  };

  if (summary) {
    return (
      <Modal title="Import Complete" onClose={onClose} width={500}>
        <div className="flex flex-col gap-3 text-sm">
          <div className="grid grid-cols-2 gap-2 p-3 bg-bg-base rounded border border-border">
            <div>Total Rows: <strong className="text-text-primary">{summary.total_rows}</strong></div>
            <div>Imported: <strong className="text-status-green">{summary.imported_rows}</strong></div>
            <div>Skipped/Dup: <strong className="text-status-yellow">{summary.skipped_rows}</strong></div>
            <div>Invalid: <strong className="text-status-red">{summary.invalid_rows}</strong></div>
          </div>
          {summary.errors.length > 0 && (
            <div className="text-xs max-h-40 overflow-y-auto mt-2">
              <strong className="text-text-muted mb-1 block">Errors:</strong>
              {summary.errors.map((e: any, i: number) => (
                <div key={i} className="text-status-red">Row {e.row}: {e.reason}</div>
              ))}
            </div>
          )}
          <div className="flex justify-end mt-4">
            <button className="btn btn-primary" onClick={onClose}>Close</button>
          </div>
        </div>
      </Modal>
    );
  }

  return (
    <Modal title="Import Symbols (CSV)" onClose={onClose} width={500}>
      <div className="flex flex-col gap-4">
        <p className="text-xs text-text-muted">
          Paste CSV content below. Required headers: <code className="text-accent bg-accent/10 px-1 rounded">symbol,exchange,token</code>
        </p>
        <textarea
          className="db-input min-h-[200px] font-mono text-[11px]"
          placeholder="symbol,exchange,token&#10;RELIANCE,NSE,2885&#10;TCS,NSE,2953"
          value={csvText}
          onChange={(e) => setCsvText(e.target.value)}
        />
        <div className="flex justify-end gap-2">
          <button className="btn btn-ghost" onClick={onClose}>Cancel</button>
          <button className="btn btn-primary" onClick={handleImport} disabled={loading || !csvText.trim()}>
            {loading ? 'Importing...' : 'Import CSV'}
          </button>
        </div>
      </div>
    </Modal>
  );
}
