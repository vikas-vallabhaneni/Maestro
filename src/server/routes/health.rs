use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::db;
use crate::server::error::AppError;
use crate::server::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub library_path: String,
    pub track_count: i64,
}

pub async fn handler(State(state): State<AppState>) -> Result<Json<HealthResponse>, AppError> {
    let track_count = db::tracks::count(&state.db).await?;
    Ok(Json(HealthResponse {
        ok: true,
        library_path: state.library_root.display().to_string(),
        track_count,
    }))
}
