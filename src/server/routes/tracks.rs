use std::path::Path;

use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::{header, Request};
use axum::response::Response;
use axum::Json;
use serde::Serialize;
use tower::ServiceExt;
use tower_http::services::ServeFile;

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

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
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

pub async fn stream(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
    request: Request<Body>,
) -> Result<Response, AppError> {
    let id_bytes = hex_decode(&id).ok_or(AppError::NotFound)?;

    let path_str = db::tracks::find_path_by_id(&state.db, &id_bytes)
        .await?
        .ok_or(AppError::NotFound)?;

    let file_path = Path::new(&path_str);

    let canonical = file_path.canonicalize().map_err(|e| {
        tracing::error!(path = %path_str, error = %e, "failed to canonicalize track path");
        AppError::NotFound
    })?;

    if !canonical.starts_with(&state.library_root) {
        tracing::error!(
            path = %path_str,
            library_root = %state.library_root.display(),
            "path traversal attempt blocked"
        );
        return Err(AppError::NotFound);
    }

    let is_opus = canonical.extension().and_then(|e| e.to_str()) == Some("opus");

    let mut response = ServeFile::new(&canonical)
        .oneshot(request)
        .await?
        .map(Body::new);

    if is_opus {
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, "audio/ogg".parse().unwrap());
    }

    Ok(response)
}
