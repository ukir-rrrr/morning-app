//! ローカルファイル DB に対して書き込み層の冪等性を確認する。

use morning_collector::config::DatabaseTarget;
use morning_collector::db::{DailyMetricsRow, ExerciseRow, SleepSessionRow, Writer};

async fn writer(dir: &tempfile::TempDir) -> Writer {
    let path = dir.path().join("test.db").to_string_lossy().into_owned();
    let w = Writer::connect(&DatabaseTarget::Local { path })
        .await
        .unwrap();
    w.migrate().await.unwrap();
    w.migrate().await.unwrap();
    w
}

async fn count(w: &Writer, table: &str) -> i64 {
    let mut rows = w
        .connection()
        .query(&format!("SELECT COUNT(*) FROM {table}"), ())
        .await
        .unwrap();
    rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap()
}

#[tokio::test]
async fn sleep_upsert_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let w = writer(&dir).await;
    let mut row = SleepSessionRow {
        point_name: "users/me/dataTypes/sleep/dataPoints/1".into(),
        date: "2026-11-24".into(),
        start_time: "2026-11-23T14:00:00Z".into(),
        end_time: "2026-11-23T21:00:00Z".into(),
        minutes_asleep: Some(400),
        stages_json: Some("[]".into()),
        main_sleep: true,
        ..Default::default()
    };
    w.upsert_sleep_sessions(&[row.clone()]).await.unwrap();
    w.upsert_sleep_sessions(&[row.clone()]).await.unwrap();
    row.minutes_asleep = Some(410);
    w.upsert_sleep_sessions(&[row]).await.unwrap();
    assert_eq!(count(&w, "sleep_sessions").await, 1);

    let mut rows = w
        .connection()
        .query("SELECT minutes_asleep, main_sleep FROM sleep_sessions", ())
        .await
        .unwrap();
    let r = rows.next().await.unwrap().unwrap();
    assert_eq!(r.get::<i64>(0).unwrap(), 410);
    assert_eq!(r.get::<i64>(1).unwrap(), 1);
}

#[tokio::test]
async fn daily_metrics_merge_partial_rows() {
    let dir = tempfile::tempdir().unwrap();
    let w = writer(&dir).await;
    let hrv = DailyMetricsRow {
        date: "2026-11-24".into(),
        hrv_avg_ms: Some(42.3),
        nonrem_hr: Some(54),
        ..Default::default()
    };
    let spo2 = DailyMetricsRow {
        date: "2026-11-24".into(),
        spo2_avg: Some(95.6),
        ..Default::default()
    };
    w.upsert_daily_metrics(&[hrv.clone(), spo2.clone()])
        .await
        .unwrap();
    w.upsert_daily_metrics(&[spo2, hrv]).await.unwrap();
    assert_eq!(count(&w, "daily_metrics").await, 1);

    let mut rows = w
        .connection()
        .query(
            "SELECT hrv_avg_ms, nonrem_hr, spo2_avg, resting_hr FROM daily_metrics WHERE date = '2026-11-24'",
            (),
        )
        .await
        .unwrap();
    let r = rows.next().await.unwrap().unwrap();
    assert_eq!(r.get::<f64>(0).unwrap(), 42.3);
    assert_eq!(r.get::<i64>(1).unwrap(), 54);
    assert_eq!(r.get::<f64>(2).unwrap(), 95.6);
    assert_eq!(r.get::<Option<i64>>(3).unwrap(), None);
}

#[tokio::test]
async fn exercise_upsert_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let w = writer(&dir).await;
    let row = ExerciseRow {
        point_name: "users/me/dataTypes/exercise/dataPoints/1".into(),
        date: "2026-11-23".into(),
        start_time: "2026-11-22T22:30:00Z".into(),
        end_time: "2026-11-22T23:05:12Z".into(),
        exercise_type: Some("RUNNING".into()),
        duration_sec: Some(1984.5),
        platform: Some("FITBIT".into()),
        ..Default::default()
    };
    w.upsert_exercises(&[row.clone(), row]).await.unwrap();
    assert_eq!(count(&w, "exercises").await, 1);
}
