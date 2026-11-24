use chrono::{Days, NaiveDate};
use clap::Parser;

use crate::{Error, Result};

#[derive(Debug, Clone, Parser)]
#[command(name = "morning-collector", version, about)]
pub struct Cli {
    /// バックフィル開始日（YYYY-MM-DD）。指定時は この日〜今日 を取得する
    #[arg(long, value_name = "YYYY-MM-DD", value_parser = parse_date)]
    pub from: Option<NaiveDate>,

    /// 通常時に取得する日数（今日を含めて遡る日数）
    #[arg(long, value_name = "N", default_value_t = 3)]
    pub days: u32,
}

/// 取得対象の日付範囲（両端を含む、ユーザーのローカル日付）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

impl DateRange {
    /// フィルタ用の排他的終端（end の翌日）。
    pub fn end_exclusive(&self) -> NaiveDate {
        self.end + Days::new(1)
    }
}

impl Cli {
    pub fn date_range(&self, today: NaiveDate) -> Result<DateRange> {
        let start = match self.from {
            Some(from) => {
                if from > today {
                    return Err(Error::InvalidArgs(format!(
                        "--from {from} is in the future (today is {today})"
                    )));
                }
                from
            }
            None => {
                if self.days == 0 {
                    return Err(Error::InvalidArgs("--days must be >= 1".into()));
                }
                today - Days::new(u64::from(self.days - 1))
            }
        };
        Ok(DateRange { start, end: today })
    }
}

fn parse_date(s: &str) -> std::result::Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| format!("expected YYYY-MM-DD: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        parse_date(s).unwrap()
    }

    #[test]
    fn default_is_last_three_days() {
        let cli = Cli::try_parse_from(["morning-collector"]).unwrap();
        assert_eq!(cli.days, 3);
        let r = cli.date_range(d("2026-11-24")).unwrap();
        assert_eq!(r.start, d("2026-11-22"));
        assert_eq!(r.end, d("2026-11-24"));
        assert_eq!(r.end_exclusive(), d("2026-11-25"));
    }

    #[test]
    fn from_overrides_days() {
        let cli = Cli::try_parse_from(["morning-collector", "--from", "2026-10-01", "--days", "7"])
            .unwrap();
        let r = cli.date_range(d("2026-11-24")).unwrap();
        assert_eq!(r.start, d("2026-10-01"));
    }

    #[test]
    fn rejects_invalid_input() {
        assert!(Cli::try_parse_from(["morning-collector", "--from", "2026/10/01"]).is_err());
        let cli = Cli::try_parse_from(["morning-collector", "--days", "0"]).unwrap();
        assert!(cli.date_range(d("2026-11-24")).is_err());
        let cli = Cli::try_parse_from(["morning-collector", "--from", "2026-12-01"]).unwrap();
        assert!(cli.date_range(d("2026-11-24")).is_err());
    }
}
