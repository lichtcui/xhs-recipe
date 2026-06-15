mod routes;
mod splitter;
mod error;

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Shared application state.
pub struct AppState {
    /// Limits concurrent /process requests.
    pub semaphore: Semaphore,
}

#[tokio::main]
async fn main() {
    // Parse --port argument or env PORT, default 3000
    let port = std::env::args()
        .position(|a| a == "--port")
        .and_then(|i| std::env::args().nth(i + 1))
        .or_else(|| std::env::var("PORT").ok())
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);

    let state = Arc::new(AppState {
        semaphore: Semaphore::new(3),
    });

    let app = routes::app(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("xhs-recipe-server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind to address");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("shutdown signal received, cleaning up...");
}
