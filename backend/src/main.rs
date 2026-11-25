//! QIIIN AI Hybrid Server — API gateway.
//! Minimal, static aarch64-musl build. Binds to localhost by default.
//! Inference is delegated to a local llama-server; no raw model access is exposed.

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_BODY: usize = 1024 * 1024; // 1 MiB request cap

#[derive(Clone)]
struct AppState {
    client: reqwest::Client,
    token: String,
    llama_base: String,
    status_file: String,
    rate: Arc<Mutex<HashMap<String, (u64, u32)>>>,
    counter: Arc<AtomicU64>,
}

#[derive(Serialize)]
struct Health {
    ok: bool,
    service: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
struct ApiError {
    error: String,
    request_id: String,
}

#[derive(Serialize)]
struct SystemStatus {
    ok: bool,
    source: String,
    data: serde_json::Value,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn request_id(state: &AppState) -> String {
    let n = state.counter.fetch_add(1, Ordering::Relaxed);
    format!("{:x}-{:x}", now_secs(), n)
}

fn api_error(status: StatusCode, message: &str, id: &str) -> Response {
    (
        status,
        Json(ApiError {
            error: message.to_string(),
            request_id: id.to_string(),
        }),
    )
        .into_response()
}

/// Authentication + minimal per-token rate limit (60 req / 60 s).
fn authorize(state: &AppState, req: &Request) -> Result<(), (StatusCode, String)> {
    if state.token.is_empty() {
        return Err((StatusCode::SERVICE_UNAVAILABLE, "auth token not configured".into()));
    }
    let supplied = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if supplied != format!("Bearer {}", state.token) {
        return Err((StatusCode::UNAUTHORIZED, "invalid or missing bearer token".into()));
    }

    let mut table = state.rate.lock().unwrap();
    let (window, hits) = table.entry("default".to_string()).or_insert((now_secs(), 0));
    if now_secs().saturating_sub(*window) >= 60 {
        *window = now_secs();
        *hits = 0;
    }
    *hits += 1;
    if *hits > 60 {
        return Err((StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded".into()));
    }
    Ok(())
}

async fn health() -> Json<Health> {
    Json(Health {
        ok: true,
        service: "qiiin-ai-gateway",
        version: VERSION,
    })
}

async fn models(State(state): State<AppState>, req: Request) -> Response {
    let id = request_id(&state);
    if let Err((status, msg)) = authorize(&state, &req) {
        return api_error(status, &msg, &id);
    }
    let url = format!("{}/v1/models", state.llama_base);
    match state
        .client
        .get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await
    {
        Ok(upstream) => {
            let code = StatusCode::from_u16(upstream.status().as_u16())
                .unwrap_or(StatusCode::BAD_GATEWAY);
            match upstream.bytes().await {
                Ok(body) => (code, body).into_response(),
                Err(_) => api_error(StatusCode::BAD_GATEWAY, "backend read failed", &id),
            }
        }
        Err(_) => api_error(StatusCode::SERVICE_UNAVAILABLE, "inference backend unavailable", &id),
    }
}

async fn chat(State(state): State<AppState>, req: Request) -> Response {
    let id = request_id(&state);
    if let Err((status, msg)) = authorize(&state, &req) {
        return api_error(status, &msg, &id);
    }
    let body = match axum::body::to_bytes(req.into_body(), MAX_BODY).await {
        Ok(bytes) => bytes,
        Err(_) => return api_error(StatusCode::PAYLOAD_TOO_LARGE, "request body too large or unreadable", &id),
    };
    if body.len() > MAX_BODY {
        return api_error(StatusCode::PAYLOAD_TOO_LARGE, "request body too large", &id);
    }
    let url = format!("{}/v1/chat/completions", state.llama_base);
    match state
        .client
        .post(&url)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await
    {
        Ok(upstream) => {
            let code = StatusCode::from_u16(upstream.status().as_u16())
                .unwrap_or(StatusCode::BAD_GATEWAY);
            let stream = upstream.bytes_stream();
            Response::builder()
                .status(code)
                .header(header::CONTENT_TYPE, "text/event-stream")
                .header("x-request-id", id.as_str())
                .body(Body::from_stream(stream))
                .unwrap_or_else(|_| api_error(StatusCode::INTERNAL_SERVER_ERROR, "stream build failed", &id))
        }
        Err(_) => api_error(StatusCode::SERVICE_UNAVAILABLE, "inference backend unavailable", &id),
    }
}

async fn system_status(State(state): State<AppState>, req: Request) -> Response {
    let id = request_id(&state);
    if let Err((status, msg)) = authorize(&state, &req) {
        return api_error(status, &msg, &id);
    }
    match std::fs::read_to_string(&state.status_file) {
        Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(data) => Json(SystemStatus {
                ok: true,
                source: state.status_file.clone(),
                data,
            })
            .into_response(),
            Err(_) => api_error(StatusCode::BAD_GATEWAY, "status file is not valid JSON", &id),
        },
        Err(_) => Json(SystemStatus {
            ok: true,
            source: "unavailable".into(),
            data: serde_json::json!({}),
        })
        .into_response(),
    }
}

#[tokio::main]
async fn main() {
    let bind = std::env::var("QIIIN_BIND").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let token = std::env::var("QIIIN_AUTH_TOKEN").unwrap_or_default();
    let llama_base = std::env::var("QIIIN_LLAMA_BASE")
        .unwrap_or_else(|_| "http://127.0.0.1:8081".into());
    let status_file = std::env::var("QIIIN_STATUS_FILE")
        .unwrap_or_else(|_| "/var/run/qiiin-ai/system.json".into());

    if token.is_empty() {
        eprintln!("WARNING: QIIIN_AUTH_TOKEN is empty; /v1/* endpoints will return 503");
    }

    let state = AppState {
        client: reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("failed to build HTTP client"),
        token,
        llama_base,
        status_file,
        rate: Arc::new(Mutex::new(HashMap::new())),
        counter: Arc::new(AtomicU64::new(1)),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/models", get(models))
        .route("/v1/chat/completions", post(chat))
        .route("/v1/system/status", get(system_status))
        .with_state(state);

    let addr: std::net::SocketAddr = bind.parse().expect("invalid QIIIN_BIND address");
    eprintln!("qiiin-ai-gateway {VERSION} listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");
    axum::serve(listener, app).await.expect("server failed");
}
