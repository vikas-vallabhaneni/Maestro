use crate::db;
use sqlx::SqlitePool;
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub enum Classification {
    Unchanged { id: Vec<u8> },
    Changed { id: Vec<u8>, rehashed: bool },
    Moved { id: Vec<u8> },
    New { id: Vec<u8> },
}

pub fn hash_file(path: &Path) -> anyhow::Result<Vec<u8>> {
    let bytes = std::fs::read(path)?;
    let hash = blake3::hash(&bytes);
    Ok(hash.as_bytes()[..16].to_vec())
}

pub async fn classify(
    pool: &SqlitePool,
    path: &str,
    size: i64,
    mtime: i64,
) -> Result<Classification, anyhow::Error> {
    if let Some(stat) = db::tracks::find_stat_by_path(pool, path).await? {
        if stat.size == size && stat.mtime == mtime {
            return Ok(Classification::Unchanged { id: stat.id });
        }

        let id = hash_file(Path::new(path))?;
        if id == stat.id {
            db::tracks::update_stat(pool, &id, size, mtime).await?;
            return Ok(Classification::Changed {
                id,
                rehashed: false,
            });
        }

        return Ok(Classification::Changed { id, rehashed: true });
    }

    let id = hash_file(Path::new(path))?;

    if let Some(existing) = db::tracks::find_by_id(pool, &id).await? {
        if existing.path != path {
            return Ok(Classification::Moved { id });
        }
    }

    Ok(Classification::New { id })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::str::FromStr;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::from_str("sqlite::memory:").expect("valid connection string"),
            )
            .await
            .expect("open in-memory db");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("run migrations");
        pool
    }

    async fn insert_track(pool: &SqlitePool, id: &[u8], path: &str, size: i64, mtime: i64) {
        db::tracks::insert_or_update(
            pool,
            &db::tracks::TrackRow {
                id: id.to_vec(),
                path: path.to_string(),
                size,
                mtime,
                duration_ms: None,
                title: Some("Test".to_string()),
                artist: None,
                album: None,
                track_no: None,
            },
        )
        .await
        .expect("insert track");
    }

    #[tokio::test]
    async fn new_file_classifies_as_new() {
        let pool = test_pool().await;
        let tmp = tempfile::NamedTempFile::new().expect("create temp file");
        std::fs::write(tmp.path(), b"brand new audio").expect("write");
        let path_str = tmp.path().to_string_lossy().to_string();
        let result = classify(&pool, &path_str, 15, 123_456)
            .await
            .expect("classify should succeed");
        assert!(
            matches!(result, Classification::New { .. }),
            "expected New, got {result:?}"
        );
    }

    #[tokio::test]
    async fn unchanged_file_classifies_as_unchanged() {
        let pool = test_pool().await;
        let id = blake3::hash(b"audio data").as_bytes()[..16].to_vec();
        insert_track(&pool, &id, "/music/song.flac", 1000, 123_456).await;

        let result = classify(&pool, "/music/song.flac", 1000, 123_456)
            .await
            .expect("classify");
        assert!(
            matches!(result, Classification::Unchanged { .. }),
            "expected Unchanged, got {result:?}"
        );
    }

    #[tokio::test]
    async fn changed_mtime_same_hash_classifies_as_changed() {
        let pool = test_pool().await;

        let tmp = tempfile::NamedTempFile::new().expect("create temp file");
        std::fs::write(tmp.path(), b"stable audio data").expect("write");
        let id = hash_file(tmp.path()).expect("hash");
        let path_str = tmp.path().to_string_lossy().to_string();

        insert_track(&pool, &id, &path_str, 17, 100).await;

        let result = classify(&pool, &path_str, 17, 200).await.expect("classify");
        assert!(
            matches!(
                result,
                Classification::Changed {
                    rehashed: false,
                    ..
                }
            ),
            "expected Changed(rehashed=false), got {result:?}"
        );
    }

    #[tokio::test]
    async fn changed_mtime_different_hash_classifies_as_changed() {
        let pool = test_pool().await;

        let tmp = tempfile::NamedTempFile::new().expect("create temp file");
        std::fs::write(tmp.path(), b"new audio data").expect("write");
        let old_id = blake3::hash(b"old audio data").as_bytes()[..16].to_vec();
        let path_str = tmp.path().to_string_lossy().to_string();

        insert_track(&pool, &old_id, &path_str, 14, 100).await;

        let result = classify(&pool, &path_str, 14, 200).await.expect("classify");
        assert!(
            matches!(result, Classification::Changed { rehashed: true, .. }),
            "expected Changed(rehashed=true), got {result:?}"
        );
    }

    #[tokio::test]
    async fn moved_file_classifies_as_moved() {
        let pool = test_pool().await;

        let tmp = tempfile::NamedTempFile::new().expect("create temp file");
        std::fs::write(tmp.path(), b"movable audio").expect("write");
        let id = hash_file(tmp.path()).expect("hash");

        insert_track(&pool, &id, "/music/old_location.flac", 13, 100).await;

        let path_str = tmp.path().to_string_lossy().to_string();
        let result = classify(&pool, &path_str, 13, 100).await.expect("classify");
        assert!(
            matches!(result, Classification::Moved { .. }),
            "expected Moved, got {result:?}"
        );
    }
}
