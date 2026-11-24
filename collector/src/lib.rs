//! Google Health API から睡眠・健康データを取得し、Turso (libSQL) に冪等に書き込む。

pub mod auth;
pub mod cli;
pub mod collect;
pub mod config;
pub mod db;
pub mod error;
pub mod health;
pub mod mapping;

pub use error::{Error, Result, EXIT_AUTH_EXPIRED, EXIT_FAILURE, EXIT_OK};

/// 起動 → 収集 → 終了 の 1 回分を実行する。
pub async fn run(cli: cli::Cli) -> Result<()> {
    let today = chrono::Local::now().date_naive();
    let range = cli.date_range(today)?;
    let config = config::Config::from_env()?;

    let writer = db::Writer::connect(&config.database).await?;
    writer.migrate().await?;

    let credentials = config.google_credentials()?;
    let token = auth::obtain_access_token(credentials).await?;

    let client = health::HealthClient::new(health::DEFAULT_BASE_URL)?;
    let summary = collect::collect(&client, &token, &writer, &range).await?;
    eprintln!(
        "[morning-collector] done {}..={}: sleep={} exercises={} daily={}",
        range.start, range.end, summary.sleep_sessions, summary.exercises, summary.daily_metrics
    );
    Ok(())
}
