/// Interval enum — PRD §15.3
///
/// Enumerates all time intervals the aggregation engine can support.
/// MVP activates only Tick → OneMinute. All other intervals are infrastructure-only.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[serde(rename_all = "lowercase")]
pub enum Interval {
    Tick,
    #[serde(rename = "1s")]
    OneSecond,
    #[serde(rename = "5s")]
    FiveSeconds,
    #[serde(rename = "1m")]
    OneMinute,
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "15m")]
    FifteenMinutes,
    #[serde(rename = "30m")]
    ThirtyMinutes,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "1d")]
    Daily,
}

impl Interval {
    /// Duration of one bar in seconds (for non-tick intervals).
    pub fn duration_secs(&self) -> Option<u64> {
        match self {
            Interval::Tick => None,
            Interval::OneSecond => Some(1),
            Interval::FiveSeconds => Some(5),
            Interval::OneMinute => Some(60),
            Interval::FiveMinutes => Some(300),
            Interval::FifteenMinutes => Some(900),
            Interval::ThirtyMinutes => Some(1800),
            Interval::OneHour => Some(3600),
            Interval::Daily => Some(86400),
        }
    }

    /// Returns the canonical string representation used in DB and IPC.
    pub fn as_str(&self) -> &'static str {
        match self {
            Interval::Tick => "tick",
            Interval::OneSecond => "1s",
            Interval::FiveSeconds => "5s",
            Interval::OneMinute => "1m",
            Interval::FiveMinutes => "5m",
            Interval::FifteenMinutes => "15m",
            Interval::ThirtyMinutes => "30m",
            Interval::OneHour => "1h",
            Interval::Daily => "1d",
        }
    }
}

impl std::fmt::Display for Interval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Interval {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tick" => Ok(Interval::Tick),
            "1s"   => Ok(Interval::OneSecond),
            "5s"   => Ok(Interval::FiveSeconds),
            "1m"   => Ok(Interval::OneMinute),
            "5m"   => Ok(Interval::FiveMinutes),
            "15m"  => Ok(Interval::FifteenMinutes),
            "30m"  => Ok(Interval::ThirtyMinutes),
            "1h"   => Ok(Interval::OneHour),
            "1d"   => Ok(Interval::Daily),
            other  => Err(format!("Unknown interval: '{}'", other)),
        }
    }
}
