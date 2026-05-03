use clap::Parser;
use maestro::server::AppState;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

#[derive(clap::Parser, Debug)]
#[command(name = "maestro", about = "Personal music server")]
struct Config {
    #[arg(long, env = "MAESTRO_LIBRARY")]
    library: std::path::PathBuf,

    #[arg(long, default_value = "127.0.0.1")]
    host: std::net::IpAddr,

    #[arg(long, default_value_t = 4173)]
    port: u16,

    #[arg(long)]
    data_dir: Option<std::path::PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("maestro=info")),
        )
        .init();

    let cfg = Config::parse();

    if !cfg.library.exists() {
        anyhow::bail!("--library {}: path does not exist", cfg.library.display());
    }
    if !cfg.library.is_dir() {
        anyhow::bail!(
            "--library {}: path is not a directory",
            cfg.library.display()
        );
    }
    let library_root = cfg.library.canonicalize()?;

    let data_dir = cfg.data_dir.unwrap_or_else(|| {
        directories::ProjectDirs::from("", "", "maestro")
            .expect("unable to determine platform data directory")
            .data_dir()
            .to_path_buf()
    });
    std::fs::create_dir_all(&data_dir)?;
    let db_path = data_dir.join("maestro.db");

    tracing::info!(db = %db_path.display(), "opening database");
    let pool = maestro::db::open_pool(&db_path).await?;
    maestro::db::run_migrations(&pool).await?;

    let shutdown_token = CancellationToken::new();

    let state = AppState {
        db: pool,
        library_root,
        current_scan: Arc::new(Mutex::new(None)),
        shutdown_token: shutdown_token.clone(),
    };

    let addr = SocketAddr::from((cfg.host, cfg.port));

    if !cfg.host.is_loopback() {
        tracing::warn!(
            host = %cfg.host,
            "binding to a non-loopback address — there is no authentication; \
             anyone on the network can read your music"
        );
    }

    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "listening");

    let scan_pool = state.db.clone();
    let scan_root = state.library_root.clone();
    let scan_cancel = shutdown_token.clone();
    let scan_handle = tokio::spawn(async move {
        match maestro::scan::auto_scan(&scan_root, &scan_pool, scan_cancel).await {
            Ok(report) => tracing::info!(
                files_seen = report.files_seen,
                files_new = report.files_new,
                files_moved = report.files_moved,
                "auto-scan complete"
            ),
            Err(err) => tracing::error!(error = %err, "auto-scan failed"),
        }
    });

    axum::serve(listener, maestro::server::app(state))
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            shutdown_token.cancel();
        })
        .await?;

    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), scan_handle).await;

    tracing::info!("shut down cleanly");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
