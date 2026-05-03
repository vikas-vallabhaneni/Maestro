use sqlx::SqlitePool;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct ScanRunRow {
    pub id: i64,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub files_seen: i64,
    pub files_new: i64,
    pub files_moved: i64,
    pub error_message: Option<String>,
}

pub async fn create(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("INSERT INTO scan_runs DEFAULT VALUES RETURNING id")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

pub async fn get(pool: &SqlitePool, id: i64) -> Result<Option<ScanRunRow>, sqlx::Error> {
    sqlx::query_as::<_, ScanRunRow>(
        "SELECT id, started_at, finished_at, files_seen, files_new, files_moved, error_message
         FROM scan_runs WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn finish(
    pool: &SqlitePool,
    id: i64,
    files_seen: i64,
    files_new: i64,
    files_moved: i64,
    error_message: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE scan_runs
         SET finished_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
             files_seen = ?2,
             files_new = ?3,
             files_moved = ?4,
             error_message = ?5
         WHERE id = ?1",
    )
    .bind(id)
    .bind(files_seen)
    .bind(files_new)
    .bind(files_moved)
    .bind(error_message)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_files_seen(
    pool: &SqlitePool,
    id: i64,
    files_seen: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE scan_runs SET files_seen = ?2 WHERE id = ?1")
        .bind(id)
        .bind(files_seen)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn is_finished(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM scan_runs WHERE id = ?1 AND finished_at IS NOT NULL")
            .bind(id)
            .fetch_one(pool)
            .await?;
    Ok(row.0 > 0)
}
