//! Health API クライアントのページング・エラー処理をモックサーバで確認する。

use morning_collector::auth::AccessToken;
use morning_collector::health::{data_types, HealthClient};
use morning_collector::{Error, EXIT_AUTH_EXPIRED};
use serde_json::json;
use wiremock::matchers::{header, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

const DP_PATH: &str = "/v4/users/me/dataTypes/daily-oxygen-saturation/dataPoints";

fn spo2_point(day: u32) -> serde_json::Value {
    json!({
        "dailyOxygenSaturation": {
            "date": { "year": 2026, "month": 11, "day": day },
            "averagePercentage": 95.0,
            "lowerBoundPercentage": 93.0,
            "upperBoundPercentage": 98.0
        }
    })
}

fn client(server: &MockServer) -> HealthClient {
    HealthClient::new(format!("{}/v4", server.uri())).unwrap()
}

#[tokio::test]
async fn follows_next_page_token() {
    let server = MockServer::start().await;
    let filter = "daily_oxygen_saturation.date >= \"2026-11-22\"";

    Mock::given(method("GET"))
        .and(path(DP_PATH))
        .and(header("authorization", "Bearer test-token"))
        .and(query_param("filter", filter))
        .and(query_param("pageSize", "2"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "dataPoints": [spo2_point(24), spo2_point(23)],
            "nextPageToken": "page-2"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(DP_PATH))
        .and(query_param("pageToken", "page-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "dataPoints": [spo2_point(22)]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let points = client(&server)
        .list_all_data_points(
            &AccessToken::new("test-token"),
            data_types::DAILY_OXYGEN_SATURATION,
            Some(filter),
            Some(2),
        )
        .await
        .unwrap();
    let days: Vec<i64> = points
        .iter()
        .map(|p| p.daily_oxygen_saturation.as_ref().unwrap().date.day)
        .collect();
    assert_eq!(days, vec![24, 23, 22]);
}

#[tokio::test]
async fn unauthorized_maps_to_exit_code_2() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(DP_PATH))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let err = client(&server)
        .list_all_data_points(
            &AccessToken::new("expired"),
            data_types::DAILY_OXYGEN_SATURATION,
            None,
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Auth(_)));
    assert_eq!(err.exit_code(), EXIT_AUTH_EXPIRED);
}

#[tokio::test]
async fn server_error_maps_to_exit_code_1() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(DP_PATH))
        .respond_with(ResponseTemplate::new(503).set_body_string("unavailable"))
        .mount(&server)
        .await;

    let err = client(&server)
        .list_all_data_points(
            &AccessToken::new("t"),
            data_types::DAILY_OXYGEN_SATURATION,
            None,
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Api { status: 503, .. }));
    assert_eq!(err.exit_code(), 1);
}

#[tokio::test]
async fn stuck_page_token_is_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(DP_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "dataPoints": [],
            "nextPageToken": "same"
        })))
        .mount(&server)
        .await;

    let err = client(&server)
        .list_all_data_points(
            &AccessToken::new("t"),
            data_types::DAILY_OXYGEN_SATURATION,
            None,
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Pagination(_)));
}
