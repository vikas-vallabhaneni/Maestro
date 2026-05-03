pub mod error;
pub mod routes;

use axum::Router;
use sqlx::SqlitePool;
use std::path::PathBuf;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub library_root: PathBuf,
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/health", axum::routing::get(routes::health::handler))
        .route("/api/tracks", axum::routing::get(routes::tracks::list))
        .nest_service("/assets", ServeDir::new("web/assets"))
        .fallback_service(ServeDir::new("web"))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
