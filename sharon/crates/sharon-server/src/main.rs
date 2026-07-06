//! Sharon — optionaler Koordinations-Server für LuxiFer.
//! REST API + WebSocket-Events. Keine Maschinensteuerung.

use axum::{routing::get, Json, Router};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/projects", get(list_projects));

    let addr = "0.0.0.0:7878";
    tracing::info!("Sharon lauscht auf {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "ok"
}

async fn list_projects() -> Json<Vec<sharon_core::ProjectMeta>> {
    // TODO: an ProjectStore anbinden.
    Json(vec![])
}
