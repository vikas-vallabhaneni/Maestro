use maestro::server::{app, AppState};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub async fn spawn_test_server(
    library_root: PathBuf,
) -> (SocketAddr, SqlitePool, CancellationToken) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::from_str("sqlite::memory:").expect("valid connection string"),
        )
        .await
        .expect("failed to open in-memory database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    let shutdown_token = CancellationToken::new();

    let state = AppState {
        db: pool.clone(),
        library_root,
        current_scan: Arc::new(Mutex::new(None)),
        shutdown_token: shutdown_token.clone(),
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind");
    let addr = listener.local_addr().expect("failed to get local addr");

    tokio::spawn(async move {
        axum::serve(listener, app(state)).await.ok();
    });

    (addr, pool, shutdown_token)
}
