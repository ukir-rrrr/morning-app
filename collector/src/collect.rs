//! 収集の流れ：各データ型を取得 → 行へ変換 → upsert。

use crate::auth::AccessToken;
use crate::cli::DateRange;
use crate::db::Writer;
use crate::health::{self, data_types, HealthClient};
use crate::{mapping, Result};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Summary {
    pub sleep_sessions: usize,
    pub exercises: usize,
    pub daily_metrics: usize,
}

fn warn_skipped(data_type: &str, total: usize, kept: usize) {
    if kept < total {
        eprintln!(
            "[morning-collector] warn: {data_type}: skipped {} of {total} data points (missing name/date)",
            total - kept
        );
    }
}

pub async fn collect(
    client: &HealthClient,
    token: &AccessToken,
    writer: &Writer,
    range: &DateRange,
) -> Result<Summary> {
    let mut summary = Summary::default();

    let filter = health::sleep_filter(range);
    let points = client
        .list_all_data_points(
            token,
            data_types::SLEEP,
            Some(&filter),
            Some(health::SESSION_PAGE_SIZE),
        )
        .await?;
    let rows: Vec<_> = points.iter().filter_map(mapping::sleep_row).collect();
    warn_skipped(data_types::SLEEP, points.len(), rows.len());
    summary.sleep_sessions = writer.upsert_sleep_sessions(&rows).await?;

    let filter = health::exercise_filter(range);
    let points = client
        .list_all_data_points(
            token,
            data_types::EXERCISE,
            Some(&filter),
            Some(health::SESSION_PAGE_SIZE),
        )
        .await?;
    let rows: Vec<_> = points.iter().filter_map(mapping::exercise_row).collect();
    warn_skipped(data_types::EXERCISE, points.len(), rows.len());
    summary.exercises = writer.upsert_exercises(&rows).await?;

    for data_type in [
        data_types::DAILY_HEART_RATE_VARIABILITY,
        data_types::DAILY_OXYGEN_SATURATION,
        data_types::DAILY_RESTING_HEART_RATE,
    ] {
        let filter = health::daily_filter(data_type, range);
        let points = client
            .list_all_data_points(token, data_type, Some(&filter), None)
            .await?;
        let rows: Vec<_> = points
            .iter()
            .filter_map(mapping::daily_metrics_row)
            .collect();
        warn_skipped(data_type, points.len(), rows.len());
        summary.daily_metrics += writer.upsert_daily_metrics(&rows).await?;
    }

    Ok(summary)
}
