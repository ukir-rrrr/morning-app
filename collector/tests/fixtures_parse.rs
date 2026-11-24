//! 実レスポンス形のフィクスチャがパース・変換できることを確認する。

use morning_collector::health::models::ListDataPointsResponse;
use morning_collector::mapping;

fn load(name: &str) -> ListDataPointsResponse {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    serde_json::from_str(&json).unwrap_or_else(|e| panic!("{name}: {e}"))
}

#[test]
fn sleep_main_sleep_with_stages() {
    let resp = load("sleep_main.json");
    assert_eq!(resp.data_points.len(), 1);
    assert_eq!(resp.next_page_token.as_deref(), Some(""));

    let dp = &resp.data_points[0];
    let sleep = dp.sleep.as_ref().unwrap();
    assert!(sleep.metadata.main_sleep);
    assert_eq!(sleep.interval.start_time, "2026-11-23T14:12:30Z");
    let summary = sleep.summary.as_ref().unwrap();
    assert_eq!(summary.minutes_asleep, Some(397));
    assert_eq!(summary.stage_minutes("LIGHT"), Some(217));
    assert_eq!(sleep.stages.len(), 5);

    let row = mapping::sleep_row(dp).unwrap();
    assert_eq!(
        row.point_name,
        "users/me/dataTypes/sleep/dataPoints/48291736501"
    );
    assert_eq!(row.date, "2026-11-24");
    assert_eq!(row.end_time, "2026-11-23T21:41:00Z");
    assert_eq!(row.minutes_asleep, Some(397));
    assert_eq!(row.minutes_awake, Some(51));
    assert_eq!(row.minutes_light, Some(217));
    assert_eq!(row.minutes_deep, Some(84));
    assert_eq!(row.minutes_rem, Some(96));
    assert!(row.main_sleep);

    let stages: serde_json::Value =
        serde_json::from_str(row.stages_json.as_deref().unwrap()).unwrap();
    assert_eq!(stages.as_array().unwrap().len(), 5);
    assert_eq!(stages[1]["type"], "DEEP");
}

#[test]
fn sleep_nap_classic() {
    let resp = load("sleep_nap.json");
    assert_eq!(resp.next_page_token, None);
    let dp = &resp.data_points[0];
    let sleep = dp.sleep.as_ref().unwrap();
    assert!(!sleep.metadata.main_sleep);
    assert!(sleep.metadata.nap);
    assert_eq!(sleep.sleep_type.as_deref(), Some("CLASSIC"));

    // civilEndTime が無いので endTime + endUtcOffset から起床日を求める
    let row = mapping::sleep_row(dp).unwrap();
    assert_eq!(row.date, "2026-11-24");
    assert_eq!(row.minutes_asleep, Some(44));
    assert_eq!(row.minutes_awake, None);
    assert_eq!(row.minutes_deep, None);
    assert!(!row.main_sleep);
}

#[test]
fn daily_heart_rate_variability() {
    let resp = load("daily_hrv.json");
    let dp = &resp.data_points[0];
    let hrv = dp.daily_heart_rate_variability.as_ref().unwrap();
    assert_eq!(hrv.non_rem_heart_rate_beats_per_minute, Some(54));

    let row = mapping::daily_metrics_row(dp).unwrap();
    assert_eq!(row.date, "2026-11-24");
    assert_eq!(row.hrv_avg_ms, Some(42.317));
    assert_eq!(row.hrv_deep_ms, Some(48.902));
    assert_eq!(row.nonrem_hr, Some(54));
    assert_eq!(row.spo2_avg, None);
}

#[test]
fn daily_oxygen_saturation() {
    let resp = load("daily_spo2.json");
    let row = mapping::daily_metrics_row(&resp.data_points[0]).unwrap();
    assert_eq!(row.date, "2026-11-24");
    assert_eq!(row.spo2_avg, Some(95.6));
    assert_eq!(row.spo2_low, Some(93.1));
    assert_eq!(row.spo2_high, Some(98.2));
    assert_eq!(row.hrv_avg_ms, None);
}

#[test]
fn daily_resting_heart_rate() {
    let resp = load("daily_resting_hr.json");
    let row = mapping::daily_metrics_row(&resp.data_points[0]).unwrap();
    assert_eq!(row.date, "2026-11-24");
    assert_eq!(row.resting_hr, Some(58));
}

#[test]
fn exercise_run() {
    let resp = load("exercise.json");
    let dp = &resp.data_points[0];
    let metrics = dp
        .exercise
        .as_ref()
        .unwrap()
        .metrics_summary
        .as_ref()
        .unwrap();
    assert_eq!(metrics.steps, Some(5321));
    assert_eq!(metrics.average_heart_rate_beats_per_minute, Some(148));

    let row = mapping::exercise_row(dp).unwrap();
    assert_eq!(
        row.point_name,
        "users/me/dataTypes/exercise/dataPoints/7340012345"
    );
    assert_eq!(row.date, "2026-11-23");
    assert_eq!(row.exercise_type.as_deref(), Some("RUNNING"));
    assert_eq!(row.duration_sec, Some(1984.5));
    assert_eq!(row.distance_mm, Some(5_023_400.0));
    assert_eq!(row.calories_kcal, Some(312.4));
    assert_eq!(row.platform.as_deref(), Some("FITBIT"));
}
