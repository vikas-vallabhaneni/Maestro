use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::db;
use crate::server::error::AppError;
use crate::server::AppState;

#[derive(Serialize)]
pub struct TrackResponse {
    pub id: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track_no: Option<i64>,
    pub duration_ms: Option<i64>,
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<TrackResponse>>, AppError> {
    let rows = db::tracks::list(&state.db).await?;
    let tracks = rows
        .into_iter()
        .map(|row| TrackResponse {
            id: hex_encode(&row.id),
            title: row.title,
            artist: row.artist,
            album: row.album,
            track_no: row.track_no,
            duration_ms: row.duration_ms,
        })
        .collect();
    Ok(Json(tracks))
}
