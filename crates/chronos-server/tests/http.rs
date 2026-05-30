//! HTTP integration tests driving the real axum router in-process (no socket).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chronos_embedded::FactStore;
use chronos_server::{build_router, AppState};
use std::sync::Arc;
use tower::ServiceExt; // for `oneshot`

fn app() -> axum::Router {
    build_router(AppState::new(Arc::new(FactStore::new())))
}

async fn post_json(
    app: axum::Router,
    uri: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    };
    (status, json)
}

#[tokio::test]
async fn healthz_ok() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn ingest_then_search_returns_cited_current_fact() {
    let app = app();

    let (s1, _) = post_json(
        app.clone(),
        "/v1/memory",
        serde_json::json!({
            "subject": "Alice", "predicate": "lives_in", "object": "Beijing",
            "valid_from": 1000, "doc": 10, "chunk": 1
        }),
    )
    .await;
    assert_eq!(s1, StatusCode::OK);

    let (s2, _) = post_json(
        app.clone(),
        "/v1/memory",
        serde_json::json!({
            "subject": "Alice", "predicate": "lives_in", "object": "Shanghai",
            "valid_from": 2000, "doc": 20, "chunk": 1
        }),
    )
    .await;
    assert_eq!(s2, StatusCode::OK);

    let (s3, body) = post_json(
        app.clone(),
        "/v1/search",
        serde_json::json!({
            "query": "MATCH (n) WHERE SIMILAR(n, \"Alice lives\") RETURN CONTEXT(cite = true)"
        }),
    )
    .await;
    assert_eq!(s3, StatusCode::OK);

    let text = body["text"].as_str().unwrap();
    assert!(text.contains("Shanghai"), "got: {text}");
    assert!(!text.contains("Beijing"), "got: {text}");
    assert_eq!(body["citations"][0]["doc"].as_u64(), Some(20));
}

#[tokio::test]
async fn point_in_time_search_over_http() {
    let app = app();
    for (obj, vf, doc) in [("Beijing", 1000, 10), ("Shanghai", 2000, 20)] {
        post_json(
            app.clone(),
            "/v1/memory",
            serde_json::json!({
                "subject": "Alice", "predicate": "lives_in", "object": obj,
                "valid_from": vf, "doc": doc, "chunk": 1
            }),
        )
        .await;
    }

    let (_, body) = post_json(
        app.clone(),
        "/v1/search",
        serde_json::json!({
            "query": "WHERE SIMILAR(x, \"Alice\") AS OF VALID TIME 1500 RETURN CONTEXT(cite = true)"
        }),
    )
    .await;
    assert!(body["text"].as_str().unwrap().contains("Beijing"));
    assert_eq!(body["citations"][0]["doc"].as_u64(), Some(10));
}

#[tokio::test]
async fn communities_group_connected_entities() {
    let app = app();
    // Two disjoint clusters: {Alice, Beijing} and {Bob, Tokyo}.
    for (s, o, doc) in [("Alice", "Beijing", 10), ("Bob", "Tokyo", 20)] {
        post_json(
            app.clone(),
            "/v1/memory",
            serde_json::json!({
                "subject": s, "predicate": "lives_in", "object": o,
                "valid_from": 1000, "doc": doc, "chunk": 1
            }),
        )
        .await;
    }

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/v1/communities")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let comms = body["communities"].as_array().unwrap();
    assert_eq!(comms.len(), 2, "got: {body}");
    // Each community's summary mentions its members and current facts.
    let summaries: Vec<&str> = comms
        .iter()
        .map(|c| c["summary"].as_str().unwrap())
        .collect();
    assert!(summaries
        .iter()
        .any(|s| s.contains("Alice") && s.contains("Beijing")));
    assert!(summaries
        .iter()
        .any(|s| s.contains("Bob") && s.contains("Tokyo")));
}

#[tokio::test]
async fn bad_query_is_400() {
    let (status, _) = post_json(
        app(),
        "/v1/search",
        serde_json::json!({ "query": "MATCH ( <bad" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
