pub mod identity;
pub mod tags;
pub mod walk;

use crate::db;
use sqlx::SqlitePool;
use std::path::Path;

#[derive(Debug)]
pub struct ScanReport {
    pub files_seen: i64,
    pub files_new: i64,
}

pub async fn auto_scan(root: &Path, pool: &SqlitePool) -> anyhow::Result<ScanReport> {
    let scan_id = db::scans::create(pool).await?;
    match scan_dir(root, pool, scan_id).await {
        Ok(report) => Ok(report),
        Err(err) => {
            let msg = format!("{err:#}");
            let _ = db::scans::finish(pool, scan_id, 0, 0, Some(&msg)).await;
            Err(err)
        }
    }
}

pub async fn scan_dir(root: &Path, pool: &SqlitePool, scan_id: i64) -> anyhow::Result<ScanReport> {
    let files = walk::audio_files(root);
    let files_seen = i64::try_from(files.len())?;
    let mut files_new: i64 = 0;

    for path in &files {
        let id = identity::hash_file(path)?;
        let classification = identity::classify(pool, &id).await?;

        match classification {
            identity::Classification::New { id } => {
                let meta = std::fs::metadata(path)?;
                let mtime = meta
                    .modified()?
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs();
                let size = i64::try_from(meta.len())?;

                let snapshot = tags::read_tags(path)?;

                let track = db::tracks::TrackRow {
                    id,
                    path: path.to_string_lossy().to_string(),
                    size,
                    mtime: i64::try_from(mtime)?,
                    duration_ms: snapshot.duration.as_millis().try_into().ok(),
                    title: snapshot.title,
                    artist: snapshot.artist,
                    album: snapshot.album,
                    track_no: snapshot.track_no.map(i64::from),
                };

                db::tracks::insert_or_update(pool, &track).await?;
                files_new += 1;
            }
        }
    }

    db::scans::finish(pool, scan_id, files_seen, files_new, None).await?;

    Ok(ScanReport {
        files_seen,
        files_new,
    })
}
