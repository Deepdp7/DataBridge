# Product Requirements Document (PRD)
# DataBridge — Windows Desktop Market Data Bridge for AmiBroker

**Version:** 1.0
**Status:** Draft — Implementation Ready
**Owner:** Product Engineering
**Platform:** Windows 10/11 (x64)

---

## 1. Executive Summary

DataBridge is a Windows-first desktop application that connects a supported stock broker's market data API to **AmiBroker**, acting as a reliable, self-healing bridge for both historical and real-time tick data. It automatically backfills up to one year of historical 1-minute OHLCV data for every configured symbol, streams live tick-by-tick data during market hours, normalizes broker-specific data into a common internal format, and delivers both historical and real-time data into AmiBroker through a native Data Plugin DLL built with the AmiBroker Development Kit (ADK).

The application is built on Tauri 2 with a React/TypeScript frontend and a Rust core engine, backed by SQLite for local storage. The architecture is explicitly designed to support additional brokers in the future without rewriting the core, and to extend storage to DuckDB/Parquet for high-volume tick data without a full redesign.

DataBridge is purpose-built to support **AmiBroker-based algo trading workflows**, but strictly as the **market-data layer** in that workflow. The intended flow is:

```text
Broker → DataBridge → AmiBroker → Algo Strategy
```

DataBridge provides:
- 1-year historical market data
- Live tick-by-tick market data
- Real-time 1-minute OHLCV aggregation
- Reliable data delivery to AmiBroker
- Auto reconnect and gap recovery

DataBridge does **NOT** provide, in the MVP:
- Order execution
- Buy/Sell order routing
- Broker order management
- Strategy execution inside DataBridge

All order routing, strategy execution, and trade decisions remain entirely within AmiBroker (and any AFL/algo layer built on top of it) or the broker's own order-entry systems — never inside DataBridge itself.

---

## 2. Problem Statement

Traders and analysts who use AmiBroker for charting and strategy development need a dependable way to feed it both historical and real-time market data from their broker account. Existing solutions are often:

- Broker-specific and hard to extend
- Fragile around reconnection, session expiry, and network interruptions
- Prone to data gaps, duplicate candles, or corrupted historical series
- Poorly instrumented, leaving users unable to diagnose why data stopped flowing
- Not designed for hundreds/thousands of symbols with efficient batch backfill and incremental sync

DataBridge solves this by providing a purpose-built, professional-grade bridge application with automatic recovery, gap detection, and a clear operational dashboard — while cleanly separating broker logic, core data processing, and AmiBroker integration.

---

## 3. Product Vision

To be the most reliable, transparent, and extensible market-data bridge for AmiBroker users — one that "just works" in the background, self-heals from interruptions, and gives the user complete visibility into data health, without ever compromising data integrity or requiring babysitting.

Long-term vision: a multi-broker, multi-timeframe data engine that remains architecturally broker-agnostic and can extend into tick-level historical storage and market replay, while staying strictly a **data** product. DataBridge exists to reliably power AmiBroker-based algo trading workflows with clean, complete market data — it is not, and will not become, an order execution or trade routing product; execution and strategy logic remain the responsibility of AmiBroker and the broker's own systems.

---

## 4. Target Users

| User Type | Description | Needs |
|---|---|---|
| Retail algo/discretionary trader | Uses AmiBroker for charting/backtesting, has a broker API subscription | Reliable live + historical feed, minimal setup |
| Semi-professional trader managing large watchlists | Tracks hundreds of symbols (NSE cash, F&O) | Bulk CSV import, fast backfill, low-latency live feed |
| Small trading desk / prop-adjacent user | Runs AmiBroker on a dedicated Windows machine 24/7 | Auto-start, tray operation, robust auto-recovery, logs for diagnostics |
| AmiBroker power user / AFL developer | Wants clean, gap-free 1-minute data for backtesting | Data integrity tools, gap detection/repair, historical browsing |

---

## 5. Goals

1. Provide a stable bridge between one supported broker API and AmiBroker via a native Data Plugin.
2. Support CSV-based bulk symbol import with clear validation and import summaries.
3. Automatically backfill 1 year of 1-minute historical OHLCV data per symbol, then keep data current via incremental (gap-only) sync.
4. Stream tick-by-tick live data during market hours and normalize it into a common internal tick model.
5. Aggregate ticks into 1-minute OHLCV bars in real time, with infrastructure for additional intervals later.
6. Automatically recover from network, API, WebSocket, session, and application-restart interruptions without data loss or duplication.
7. Provide a professional, dark-themed, information-dense desktop dashboard with live status, logs, and settings.
8. Run quietly in the Windows system tray and optionally auto-start with Windows.
9. Architect the system so a second/third broker adapter can be added without touching the core engine or the AmiBroker plugin.
10. Keep the AmiBroker plugin fully decoupled from broker APIs — it only ever talks to the DataBridge Core.

---

## 6. Non-Goals (Out of Scope for MVP and Near-Term)

DataBridge is built specifically to power AmiBroker-based algo trading, but only as the data-delivery layer (`Broker → DataBridge → AmiBroker → Algo Strategy`). Accordingly, DataBridge itself does **not** provide:

- Order execution
- Buy/Sell order routing
- Broker order management
- Strategy execution inside DataBridge
- Portfolio management or P&L tracking
- Cloud synchronization or remote access
- Mobile application
- Web-based dashboard
- Multi-user / SaaS / multi-tenant operation
- Paid subscription billing infrastructure
- AI-driven features or predictive analytics
- Options chain analytics
- Full tick-level historical backfill (unless a future phase's broker capability and storage design explicitly support it)

These are documented in detail in Section 23 (MVP Scope) and Section 24 (Future Roadmap).

---

## 7. Core Features

1. **Broker Connectivity** — authenticate, maintain session, detect and refresh expiry.
2. **Symbol Management** — manual add, CSV bulk import, enable/disable, search/filter, sync status.
3. **Historical Backfill Engine** — 1-year initial backfill, incremental gap-only sync, retry/backoff.
4. **Live Tick Engine** — WebSocket subscription, tick normalization, reconnection, resubscription.
5. **Tick-to-Bar Aggregation** — real-time 1-minute OHLCV construction from ticks.
6. **AmiBroker Native Plugin** — historical + real-time data delivery via `DataBridge.dll`.
7. **Auto Recovery** — reconnect, resubscribe, gap-detect, gap-backfill across every failure mode.
8. **Dashboard** — connection status, symbol counts, backfill progress, tick rate, resource usage.
9. **Symbols, Live Monitor, Historical Data, Logs, Settings pages.**
10. **System Tray & Windows Startup integration.**
11. **Data Integrity Tooling** — duplicate/gap/out-of-order detection and repair.
12. **Security** — credential protection, log redaction, local-only IPC, CSV validation.

---

## 8. User Stories

### Broker Connection
- As a user, I want to enter my broker API credentials once and have DataBridge maintain my session, so I don't need to manually re-authenticate every day.
- As a user, I want to be clearly told when my session has expired and see DataBridge automatically attempt to re-authenticate.

### Symbol Management
- As a user, I want to import a CSV of hundreds of symbols and see exactly how many were imported, skipped, or invalid.
- As a user, I want to enable/disable individual symbols without deleting their historical data.

### Historical Backfill
- As a user, when I add a new symbol, I want DataBridge to automatically fetch a year of 1-minute history without my intervention.
- As a user, I want DataBridge to only download the data it's missing on subsequent syncs, not re-download everything.
- As a user, I want to see per-symbol backfill progress and retry failed backfills manually if needed.

### Live Data
- As a user, I want live ticks to start flowing automatically at market open for all enabled symbols.
- As a user, I want DataBridge to reconnect and resubscribe automatically if my internet drops mid-session.

### AmiBroker Integration
- As a user, I want AmiBroker to show correct historical and live data for my symbols without manual configuration beyond installing the plugin.
- As a user, I want to know immediately if the AmiBroker plugin loses connection to DataBridge Core.

### Operations
- As a user, I want DataBridge to start with Windows and run minimized in the tray so I don't have to think about it.
- As a user, I want to see logs and error details when something goes wrong, without exposing my API secrets.

---

## 9. Functional Requirements

### 9.1 Broker Authentication
- FR-1: The system shall support automatic OAuth 2.0 authorization flows (e.g., FYERS V3). It shall check for a valid session on startup; if none exists, it shall automatically open the broker's login page in the user's default browser, handle the callback redirect, exchange the auth code for an access token, and auto-connect.
- FR-2: Credentials (access tokens, session tokens) shall be stored using OS-level secure storage (Windows Credential Manager / DPAPI) rather than plain text where technically feasible. The user shall not be required to manually paste access tokens.
- FR-3: The system shall validate session validity on startup and periodically thereafter.
- FR-4: The system shall detect token expiry and attempt automated re-authentication where the broker API supports it.
- FR-5: Authentication errors shall be surfaced with a clear, user-readable reason (not a raw API error dump).
- FR-6: Credentials and tokens shall never be written to standard application logs.
- FR-6b: Upon successful authorization, the system shall immediately and automatically synchronize the latest broker symbol master.

### 9.2 Symbol Management
- FR-7: Users shall be able to add symbols manually. The system shall support automatic Symbol Master Synchronization, downloading the latest symbol master from the broker, detecting added/removed/expired/changed instruments, and updating token mappings automatically without manual token entry.
- FR-8: Users shall be able to import and export symbols via CSV (columns: `symbol, exchange, segment, token, instrument_type, expiry, strike, option_type, enabled`). The export format must be stable for re-import.
- FR-9: The system shall validate every CSV row during import, checking for duplicates, and produce an import summary (total/valid/duplicate/invalid/imported/skipped) and report invalid rows clearly.
- FR-10: Users shall be able to enable/disable, remove, edit, and search/filter symbols by exchange, segment, or status.
- FR-11: Each symbol shall display its historical sync status and live subscription status. The Symbol Setup page shall also display the last symbol master sync time, sync status, instrument counts, and a manual "Sync Now" button.
- FR-11b: The system shall support specific instrument segments: EQ.NSE, FUTSTK.NFO, FUTIDX.NFO, OPTIDX.NFO, OPTSTK.NFO, COMDTY.MCX, FUTCOM.MCX, FUTIDX.MCX, OPTFUT.MCX, OPTIDX.MCX, mapped correctly to broker-specific identifiers using a normalized internal segment model.

### 9.3 Historical Backfill
- FR-12: On symbol add/enable, the system shall automatically enqueue a 1-year, 1-minute OHLCV backfill job.
- FR-13: The system shall never re-download the full history once an initial backfill is complete; it shall detect and fetch only the missing period (from last stored timestamp to now).
- FR-14: The system shall respect broker API rate limits via a token-bucket or leaky-bucket limiter per broker adapter.
- FR-15: Failed backfill requests shall retry with exponential backoff up to a configurable maximum attempt count.
- FR-16: Backfill jobs shall be resumable after an application restart or crash.
- FR-17: Historical inserts shall be idempotent (safe to re-run without creating duplicates).

### 9.4 Live Tick Engine
- FR-18: The system shall connect to the broker's WebSocket feed and subscribe to all enabled symbols during market hours.
- FR-19: Ticks shall be normalized into the internal Tick model (Section 15) and processed in a non-blocking pipeline with bounded queues to prevent memory issues.
- FR-20: The system shall automatically reconnect on WebSocket disconnect and resubscribe all previously subscribed symbols.
- FR-21: The system shall maintain a heartbeat/ping-pong cycle per broker protocol requirements.
- FR-22: The system shall track and expose tick-processing statistics (ticks/sec, 1-second bars/sec, dropped ticks, duplicate ticks, processing latency, queue depth) to ensure sufficient performance.

### 9.5 Tick-to-Bar Aggregation
- FR-23: The system shall aggregate incoming ticks into **1-second** and **1-minute** OHLCV bars in real time. The 1-second bars must be generated purely from real-time ticks.
- FR-24: The aggregation engine shall correctly handle first-tick-of-interval, high/low/close updates, volume accumulation, missing ticks, and market session boundaries, prioritizing minimal allocations in the hot tick path.
- FR-25: The 1-second bars shall be efficiently batched and pushed to AmiBroker, supporting smooth 1-second charting.

### 9.6 AmiBroker Integration
- FR-26: The system shall provide a native Data Plugin DLL (`DataBridge.dll`) built against the AmiBroker ADK.
- FR-27: The plugin shall retrieve historical data from DataBridge Core.
- FR-28: The plugin shall receive real-time 1-second and 1-minute updates from DataBridge Core via local IPC and push them into AmiBroker efficiently.
- FR-29: The plugin shall handle unknown/unmapped symbols gracefully with a clear "symbol not found" state.
- FR-30: The user must be able to select the AmiBroker Plugins folder during setup and install the DLL automatically.

### 9.7 Recovery
- FR-31: The system shall automatically recover from internet loss, API timeout, WebSocket disconnect, session expiry, application restart, unexpected crash, partial historical download, database write failure, and AmiBroker plugin disconnect, per the flows in Section 22.
- FR-32: On reconnection, the system shall always check for and backfill any historical gap created during the outage before resuming/alongside live ticks.
- FR-33: Duplicate ticks and duplicate candles shall be detected and rejected during recovery merges.

### 9.8 Dashboard & UI
- FR-34: The dashboard shall display broker, WebSocket, and AmiBroker connection status; symbol counts; backfill progress; live tick rate; CPU/memory usage; database size; last tick time; and error count.
- FR-35: The Symbols, Live Monitor, Historical Data, Logs, and Settings pages shall be implemented as specified in Sections 18.2–18.6.
- FR-36: The Live Monitor shall use list virtualization and shall not render unbounded DOM rows for high-frequency data.

### 9.9 System Integration
- FR-37: The system shall support minimizing to the Windows system tray with color-coded status (green/yellow/red).
- FR-38: The system shall support an optional "Start with Windows" mode that starts the core engine without requiring the dashboard to open.

---

## 10. System Architecture

```text
                    ┌──────────────────────┐
                    │      Broker API       │
                    │  REST + WebSocket     │
                    └──────────┬────────────┘
                               │
                               ▼
                    ┌──────────────────────┐
                    │      Rust Core        │
                    │                       │
                    │ Broker Adapter        │
                    │ Symbol Manager        │
                    │ Historical Engine     │
                    │ Live Tick Engine      │
                    │ Data Normalizer       │
                    │ Storage Engine        │
                    │ Recovery Manager      │
                    └──────────┬────────────┘
                               │
                  ┌────────────┴────────────┐
                  ▼                          ▼
          ┌──────────────┐          ┌────────────────┐
          │ SQLite        │          │ Local IPC/API   │
          │ Data Store    │          │ / WebSocket     │
          └──────────────┘          └───────┬─────────┘
                                             │
                                             ▼
                                  ┌────────────────────┐
                                  │ DataBridge.dll      │
                                  │ AmiBroker Plugin     │
                                  └─────────┬───────────┘
                                             │
                                             ▼
                                      ┌───────────┐
                                      │ AmiBroker │
                                      └───────────┘
```

### 10.1 Layer Responsibilities

**Broker API (external)**
Provides authentication, REST endpoints for symbol master/historical data, and a WebSocket feed for live ticks. Treated entirely as an external dependency accessed only through the Broker Adapter.

**Rust Core (DataBridge Core)**
The heart of the system, running as a set of async Tokio tasks:
- *Broker Adapter*: Implements the `BrokerAdapter` trait for a specific broker; the only layer allowed to know broker-specific API shapes.
- *Symbol Manager*: Owns the symbol master, CSV import/validation, and symbol-to-token mapping.
- *Historical Engine*: Manages the backfill queue, per-symbol jobs, rate limiting, retries, and gap detection.
- *Live Tick Engine*: Manages the WebSocket connection lifecycle, subscriptions, and raw tick ingestion.
- *Data Normalizer*: Converts broker-specific historical bars and ticks into DataBridge's internal models.
- *Storage Engine*: Persists all state to SQLite behind a repository abstraction (swap-ready for DuckDB/Parquet for tick-scale data).
- *Recovery Manager*: Observes connection/health state and drives reconnect → re-auth → resubscribe → gap-check → gap-backfill → resume sequences.

**SQLite Data Store**
Local persistence for configuration, symbol master, historical bars, backfill/job state, logs, and settings (see Section 17).

**Local IPC/API (Local WebSocket / Named Pipe / Localhost TCP)**
Exposes a local, loopback-only interface used by (a) the React/Tauri frontend and (b) the AmiBroker plugin. See Section 14 for the chosen mechanism and rationale.

**DataBridge.dll (AmiBroker Plugin)**
A native C++ plugin built with the AmiBroker ADK. It never talks to any broker directly — only to DataBridge Core over local IPC. It serves historical data on request and pushes real-time updates as they arrive.

**AmiBroker**
The end consumer of both historical and real-time data, driven entirely through the standard AmiBroker Data Plugin interface.

---

## 11. Data Architecture

### 11.1 Data Flow Summary

```text
Broker Historical API → Historical Engine → Normalizer → SQLite (historical_bars)
                                                              │
                                                              ▼
                                              Local IPC ← Storage Engine (read path)
                                                              │
                                                              ▼
                                                     DataBridge.dll → AmiBroker

Broker WebSocket → Live Tick Engine → Normalizer → Tick-to-Bar Aggregator
                                            │                  │
                                            ▼                  ▼
                                  Local IPC (tick stream)   SQLite (recent bars, optional)
                                            │
                                            ▼
                                  DataBridge.dll → AmiBroker
```

### 11.2 Storage Evolution Strategy

The Storage Engine is implemented behind a repository trait (e.g., `HistoricalRepository`, `TickRepository`) so that:
- MVP: all data (metadata + historical OHLCV) lives in SQLite.
- Future: high-volume tick storage can be redirected to DuckDB/Parquet files on disk, with SQLite retained for metadata, configuration, and sync state. No calling code outside the Storage Engine needs to change, since consumers depend only on repository trait interfaces.

### 11.3 Timezone Strategy
- All internally stored timestamps are UTC (ISO 8601 / Unix epoch millis) in SQLite.
- Exchange/session-local time (e.g., IST for NSE) is applied only at the presentation layer (UI) and, where required, at the AmiBroker plugin boundary, using the exchange timezone defined in Market Session Management (Section 16).
- The Data Normalizer is exclusively responsible for converting broker-supplied timestamps into UTC before storage.

---

## 12. Broker Adapter Architecture

### 12.1 Abstraction

```rust
#[async_trait]
pub trait BrokerAdapter: Send + Sync {
    async fn authenticate(&self, creds: BrokerCredentials) -> Result<Session, BrokerError>;
    async fn refresh_session(&self, session: &Session) -> Result<Session, BrokerError>;
    async fn get_symbol_master(&self) -> Result<Vec<BrokerSymbol>, BrokerError>;
    async fn get_historical_data(
        &self,
        token: &str,
        interval: Interval,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<RawBar>, BrokerError>;
    async fn connect_websocket(&self) -> Result<WsHandle, BrokerError>;
    async fn subscribe_symbols(&self, ws: &WsHandle, tokens: &[String]) -> Result<(), BrokerError>;
    async fn unsubscribe_symbols(&self, ws: &WsHandle, tokens: &[String]) -> Result<(), BrokerError>;
    async fn disconnect(&self, ws: &WsHandle) -> Result<(), BrokerError>;
}
```

The first concrete implementation is `BrokerAdapterV1`, registered in a small adapter registry keyed by broker ID. No module outside `broker/<broker_name>/` may reference broker-specific request/response shapes, field names, or rate-limit rules.

### 12.2 Adding a New Broker (Future)
1. Create `broker/<new_broker>/` implementing `BrokerAdapter`.
2. Implement broker-specific auth flow, REST client, WebSocket client, and field mapping into `BrokerSymbol` / `RawBar` / `RawTick`.
3. Register the adapter in the broker registry with a unique broker ID.
4. Add broker-specific settings fields to the Settings UI (data-driven, not hardcoded per broker).
5. No changes required in Historical Engine, Live Tick Engine, Storage Engine, Recovery Manager, or the AmiBroker plugin.

### 12.3 Edge Cases
- Broker returns partial/malformed symbol master → log and skip malformed rows, do not fail entire sync.
- Broker WebSocket protocol differs (binary vs JSON) → handled entirely inside the adapter's decode step, upstream code only ever sees normalized `RawTick`.
- Broker enforces different rate limits for historical vs real-time endpoints → each adapter exposes its own rate-limit configuration consumed by the Historical Engine's limiter.

---

## 13. Historical Backfill Architecture

### 13.1 Purpose
Ensure every enabled symbol has a complete, gap-free 1-year, 1-minute OHLCV history, and keep it current with minimal API usage.

### 13.2 Pipeline

```text
Symbol enabled/added
        │
        ▼
Backfill Queue (priority-ordered)
        │
        ▼
Per-symbol Backfill Job
        │
        ▼
Check last stored timestamp (if any)
        │
   ┌────┴─────┐
   │          │
 None      Exists
   │          │
   ▼          ▼
Full 1yr   Missing-period only
 fetch        fetch
   │          │
   └────┬─────┘
        ▼
Batch requests (respecting broker limits)
        │
        ▼
Validate + normalize (Section 13.5)
        │
        ▼
Idempotent upsert into historical_bars
        │
        ▼
Update backfill_jobs status + progress
```

### 13.3 Job Model
- **Backfill Queue**: FIFO with priority override (manual "Backfill Now" jumps the queue).
- **Per-symbol Job**: tracks symbol_id, requested range, fetched range, status, attempt count, last error.
- **Batch Requests**: historical ranges are chunked (e.g., 30–90 day windows) to respect broker payload/time-range limits.
- **Rate Limiting**: token-bucket limiter configured per broker adapter; jobs queue/wait rather than fail when the bucket is empty.
- **Retry Policy**: exponential backoff (e.g., 1s, 2s, 4s, 8s... capped) with a max attempt count (default 5) before marking `Failed`.
- **Partial Failure Handling**: a job that succeeds for part of a range is marked `Partial`, retaining the fetched portion and re-queuing only the remaining sub-range.
- **Duplicate Prevention**: unique constraint on `(symbol_id, timestamp)` in `historical_bars`; upserts are idempotent.
- **Resume After Restart**: job state is persisted in `backfill_jobs`; on startup, any job not in a terminal state (`Completed`/`Failed`) is re-queued.
- **Cancellation**: user can cancel a `Queued` or `Downloading` job; already-fetched data is retained.
- **Priority**: manual symbol backfill requests and newly added symbols are prioritized over routine incremental syncs.

### 13.4 Gap Detection & Incremental Sync
- On each sync cycle (scheduled and on-reconnect), the engine compares the latest stored bar timestamp against current time (bounded by market session) and requests only the missing window.
- A dedicated **Gap Detection** pass also scans stored data for internal gaps (e.g., caused by a past partial failure) by checking for missing expected minute-bars within market hours, and enqueues repair jobs for `backfill_gaps`.

### 13.5 Data Validation Rules (feeding into Section 13.6/Data Integrity)
- Reject bars with `high < low`, `high < open`, `high < close`, `low > open`, or `low > close`.
- Flag (not necessarily reject) zero-volume bars during active market hours as suspicious.
- Reject bars with non-monotonic or duplicate timestamps within the same fetch batch (dedupe before insert).
- Normalize all timestamps to UTC before validation.

### 13.6 Edge Cases
- Broker historical API returns fewer bars than the requested range implies (holiday/half-day) → treated as valid, not a gap, if it aligns with the holiday calendar (Section 16).
- Symbol newly listed (less than 1 year of history exists) → backfill only from listing date; mark job `Completed` once broker returns an empty page for earlier dates.
- Backfill running when application is closed → job persists as `Downloading`/`Queued` and resumes on next launch.

---

## 14. Live Tick Architecture

### 14.1 Purpose
Deliver low-latency, gap-aware, normalized tick data from the broker WebSocket to both the local database (for recent-bar reconstruction) and AmiBroker, for all enabled symbols during market hours.

### 14.2 Pipeline

```text
Market session becomes "Open"
        │
        ▼
Live Tick Engine connects WebSocket
        │
        ▼
Subscribe to all enabled symbols
        │
        ▼
Raw broker tick received
        │
        ▼
Data Normalizer → internal Tick model
        │
        ▼
Tick-to-Bar Aggregator (1-min OHLCV)
        │
   ┌────┴─────┐
   ▼          ▼
Local IPC   SQLite (optional live buffer)
   │
   ▼
DataBridge.dll → AmiBroker
```

### 14.3 Requirements Detail
- Connection uses a dedicated Tokio task with its own supervised lifecycle, independent of REST/backfill tasks.
- Subscriptions are batched to respect broker per-message symbol limits (if any).
- Heartbeat/ping-pong follows the broker's protocol; missed heartbeats beyond a threshold trigger a proactive reconnect rather than waiting for a hard disconnect.
- Sequence/order handling: if the broker provides sequence numbers, out-of-order or skipped sequences are logged and, where feasible, trigger a targeted re-sync request.
- Duplicate tick protection: ticks with an identical `(symbol, exchange_timestamp, ltp, last_quantity)` fingerprint received within a short window are deduplicated before aggregation.
- Tick processing statistics (ticks/sec, per-symbol last tick time, dropped/duplicate counts) are maintained in-memory and exposed via the local API for the dashboard.

### 14.4 Edge Cases
- Symbol added mid-session → immediately subscribed without requiring a full reconnect.
- WebSocket delivers a tick for an unsubscribed/unknown token → logged and discarded, does not crash the pipeline.
- Extremely high tick volume (F&O expiry days) → backpressure is applied via bounded Tokio channels (Section 21) rather than unbounded buffering.

---

## 15. Internal Tick, Bar, and Segment Models

### 15.1 Segment Model

```text
Segment
- exchange
- segment              (e.g., EQ.NSE, FUTSTK.NFO)
- instrument_type
- symbol
- broker_token
- expiry               (nullable)
- strike               (nullable)
- option_type          (nullable)
- lot_size
- tick_size
```

### 15.2 Tick

```text
Tick
- symbol
- exchange
- broker_token
- timestamp            (UTC, normalized)
- ltp
- last_quantity
- volume               (nullable)
- bid                  (nullable)
- bid_quantity         (nullable)
- ask                  (nullable)
- ask_quantity         (nullable)
- open                 (nullable)
- high                 (nullable)
- low                  (nullable)
- previous_close       (nullable)
```

Fields not supported by a given broker are always `null`, never a sentinel value like `0`.

### 15.3 Bar (OHLCV)

```text
Bar
- symbol
- exchange
- interval            (e.g., "1s", "1m")
- timestamp           (bar open time, UTC)
- open
- high
- low
- close
- volume
- source              ("backfill" | "live_aggregation")
```

### 15.4 Tick-to-Bar Aggregation Engine
Supports (infrastructure-level) intervals: tick, 1s, 5s, 1m, 5m, 15m, 30m, 1h, daily. MVP activates **tick → 1-second → 1-minute**.

State machine per symbol:
```text
First tick of minute   → open bar (O=H=L=C=ltp, V=last_quantity)
Subsequent tick        → update H (max), L (min), C (=ltp), V (+=last_quantity or delta)
Minute boundary crossed → close current bar, emit to IPC + storage, open new bar
No ticks in a minute   → no bar emitted (AmiBroker/analytics treat as no trade; documented behavior, not a bug)
Market session close   → force-close the in-progress bar
```

---

## 16. Market Session Management

### 16.1 Requirements
- Configurable market open/close times per exchange, stored with exchange timezone (e.g., `Asia/Kolkata` for NSE).
- Pre-market window flagged as a **future feature** (config field reserved, not activated in MVP).
- Weekend detection based on exchange calendar (no live engine activation on non-trading weekdays).
- Holiday calendar: an editable list of exchange holidays (importable, with sane defaults for the primary exchange) used to suppress both live activation and gap-detection false positives.
- Session state machine drives when the Live Tick Engine connects/disconnects and when the Historical Engine treats "no data" as expected vs. a gap.

### 16.2 Session States

```text
Market Closed → Market Opening → Market Open → Market Closing → Market Closed
```

- **Market Closed**: Live Tick Engine disconnected (or idle); dashboard shows closed status.
- **Market Opening**: pre-open window (reserved); WebSocket connection is pre-warmed shortly before open.
- **Market Open**: WebSocket connected and subscribed; ticks flow, aggregation active.
- **Market Closing**: final ticks processed, last bar of the day force-closed.
- **Market Closed**: WebSocket may be gracefully disconnected to conserve broker connection quota; end-of-day backfill sync runs to catch any final bars.

---

## 17. Database Schema (SQLite)

```sql
-- Broker configuration
CREATE TABLE brokers (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    broker_key      TEXT NOT NULL UNIQUE,      -- e.g. 'broker_v1'
    display_name    TEXT NOT NULL,
    api_base_url    TEXT,
    ws_base_url     TEXT,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- Session/auth state (tokens stored via OS secure store; this table holds references/metadata only)
CREATE TABLE broker_sessions (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    broker_id           INTEGER NOT NULL REFERENCES brokers(id),
    credential_ref      TEXT NOT NULL,   -- opaque reference into OS credential store
    session_status      TEXT NOT NULL,   -- 'active' | 'expired' | 'invalid' | 'unknown'
    token_expires_at    TEXT,
    last_validated_at   TEXT,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL
);

-- Symbol master
CREATE TABLE symbols (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol           TEXT NOT NULL,
    exchange         TEXT NOT NULL,
    instrument_type  TEXT NOT NULL DEFAULT 'EQ',
    expiry           TEXT,
    strike           REAL,
    option_type      TEXT,             -- 'CE' | 'PE' | NULL
    tick_size        REAL,
    lot_size         INTEGER,
    status           TEXT NOT NULL DEFAULT 'enabled',  -- 'enabled' | 'disabled'
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL,
    UNIQUE(symbol, exchange, instrument_type, expiry, strike, option_type)
);
CREATE INDEX idx_symbols_exchange ON symbols(exchange);
CREATE INDEX idx_symbols_status ON symbols(status);

-- Broker-specific token mapping
CREATE TABLE symbol_mappings (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol_id       INTEGER NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
    broker_id       INTEGER NOT NULL REFERENCES brokers(id),
    broker_symbol   TEXT NOT NULL,
    broker_token    TEXT NOT NULL,
    sync_status     TEXT NOT NULL DEFAULT 'not_started', -- see Section 10 states
    last_synced_at  TEXT,
    UNIQUE(broker_id, broker_token),
    UNIQUE(symbol_id, broker_id)
);
CREATE INDEX idx_symbol_mappings_token ON symbol_mappings(broker_id, broker_token);

-- Historical OHLCV bars
CREATE TABLE historical_bars (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol_id   INTEGER NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
    interval    TEXT NOT NULL DEFAULT '1m',
    timestamp   TEXT NOT NULL,   -- UTC ISO8601
    open        REAL NOT NULL,
    high        REAL NOT NULL,
    low         REAL NOT NULL,
    close       REAL NOT NULL,
    volume      INTEGER NOT NULL DEFAULT 0,
    source      TEXT NOT NULL DEFAULT 'backfill',
    UNIQUE(symbol_id, interval, timestamp)
);
CREATE INDEX idx_bars_symbol_timestamp ON historical_bars(symbol_id, timestamp);

-- Backfill job tracking
CREATE TABLE backfill_jobs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol_id       INTEGER NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
    requested_from  TEXT NOT NULL,
    requested_to    TEXT NOT NULL,
    fetched_up_to   TEXT,
    status          TEXT NOT NULL DEFAULT 'queued', -- queued|downloading|processing|completed|partial|failed|retrying
    attempt_count   INTEGER NOT NULL DEFAULT 0,
    last_error      TEXT,
    priority        INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);
CREATE INDEX idx_backfill_jobs_symbol ON backfill_jobs(symbol_id);
CREATE INDEX idx_backfill_jobs_status ON backfill_jobs(status);

-- Detected/repaired gaps
CREATE TABLE backfill_gaps (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol_id     INTEGER NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
    gap_start     TEXT NOT NULL,
    gap_end       TEXT NOT NULL,
    detected_at   TEXT NOT NULL,
    repaired_at   TEXT,
    status        TEXT NOT NULL DEFAULT 'open' -- open|repairing|repaired|unrepairable
);
CREATE INDEX idx_gaps_symbol ON backfill_gaps(symbol_id, status);

-- Data quality check results
CREATE TABLE data_quality_checks (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol_id     INTEGER NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
    check_type    TEXT NOT NULL,  -- duplicate|out_of_order|missing_ts|invalid_ohlc|zero_volume|invalid_price|tz_mismatch
    details       TEXT,
    detected_at   TEXT NOT NULL,
    resolved      INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_dq_symbol ON data_quality_checks(symbol_id);

-- Live connection/subscription status
CREATE TABLE live_status (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    broker_id          INTEGER NOT NULL REFERENCES brokers(id),
    ws_connected       INTEGER NOT NULL DEFAULT 0,
    last_tick_at       TEXT,
    ticks_per_sec      REAL DEFAULT 0,
    active_subscriptions INTEGER DEFAULT 0,
    updated_at         TEXT NOT NULL
);

-- Application settings (key-value)
CREATE TABLE app_settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

-- Structured system logs
CREATE TABLE system_logs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp   TEXT NOT NULL,
    level       TEXT NOT NULL,      -- INFO|WARN|ERROR|DEBUG
    module      TEXT NOT NULL,      -- Broker|Authentication|WebSocket|Historical|Backfill|Database|AmiBroker|System
    message     TEXT NOT NULL,
    symbol      TEXT,
    error_code  TEXT
);
CREATE INDEX idx_logs_timestamp ON system_logs(timestamp);
CREATE INDEX idx_logs_module_level ON system_logs(module, level);
```

**Notes:**
- `UNIQUE(symbol_id, interval, timestamp)` on `historical_bars` is the primary duplicate-prevention constraint; all inserts use `INSERT ... ON CONFLICT DO UPDATE` (upsert) semantics.
- `symbol_mappings` enforces one mapping per (broker, token) and one mapping per (symbol, broker), preventing token collisions.
- Composite indexes on `(symbol_id, timestamp)` and `(broker_id, broker_token)` satisfy the two hot lookup paths called out in the requirements.

---

## 18. UI/UX Specification

### 18.1 Style Direction
Dark, professional trading-terminal aesthetic: compact, information-dense tables; unambiguous color-coded connection indicators (green/yellow/red); minimal animation; clear, non-alarmist error styling; monospace or semi-condensed typography for numeric tables; keyboard navigation for symbol/table pages; no decorative chrome.

### 18.2 Dashboard

```text
DataBridge

Broker         ● Connected
WebSocket      ● Connected
AmiBroker      ● Connected

Symbols            325
Backfill            87%
Live Symbols        325
Ticks/sec         1,248
Errors                0
```

Widgets: broker status, WebSocket status, AmiBroker status, total/active symbols, backfill progress, live tick rate, CPU usage, memory usage, database size, last received tick, error count.

### 18.3 Symbols Page

**Columns:** Symbol, Exchange, Token, Historical Status, Last Historical Date, Live Status, LTP, Last Tick
**Actions:** Import CSV, Add Symbol, Remove, Enable, Disable, Backfill, Retry, Refresh

### 18.4 Live Monitor

**Columns:** Timestamp, Symbol, LTP, Last Qty, Volume, Bid, Ask, Latency
**Stats:** Ticks/sec, Total ticks, Active WebSocket subscriptions, Last tick time, Connection latency (where measurable)
**Constraints:** search/filter available; rows are virtualized (windowed rendering) — the DOM never holds the full unbounded tick history.

### 18.5 Historical Data Page

Capabilities: search symbol, select date range, select timeframe, view record count, view first/last timestamp, view missing periods, re-run backfill, repair gaps.

### 18.6 Logs Page

Levels: INFO, WARN, ERROR, DEBUG
Categories: Broker, Authentication, WebSocket, Historical, Backfill, Database, AmiBroker, System
Columns: Timestamp, Level, Module, Message, Symbol (if relevant), Error code (if available)
**Rule:** API secrets/access tokens are never rendered or stored in this table.

### 18.7 Settings

- **Broker**: API credentials, session configuration
- **Data**: historical interval, data retention, storage location
- **Market**: exchange, timezone, market hours
- **AmiBroker**: plugin path, database path (if required), IPC configuration
- **Application**: start with Windows, minimize to tray, auto-start engine, log level, auto reconnect, auto update

### 18.8 System Tray

Actions: Show dashboard, Start/stop data engine, Show connection status, Exit application.
Status colors: **Green** = all systems connected, **Yellow** = warning/reconnecting, **Red** = disconnected.

---

## 19. AmiBroker Integration Architecture

### 19.1 Plugin Overview

`DataBridge.dll` is a native AmiBroker Data Plugin built with the AmiBroker ADK (C++). It is responsible only for:
1. Presenting DataBridge's local symbol list to AmiBroker.
2. Serving historical OHLCV data on request, sourced from DataBridge Core.
3. Pushing real-time bar/quote updates into AmiBroker as they arrive from DataBridge Core.

The plugin **never** calls any broker API directly and has no broker-specific code paths — it is broker-agnostic by construction, talking only to the local DataBridge Core IPC surface.

### 19.2 IPC Mechanism Choice

**Chosen approach: Localhost TCP (loopback, 127.0.0.1) using a lightweight JSON/WebSocket protocol, with a Named Pipe fallback.**

Rationale:
- **Named Pipes** are the most idiomatic low-latency Windows IPC and avoid firewall prompts, but are more complex to implement symmetrically in both Rust (Tokio) and C++ (ADK plugin), and less convenient for the local dashboard (which is a web-based Tauri/React frontend needing a WebSocket-style connection anyway).
- **Localhost TCP/WebSocket** lets the same local server (hosted by the Rust core) serve both the React/Tauri UI (already WebSocket-friendly) and the AmiBroker C++ plugin (via a small, dependency-light TCP/WebSocket client embedded in the DLL), avoiding two separate IPC implementations.
- Loopback-only binding (127.0.0.1, non-zero but fixed/configurable port) avoids any external network exposure and sidesteps most firewall friction since it never leaves the local host.
- Named Pipe support is retained as a fallback/alternative transport for environments with strict local port policies, selectable in Settings → AmiBroker → IPC configuration.

```text
DataBridge Core
       │
       ▼
Local WebSocket/TCP (127.0.0.1:<port>, loopback-only)
       │
       ▼
DataBridge.dll (embedded lightweight client)
       │
       ▼
AmiBroker
```

### 19.3 Plugin Responsibilities & Lifecycle

1. **Plugin initialization**: On AmiBroker load, the DLL initializes, reads its local config (IPC host/port, reconnect policy) and attempts to connect to DataBridge Core.
2. **Core connection**: Establishes the loopback connection; if DataBridge Core is not running, the plugin reports a clear "Core not running" status to AmiBroker rather than failing silently.
3. **Symbol lookup**: On AmiBroker's `GetQuotesEx`/symbol resolution calls, the plugin queries DataBridge Core's symbol list and maps DataBridge symbols (`RELIANCE:NSE`) into AmiBroker's flat symbol namespace (`RELIANCE`), per the mapping rules in Section 19.4.
4. **Historical data request**: AmiBroker requests historical data for a symbol/range → plugin issues a request over IPC to DataBridge Core → Core reads from `historical_bars` → returns the series → plugin converts to AmiBroker's native `Quotation` struct array.
5. **Real-time update mechanism**: DataBridge Core pushes bar/tick updates over the persistent IPC connection; the plugin buffers and applies them via AmiBroker's real-time quote update API on the next AmiBroker refresh cycle.
6. **Connection failure**: If the IPC connection drops, the plugin marks all symbols as "stale/disconnected" in its internal state and attempts periodic reconnection with backoff, without blocking AmiBroker's UI thread.
7. **Plugin shutdown**: On AmiBroker close, the plugin cleanly closes the IPC connection and releases resources.
8. **Error handling**: All plugin-side errors are surfaced through AmiBroker's standard plugin status/notification mechanisms, not native pop-ups that could block Amibroker.
9. **Version compatibility**: The plugin embeds a protocol version header on every IPC handshake; DataBridge Core rejects/warns on incompatible plugin versions rather than silently misbehaving.
10. **32-bit/64-bit build strategy**: Since modern AmiBroker is 64-bit, the primary build target is x64. A 32-bit build configuration is retained in the build system for legacy AmiBroker installations but is not part of the default MVP release pipeline.
11. **Installation**: The installer copies `DataBridge.dll` into the user's configured AmiBroker `Plugins` directory and registers default IPC settings.
12. **Plugin configuration**: A minimal `.ini`/config file (or values read from the shared local settings store) controls IPC host/port/pipe name and reconnect behavior.

### 19.4 Symbol Synchronization

```text
DataBridge:  RELIANCE:NSE   →   AmiBroker:  RELIANCE
```

- **Symbol naming**: DataBridge's canonical `symbol:exchange` pair is flattened to AmiBroker's ticker on export; collisions across exchanges (same symbol, different exchange) are disambiguated via an exchange suffix/prefix convention configurable in Settings.
- **Exchange naming**: Exchange is stored as an AmiBroker "market"/database field alongside the ticker where supported.
- **Token mapping**: Internal `broker_token` never leaves DataBridge Core; the plugin only ever deals with `symbol:exchange`.
- **Symbol discovery**: The plugin can request the full active symbol list from Core at startup to pre-populate AmiBroker's database.
- **Unknown/not-found symbol handling**: If AmiBroker requests a symbol DataBridge doesn't recognize, the plugin returns an explicit "no data" response and logs the event — it does not fabricate data.
- **Historical/real-time availability flags**: Core reports, per symbol, whether historical backfill is complete and whether live subscription is active, so the plugin/dashboard can display accurate readiness state.

---

## 20. IPC/API Specification

### 20.1 REST-style Local API (consumed by both UI and, where applicable, the plugin's control channel)

```text
GET    /status
GET    /symbols
POST   /symbols/import
POST   /symbols
DELETE /symbols/:id
POST   /symbols/:id/backfill
GET    /history/:symbol
GET    /backfill/status
GET    /live/status
GET    /logs
```

**Example — `GET /status` response:**
```json
{
  "broker_connected": true,
  "websocket_connected": true,
  "amibroker_connected": true,
  "symbols_total": 325,
  "symbols_active": 325,
  "backfill_progress_pct": 87,
  "ticks_per_sec": 1248,
  "errors_last_hour": 0
}
```

**Example — `POST /symbols/import` request/response:**
```json
// Request: multipart/form-data with CSV file

// Response
{
  "total_rows": 500,
  "valid_rows": 480,
  "duplicate_rows": 10,
  "invalid_rows": 10,
  "imported_rows": 480,
  "skipped_rows": 20
}
```

**Example — `GET /history/:symbol` response:**
```json
{
  "symbol": "RELIANCE",
  "exchange": "NSE",
  "interval": "1m",
  "first_timestamp": "2025-09-22T03:45:00Z",
  "last_timestamp": "2026-09-22T09:59:00Z",
  "record_count": 94500,
  "bars": [
    { "timestamp": "2026-09-22T09:59:00Z", "open": 1450.10, "high": 1450.55, "low": 1449.90, "close": 1450.20, "volume": 12500 }
  ]
}
```

### 20.2 Real-Time Streaming Channel (WebSocket)

**Client → Server messages:**
```json
{ "type": "subscribe", "symbols": ["RELIANCE:NSE", "TCS:NSE"] }
{ "type": "unsubscribe", "symbols": ["TCS:NSE"] }
```

**Server → Client messages:**
```json
{
  "type": "tick",
  "symbol": "RELIANCE",
  "exchange": "NSE",
  "timestamp": "2026-09-22T09:15:00.102Z",
  "ltp": 1450.10,
  "last_quantity": 50,
  "volume": 125000,
  "bid": 1450.05,
  "ask": 1450.15
}

{ "type": "status", "websocket_connected": true, "broker_connected": true }

{ "type": "error", "code": "WS_DISCONNECTED", "message": "Broker WebSocket disconnected, reconnecting..." }
```

All local IPC endpoints (REST and WebSocket) are bound to `127.0.0.1` only by default and are not exposed on any external network interface.

---

## 21. Performance Requirements

- The system shall support at least **2,000 configured symbols** and at least **500 concurrently live-subscribed symbols** on standard consumer hardware (4+ core CPU, 8GB+ RAM) without UI degradation.
- Tick ingestion → normalization → aggregation → IPC dispatch latency shall target **under 50ms** at the 95th percentile under normal load.
- The UI thread shall never perform blocking I/O or heavy computation; all data processing happens on dedicated async tasks/threads.
- Database writes shall be batched (e.g., write-behind buffer flushed every N ticks or T milliseconds) rather than one write per tick.
- Bounded Tokio channels shall be used between pipeline stages (Live Tick Engine → Normalizer → Aggregator → IPC/Storage) to provide backpressure; a full channel triggers a documented drop/log policy rather than unbounded memory growth.

### 21.1 Thread/Task Separation

```text
UI Thread (React/Tauri webview)
Core Engine (Tokio runtime, orchestration)
Historical Workers (backfill jobs, bounded concurrency)
Live Tick Worker (WebSocket + normalization + aggregation)
Database Worker (batched writes)
AmiBroker Communication (local IPC server)
```

---

## 22. Reliability & Recovery Architecture

### 22.1 Principles
No silent data loss; automatic recovery; persistent synchronization state; idempotent historical inserts; safe shutdown; database integrity; crash recovery; reconnection; resubscription; gap recovery.

### 22.2 General Auto-Recovery Flow

```text
WebSocket disconnected
        │
        ▼
Retry (backoff)
        │
        ▼
Reconnect
        │
        ▼
Re-authenticate if required
        │
        ▼
Resubscribe symbols
        │
        ▼
Check missed historical period
        │
        ▼
Backfill gap
        │
        ▼
Resume live ticks
```

### 22.3 Data Gap Recovery

```text
Live ticks stopped
        │
        ▼
Connection restored
        │
        ▼
Find last known timestamp (per symbol)
        │
        ▼
Request missing historical data
        │
        ▼
Merge missing data (idempotent upsert; duplicates rejected by UNIQUE constraint)
        │
        ▼
Resume live feed
```

Duplicate avoidance during recovery relies on the `UNIQUE(symbol_id, interval, timestamp)` constraint on `historical_bars` plus tick-level fingerprint deduplication (Section 14.3) — recovery logic always upserts, never blind-inserts.

### 22.4 Failure Mode Coverage

| Failure | Detection | Recovery Action |
|---|---|---|
| Internet disconnection | Socket errors / heartbeat timeout | Backoff retry reconnect loop |
| Broker API timeout | Request timeout | Retry with backoff, mark job retrying |
| WebSocket disconnect | Close frame / heartbeat miss | Reconnect + resubscribe |
| Broker session expiry | 401/expired-token response | Re-authenticate, then resume |
| Application restart | Startup routine | Reload persisted job/session state, resume |
| Unexpected crash | Next-launch state check | Recover from last persisted state, resume incomplete jobs |
| Partial historical download | Job status = `partial` | Re-queue only the remaining sub-range |
| Database write failure | Write error/exception | Retry write, buffer in memory briefly, alert if persistent |
| AmiBroker plugin disconnect | IPC heartbeat miss on Core side | Core keeps buffering recent state; plugin reconnects and catches up |

---

## 23. Security

- Credentials stored via OS-native secure storage (Windows Credential Manager / DPAPI-protected file) rather than plain text config files where feasible.
- API secrets and access tokens are never written to `system_logs` or any log file — a redaction filter scrubs known secret field names before any log write.
- Local IPC (REST/WebSocket to UI, and to the AmiBroker plugin) is bound to loopback (127.0.0.1) by default; no external network exposure.
- All imported CSV data is validated (Section 12) before being trusted; malformed input cannot reach the database layer unvalidated.
- The AmiBroker plugin's IPC endpoint validates a shared local handshake/version token to reduce risk of arbitrary local processes impersonating the plugin.
- A secure update mechanism is planned: signed update packages, checksum verification before applying.
- A code-signing plan is required for production Windows releases (installer and `DataBridge.dll` signed with a valid code-signing certificate) to avoid SmartScreen/AV friction and to establish authenticity.

---

## 24. Error Handling

Every error surfaced to the user follows this structure:

```text
What happened
Why it happened
What DataBridge is doing
What the user can do
```

**Covered error states:** invalid broker credentials, session expired, broker unavailable, API rate limit exceeded, historical API failure, WebSocket failure, invalid CSV, symbol not found, database failure, AmiBroker plugin unavailable, AmiBroker connection lost, disk full, insufficient permissions.

**Example:**
```text
What happened: Broker session expired.
Why: Your access token exceeded its validity window.
What DataBridge is doing: Attempting automatic re-authentication.
What you can do: If this persists, re-enter your credentials in Settings → Broker.
```

---

## 25. MVP Scope

DataBridge's MVP scope is strictly the market-data layer of the algo trading flow `Broker → DataBridge → AmiBroker → Algo Strategy`. DataBridge provides 1-year historical market data, live tick-by-tick market data, real-time 1-minute OHLCV aggregation, reliable data delivery to AmiBroker, and auto reconnect/gap recovery. DataBridge does **NOT** provide order execution, buy/sell order routing, broker order management, or strategy execution inside DataBridge — those remain the responsibility of AmiBroker and the broker's own systems.

1. Windows desktop application
2. Built on Tauri 2
3. Frontend: React + TypeScript + Vite (+ Tailwind CSS)
4. Rust core engine
5. SQLite storage
6. One broker adapter (`BrokerAdapterV1`)
7. Broker authentication
8. CSV symbol import
9. Symbol mapping
10. 1-year, 1-minute historical backfill
11. Incremental (gap-only) backfill
12. Gap detection
13. Live WebSocket ticks
14. Tick normalization
15. Tick → 1-minute aggregation
16. Auto reconnect
17. Historical gap recovery
18. DataBridge AmiBroker native plugin
19. AmiBroker historical feed
20. AmiBroker real-time feed
21. Dashboard
22. Logs
23. Settings
24. System tray
25. Windows startup
26. Installer

---

## 26. Out of Scope (Future Phases)

Order execution; buy/sell order routing; broker order management; strategy execution inside DataBridge; portfolio management; cloud synchronization; mobile app; web dashboard; multi-user SaaS; paid subscriptions; AI features; advanced analytics; options chain analytics; full tick-history backfill (unless a future phase's broker capability and storage design explicitly support it). DataBridge remains the data layer only — algo strategy execution continues to live in AmiBroker (and the broker's own order systems), per the `Broker → DataBridge → AmiBroker → Algo Strategy` flow, not in DataBridge itself.

---

## 27. Future Roadmap

| Phase | Focus |
|---|---|
| Phase 1 | Single broker + historical + live + AmiBroker (MVP) |
| Phase 2 | Multi-broker architecture (second/third `BrokerAdapter`) |
| Phase 3 | Advanced data storage (DuckDB/Parquet migration for tick-scale data) |
| Phase 4 | Tick database and advanced replay infrastructure |
| Phase 5 | Market replay (feed historical ticks into AmiBroker as if live) |
| Phase 6 | Data quality monitoring (automated, scheduled integrity audits with alerts) |
| Phase 7 | Cloud backup/sync (optional, opt-in) |
| Phase 8 | Commercial licensing infrastructure |

---

## 28. Acceptance Criteria

### CSV Import
- Given a valid CSV, when imported, then valid symbols are added, duplicates are flagged and skipped, invalid rows are reported individually, and valid rows continue to import despite invalid ones present.

### Backfill
- Given a newly added/enabled symbol, when the backfill engine runs, then DataBridge queues a 1-year backfill job, downloads historical data in rate-limited batches, stores it idempotently, marks the job `Completed`, and automatically retries on transient failures per the backoff policy.

### Live Tick
- Given the market is open, when the Live Tick Engine starts, then the WebSocket connects, all enabled symbols are subscribed, ticks are received and normalized, normalized ticks/bars reach AmiBroker via the plugin, and a forced disconnect triggers automatic reconnection and resubscription without user action.

### Recovery
- Given the connection is lost during market hours, when connectivity is restored, then DataBridge reconnects, resubscribes all previously subscribed symbols, identifies the exact missing time window per symbol, backfills that gap, and resumes live data — with no duplicate bars/ticks introduced.

### AmiBroker Integration
- Given DataBridge Core and AmiBroker are both running with the plugin installed, when AmiBroker requests data, then `DataBridge.dll` loads successfully, configured symbols resolve to the correct DataBridge internal symbols, historical data returned matches the local `historical_bars` store, and real-time updates reflect incoming ticks with no more than the defined latency target (Section 21).

### Data Integrity
- Given the integrity check tool is run, when duplicate candles, out-of-order timestamps, missing timestamps, invalid OHLC relationships, or zero/invalid volume/price entries exist, then each is individually reported with symbol and timestamp, and repairable issues can be resolved via the "Repair gaps" action.

---

## 29. Development Phases

| Phase | Deliverable |
|---|---|
| 0 — Foundations | Repo scaffolding, Tauri/React shell, Rust core skeleton, SQLite schema migration tooling |
| 1 — Broker Adapter V1 | Authentication, session management, symbol master retrieval |
| 2 — Symbol Management | Manual add, CSV import + validation, symbol mapping, Symbols page UI |
| 3 — Historical Backfill Engine | Queue, jobs, rate limiting, retries, gap detection, Historical Data page |
| 4 — Live Tick Engine | WebSocket client, normalization, reconnect/resubscribe, Live Monitor page |
| 5 — Aggregation Engine | Tick-to-1-minute bar construction |
| 6 — Local IPC Layer | REST + WebSocket local API serving UI and plugin |
| 7 — AmiBroker Plugin | ADK-based `DataBridge.dll`, historical + real-time delivery, symbol sync |
| 8 — Recovery & Resilience | Full auto-recovery matrix (Section 22), crash-safe resume |
| 9 — Dashboard, Logs, Settings | Remaining UI pages, tray integration, Windows startup |
| 10 — Hardening & Installer | Data integrity tooling, security pass, code signing, installer packaging, QA |

---

## 30. Testing Strategy

- **Unit tests (Rust):** Broker adapter trait implementations (mocked HTTP/WebSocket), Data Normalizer conversions, Tick-to-Bar aggregation state machine, backoff/retry logic, gap-detection algorithm.
- **Unit tests (C++/ADK plugin):** Symbol mapping logic, IPC message parsing, historical data conversion to AmiBroker `Quotation` structs.
- **Integration tests:** End-to-end backfill against a mocked broker REST server; live tick pipeline against a mocked WebSocket server, including forced disconnects to validate reconnect/resubscribe/gap-backfill sequences.
- **Database tests:** Idempotency of historical upserts under concurrent/duplicate inserts; migration correctness; index performance under large synthetic datasets (millions of bars).
- **Plugin integration tests:** Manual and scripted validation inside a real AmiBroker instance — symbol resolution, historical chart rendering, real-time quote updates, and behavior when DataBridge Core is stopped/restarted.
- **Load/performance tests:** Synthetic tick generator simulating peak F&O expiry-day volumes across 500+ symbols to validate latency targets and backpressure behavior.
- **Recovery/chaos tests:** Simulated internet loss, forced WebSocket kill, forced application crash mid-backfill, and forced database lock, verifying the system reaches a consistent, gap-free state afterward.
- **CSV import tests:** Full validation rule matrix (missing/duplicate/invalid fields, malformed CSV) with exact expected import summary counts.

---

## 31. Deployment / Installer Strategy

- Package as a single Windows installer (e.g., NSIS or WiX-based, compatible with Tauri's bundler) that installs:
  - The DataBridge desktop application (Tauri bundle)
  - `DataBridge.dll` copied into the user-specified AmiBroker `Plugins` directory (with a manual path picker if AmiBroker isn't auto-detected)
  - Default configuration and a first-run setup wizard (broker credentials, AmiBroker path, market/timezone defaults)
- Installer registers an optional "Start with Windows" entry (Task Scheduler or Run-key based) only if the user opts in during setup or later via Settings.
- Uninstaller removes the application and optionally offers to remove `DataBridge.dll` from AmiBroker's Plugins directory and local data (with explicit confirmation, since this deletes historical data).
- Future auto-update mechanism: signed delta/full updates fetched over HTTPS, checksum-verified before install, with rollback on failure.
- Release binaries (installer and DLL) are code-signed prior to distribution (Section 23).

---

## 32. Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Broker API rate limits throttle large-scale backfill (1000s of symbols) | Slow initial onboarding | Priority queue, batch requests, clear progress UI, allow background completion over hours |
| Broker WebSocket protocol changes without notice | Live feed breakage | Adapter isolates broker-specific decode logic; version-pinned adapter with clear error surfacing on decode failure |
| High-frequency tick volume overwhelms UI or DB writes | UI freeze / data loss | Bounded channels + backpressure, batched DB writes, virtualized UI tables |
| AmiBroker ADK version incompatibility across AmiBroker releases | Plugin fails to load | Target stable, documented ADK version; version-check on load with clear user-facing error |
| Credential leakage via logs | Security incident | Mandatory log redaction filter reviewed in code review/tests; OS-native secure credential storage |
| Data corruption from duplicate/out-of-order historical inserts | Bad backtests in AmiBroker | DB-level uniqueness constraints, idempotent upserts, dedicated data integrity tool |
| Single-broker architecture accidentally becomes tightly coupled during MVP development pressure | Expensive rework for multi-broker phase | Enforce `BrokerAdapter` trait boundary in code review; no broker-specific types outside `broker/` modules |
| Windows Defender/SmartScreen flags unsigned installer/DLL | Poor first-run experience, trust issues | Code signing plan executed before any production release |

---

## 33. Recommended Project Structure

```text
databridge/
│
├── app/                        # React/TypeScript/Vite frontend
│   ├── src/
│   ├── components/
│   ├── pages/                  # Dashboard, Symbols, LiveMonitor, HistoricalData, Logs, Settings
│   ├── services/                # Local API/WebSocket client
│   └── styles/                  # Tailwind config, theme tokens
│
├── src-tauri/                   # Rust core (Tauri backend)
│   ├── src/
│   ├── commands/                 # Tauri command handlers (bridge to frontend)
│   ├── broker/                    # BrokerAdapter trait + broker_v1/ implementation
│   │   └── broker_v1/
│   ├── websocket/                  # Live Tick Engine
│   ├── historical/                  # Historical Backfill Engine
│   ├── aggregation/                  # Tick-to-Bar Aggregator
│   ├── storage/                       # Repository traits + SQLite implementation
│   ├── ipc/                            # Local REST/WebSocket server (UI + plugin)
│   ├── recovery/                        # Recovery Manager
│   ├── session/                          # Market Session Manager
│   └── scheduler/                         # Job scheduling (backfill queue, sync cycles)
│
├── amibroker-plugin/                       # C++ AmiBroker Data Plugin (ADK)
│   ├── src/
│   ├── include/
│   ├── ipc_client/                          # Lightweight local TCP/WebSocket client
│   └── build/                                # Output: DataBridge.dll
│
├── shared/                                   # Shared type/schema definitions (JSON schema / protocol docs)
│
├── docs/                                      # This PRD, architecture diagrams, API docs
│
├── scripts/                                    # DB migrations, dev tooling
│
└── installer/                                   # NSIS/WiX installer scripts, code-signing config
```

---

## 34. Important Engineering Rules (Non-Negotiable)

1. Broker-specific code must stay inside broker adapters (`broker/<name>/`).
2. The AmiBroker plugin must not directly depend on any broker API — only on DataBridge Core's local IPC.
3. The UI thread must never block the live data engine.
4. Historical and live pipelines must be independently recoverable from each other's failures.
5. All timestamps must use a consistent internal UTC strategy, converted to local/exchange time only at presentation boundaries.
6. Historical inserts must be idempotent.
7. Duplicate ticks/candles must never corrupt the dataset — enforced at the schema and pipeline level.
8. All network operations require explicit timeout and retry handling.
9. Broker API rate limits must be respected via adapter-level limiters.
10. Application restart must resume all unfinished synchronization automatically.
11. Secrets must never be exposed in logs.
12. The architecture must support future multi-broker expansion without core/plugin rewrites.

---

*End of Document*
