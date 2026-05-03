pub mod identity;
pub mod tags;
pub mod walk;

use crate::db;
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::Path;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct ScanReport {
    pub files_seen: i64,
    pub files_new: i64,
    pub files_moved: i64,
}

pub async fn auto_scan(
    root: &Path,
    pool: &SqlitePool,
    cancel: CancellationToken,
) -> anyhow::Result<ScanReport> {
    let scan_id = db::scans::create(pool).await?;
    match scan_dir(root, pool, scan_id, cancel).await {
        Ok(report) => Ok(report),
        Err(err) => {
            let msg = format!("{err:#}");
            let _ = db::scans::finish(pool, scan_id, 0, 0, 0, Some(&msg)).await;
            Err(err)
        }
    }
}

struct ScanAccum {
    files_new: i64,
    files_moved: i64,
    seen_ids: HashSet<Vec<u8>>,
}

async fn process_file(pool: &SqlitePool, path: &Path, accum: &mut ScanAccum) -> anyhow::Result<()> {
    let path_str = path.to_string_lossy().to_string();
    let meta = std::fs::metadata(path)?;
    let mtime = meta
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let size = i64::try_from(meta.len())?;
    let mtime_i64 = i64::try_from(mtime)?;

    let classification = identity::classify(pool, &path_str, size, mtime_i64).await?;

    match classification {
        identity::Classification::Unchanged => {
            if let Some(stat) = db::tracks::find_stat_by_path(pool, &path_str).await? {
                accum.seen_ids.insert(stat.id);
            }
        }
        identity::Classification::Changed { id, rehashed } => {
            if rehashed {
                upsert_track(pool, id.clone(), path, &path_str, size, mtime_i64).await?;
                accum.files_new += 1;
            }
            accum.seen_ids.insert(id);
        }
        identity::Classification::Moved { id } => {
            db::tracks::update_path(pool, &id, &path_str).await?;
            db::tracks::update_stat(pool, &id, size, mtime_i64).await?;
            accum.seen_ids.insert(id);
            accum.files_moved += 1;
        }
        identity::Classification::New { id } => {
            upsert_track(pool, id.clone(), path, &path_str, size, mtime_i64).await?;
            accum.seen_ids.insert(id);
            accum.files_new += 1;
        }
    }
    Ok(())
}

async fn upsert_track(
    pool: &SqlitePool,
    id: Vec<u8>,
    path: &Path,
    path_str: &str,
    size: i64,
    mtime: i64,
) -> anyhow::Result<()> {
    let snapshot = tags::read_tags(path);
    let duration_ms = if snapshot.duration.is_zero() {
        None
    } else {
        snapshot.duration.as_millis().try_into().ok()
    };
    let track = db::tracks::TrackRow {
        id,
        path: path_str.to_string(),
        size,
        mtime,
        duration_ms,
        title: snapshot.title,
        artist: snapshot.artist,
        album: snapshot.album,
        track_no: snapshot.track_no.map(i64::from),
    };
    db::tracks::insert_or_update(pool, &track).await?;
    Ok(())
}

pub async fn scan_dir(
    root: &Path,
    pool: &SqlitePool,
    scan_id: i64,
    cancel: CancellationToken,
) -> anyhow::Result<ScanReport> {
    anyhow::ensure!(
        root.is_dir(),
        "library root does not exist or is not a directory: {}",
        root.display()
    );
    let files = walk::audio_files(root);
    let files_seen = i64::try_from(files.len())?;
    let mut accum = ScanAccum {
        files_new: 0,
        files_moved: 0,
        seen_ids: HashSet::new(),
    };

    for (idx, path) in files.iter().enumerate() {
        if cancel.is_cancelled() {
            let seen = i64::try_from(idx)?;
            db::scans::finish(
                pool,
                scan_id,
                seen,
                accum.files_new,
                accum.files_moved,
                Some("interrupted by shutdown"),
            )
            .await?;
            anyhow::bail!("interrupted by shutdown");
        }

        process_file(pool, path, &mut accum).await?;

        if (idx + 1) % 50 == 0 {
            let _ = db::scans::update_files_seen(pool, scan_id, i64::try_from(idx + 1)?).await;
        }
    }

    prune(pool, files_seen, &accum.seen_ids).await?;

    db::scans::finish(
        pool,
        scan_id,
        files_seen,
        accum.files_new,
        accum.files_moved,
        None,
    )
    .await?;

    Ok(ScanReport {
        files_seen,
        files_new: accum.files_new,
        files_moved: accum.files_moved,
    })
}

async fn prune(
    pool: &SqlitePool,
    files_seen: i64,
    seen_ids: &HashSet<Vec<u8>>,
) -> anyhow::Result<()> {
    let all_ids = db::tracks::all_ids(pool).await?;
    if files_seen == 0 && !all_ids.is_empty() {
        tracing::warn!("scan found no files; refusing to prune existing rows");
        return Ok(());
    }
    let to_delete: Vec<Vec<u8>> = all_ids
        .into_iter()
        .filter(|id| !seen_ids.contains(id))
        .collect();
    if !to_delete.is_empty() {
        let deleted = db::tracks::delete_ids(pool, &to_delete).await?;
        tracing::info!(pruned = deleted, "pruned vanished tracks");
    }
    Ok(())
}
