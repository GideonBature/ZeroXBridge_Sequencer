#[path = "utils.rs"]
mod utils;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use hyper;
use serde_json::json;
use tower::ServiceExt;
use utils::create_test_app;

#[tokio::test]
async fn test_post_valid_withdrawal() {
    let app = create_test_app().await;

    let request = Request::builder()
        .method("POST")
        .uri("/withdraw")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "stark_pub_key": "0xabc123",
                "amount": 5000
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(parsed.get("commitment_hash").is_some());
}

#[tokio::test]
async fn test_post_invalid_withdrawal() {
    let app = create_test_app().await;

    let request = Request::builder()
        .method("POST")
        .uri("/withdraw")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "stark_pub_key": "",
                "amount": -10
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_pending_withdrawals() {
    let app = create_test_app().await;

    let request = Request::builder()
        .method("GET")
        .uri("/withdraw")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(parsed.is_array());
}
