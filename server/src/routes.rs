use axum::extract::State;
use axum::http::Method;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tower_http::cors::{Any, CorsLayer};
use xhs_recipe::textifier;

use crate::AppState;

pub fn app(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any)
        .allow_headers(Any);

    Router::new()
        .route("/process", post(process_handler))
        .route("/health", get(health_handler))
        .layer(cors)
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct ProcessRequest {
    url: String,
    #[serde(default = "default_asr_model")]
    asr_model: String,
    #[serde(default = "default_ocr")]
    ocr_images: bool,
}

fn default_asr_model() -> String {
    "qwen3-asr-0.6b".to_string()
}

fn default_ocr() -> bool {
    true
}

async fn process_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ProcessRequest>,
) -> impl IntoResponse {
    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(32);
    let stream = ReceiverStream::new(rx);

    let state = state.clone();
    tokio::spawn(async move {
        // Acquire semaphore permit — limits concurrent requests
        let permit = match state.semaphore.acquire().await {
            Ok(p) => p,
            Err(_) => {
                let err = serde_json::json!({
                    "code": "INTERNAL_ERROR",
                    "message": "semaphore closed",
                });
                let _ = tx.send(Ok(Event::default().event("error").data(err.to_string()))).await;
                return;
            }
        };

        // Run process with 5-minute timeout
        let result = tokio::time::timeout(
            Duration::from_secs(300),
            run_process(req, tx.clone()),
        )
        .await;

        // Drop permit to release concurrency slot
        drop(permit);

        match result {
            Ok(Ok(())) => {} // success
            Ok(Err(e)) => {
                let err_json = serde_json::json!({
                    "code": "INTERNAL_ERROR",
                    "message": e.to_string(),
                });
                let _ = tx.send(Ok(Event::default().event("error").data(err_json.to_string()))).await;
            }
            Err(_) => {
                // Timeout
                let err_json = serde_json::json!({
                    "code": "TIMEOUT",
                    "message": "处理超时（超过 5 分钟）",
                });
                let _ = tx.send(Ok(Event::default().event("error").data(err_json.to_string()))).await;
            }
        }
    });

    Sse::new(stream)
        .keep_alive(KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"))
}

async fn run_process(
    req: ProcessRequest,
    tx: mpsc::Sender<Result<Event, Infallible>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::splitter;
    use xhs_recipe::sources;

    // Validate URL before heavy processing
    if !sources::supports_url(&req.url) {
        let err_json = serde_json::json!({
            "code": "INVALID_URL",
            "message": format!("unsupported URL: {}", req.url),
        });
        if tx.send(Ok(Event::default().event("error").data(err_json.to_string()))).await.is_err() {
            return Ok(()); // client disconnected
        }
        return Ok(());
    }

    // Progress callback — skips sending if channel is closed (client disconnected)
    let progress: Arc<dyn Fn(&str) + Send + Sync> = {
        let tx = tx.clone();
        Arc::new(move |stage: &str| {
            if tx.is_closed() {
                return;
            }
            let tx = tx.clone();
            let stage = stage.to_string();
            tokio::spawn(async move {
                let json = serde_json::json!({"stage": stage});
                let _ = tx.send(Ok(Event::default()
                    .event("progress")
                    .data(json.to_string()))).await;
            });
        })
    };

    // 1. Fetch
    (progress)("fetching");
    let raw = sources::fetch(&req.url).await
        .map_err(|e| format!("FETCH_FAILED: {}", e))?;

    // 2. Textify (includes download + OCR + ASR with internal progress events)
    let on_progress = Some(progress);
    let text = textifier::process(&raw, &req.asr_model, req.ocr_images, on_progress)
        .await
        .map_err(|e| format!("PROCESSING_FAILED: {}", e))?;

    // 3. Split
    let items = splitter::split(&raw, &text);

    // 4. Send result
    let result = serde_json::json!({
        "title": text.title,
        "items": items,
    });
    let _ = tx.send(Ok(Event::default()
        .event("result")
        .data(result.to_string()))).await;

    Ok(())
}

async fn health_handler() -> impl IntoResponse {
    let deps = serde_json::json!({
        "ffmpeg": xhs_recipe::which("ffmpeg").is_some(),
        "swiftc": xhs_recipe::which("swiftc").is_some(),
        "qwen_asr": xhs_recipe::which("qwen-asr").is_some(),
        "qwen_asr_model": check_asr_model(),
    });

    (axum::http::StatusCode::OK, Json(serde_json::json!({
        "status": "ok",
        "deps": deps,
    })))
}

fn check_asr_model() -> bool {
    xhs_recipe::home_dir()
        .join(".cache")
        .join("qwen-asr")
        .join("qwen3-asr-0.6b")
        .exists()
}

