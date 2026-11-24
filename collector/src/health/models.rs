//! Health API v4 のレスポンスモデル（収集に必要なフィールドのみ）。
//!
//! proto3 JSON の int64 は文字列で返るため、整数フィールドは `de::opt_i64` で受ける。
//! 防御的に double フィールドも `de::opt_f64` で文字列を許容する。未知のフィールドは無視する。

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::de;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDataPointsResponse {
    #[serde(default)]
    pub data_points: Vec<DataPoint>,
    #[serde(default)]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataPoint {
    /// 例: `users/me/dataTypes/sleep/dataPoints/1234`
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub data_source: Option<DataSource>,
    #[serde(default)]
    pub sleep: Option<Sleep>,
    #[serde(default)]
    pub exercise: Option<Exercise>,
    #[serde(default)]
    pub daily_heart_rate_variability: Option<DailyHeartRateVariability>,
    #[serde(default)]
    pub daily_oxygen_saturation: Option<DailyOxygenSaturation>,
    #[serde(default)]
    pub daily_resting_heart_rate: Option<DailyRestingHeartRate>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataSource {
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub recording_method: Option<String>,
}

/// `google.type.Date`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Date {
    #[serde(deserialize_with = "de::i64")]
    pub year: i64,
    #[serde(deserialize_with = "de::i64")]
    pub month: i64,
    #[serde(deserialize_with = "de::i64")]
    pub day: i64,
}

impl Date {
    pub fn to_naive(self) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(
            i32::try_from(self.year).ok()?,
            u32::try_from(self.month).ok()?,
            u32::try_from(self.day).ok()?,
        )
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TimeOfDay {
    #[serde(default, deserialize_with = "de::opt_i64")]
    pub hours: Option<i64>,
    #[serde(default, deserialize_with = "de::opt_i64")]
    pub minutes: Option<i64>,
    #[serde(default, deserialize_with = "de::opt_i64")]
    pub seconds: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CivilDateTime {
    pub date: Date,
    #[serde(default)]
    pub time: Option<TimeOfDay>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTimeInterval {
    /// RFC 3339 (UTC)
    pub start_time: String,
    /// RFC 3339 (UTC)
    pub end_time: String,
    /// 例: `"32400s"`
    #[serde(default)]
    pub start_utc_offset: Option<String>,
    #[serde(default)]
    pub end_utc_offset: Option<String>,
    #[serde(default)]
    pub civil_start_time: Option<CivilDateTime>,
    #[serde(default)]
    pub civil_end_time: Option<CivilDateTime>,
}

// ---- sleep ----

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sleep {
    pub interval: SessionTimeInterval,
    /// `CLASSIC` / `STAGES`
    #[serde(default, rename = "type")]
    pub sleep_type: Option<String>,
    #[serde(default)]
    pub metadata: SleepMetadata,
    #[serde(default)]
    pub summary: Option<SleepSummary>,
    #[serde(default)]
    pub stages: Vec<SleepStage>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SleepMetadata {
    #[serde(default)]
    pub main_sleep: bool,
    #[serde(default)]
    pub nap: bool,
    #[serde(default)]
    pub processed: bool,
    #[serde(default)]
    pub manually_edited: bool,
    #[serde(default)]
    pub stages_status: Option<String>,
    #[serde(default)]
    pub external_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SleepSummary {
    #[serde(default, deserialize_with = "de::opt_i64")]
    pub minutes_asleep: Option<i64>,
    #[serde(default, deserialize_with = "de::opt_i64")]
    pub minutes_awake: Option<i64>,
    #[serde(default, deserialize_with = "de::opt_i64")]
    pub minutes_in_sleep_period: Option<i64>,
    #[serde(default, deserialize_with = "de::opt_i64")]
    pub minutes_to_fall_asleep: Option<i64>,
    #[serde(default, deserialize_with = "de::opt_i64")]
    pub minutes_after_wake_up: Option<i64>,
    #[serde(default)]
    pub stages_summary: Vec<StageSummary>,
}

impl SleepSummary {
    /// `stagesSummary` から指定ステージ（`DEEP` / `LIGHT` / `REM` / `AWAKE` など）の分数を引く。
    pub fn stage_minutes(&self, stage_type: &str) -> Option<i64> {
        self.stages_summary
            .iter()
            .find(|s| s.stage_type.eq_ignore_ascii_case(stage_type))
            .and_then(|s| s.minutes)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageSummary {
    #[serde(rename = "type")]
    pub stage_type: String,
    #[serde(default, deserialize_with = "de::opt_i64")]
    pub minutes: Option<i64>,
    #[serde(default, deserialize_with = "de::opt_i64")]
    pub count: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SleepStage {
    pub start_time: String,
    pub end_time: String,
    #[serde(rename = "type")]
    pub stage_type: String,
}

// ---- daily summaries ----

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyHeartRateVariability {
    pub date: Date,
    #[serde(default, deserialize_with = "de::opt_f64")]
    pub average_heart_rate_variability_milliseconds: Option<f64>,
    #[serde(default, deserialize_with = "de::opt_f64")]
    pub deep_sleep_root_mean_square_of_successive_differences_milliseconds: Option<f64>,
    #[serde(default, deserialize_with = "de::opt_i64")]
    pub non_rem_heart_rate_beats_per_minute: Option<i64>,
    #[serde(default, deserialize_with = "de::opt_f64")]
    pub entropy: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyOxygenSaturation {
    pub date: Date,
    #[serde(default, deserialize_with = "de::opt_f64")]
    pub average_percentage: Option<f64>,
    #[serde(default, deserialize_with = "de::opt_f64")]
    pub lower_bound_percentage: Option<f64>,
    #[serde(default, deserialize_with = "de::opt_f64")]
    pub upper_bound_percentage: Option<f64>,
    #[serde(default, deserialize_with = "de::opt_f64")]
    pub standard_deviation_percentage: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyRestingHeartRate {
    pub date: Date,
    #[serde(default, deserialize_with = "de::opt_i64")]
    pub beats_per_minute: Option<i64>,
}

// ---- exercise ----

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Exercise {
    pub interval: SessionTimeInterval,
    #[serde(default)]
    pub exercise_type: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    /// 一時停止を除いた時間。例: `"1834.5s"`
    #[serde(default)]
    pub active_duration: Option<String>,
    #[serde(default)]
    pub metrics_summary: Option<MetricsSummary>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsSummary {
    #[serde(default, deserialize_with = "de::opt_f64")]
    pub calories_kcal: Option<f64>,
    #[serde(default, deserialize_with = "de::opt_f64")]
    pub distance_millimeters: Option<f64>,
    #[serde(default, deserialize_with = "de::opt_i64")]
    pub steps: Option<i64>,
    #[serde(default, deserialize_with = "de::opt_i64")]
    pub average_heart_rate_beats_per_minute: Option<i64>,
}
