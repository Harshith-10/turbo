mod api;
mod worker;
mod gc;

use std::net::SocketAddr;
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use turbo_core::config::TurboConfig;
use turbo_db::TurboDb;
use turbo_pkg::PackageCache;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "turbo_server=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Turbo Server...");

    let config = TurboConfig::new()?;
    tracing::info!("Config loaded");

    // Use paths from config (which can be overridden via turbo.toml or TURBO_PATHS_* env vars)
    let turbo_home = PathBuf::from(&config.paths.turbo_home);
    let repo_path = PathBuf::from(&config.paths.packages_path);
    let runtimes_dir = turbo_home.join("runtimes");

    tracing::info!("Turbo home: {:?}, Packages path: {:?}", turbo_home, repo_path);

    // Build in-memory package cache
    let packages = PackageCache::from_paths(repo_path, runtimes_dir.clone()).await?;
    tracing::info!("Package cache initialized");

    // Ensure sqlite file URI has proper permissions if applicable
    // This is a workaround to ensure sqlx can write to the file if it's new
    let mut db_url = config.database.url.clone();
    if db_url.starts_with("sqlite://") && !db_url.contains("mode=") {
         db_url = format!("{}?mode=rwc", db_url);
    }

    let db = TurboDb::new(&config.redis.url, &db_url).await?;
    tracing::info!("Combined DB/Queue connected");

    let workers = std::env::var("TURBO_WORKERS")
        .unwrap_or_else(|_| "10".to_string())
        .parse::<usize>()
        .unwrap_or(10);
    
    tracing::info!("Starting {} workers", workers);

    for i in 0..workers {
        let db_clone = db.clone();
        let runtimes_dir_clone = runtimes_dir.clone();
        tokio::spawn(async move {
            worker::start_worker(i, db_clone, runtimes_dir_clone).await;
        });
    }

    // Spawn Garbage Collector
    tokio::spawn(async {
        gc::start_gc().await;
    });

    let app = api::routes::app(db, packages);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

