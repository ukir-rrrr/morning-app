//! API モデル → DB 行への変換（基本形）。

use chrono::{DateTime, Duration, NaiveDate};

use crate::db::{DailyMetricsRow, ExerciseRow, SleepSessionRow};
use crate::health::models::{DataPoint, SessionTimeInterval};

/// `"32400s"` / `"1834.5s"` 形式（google.protobuf.Duration）を秒に変換する。
pub fn parse_duration_secs(s: &str) -> Option<f64> {
    s.trim().strip_suffix('s')?.parse::<f64>().ok()
}

/// RFC 3339 の時刻に UTC オフセット（あれば）を加えたローカル日付。
fn local_date(time: &str, utc_offset: Option<&str>) -> Option<NaiveDate> {
    let utc = DateTime::parse_from_rfc3339(time).ok()?.naive_utc();
    let offset = utc_offset.and_then(parse_duration_secs).unwrap_or(0.0);
    Some((utc + Duration::seconds(offset as i64)).date())
}

fn end_date(interval: &SessionTimeInterval) -> Option<NaiveDate> {
    interval
        .civil_end_time
        .as_ref()
        .and_then(|c| c.date.to_naive())
        .or_else(|| local_date(&interval.end_time, interval.end_utc_offset.as_deref()))
}

fn start_date(interval: &SessionTimeInterval) -> Option<NaiveDate> {
    interval
        .civil_start_time
        .as_ref()
        .and_then(|c| c.date.to_naive())
        .or_else(|| local_date(&interval.start_time, interval.start_utc_offset.as_deref()))
}

fn interval_secs(interval: &SessionTimeInterval) -> Option<f64> {
    let start = DateTime::parse_from_rfc3339(&interval.start_time).ok()?;
    let end = DateTime::parse_from_rfc3339(&interval.end_time).ok()?;
    Some((end - start).num_milliseconds() as f64 / 1000.0)
}

/// 睡眠ポイント → `sleep_sessions` 行。`date` は起床日（ローカル）。
pub fn sleep_row(dp: &DataPoint) -> Option<SleepSessionRow> {
    let sleep = dp.sleep.as_ref()?;
    let point_name = dp.name.clone()?;
    let date = end_date(&sleep.interval)?;
    let summary = sleep.summary.clone().unwrap_or_default();
    let stages_json = if sleep.stages.is_empty() {
        None
    } else {
        serde_json::to_string(&sleep.stages).ok()
    };
    Some(SleepSessionRow {
        point_name,
        date: date.to_string(),
        start_time: sleep.interval.start_time.clone(),
        end_time: sleep.interval.end_time.clone(),
        minutes_asleep: summary.minutes_asleep,
        minutes_awake: summary
            .minutes_awake
            .or_else(|| summary.stage_minutes("AWAKE")),
        minutes_light: summary.stage_minutes("LIGHT"),
        minutes_deep: summary.stage_minutes("DEEP"),
        minutes_rem: summary.stage_minutes("REM"),
        stages_json,
        main_sleep: sleep.metadata.main_sleep,
    })
}

/// 運動ポイント → `exercises` 行。`duration_sec` は activeDuration 優先、無ければ区間長。
pub fn exercise_row(dp: &DataPoint) -> Option<ExerciseRow> {
    let ex = dp.exercise.as_ref()?;
    let point_name = dp.name.clone()?;
    let date = start_date(&ex.interval)?;
    let metrics = ex.metrics_summary.clone().unwrap_or_default();
    Some(ExerciseRow {
        point_name,
        date: date.to_string(),
        start_time: ex.interval.start_time.clone(),
        end_time: ex.interval.end_time.clone(),
        exercise_type: ex.exercise_type.clone(),
        duration_sec: ex
            .active_duration
            .as_deref()
            .and_then(parse_duration_secs)
            .or_else(|| interval_secs(&ex.interval)),
        distance_mm: metrics.distance_millimeters,
        calories_kcal: metrics.calories_kcal,
        platform: dp.data_source.as_ref().and_then(|s| s.platform.clone()),
    })
}

/// 日次サマリ系ポイント（HRV / SpO2 / 安静時心拍）→ `daily_metrics` の部分行。
pub fn daily_metrics_row(dp: &DataPoint) -> Option<DailyMetricsRow> {
    if let Some(hrv) = &dp.daily_heart_rate_variability {
        return Some(DailyMetricsRow {
            date: hrv.date.to_naive()?.to_string(),
            hrv_avg_ms: hrv.average_heart_rate_variability_milliseconds,
            hrv_deep_ms: hrv.deep_sleep_root_mean_square_of_successive_differences_milliseconds,
            nonrem_hr: hrv.non_rem_heart_rate_beats_per_minute,
            ..Default::default()
        });
    }
    if let Some(spo2) = &dp.daily_oxygen_saturation {
        return Some(DailyMetricsRow {
            date: spo2.date.to_naive()?.to_string(),
            spo2_avg: spo2.average_percentage,
            spo2_low: spo2.lower_bound_percentage,
            spo2_high: spo2.upper_bound_percentage,
            ..Default::default()
        });
    }
    if let Some(rhr) = &dp.daily_resting_heart_rate {
        return Some(DailyMetricsRow {
            date: rhr.date.to_naive()?.to_string(),
            resting_hr: rhr.beats_per_minute,
            ..Default::default()
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_durations() {
        assert_eq!(parse_duration_secs("32400s"), Some(32400.0));
        assert_eq!(parse_duration_secs("1834.5s"), Some(1834.5));
        assert_eq!(parse_duration_secs("-3600s"), Some(-3600.0));
        assert_eq!(parse_duration_secs("10m"), None);
    }

    #[test]
    fn local_date_applies_offset() {
        let d = local_date("2026-11-23T16:30:00Z", Some("32400s")).unwrap();
        assert_eq!(d.to_string(), "2026-11-24");
        let d = local_date("2026-11-23T16:30:00Z", None).unwrap();
        assert_eq!(d.to_string(), "2026-11-23");
    }
}
