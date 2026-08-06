mod config;
mod kvmd;
mod model;
mod state;

use anyhow::Result;
use axum::{
    body::Bytes,
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{any, get, post}, Json, Router,
};
use config::Config;
use model::*;
use serde::Deserialize;
use state::AppState;
use std::{sync::atomic::Ordering, time::Duration};
use sysinfo::System;
use tokio::time::timeout;
use tower_http::{services::{ServeDir, ServeFile}, trace::TraceLayer};
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Deserialize)]
struct SessionQuery { client_id: String }

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "ipkvm_gateway=info,tower_http=info".into())
    ).init();
    let cfg = Config::load()?;
    let kvmd = kvmd::KvmdClient::new(cfg.kvmd.clone())?;
    let state = AppState::new(cfg.clone(), kvmd);
    let index = cfg.static_dir.join("index.html");
    let app = Router::new()
        .route("/api/status", get(status))
        .route("/api/metrics", get(metrics))
        .route("/api/session/acquire", post(acquire))
        .route("/api/session/release", post(release))
        .route("/ws/control", get(ws_upgrade))
        .route("/media/{*path}", any(media_proxy))
        .fallback_service(ServeDir::new(&cfg.static_dir).not_found_service(ServeFile::new(index)))
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    info!(listen = %cfg.listen, "IP-KVM gateway starting");
    let listener = tokio::net::TcpListener::bind(cfg.listen).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn authorized(s: &AppState, headers: &HeaderMap) -> bool {
    let cookie = headers.get(axum::http::header::COOKIE).and_then(|v| v.to_str().ok()).unwrap_or("");
    s.kvmd.authorize_cookie(cookie).await
}

async fn status(State(s): State<AppState>, headers: HeaderMap) -> Result<Json<StatusResponse>, StatusCode> {
    if !authorized(&s, &headers).await { return Err(StatusCode::UNAUTHORIZED); }
    let kvmd = s.kvmd.health().await;
    let live777 = reqwest::get(&s.cfg.live777.health_url).await.map(|r| r.status().is_success()).unwrap_or(false);
    Ok(Json(StatusResponse {
        kvmd, hid: kvmd, live777,
        controller: s.controller.lock().await.clone(),
        viewers: s.connections.load(Ordering::Relaxed),
        width: s.cfg.video.width, height: s.cfg.video.height, fps: s.cfg.video.fps,
        whep_url: s.cfg.live777.whep_url.clone(),
    }))
}

async fn metrics(State(s): State<AppState>, headers: HeaderMap) -> Result<Json<MetricsResponse>, StatusCode> {
    if !authorized(&s, &headers).await { return Err(StatusCode::UNAUTHORIZED); }
    let mut sys = System::new_all();
    sys.refresh_all();
    Ok(Json(MetricsResponse {
        cpu_percent: sys.global_cpu_usage(),
        memory_used_bytes: sys.used_memory(),
        memory_total_bytes: sys.total_memory(),
        websocket_connections: s.connections.load(Ordering::Relaxed),
        forwarded_events: s.forwarded.load(Ordering::Relaxed),
        rejected_events: s.rejected.load(Ordering::Relaxed),
        last_kvmd_error: s.last_error.lock().await.clone(),
    }))
}

async fn acquire(State(s): State<AppState>, headers: HeaderMap, Query(q): Query<SessionQuery>) -> Response {
    if !authorized(&s, &headers).await { return StatusCode::UNAUTHORIZED.into_response(); }
    if q.client_id.is_empty() { return (StatusCode::BAD_REQUEST, "client_id required").into_response(); }
    let mut owner = s.controller.lock().await;
    if owner.as_ref().is_some_and(|v| v != &q.client_id) {
        return (StatusCode::CONFLICT, "another client is controlling").into_response();
    }
    *owner = Some(q.client_id);
    StatusCode::NO_CONTENT.into_response()
}

async fn release(State(s): State<AppState>, headers: HeaderMap, Query(q): Query<SessionQuery>) -> Response {
    if !authorized(&s, &headers).await { return StatusCode::UNAUTHORIZED.into_response(); }
    let mut owner = s.controller.lock().await;
    if owner.as_deref() == Some(q.client_id.as_str()) {
        *owner = None;
        drop(owner);
        let _ = s.kvmd.release_all().await;
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn ws_upgrade(ws: WebSocketUpgrade, State(s): State<AppState>, headers: HeaderMap, Query(q): Query<SessionQuery>) -> Response {
    if !authorized(&s, &headers).await { return StatusCode::UNAUTHORIZED.into_response(); }
    let id = if q.client_id.is_empty() { Uuid::new_v4().to_string() } else { q.client_id };
    ws.on_upgrade(move |socket| control_socket(socket, s, id)).into_response()
}

async fn media_proxy(
    State(s): State<AppState>,
    Path(path): Path<String>,
    method: axum::http::Method,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !authorized(&s, &headers).await { return StatusCode::UNAUTHORIZED.into_response(); }
    let target = format!("{}/{}", s.cfg.live777.base_url.trim_end_matches('/'), path);
    let client = reqwest::Client::new();
    let mut request = client.request(method, target).body(body.to_vec());
    for name in [axum::http::header::CONTENT_TYPE, axum::http::header::ACCEPT] {
        if let Some(value) = headers.get(&name) { request = request.header(name, value); }
    }
    match request.send().await {
        Ok(upstream) => {
            let status = upstream.status();
            let content_type = upstream.headers().get(reqwest::header::CONTENT_TYPE).cloned();
            let location = upstream.headers().get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .map(|v| format!("/ipkvm/media/{}", v.trim_start_matches('/')));
            let bytes = upstream.bytes().await.unwrap_or_default();
            let mut response = axum::http::Response::builder().status(status);
            if let Some(value) = content_type { response = response.header(axum::http::header::CONTENT_TYPE, value); }
            if let Some(value) = location { response = response.header(axum::http::header::LOCATION, value); }
            response.body(axum::body::Body::from(bytes)).unwrap().into_response()
        }
        Err(error) => (StatusCode::BAD_GATEWAY, format!("Live777 proxy error: {error}")).into_response(),
    }
}

async fn control_socket(mut socket: WebSocket, s: AppState, id: String) {
    s.connected();
    s.sessions.lock().await.insert(id.clone(), 0);
    while let Some(Ok(message)) = socket.recv().await {
        let Message::Text(text) = message else { continue };
        let parsed: ControlMessage = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => { let _ = send_reply(&mut socket, None, false, &format!("invalid message: {e}")).await; continue; }
        };
        if matches!(parsed.kind, ControlKind::Ping) {
            let _ = send_reply(&mut socket, Some(parsed.seq), true, "pong").await;
            continue;
        }
        let is_owner = s.controller.lock().await.as_deref() == Some(id.as_str());
        if !is_owner {
            s.rejected.fetch_add(1, Ordering::Relaxed);
            let _ = send_reply(&mut socket, Some(parsed.seq), false, "read-only session").await;
            continue;
        }
        let mut sessions = s.sessions.lock().await;
        let last = sessions.entry(id.clone()).or_default();
        if parsed.seq <= *last && matches!(parsed.kind, ControlKind::MouseMoveAbs | ControlKind::MouseMoveRel) {
            s.rejected.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        *last = (*last).max(parsed.seq);
        drop(sessions);
        match timeout(Duration::from_secs(2), s.kvmd.dispatch(&parsed)).await {
            Ok(Ok(())) => {
                s.forwarded.fetch_add(1, Ordering::Relaxed);
                let _ = send_reply(&mut socket, Some(parsed.seq), true, "forwarded").await;
            }
            result => {
                let error = format!("KVMD dispatch failed: {result:?}");
                *s.last_error.lock().await = Some(error.clone());
                warn!(%error);
                let _ = send_reply(&mut socket, Some(parsed.seq), false, &error).await;
            }
        }
    }
    s.sessions.lock().await.remove(&id);
    let mut owner = s.controller.lock().await;
    if owner.as_deref() == Some(id.as_str()) {
        *owner = None;
        drop(owner);
        let _ = s.kvmd.release_all().await;
    }
    s.disconnected();
}

async fn send_reply(socket: &mut WebSocket, seq: Option<u64>, ok: bool, message: &str) -> Result<(), axum::Error> {
    let body = serde_json::to_string(&ServerMessage { kind: if ok { "ack" } else { "error" }, seq, ok, message }).unwrap();
    socket.send(Message::Text(body.into())).await
}
