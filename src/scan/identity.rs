use sqlx::SqlitePool;
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub enum Classification {
    New { id: Vec<u8> },
}

pub fn hash_file(path: &Path) -> anyhow::Result<Vec<u8>> {
    let bytes = std::fs::read(path)?;
    let hash = blake3::hash(&bytes);
    Ok(hash.as_bytes()[..16].to_vec())
}

pub async fn classify(pool: &SqlitePool, id: &[u8]) -> Result<Classification, sqlx::Error> {
    let existing: Option<(Vec<u8>,)> = sqlx::query_as("SELECT id FROM tracks WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    match existing {
        // #5 will add Unchanged/Changed/Moved branches here
        None | Some(_) => Ok(Classification::New { id: id.to_vec() }),
    }
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

    #[tokio::test]
    async fn new_file_classifies_as_new() {
        let pool = test_pool().await;
        let id = blake3::hash(b"some audio bytes").as_bytes()[..16].to_vec();

        let result = classify(&pool, &id).await.expect("classify should succeed");
        assert_eq!(result, Classification::New { id: id.clone() });
    }
}
