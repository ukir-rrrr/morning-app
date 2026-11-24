use std::time::Duration;

use crate::auth::AccessToken;
use crate::error::AuthError;
use crate::{Error, Result};

use super::models::{DataPoint, ListDataPointsResponse};

pub const DEFAULT_BASE_URL: &str = "https://health.googleapis.com/v4";

/// 無限ページングを防ぐための上限。
const MAX_PAGES: usize = 1000;
const ERROR_BODY_LIMIT: usize = 512;

/// `GET /v4/users/me/dataTypes/{dataType}/dataPoints` のクエリパラメータ。
#[derive(Debug, Clone, Default)]
pub struct ListRequest<'a> {
    pub filter: Option<&'a str>,
    pub page_size: Option<u32>,
    pub page_token: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct HealthClient {
    http: reqwest::Client,
    base_url: String,
}

impl HealthClient {
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("morning-collector/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            http,
            base_url: base_url.into().trim_end_matches('/').to_string(),
        })
    }

    fn data_points_url(&self, data_type: &str) -> String {
        format!(
            "{}/users/me/dataTypes/{}/dataPoints",
            self.base_url, data_type
        )
    }

    /// 1 ページ分を取得する。
    pub async fn list_data_points(
        &self,
        token: &AccessToken,
        data_type: &str,
        req: &ListRequest<'_>,
    ) -> Result<ListDataPointsResponse> {
        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(filter) = req.filter {
            query.push(("filter", filter.to_string()));
        }
        if let Some(size) = req.page_size {
            query.push(("pageSize", size.to_string()));
        }
        if let Some(page_token) = req.page_token {
            query.push(("pageToken", page_token.to_string()));
        }

        let resp = self
            .http
            .get(self.data_points_url(data_type))
            .bearer_auth(token.as_str())
            .query(&query)
            .send()
            .await?;

        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(AuthError::Unauthorized.into());
        }
        let body = resp.bytes().await?;
        if !status.is_success() {
            let mut text = String::from_utf8_lossy(&body).into_owned();
            if text.len() > ERROR_BODY_LIMIT {
                let cut = (0..=ERROR_BODY_LIMIT)
                    .rev()
                    .find(|&i| text.is_char_boundary(i))
                    .unwrap_or(0);
                text.truncate(cut);
                text.push('…');
            }
            return Err(Error::Api {
                status: status.as_u16(),
                body: text,
            });
        }
        Ok(serde_json::from_slice(&body)?)
    }

    /// `nextPageToken` が尽きるまでページングして全件を返す。
    pub async fn list_all_data_points(
        &self,
        token: &AccessToken,
        data_type: &str,
        filter: Option<&str>,
        page_size: Option<u32>,
    ) -> Result<Vec<DataPoint>> {
        let mut all = Vec::new();
        let mut page_token: Option<String> = None;
        for _ in 0..MAX_PAGES {
            let req = ListRequest {
                filter,
                page_size,
                page_token: page_token.as_deref(),
            };
            let page = self.list_data_points(token, data_type, &req).await?;
            all.extend(page.data_points);
            match page.next_page_token.filter(|t| !t.is_empty()) {
                Some(next) if Some(&next) != page_token.as_ref() => page_token = Some(next),
                Some(_) => {
                    return Err(Error::Pagination(format!(
                        "{data_type}: nextPageToken did not advance"
                    )))
                }
                None => return Ok(all),
            }
        }
        Err(Error::Pagination(format!(
            "{data_type}: exceeded {MAX_PAGES} pages"
        )))
    }
}
