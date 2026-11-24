//! Turso (libSQL) への書き込み層。すべて upsert で冪等に書く。

use libsql::{params, Builder, Connection, Database};

use crate::config::DatabaseTarget;
use crate::Result;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS sleep_sessions (
    point_name     TEXT UNIQUE,
    date           TEXT,
    start_time     TEXT,
    end_time       TEXT,
    minutes_asleep INTEGER,
    minutes_awake  INTEGER,
    minutes_light  INTEGER,
    minutes_deep   INTEGER,
    minutes_rem    INTEGER,
    stages_json    TEXT,
    main_sleep     INTEGER
);
CREATE TABLE IF NOT EXISTS daily_metrics (
    date        TEXT UNIQUE,
    hrv_avg_ms  REAL,
    hrv_deep_ms REAL,
    nonrem_hr   INTEGER,
    spo2_avg    REAL,
    spo2_low    REAL,
    spo2_high   REAL,
    resting_hr  INTEGER,
    steps       INTEGER
);
CREATE TABLE IF NOT EXISTS exercises (
    point_name    TEXT UNIQUE,
    date          TEXT,
    start_time    TEXT,
    end_time      TEXT,
    exercise_type TEXT,
    duration_sec  REAL,
    distance_mm   REAL,
    calories_kcal REAL,
    platform      TEXT
);
";

const UPSERT_SLEEP: &str = "
INSERT INTO sleep_sessions (
    point_name, date, start_time, end_time, minutes_asleep, minutes_awake,
    minutes_light, minutes_deep, minutes_rem, stages_json, main_sleep
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
ON CONFLICT(point_name) DO UPDATE SET
    date = excluded.date,
    start_time = excluded.start_time,
    end_time = excluded.end_time,
    minutes_asleep = excluded.minutes_asleep,
    minutes_awake = excluded.minutes_awake,
    minutes_light = excluded.minutes_light,
    minutes_deep = excluded.minutes_deep,
    minutes_rem = excluded.minutes_rem,
    stages_json = excluded.stages_json,
    main_sleep = excluded.main_sleep
";

// daily_metrics は複数のデータ型から列ごとに埋まるため、NULL で既存値を上書きしない。
const UPSERT_DAILY: &str = "
INSERT INTO daily_metrics (
    date, hrv_avg_ms, hrv_deep_ms, nonrem_hr, spo2_avg, spo2_low, spo2_high, resting_hr, steps
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
ON CONFLICT(date) DO UPDATE SET
    hrv_avg_ms = COALESCE(excluded.hrv_avg_ms, daily_metrics.hrv_avg_ms),
    hrv_deep_ms = COALESCE(excluded.hrv_deep_ms, daily_metrics.hrv_deep_ms),
    nonrem_hr = COALESCE(excluded.nonrem_hr, daily_metrics.nonrem_hr),
    spo2_avg = COALESCE(excluded.spo2_avg, daily_metrics.spo2_avg),
    spo2_low = COALESCE(excluded.spo2_low, daily_metrics.spo2_low),
    spo2_high = COALESCE(excluded.spo2_high, daily_metrics.spo2_high),
    resting_hr = COALESCE(excluded.resting_hr, daily_metrics.resting_hr),
    steps = COALESCE(excluded.steps, daily_metrics.steps)
";

const UPSERT_EXERCISE: &str = "
INSERT INTO exercises (
    point_name, date, start_time, end_time, exercise_type, duration_sec,
    distance_mm, calories_kcal, platform
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
ON CONFLICT(point_name) DO UPDATE SET
    date = excluded.date,
    start_time = excluded.start_time,
    end_time = excluded.end_time,
    exercise_type = excluded.exercise_type,
    duration_sec = excluded.duration_sec,
    distance_mm = excluded.distance_mm,
    calories_kcal = excluded.calories_kcal,
    platform = excluded.platform
";

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SleepSessionRow {
    pub point_name: String,
    pub date: String,
    pub start_time: String,
    pub end_time: String,
    pub minutes_asleep: Option<i64>,
    pub minutes_awake: Option<i64>,
    pub minutes_light: Option<i64>,
    pub minutes_deep: Option<i64>,
    pub minutes_rem: Option<i64>,
    pub stages_json: Option<String>,
    pub main_sleep: bool,
}

/// 列が `None` のものは既存値を保持する（部分更新）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DailyMetricsRow {
    pub date: String,
    pub hrv_avg_ms: Option<f64>,
    pub hrv_deep_ms: Option<f64>,
    pub nonrem_hr: Option<i64>,
    pub spo2_avg: Option<f64>,
    pub spo2_low: Option<f64>,
    pub spo2_high: Option<f64>,
    pub resting_hr: Option<i64>,
    pub steps: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExerciseRow {
    pub point_name: String,
    pub date: String,
    pub start_time: String,
    pub end_time: String,
    pub exercise_type: Option<String>,
    pub duration_sec: Option<f64>,
    pub distance_mm: Option<f64>,
    pub calories_kcal: Option<f64>,
    pub platform: Option<String>,
}

pub struct Writer {
    _db: Database,
    conn: Connection,
}

impl Writer {
    pub async fn connect(target: &DatabaseTarget) -> Result<Self> {
        let db = match target {
            DatabaseTarget::Remote { url, auth_token } => {
                Builder::new_remote(url.clone(), auth_token.expose().to_string())
                    .build()
                    .await?
            }
            DatabaseTarget::Local { path } => Builder::new_local(path).build().await?,
        };
        let conn = db.connect()?;
        Ok(Self { _db: db, conn })
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// テーブルを作成する（何度実行してもよい）。
    pub async fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(SCHEMA).await?;
        Ok(())
    }

    pub async fn upsert_sleep_sessions(&self, rows: &[SleepSessionRow]) -> Result<usize> {
        if rows.is_empty() {
            return Ok(0);
        }
        let tx = self.conn.transaction().await?;
        for r in rows {
            tx.execute(
                UPSERT_SLEEP,
                params![
                    r.point_name.as_str(),
                    r.date.as_str(),
                    r.start_time.as_str(),
                    r.end_time.as_str(),
                    r.minutes_asleep,
                    r.minutes_awake,
                    r.minutes_light,
                    r.minutes_deep,
                    r.minutes_rem,
                    r.stages_json.as_deref(),
                    i64::from(r.main_sleep),
                ],
            )
            .await?;
        }
        tx.commit().await?;
        Ok(rows.len())
    }

    pub async fn upsert_daily_metrics(&self, rows: &[DailyMetricsRow]) -> Result<usize> {
        if rows.is_empty() {
            return Ok(0);
        }
        let tx = self.conn.transaction().await?;
        for r in rows {
            tx.execute(
                UPSERT_DAILY,
                params![
                    r.date.as_str(),
                    r.hrv_avg_ms,
                    r.hrv_deep_ms,
                    r.nonrem_hr,
                    r.spo2_avg,
                    r.spo2_low,
                    r.spo2_high,
                    r.resting_hr,
                    r.steps,
                ],
            )
            .await?;
        }
        tx.commit().await?;
        Ok(rows.len())
    }

    pub async fn upsert_exercises(&self, rows: &[ExerciseRow]) -> Result<usize> {
        if rows.is_empty() {
            return Ok(0);
        }
        let tx = self.conn.transaction().await?;
        for r in rows {
            tx.execute(
                UPSERT_EXERCISE,
                params![
                    r.point_name.as_str(),
                    r.date.as_str(),
                    r.start_time.as_str(),
                    r.end_time.as_str(),
                    r.exercise_type.as_deref(),
                    r.duration_sec,
                    r.distance_mm,
                    r.calories_kcal,
                    r.platform.as_deref(),
                ],
            )
            .await?;
        }
        tx.commit().await?;
        Ok(rows.len())
    }
}
