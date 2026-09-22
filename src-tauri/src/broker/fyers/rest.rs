use reqwest::Client;
use serde::Deserialize;
use chrono::{DateTime, Utc};

use crate::broker::BrokerError;
use crate::models::{BrokerSymbol, RawBar};
use crate::models::interval::Interval;

#[derive(Deserialize, Debug)]
struct FyersHistoryResponse {
    s: String,
    message: Option<String>,
    candles: Option<Vec<Vec<f64>>>,
}

pub struct FyersRest {}

impl FyersRest {
    pub async fn get_symbol_master() -> Result<Vec<BrokerSymbol>, BrokerError> {
        Ok(Vec::new()) // Basic implementation, expects user to upload CSV
    }

    pub async fn fetch_historical_bars(
        auth_token: &str,
        token: &str,
        interval: Interval,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<RawBar>, BrokerError> {
        let resolution_str = match interval {
            Interval::OneMinute => "1",
            Interval::FiveMinutes => "5",
            Interval::FifteenMinutes => "15",
            Interval::ThirtyMinutes => "30",
            Interval::OneHour => "60",
            Interval::Daily => "1D",
            _ => "1",
        };

        let range_from = from.timestamp();
        let range_to = to.timestamp();

        let url = format!(
            "https://api.fyers.in/data/history?symbol={}&resolution={}&date_format=1&range_from={}&range_to={}&cont_flag=1",
            token, resolution_str, range_from, range_to
        );

        let client = Client::new();
        let resp = client.get(&url)
            .header("Authorization", auth_token)
            .send()
            .await
            .map_err(|e| BrokerError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(BrokerError::Other(format!("HTTP {}: {}", status, text)));
        }

        let history_resp: FyersHistoryResponse = resp.json()
            .await
            .map_err(|e| BrokerError::InvalidResponse(format!("Failed to parse JSON: {}", e)))?;

        if history_resp.s != "ok" {
            let msg = history_resp.message.unwrap_or_else(|| "Unknown error".to_string());
            return Err(BrokerError::Other(format!("Fyers History API error: {}", msg)));
        }

        let mut bars = Vec::new();
        if let Some(candles) = history_resp.candles {
            for candle in candles {
                if candle.len() >= 6 {
                    let timestamp_ms = (candle[0] as i64) * 1000;
                    bars.push(RawBar {
                        broker_timestamp_ms: timestamp_ms,
                        open: candle[1],
                        high: candle[2],
                        low: candle[3],
                        close: candle[4],
                        volume: candle[5] as u64,
                    });
                }
            }
        }

        Ok(bars)
    }
}
