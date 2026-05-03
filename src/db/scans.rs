use sqlx::SqlitePool;

pub async fn create(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("INSERT INTO scan_runs DEFAULT VALUES RETURNING id")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

pub async fn finish(
    pool: &SqlitePool,
    id: i64,
    files_seen: i64,
    files_new: i64,
    error_message: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE scan_runs
         SET finished_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
             files_seen = ?2,
             files_new = ?3,
             error_message = ?4
         WHERE id = ?1",
    )
    .bind(id)
    .bind(files_seen)
    .bind(files_new)
    .bind(error_message)
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
