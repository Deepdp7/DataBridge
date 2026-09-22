-- DataBridge SQLite Migration 001 — Full schema from PRD §17
-- All timestamps stored as UTC ISO8601 TEXT.
-- UNIQUE constraints enforce idempotent inserts throughout the pipeline.

PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

-- ──────────────────────────────────────────────────────────────
-- Broker configuration
-- ──────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS brokers (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    broker_key      TEXT    NOT NULL UNIQUE,    -- e.g. 'mock', 'broker_v1'
    display_name    TEXT    NOT NULL,
    api_base_url    TEXT,
    ws_base_url     TEXT,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT    NOT NULL,
    updated_at      TEXT    NOT NULL
);

-- ──────────────────────────────────────────────────────────────
-- Session / auth state
-- Tokens are stored via OS credential store; this table holds only
-- opaque references and metadata — never raw token values.
-- ──────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS broker_sessions (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    broker_id           INTEGER NOT NULL REFERENCES brokers(id) ON DELETE CASCADE,
    credential_ref      TEXT    NOT NULL,   -- opaque key into OS credential store
    session_status      TEXT    NOT NULL DEFAULT 'unknown',
                                            -- active | expired | invalid | unknown
    token_expires_at    TEXT,               -- UTC ISO8601, nullable
    last_validated_at   TEXT,               -- UTC ISO8601, nullable
    created_at          TEXT    NOT NULL,
    updated_at          TEXT    NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sessions_broker ON broker_sessions(broker_id);

-- ──────────────────────────────────────────────────────────────
-- Symbol master (broker-agnostic)
-- ──────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS symbols (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol           TEXT    NOT NULL,
    exchange         TEXT    NOT NULL,
    instrument_type  TEXT    NOT NULL DEFAULT 'EQ',
    expiry           TEXT,               -- 'YYYY-MM-DD' or NULL
    strike           REAL,
    option_type      TEXT,               -- 'CE' | 'PE' | NULL
    tick_size        REAL,
    lot_size         INTEGER,
    status           TEXT    NOT NULL DEFAULT 'enabled',  -- enabled | disabled
    created_at       TEXT    NOT NULL,
    updated_at       TEXT    NOT NULL,
    UNIQUE(symbol, exchange, instrument_type, expiry, strike, option_type)
);
CREATE INDEX IF NOT EXISTS idx_symbols_exchange ON symbols(exchange);
CREATE INDEX IF NOT EXISTS idx_symbols_status   ON symbols(status);

-- ──────────────────────────────────────────────────────────────
-- Broker-specific token mapping (one row per broker per symbol)
-- ──────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS symbol_mappings (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol_id       INTEGER NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
    broker_id       INTEGER NOT NULL REFERENCES brokers(id)  ON DELETE CASCADE,
    broker_symbol   TEXT    NOT NULL,
    broker_token    TEXT    NOT NULL,
    sync_status     TEXT    NOT NULL DEFAULT 'not_started',
                            -- not_started | queued | downloading | processing |
                            -- completed | partial | failed | retrying
    last_synced_at  TEXT,
    UNIQUE(broker_id, broker_token),
    UNIQUE(symbol_id, broker_id)
);
CREATE INDEX IF NOT EXISTS idx_symbol_mappings_token ON symbol_mappings(broker_id, broker_token);

-- ──────────────────────────────────────────────────────────────
-- Historical OHLCV bars
-- UNIQUE on (symbol_id, interval, timestamp) prevents duplicates.
-- All inserts use ON CONFLICT DO UPDATE (upsert) semantics.
-- ──────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS historical_bars (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol_id   INTEGER NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
    interval    TEXT    NOT NULL DEFAULT '1m',
    timestamp   TEXT    NOT NULL,   -- UTC ISO8601 bar open time
    open        REAL    NOT NULL,
    high        REAL    NOT NULL,
    low         REAL    NOT NULL,
    close       REAL    NOT NULL,
    volume      INTEGER NOT NULL DEFAULT 0,
    source      TEXT    NOT NULL DEFAULT 'backfill',  -- backfill | live_aggregation
    UNIQUE(symbol_id, interval, timestamp)
);
CREATE INDEX IF NOT EXISTS idx_bars_symbol_timestamp ON historical_bars(symbol_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_bars_symbol_interval  ON historical_bars(symbol_id, interval, timestamp);

-- ──────────────────────────────────────────────────────────────
-- Backfill job tracking
-- Jobs persist across application restarts; status drives recovery.
-- ──────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS backfill_jobs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol_id       INTEGER NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
    requested_from  TEXT    NOT NULL,   -- UTC ISO8601
    requested_to    TEXT    NOT NULL,   -- UTC ISO8601
    fetched_up_to   TEXT,               -- UTC ISO8601, updated as chunks complete
    status          TEXT    NOT NULL DEFAULT 'queued',
                            -- queued | downloading | processing | completed |
                            -- partial | failed | retrying | cancelled
    attempt_count   INTEGER NOT NULL DEFAULT 0,
    last_error      TEXT,
    priority        INTEGER NOT NULL DEFAULT 0,  -- higher = higher priority
    created_at      TEXT    NOT NULL,
    updated_at      TEXT    NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_backfill_jobs_symbol ON backfill_jobs(symbol_id);
CREATE INDEX IF NOT EXISTS idx_backfill_jobs_status ON backfill_jobs(status);
CREATE INDEX IF NOT EXISTS idx_backfill_jobs_priority ON backfill_jobs(priority DESC, created_at ASC);

-- ──────────────────────────────────────────────────────────────
-- Detected / repaired data gaps
-- ──────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS backfill_gaps (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol_id     INTEGER NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
    gap_start     TEXT    NOT NULL,   -- UTC ISO8601
    gap_end       TEXT    NOT NULL,   -- UTC ISO8601
    detected_at   TEXT    NOT NULL,
    repaired_at   TEXT,
    status        TEXT    NOT NULL DEFAULT 'open'
                            -- open | repairing | repaired | unrepairable
);
CREATE INDEX IF NOT EXISTS idx_gaps_symbol ON backfill_gaps(symbol_id, status);

-- ──────────────────────────────────────────────────────────────
-- Data quality check results
-- ──────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS data_quality_checks (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol_id     INTEGER NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
    check_type    TEXT    NOT NULL,
                  -- duplicate | out_of_order | missing_ts | invalid_ohlc |
                  -- zero_volume | invalid_price | tz_mismatch
    details       TEXT,
    detected_at   TEXT    NOT NULL,
    resolved      INTEGER NOT NULL DEFAULT 0   -- 0 = open, 1 = resolved
);
CREATE INDEX IF NOT EXISTS idx_dq_symbol ON data_quality_checks(symbol_id);
CREATE INDEX IF NOT EXISTS idx_dq_resolved ON data_quality_checks(resolved, symbol_id);

-- ──────────────────────────────────────────────────────────────
-- Live WebSocket connection / subscription status
-- ──────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS live_status (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    broker_id            INTEGER NOT NULL REFERENCES brokers(id) ON DELETE CASCADE,
    ws_connected         INTEGER NOT NULL DEFAULT 0,
    last_tick_at         TEXT,
    ticks_per_sec        REAL    DEFAULT 0,
    active_subscriptions INTEGER DEFAULT 0,
    updated_at           TEXT    NOT NULL,
    UNIQUE(broker_id)
);

-- ──────────────────────────────────────────────────────────────
-- Application settings (key-value store)
-- ──────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS app_settings (
    key         TEXT    PRIMARY KEY,
    value       TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL
);

-- ──────────────────────────────────────────────────────────────
-- Structured system logs
-- API secrets / access tokens are NEVER written here (see log redaction filter).
-- ──────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS system_logs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp   TEXT    NOT NULL,
    level       TEXT    NOT NULL,   -- INFO | WARN | ERROR | DEBUG
    module      TEXT    NOT NULL,
                        -- Broker | Authentication | WebSocket | Historical |
                        -- Backfill | Database | AmiBroker | System
    message     TEXT    NOT NULL,
    symbol      TEXT,
    error_code  TEXT
);
CREATE INDEX IF NOT EXISTS idx_logs_timestamp      ON system_logs(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_logs_module_level   ON system_logs(module, level);
CREATE INDEX IF NOT EXISTS idx_logs_level          ON system_logs(level);

-- ──────────────────────────────────────────────────────────────
-- Seed the mock broker row (development / no-config default)
-- ──────────────────────────────────────────────────────────────
INSERT OR IGNORE INTO brokers (broker_key, display_name, is_active, created_at, updated_at)
VALUES (
    'mock',
    'Mock Broker (Development)',
    1,
    datetime('now'),
    datetime('now')
);

-- Default application settings
INSERT OR IGNORE INTO app_settings (key, value, updated_at) VALUES
    ('active_broker_id', 'mock',    datetime('now')),
    ('log_level',        'INFO',    datetime('now')),
    ('ipc_port',         '7421',    datetime('now')),
    ('start_with_windows', 'false', datetime('now')),
    ('minimize_to_tray', 'true',    datetime('now')),
    ('auto_reconnect',   'true',    datetime('now')),
    ('max_backfill_concurrent', '3', datetime('now')),
    ('exchange',         'NSE',     datetime('now')),
    ('timezone',         'Asia/Kolkata', datetime('now')),
    ('market_open_time', '09:15',   datetime('now')),
    ('market_close_time','15:30',   datetime('now'));
