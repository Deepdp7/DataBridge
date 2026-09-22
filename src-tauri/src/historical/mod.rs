/// Historical Backfill Engine — PRD §13
///
/// Manages the queue of backfill jobs, rates API calls via a token-bucket limiter,
/// handles chunked range requests, exponential backoff retries, gap detection,
/// and persists job state so jobs survive application restarts.

pub mod gap_detector;
pub mod queue;
pub mod validator;

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use governor::{Quota, RateLimiter};
use governor::state::{InMemoryState, NotKeyed};
use governor::clock::DefaultClock;
use nonzero_ext::nonzero;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, warn};

use crate::broker::{BrokerAdapter, BrokerError};
use crate::error::{AppError, Result};
use crate::models::{Bar, BarSource, Interval};
use crate::storage::repository::{
    BackfillJob, BackfillJobStatus, BackfillJobUpdate, BackfillRepository,
    HistoricalRepository, NewBackfillJob, NewBar, SymbolRepository,
};

/// Maximum retry attempts before marking a job as Failed (PRD §13.3)
const MAX_ATTEMPT_COUNT: i64 = 5;
/// Chunk size for historical requests (PRD §13.3)
const CHUNK_DAYS: i64 = 60;

/// Command messages sent to the backfill engine task.
#[derive(Debug)]
pub enum BackfillCommand {
    /// Enqueue a backfill job for a symbol (priority: normal = 0, high = 10)
    Enqueue { symbol_id: i64, priority: i64 },
    /// Cancel a specific job
    Cancel { job_id: i64 },
    /// Pause all processing
    Pause,
    /// Resume all processing
    Resume,
    /// Shutdown
    Shutdown,
}

/// Status event emitted by the engine for the dashboard.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BackfillStatus {
    pub queued: usize,
    pub downloading: usize,
    pub completed: usize,
    pub failed: usize,
    pub overall_progress_pct: u8,
}

type GovRateLimiter = RateLimiter<NotKeyed, InMemoryState, DefaultClock>;

pub struct HistoricalEngine {
    adapter: Arc<dyn BrokerAdapter>,
    symbol_repo: Arc<dyn SymbolRepository>,
    historical_repo: Arc<dyn HistoricalRepository>,
    backfill_repo: Arc<dyn BackfillRepository>,
    cmd_tx: mpsc::Sender<BackfillCommand>,
    cmd_rx: Arc<Mutex<mpsc::Receiver<BackfillCommand>>>,
    rate_limiter: Arc<GovRateLimiter>,
    paused: Arc<Mutex<bool>>,
}

impl HistoricalEngine {
    pub fn new(
        adapter: Arc<dyn BrokerAdapter>,
        symbol_repo: Arc<dyn SymbolRepository>,
        historical_repo: Arc<dyn HistoricalRepository>,
        backfill_repo: Arc<dyn BackfillRepository>,
    ) -> Self {
        let rate_cfg = adapter.rate_limit_config();
        // Convert requests/sec to quota
        let rps = rate_cfg.historical_requests_per_second.max(1.0) as u32;
        let quota = Quota::per_second(nonzero!(1u32)).allow_burst(nonzero!(1u32));
        // Use the configured RPS — governor requires NonZeroU32
        let actual_quota = Quota::per_second(
            std::num::NonZeroU32::new(rps).unwrap_or(nonzero!(3u32))
        );
        let rate_limiter = Arc::new(RateLimiter::direct(actual_quota));

        let (cmd_tx, cmd_rx) = mpsc::channel(128);

        HistoricalEngine {
            adapter,
            symbol_repo,
            historical_repo,
            backfill_repo,
            cmd_tx,
            cmd_rx: Arc::new(Mutex::new(cmd_rx)),
            rate_limiter,
            paused: Arc::new(Mutex::new(false)),
        }
    }

    /// Command sender — clone this to enqueue jobs from outside the engine.
    pub fn command_sender(&self) -> mpsc::Sender<BackfillCommand> {
        self.cmd_tx.clone()
    }

    /// Queue a 1-year backfill for a symbol. Called when a symbol is added/enabled.
    pub async fn enqueue_initial_backfill(&self, symbol_id: i64) -> Result<i64> {
        let to   = Utc::now();
        let from = to - chrono::Duration::days(365);

        // Check if there's already a non-failed, non-cancelled job
        let existing = self.backfill_repo.list_jobs_for_symbol(symbol_id).await?;
        let active = existing.iter().any(|j| !j.status.is_terminal());
        if active {
            debug!(symbol_id, "Skipping enqueue — active job already exists");
            return Ok(-1);
        }

        // Determine actual start: if we already have some data, fetch only the gap
        let actual_from = match self.historical_repo.latest_bar_timestamp(symbol_id, Interval::OneMinute).await? {
            Some(latest) => {
                info!(symbol_id, latest = %latest, "Incremental sync — fetching from last stored bar");
                latest + chrono::Duration::minutes(1)
            }
            None => from,
        };

        let job_id = self.backfill_repo.create_job(&NewBackfillJob {
            symbol_id,
            requested_from: actual_from,
            requested_to: to,
            priority: 0,
        }).await?;

        info!(symbol_id, job_id, from = %actual_from, to = %to, "Backfill job enqueued");
        Ok(job_id)
    }

    /// Main engine loop — runs as a Tokio task. Processes jobs from the queue.
    pub async fn run(self: Arc<Self>) {
        info!("Historical Backfill Engine started");

        // On startup, load and re-queue any resumable jobs (PRD §13.3)
        match self.backfill_repo.load_resumable_jobs().await {
            Ok(jobs) => {
                if !jobs.is_empty() {
                    info!(count = jobs.len(), "Resuming backfill jobs from previous session");
                    for job in jobs {
                        // Reset downloading/processing to queued so they restart cleanly
                        if job.status == BackfillJobStatus::Downloading
                            || job.status == BackfillJobStatus::Processing
                        {
                            let _ = self.backfill_repo.update_job_status(job.id, &BackfillJobUpdate {
                                status: BackfillJobStatus::Queued,
                                fetched_up_to: job.fetched_up_to,
                                attempt_count: None,
                                last_error: None,
                            }).await;
                        }
                    }
                }
            }
            Err(e) => error!("Failed to load resumable jobs: {}", e),
        }

        let mut ticker = tokio::time::interval(Duration::from_secs(5));

        loop {
            ticker.tick().await;

            // Check for pause
            if *self.paused.lock().await {
                continue;
            }

            // Process pending jobs (up to N concurrent via spawned tasks)
            self.process_pending_jobs().await;
        }
    }

    async fn process_pending_jobs(self: &Arc<Self>) {
        let jobs = match self.backfill_repo.list_jobs_by_status(&[
            BackfillJobStatus::Queued,
            BackfillJobStatus::Retrying,
        ]).await {
            Ok(j) => j,
            Err(e) => { error!("Failed to list pending jobs: {}", e); return; }
        };

        // Limit concurrent downloads (configurable; default 3)
        let active_count = match self.backfill_repo.list_jobs_by_status(&[
            BackfillJobStatus::Downloading,
        ]).await {
            Ok(j) => j.len(),
            Err(_) => 0,
        };

        const MAX_CONCURRENT: usize = 3;
        let available_slots = MAX_CONCURRENT.saturating_sub(active_count);

        for job in jobs.into_iter().take(available_slots) {
            let engine = Arc::clone(self);
            tokio::spawn(async move {
                engine.execute_job(job).await;
            });
        }
    }

    async fn execute_job(self: Arc<Self>, job: BackfillJob) {
        let symbol_id = job.symbol_id;
        let job_id    = job.id;

        // Get the symbol's broker token
        // (In a real scenario we'd look up the token from symbol_mappings)
        // For now, just use symbol_id as a string token for the mock adapter
        let token = job.symbol_id.to_string();

        info!(symbol_id, job_id, "Starting backfill job execution");

        // Mark as downloading
        let _ = self.backfill_repo.update_job_status(job_id, &BackfillJobUpdate {
            status: BackfillJobStatus::Downloading,
            fetched_up_to: None,
            attempt_count: Some(job.attempt_count + 1),
            last_error: None,
        }).await;

        // Determine range to fetch (start from fetched_up_to if partial)
        let start = job.fetched_up_to
            .map(|t| t + chrono::Duration::minutes(1))
            .unwrap_or(job.requested_from);
        let end = job.requested_to;

        let max_days = self.adapter.max_historical_days_per_request() as i64;
        let chunk_days = CHUNK_DAYS.min(max_days);

        let mut cursor = start;
        let mut total_bars = 0usize;
        let mut error: Option<String> = None;

        while cursor < end {
            let chunk_end = (cursor + chrono::Duration::days(chunk_days)).min(end);

            // Rate limit
            self.rate_limiter.until_ready().await;

            match self.adapter.get_historical_data(
                &token,
                Interval::OneMinute,
                cursor,
                chunk_end,
            ).await {
                Ok(raw_bars) => {
                    let validated = validator::validate_bars(&raw_bars, cursor);
                    let new_bars: Vec<NewBar> = validated.into_iter().map(|rb| {
                        let ts = DateTime::<Utc>::from_timestamp_millis(rb.broker_timestamp_ms)
                            .unwrap_or(cursor);
                        NewBar {
                            symbol_id,
                            interval: Interval::OneMinute.as_str().to_string(),
                            timestamp: ts,
                            open:   rb.open,
                            high:   rb.high,
                            low:    rb.low,
                            close:  rb.close,
                            volume: rb.volume,
                            source: BarSource::Backfill.to_string(),
                        }
                    }).collect();

                    let inserted = match self.historical_repo.upsert_bars(&new_bars).await {
                        Ok(n) => n,
                        Err(e) => {
                            error!(symbol_id, job_id, "DB write error: {}", e);
                            0
                        }
                    };
                    total_bars += inserted;

                    // Update fetched_up_to progress
                    let _ = self.backfill_repo.update_job_status(job_id, &BackfillJobUpdate {
                        status: BackfillJobStatus::Downloading,
                        fetched_up_to: Some(chunk_end),
                        attempt_count: None,
                        last_error: None,
                    }).await;

                    cursor = chunk_end + chrono::Duration::minutes(1);
                }
                Err(BrokerError::RateLimited { retry_after_secs }) => {
                    warn!(symbol_id, job_id, secs = retry_after_secs, "Rate limited — waiting");
                    tokio::time::sleep(Duration::from_secs(retry_after_secs)).await;
                    // Don't advance cursor — retry this chunk
                }
                Err(e) => {
                    error!(symbol_id, job_id, "Broker error fetching chunk: {}", e);
                    error = Some(e.to_string());
                    break;
                }
            }
        }

        // Final status
        if error.is_some() && job.attempt_count + 1 >= MAX_ATTEMPT_COUNT {
            let _ = self.backfill_repo.update_job_status(job_id, &BackfillJobUpdate {
                status: BackfillJobStatus::Failed,
                fetched_up_to: None,
                attempt_count: None,
                last_error: error,
            }).await;
            warn!(symbol_id, job_id, "Backfill job failed after max attempts");
        } else if error.is_some() {
            // Schedule retry with exponential backoff
            let backoff_secs = 2u64.pow((job.attempt_count as u32).min(4)); // 1,2,4,8,16s
            tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
            let _ = self.backfill_repo.update_job_status(job_id, &BackfillJobUpdate {
                status: BackfillJobStatus::Retrying,
                fetched_up_to: None,
                attempt_count: None,
                last_error: error,
            }).await;
            warn!(symbol_id, job_id, backoff_secs, "Backfill job scheduled for retry");
        } else {
            let _ = self.backfill_repo.update_job_status(job_id, &BackfillJobUpdate {
                status: BackfillJobStatus::Completed,
                fetched_up_to: Some(end),
                attempt_count: None,
                last_error: None,
            }).await;
            info!(symbol_id, job_id, bars = total_bars, "Backfill job completed");
        }
    }
}
