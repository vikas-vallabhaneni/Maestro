use sqlx::SqlitePool;

#[derive(Debug, sqlx::FromRow)]
pub struct TrackRow {
    pub id: Vec<u8>,
    pub path: String,
    pub size: i64,
    pub mtime: i64,
    pub duration_ms: Option<i64>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track_no: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct StatRow {
    pub id: Vec<u8>,
    pub path: String,
    pub size: i64,
    pub mtime: i64,
}

pub async fn count(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tracks")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

pub async fn insert_or_update(pool: &SqlitePool, track: &TrackRow) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO tracks (id, path, size, mtime, duration_ms, title, artist, album, track_no)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(id) DO UPDATE SET
           path = excluded.path,
           size = excluded.size,
           mtime = excluded.mtime,
           duration_ms = excluded.duration_ms,
           title = excluded.title,
           artist = excluded.artist,
           album = excluded.album,
           track_no = excluded.track_no",
    )
    .bind(&track.id)
    .bind(&track.path)
    .bind(track.size)
    .bind(track.mtime)
    .bind(track.duration_ms)
    .bind(&track.title)
    .bind(&track.artist)
    .bind(&track.album)
    .bind(track.track_no)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<TrackRow>, sqlx::Error> {
    sqlx::query_as::<_, TrackRow>(
        "SELECT id, path, size, mtime, duration_ms, title, artist, album, track_no FROM tracks
         ORDER BY artist, album, track_no, title",
    )
    .fetch_all(pool)
    .await
}

pub async fn find_stat_by_path(
    pool: &SqlitePool,
    path: &str,
) -> Result<Option<StatRow>, sqlx::Error> {
    sqlx::query_as::<_, StatRow>("SELECT id, path, size, mtime FROM tracks WHERE path = ?1")
        .bind(path)
        .fetch_optional(pool)
        .await
}

pub async fn find_by_id(pool: &SqlitePool, id: &[u8]) -> Result<Option<StatRow>, sqlx::Error> {
    sqlx::query_as::<_, StatRow>("SELECT id, path, size, mtime FROM tracks WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn update_path(pool: &SqlitePool, id: &[u8], new_path: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE tracks SET path = ?2 WHERE id = ?1")
        .bind(id)
        .bind(new_path)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_stat(
    pool: &SqlitePool,
    id: &[u8],
    size: i64,
    mtime: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE tracks SET size = ?2, mtime = ?3 WHERE id = ?1")
        .bind(id)
        .bind(size)
        .bind(mtime)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn all_ids(pool: &SqlitePool) -> Result<Vec<Vec<u8>>, sqlx::Error> {
    let rows: Vec<(Vec<u8>,)> = sqlx::query_as("SELECT id FROM tracks")
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

pub async fn delete_ids(pool: &SqlitePool, ids: &[Vec<u8>]) -> Result<u64, sqlx::Error> {
    let mut deleted = 0u64;
    for chunk in ids.chunks(100) {
        let placeholders: Vec<String> = (1..=chunk.len()).map(|i| format!("?{i}")).collect();
        let sql = format!(
            "DELETE FROM tracks WHERE id IN ({})",
            placeholders.join(", ")
        );
        let mut query = sqlx::query(&sql);
        for id in chunk {
            query = query.bind(id);
        }
        let result = query.execute(pool).await?;
        deleted += result.rows_affected();
    }
    Ok(deleted)
}
