pub mod error;
pub mod routes;

use axum::Router;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub library_root: PathBuf,
    pub current_scan: Arc<Mutex<Option<i64>>>,
    pub shutdown_token: CancellationToken,
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/health", axum::routing::get(routes::health::handler))
        .route("/api/tracks", axum::routing::get(routes::tracks::list))
        .route(
            "/api/tracks/{id}/stream",
            axum::routing::get(routes::tracks::stream),
        )
        .route("/api/scans", axum::routing::post(routes::scans::create))
        .route("/api/scans/{id}", axum::routing::get(routes::scans::get))
        .nest_service("/assets", ServeDir::new("web/assets"))
        .fallback_service(ServeDir::new("web"))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
