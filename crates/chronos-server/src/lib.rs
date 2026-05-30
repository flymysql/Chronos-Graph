//! HTTP/REST service layer over the Chronos-Graph engine.
//!
//! Exposes the engine for external clients and SDKs. gRPC is planned but
//! deferred until `protoc` is part of CI; the REST surface is the M3 contract.
//!
//! Endpoints:
//! - `GET  /healthz`     -> liveness
//! - `POST /v1/memory`   -> ingest a fact (UPSERT_FACT)
//! - `POST /v1/search`   -> question -> cited, point-in-time context
//! - `GET  /v1/communities` -> level-0 community summaries (global view)

pub mod acl;
pub mod session;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chronos_common::{ChunkId, DocId, Timestamp};
use chronos_embedded::{FactStore, MemoryRetriever};
use chronos_temporal::ConflictPolicy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<FactStore>,
}

impl AppState {
    pub fn new(store: Arc<FactStore>) -> Self {
        Self { store }
    }
}

/// Build the application router.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/memory", post(add_memory))
        .route("/v1/search", post(search))
        .route("/v1/communities", get(communities))
        .with_state(state)
}

/// Bind to `addr` and serve until the process is terminated.
pub async fn serve(addr: std::net::SocketAddr, state: AppState) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "chronos-server listening");
    axum::serve(listener, build_router(state)).await
}

async fn healthz() -> &'static str {
    "ok"
}

#[derive(Debug, Deserialize)]
struct MemoryReq {
    subject: String,
    predicate: String,
    object: String,
    /// Real-world valid-from time, in epoch milliseconds.
    valid_from: i64,
    doc: u64,
    chunk: u64,
    /// "unique" (default) or "append".
    #[serde(default)]
    policy: Option<String>,
}

#[derive(Debug, Serialize)]
struct MemoryResp {
    edge_id: u64,
}

async fn add_memory(
    State(state): State<AppState>,
    Json(req): Json<MemoryReq>,
) -> Result<Json<MemoryResp>, ApiError> {
    let policy = match req.policy.as_deref() {
        Some("append") => ConflictPolicy::AppendOnly,
        _ => ConflictPolicy::UniqueSubjectPredicate,
    };
    let edge = state.store.ingest(
        &req.subject,
        &req.predicate,
        &req.object,
        Timestamp::from_millis(req.valid_from),
        DocId::new(req.doc),
        ChunkId::new(req.chunk),
        policy,
    )?;
    Ok(Json(MemoryResp {
        edge_id: edge.raw(),
    }))
}

#[derive(Debug, Deserialize)]
struct SearchReq {
    query: String,
}

#[derive(Debug, Serialize)]
struct CitationJson {
    doc: u64,
    chunk: u64,
    snippet: Option<String>,
}

#[derive(Debug, Serialize)]
struct SearchResp {
    text: String,
    citations: Vec<CitationJson>,
}

async fn search(
    State(state): State<AppState>,
    Json(req): Json<SearchReq>,
) -> Result<Json<SearchResp>, ApiError> {
    let retriever = MemoryRetriever::new(&state.store);
    let block = retriever.answer(&req.query)?;
    Ok(Json(SearchResp {
        text: block.text,
        citations: block
            .citations
            .into_iter()
            .map(|c| CitationJson {
                doc: c.source.doc.raw(),
                chunk: c.source.chunk.raw(),
                snippet: c.snippet,
            })
            .collect(),
    }))
}

#[derive(Debug, Serialize)]
struct CommunityJson {
    id: u64,
    members: Vec<String>,
    summary: String,
}

#[derive(Debug, Serialize)]
struct CommunitiesResp {
    communities: Vec<CommunityJson>,
}

async fn communities(State(state): State<AppState>) -> Result<Json<CommunitiesResp>, ApiError> {
    let comms = state.store.community_summaries()?;
    Ok(Json(CommunitiesResp {
        communities: comms
            .into_iter()
            .map(|c| CommunityJson {
                id: c.id,
                members: c.members,
                summary: c.summary,
            })
            .collect(),
    }))
}

/// Maps engine errors to HTTP responses.
pub struct ApiError(chronos_common::Error);

impl From<chronos_common::Error> for ApiError {
    fn from(e: chronos_common::Error) -> Self {
        ApiError(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        use chronos_common::Error::*;
        let status = match self.0 {
            NotFound(_) => StatusCode::NOT_FOUND,
            Query(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = Json(serde_json::json!({ "error": self.0.to_string() }));
        (status, body).into_response()
    }
}
