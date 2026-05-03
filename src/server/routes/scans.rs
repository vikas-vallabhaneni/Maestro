use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

use crate::db;
use crate::server::error::AppError;
use crate::server::AppState;

#[derive(Serialize)]
pub struct CreateScanResponse {
    pub id: i64,
    pub already_running: bool,
}

#[derive(Serialize)]
pub struct ScanRunResponse {
    pub id: i64,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub files_seen: i64,
    pub files_new: i64,
    pub files_moved: i64,
    pub error_message: Option<String>,
}

pub async fn create(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<CreateScanResponse>), AppError> {
    let mut guard = state.current_scan.lock().await;

    if let Some(running_id) = *guard {
        if !db::scans::is_finished(&state.db, running_id).await? {
            return Ok((
                StatusCode::OK,
                Json(CreateScanResponse {
                    id: running_id,
                    already_running: true,
                }),
            ));
        }
    }

    let scan_id = db::scans::create(&state.db).await?;
    *guard = Some(scan_id);
    drop(guard);

    let pool = state.db.clone();
    let root = state.library_root.clone();
    let cancel = state.shutdown_token.clone();
    let current_scan = state.current_scan.clone();

    tokio::spawn(async move {
        let result = crate::scan::scan_dir(&root, &pool, scan_id, cancel).await;
        if let Err(err) = &result {
            tracing::error!(scan_id, error = %err, "scan failed");
            let msg = format!("{err:#}");
            let _ = db::scans::finish(&pool, scan_id, 0, 0, 0, Some(&msg)).await;
        }
        let mut guard = current_scan.lock().await;
        if *guard == Some(scan_id) {
            *guard = None;
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(CreateScanResponse {
            id: scan_id,
            already_running: false,
        }),
    ))
}

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ScanRunResponse>, AppError> {
    let row = db::scans::get(&state.db, id)
        .await?
        .ok_or_else(|| AppError::not_found("scan run not found"))?;

    Ok(Json(ScanRunResponse {
        id: row.id,
        started_at: row.started_at,
        finished_at: row.finished_at,
        files_seen: row.files_seen,
        files_new: row.files_new,
        files_moved: row.files_moved,
        error_message: row.error_message,
    }))
}
