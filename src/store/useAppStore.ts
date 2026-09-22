/**
 * DataBridge — Global application store (Zustand)
 * Holds all reactive state consumed by UI components.
 */

import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

// ── Types ───────────────────────────────────────────────────

export interface SystemStatus {
  broker_connected: boolean;
  websocket_connected: boolean;
  amibroker_connected: boolean;
  session_status: string;
  symbols_total: number;
  symbols_active: number;
  backfill_progress_pct: number;
  ticks_per_sec: number;
  errors_last_hour: number;
  ipc_port: string;
  active_broker: string;
}

export interface Symbol {
  id: number;
  symbol: string;
  exchange: string;
  instrument_type: string;
  expiry: string | null;
  strike: number | null;
  option_type: string | null;
  tick_size: number | null;
  lot_size: number | null;
  status: 'enabled' | 'disabled';
  created_at: string;
  updated_at: string;
}

export interface BackfillStatus {
  queued: number;
  downloading: number;
  completed: number;
  failed: number;
}

export interface LogEntry {
  id: number;
  timestamp: string;
  level: 'INFO' | 'WARN' | 'ERROR' | 'DEBUG';
  module: string;
  message: string;
  symbol: string | null;
  error_code: string | null;
}

export interface ImportSummary {
  total_rows: number;
  valid_rows: number;
  duplicate_rows: number;
  invalid_rows: number;
  imported_rows: number;
  skipped_rows: number;
  errors: Array<{ row: number; symbol: string | null; reason: string }>;
}

export interface TickStats {
  ticks_per_sec: number;
  total_ticks: number;
  duplicate_ticks: number;
  dropped_ticks: number;
  active_subscriptions: number;
  ws_state: string;
  last_tick_at: number | null;
}

export interface Settings {
  [key: string]: string;
}

// ── Store ───────────────────────────────────────────────────

interface AppStore {
  // System
  status: SystemStatus | null;
  isLoading: boolean;

  // Symbols
  symbols: Symbol[];
  symbolsLoading: boolean;

  // Backfill
  backfillStatus: BackfillStatus | null;

  // Logs
  logs: LogEntry[];
  logsLoading: boolean;

  // Live
  tickStats: TickStats | null;

  // Settings
  settings: Settings;

  // Error
  lastError: string | null;

  // Actions
  fetchStatus: () => Promise<void>;
  fetchSymbols: (filter?: { exchange?: string; status?: string; search?: string }) => Promise<void>;
  fetchBackfillStatus: () => Promise<void>;
  fetchLogs: (filter?: { level?: string; module?: string }) => Promise<void>;
  fetchTickStats: () => Promise<void>;
  fetchSettings: () => Promise<void>;

  addSymbol: (symbol: string, exchange: string, token: string) => Promise<void>;
  removeSymbol: (id: number) => Promise<void>;
  enableSymbol: (id: number) => Promise<void>;
  disableSymbol: (id: number) => Promise<void>;
  importCsv: (content: string) => Promise<ImportSummary>;
  triggerBackfill: (symbolId: number) => Promise<void>;
  setSetting: (key: string, value: string) => Promise<void>;
  saveBrokerCredentials: (brokerId: string, apiKey: string, apiSecret: string) => Promise<void>;
  clearError: () => void;
}

export const useAppStore = create<AppStore>((set, get) => ({
  status: null,
  isLoading: false,
  symbols: [],
  symbolsLoading: false,
  backfillStatus: null,
  logs: [],
  logsLoading: false,
  tickStats: null,
  settings: {},
  lastError: null,

  fetchStatus: async () => {
    try {
      const status = await invoke<SystemStatus>('get_system_status');
      set({ status });
    } catch (e) {
      set({ lastError: String(e) });
    }
  },

  fetchSymbols: async (filter = {}) => {
    set({ symbolsLoading: true });
    try {
      const symbols = await invoke<Symbol[]>('list_symbols', {
        exchange: filter.exchange ?? null,
        statusFilter: filter.status ?? null,
        search: filter.search ?? null,
        limit: 1000,
        offset: 0,
      });
      set({ symbols, symbolsLoading: false });
    } catch (e) {
      set({ symbolsLoading: false, lastError: String(e) });
    }
  },

  fetchBackfillStatus: async () => {
    try {
      const backfillStatus = await invoke<BackfillStatus>('get_backfill_status');
      set({ backfillStatus });
    } catch (_) {}
  },

  fetchLogs: async (filter = {}) => {
    set({ logsLoading: true });
    try {
      const logs = await invoke<LogEntry[]>('get_logs', {
        level: filter.level ?? null,
        module: filter.module ?? null,
        limit: 500,
        offset: 0,
      });
      set({ logs, logsLoading: false });
    } catch (e) {
      set({ logsLoading: false, lastError: String(e) });
    }
  },

  fetchTickStats: async () => {
    try {
      const tickStats = await invoke<TickStats>('get_tick_stats');
      set({ tickStats });
    } catch (_) {}
  },

  fetchSettings: async () => {
    try {
      const settings = await invoke<Settings>('get_all_settings');
      set({ settings });
    } catch (_) {}
  },

  addSymbol: async (symbol, exchange, token) => {
    await invoke('add_symbol', { symbol, exchange, token });
    await get().fetchSymbols();
  },

  removeSymbol: async (id) => {
    await invoke('remove_symbol', { symbolId: id });
    await get().fetchSymbols();
  },

  enableSymbol: async (id) => {
    await invoke('enable_symbol', { symbolId: id });
    await get().fetchSymbols();
  },

  disableSymbol: async (id) => {
    await invoke('disable_symbol', { symbolId: id });
    await get().fetchSymbols();
  },

  importCsv: async (content) => {
    const result = await invoke<ImportSummary>('import_csv', { csvContent: content });
    await get().fetchSymbols();
    return result;
  },

  triggerBackfill: async (symbolId) => {
    await invoke('trigger_backfill', { symbolId });
    await get().fetchBackfillStatus();
  },

  setSetting: async (key, value) => {
    await invoke('set_setting', { key, value });
    await get().fetchSettings();
  },

  saveBrokerCredentials: async (brokerId, apiKey, apiSecret) => {
    await invoke('save_broker_credentials', {
      brokerId,
      apiKey,
      apiSecret,
      extra: {},
    });
  },

  clearError: () => set({ lastError: null }),
}));
