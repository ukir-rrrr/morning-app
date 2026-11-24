//! Google Health API (v4) クライアントとレスポンスモデル。

mod client;
pub mod de;
pub mod models;

pub use client::{HealthClient, ListRequest, DEFAULT_BASE_URL};

use crate::cli::DateRange;

pub mod data_types {
    pub const SLEEP: &str = "sleep";
    pub const EXERCISE: &str = "exercise";
    pub const DAILY_HEART_RATE_VARIABILITY: &str = "daily-heart-rate-variability";
    pub const DAILY_OXYGEN_SATURATION: &str = "daily-oxygen-saturation";
    pub const DAILY_RESTING_HEART_RATE: &str = "daily-resting-heart-rate";
}

/// sleep / exercise の 1 ページあたりの最大件数（API 上限）。
pub const SESSION_PAGE_SIZE: u32 = 25;

/// フィルタ式で使うフィールド名のプレフィックス（`daily-oxygen-saturation` → `daily_oxygen_saturation`）。
fn filter_prefix(data_type: &str) -> String {
    data_type.replace('-', "_")
}

/// 睡眠は起床日（civil end）で範囲指定する。
pub fn sleep_filter(range: &DateRange) -> String {
    format!(
        "sleep.interval.civil_end_time >= \"{}\" AND sleep.interval.civil_end_time < \"{}\"",
        range.start,
        range.end_exclusive()
    )
}

pub fn exercise_filter(range: &DateRange) -> String {
    format!(
        "exercise.interval.civil_start_time >= \"{}\" AND exercise.interval.civil_start_time < \"{}\"",
        range.start,
        range.end_exclusive()
    )
}

/// 日次サマリ系（daily-*）のデータ型用フィルタ。
pub fn daily_filter(data_type: &str, range: &DateRange) -> String {
    let prefix = filter_prefix(data_type);
    format!(
        "{prefix}.date >= \"{}\" AND {prefix}.date < \"{}\"",
        range.start,
        range.end_exclusive()
    )
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn builds_filters() {
        let range = DateRange {
            start: NaiveDate::from_ymd_opt(2026, 11, 22).unwrap(),
            end: NaiveDate::from_ymd_opt(2026, 11, 24).unwrap(),
        };
        assert_eq!(
            daily_filter(data_types::DAILY_HEART_RATE_VARIABILITY, &range),
            "daily_heart_rate_variability.date >= \"2026-11-22\" AND daily_heart_rate_variability.date < \"2026-11-25\""
        );
        assert_eq!(
            sleep_filter(&range),
            "sleep.interval.civil_end_time >= \"2026-11-22\" AND sleep.interval.civil_end_time < \"2026-11-25\""
        );
        assert!(exercise_filter(&range).starts_with("exercise.interval.civil_start_time >= "));
    }
}
