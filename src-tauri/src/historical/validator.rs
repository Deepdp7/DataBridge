/// Bar validation rules — PRD §13.5
///
/// Applied to every batch of raw bars before they are upserted into the DB.

use crate::models::bar::RawBar;
use chrono::{DateTime, Utc};
use tracing::warn;

/// Validate a slice of RawBars and return only those that pass all rules.
pub fn validate_bars(bars: &[RawBar], context_from: DateTime<Utc>) -> Vec<RawBar> {
    let mut valid = Vec::with_capacity(bars.len());
    let mut prev_ts: Option<i64> = None;

    for bar in bars {
        // Rule 1: OHLC relationship validity
        if bar.high < bar.low {
            warn!(ts = bar.broker_timestamp_ms, "Rejected bar: high < low");
            continue;
        }
        if bar.high < bar.open || bar.high < bar.close {
            warn!(ts = bar.broker_timestamp_ms, "Rejected bar: high < open or close");
            continue;
        }
        if bar.low > bar.open || bar.low > bar.close {
            warn!(ts = bar.broker_timestamp_ms, "Rejected bar: low > open or close");
            continue;
        }

        // Rule 2: No duplicate timestamps within a batch
        if let Some(prev) = prev_ts {
            if bar.broker_timestamp_ms == prev {
                warn!(ts = bar.broker_timestamp_ms, "Rejected bar: duplicate timestamp in batch");
                continue;
            }
            // Rule 3: Non-monotonic timestamp
            if bar.broker_timestamp_ms < prev {
                warn!(ts = bar.broker_timestamp_ms, "Rejected bar: non-monotonic timestamp");
                continue;
            }
        }

        // Rule 4: Flag (but don't reject) zero-volume bars during market hours
        // (holiday/illiquid symbols may legitimately have zero volume)
        if bar.volume == 0 {
            warn!(ts = bar.broker_timestamp_ms, "Suspicious: zero-volume bar");
            // Not rejected — still inserted, flagged for data quality check
        }

        prev_ts = Some(bar.broker_timestamp_ms);
        valid.push(bar.clone());
    }

    valid
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bar(ts: i64, o: f64, h: f64, l: f64, c: f64) -> RawBar {
        RawBar { broker_timestamp_ms: ts, open: o, high: h, low: l, close: c, volume: 1000 }
    }

    #[test]
    fn rejects_high_lt_low() {
        let bars = vec![make_bar(1000, 100.0, 98.0, 99.0, 100.0)]; // high < low
        let valid = validate_bars(&bars, Utc::now());
        assert!(valid.is_empty());
    }

    #[test]
    fn accepts_valid_bar() {
        let bars = vec![make_bar(1000, 100.0, 105.0, 98.0, 102.0)];
        let valid = validate_bars(&bars, Utc::now());
        assert_eq!(valid.len(), 1);
    }

    #[test]
    fn rejects_duplicate_timestamps() {
        let bars = vec![
            make_bar(1000, 100.0, 105.0, 98.0, 102.0),
            make_bar(1000, 101.0, 106.0, 99.0, 103.0), // same ts
        ];
        let valid = validate_bars(&bars, Utc::now());
        assert_eq!(valid.len(), 1);
    }
}
