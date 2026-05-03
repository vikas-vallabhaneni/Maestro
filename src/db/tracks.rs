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

pub async fn find_path_by_id(pool: &SqlitePool, id: &[u8]) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as("SELECT path FROM tracks WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.0))
}
