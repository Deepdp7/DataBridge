/// Gap Detector — PRD §13.4
///
/// Scans stored 1-minute bars for a symbol and identifies missing minute-slots
/// that should have data based on market session hours and the holiday calendar.

use chrono::{DateTime, Datelike, Duration, NaiveTime, TimeZone, Utc, Weekday};
use tracing::{debug, info};

use crate::error::Result;
use crate::models::Interval;
use crate::session::ExchangeConfig;
use crate::storage::repository::{
    BackfillRepository, HistoricalRepository, NewBackfillGap,
};

pub struct GapDetector {
    pub exchange_config: ExchangeConfig,
}

impl GapDetector {
    pub fn new(exchange_config: ExchangeConfig) -> Self {
        GapDetector { exchange_config }
    }

    /// Scan stored bars for a symbol and detect internal gaps.
    /// Returns the number of gaps found and recorded.
    pub async fn detect_and_record_gaps(
        &self,
        symbol_id: i64,
        historical_repo: &dyn HistoricalRepository,
        backfill_repo: &dyn BackfillRepository,
    ) -> Result<usize> {
        let bars = historical_repo
            .query_bars(
                symbol_id,
                Interval::OneMinute,
                // Scan the last 30 days
                Utc::now() - Duration::days(30),
                Utc::now(),
            )
            .await?;

        if bars.len() < 2 {
            return Ok(0);
        }

        let tz_str = &self.exchange_config.timezone;
        let tz: chrono_tz::Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
        let open_time = NaiveTime::parse_from_str(&self.exchange_config.market_open, "%H:%M")
            .unwrap_or_else(|_| NaiveTime::from_hms_opt(9, 15, 0).unwrap());
        let close_time = NaiveTime::parse_from_str(&self.exchange_config.market_close, "%H:%M")
            .unwrap_or_else(|_| NaiveTime::from_hms_opt(15, 30, 0).unwrap());

        let mut gaps_found = 0usize;
        let mut prev_ts = bars[0].timestamp;

        for bar in &bars[1..] {
            let expected_next = prev_ts + Duration::minutes(1);
            if bar.timestamp > expected_next {
                // There's a gap — verify it's within market hours and not a holiday/weekend
                let gap_start_local = expected_next.with_timezone(&tz);
                let gap_end_local   = bar.timestamp.with_timezone(&tz);

                // Only flag as a gap if it crosses trading minutes
                let is_real_gap = self.gap_crosses_trading_time(
                    gap_start_local.naive_local(),
                    gap_end_local.naive_local(),
                    open_time,
                    close_time,
                );

                if is_real_gap {
                    debug!(
                        symbol_id,
                        gap_start = %expected_next,
                        gap_end = %bar.timestamp,
                        "Gap detected"
                    );
                    backfill_repo.create_gap(&NewBackfillGap {
                        symbol_id,
                        gap_start: expected_next,
                        gap_end: bar.timestamp,
                    }).await?;
                    gaps_found += 1;
                }
            }
            prev_ts = bar.timestamp;
        }

        if gaps_found > 0 {
            info!(symbol_id, gaps_found, "Gap detection complete");
        }

        Ok(gaps_found)
    }

    fn gap_crosses_trading_time(
        &self,
        start: chrono::NaiveDateTime,
        end: chrono::NaiveDateTime,
        open: NaiveTime,
        close: NaiveTime,
    ) -> bool {
        let mut cursor = start;
        while cursor < end {
            let weekday = cursor.weekday();
            if weekday != Weekday::Sat && weekday != Weekday::Sun {
                let date_str = cursor.date().format("%Y-%m-%d").to_string();
                if !self.exchange_config.holidays.contains(&date_str) {
                    let t = cursor.time();
                    if t >= open && t < close {
                        return true;
                    }
                }
            }
            cursor += Duration::minutes(1);
            // Optimize: if more than 1 day gap, skip to next day
            if end - cursor > Duration::hours(24) {
                cursor = cursor.date().succ_opt().unwrap().and_hms_opt(0, 0, 0).unwrap();
            }
        }
        false
    }
}
