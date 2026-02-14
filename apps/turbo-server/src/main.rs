mod api;
mod gc;
mod worker;

use std::net::SocketAddr;
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use turbo_core::config::TurboConfig;
use turbo_db::TurboDb;

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
    let runtimes_dir = turbo_home.join("runtimes");

    tracing::info!("Turbo home: {:?}", turbo_home);

    let db = TurboDb::new(&config.redis.url).await?;
    tracing::info!("Combined DB/Queue connected");

    // Populate runtimes
    match populate_runtimes(&db, &runtimes_dir).await {
        Ok(_) => tracing::info!("Runtimes populated"),
        Err(e) => tracing::error!("Failed to populate runtimes: {}", e),
    }

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

    let app = api::routes::app(db);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn populate_runtimes(db: &TurboDb, runtimes_dir: &PathBuf) -> anyhow::Result<()> {
    use tokio::fs;
    use turbo_core::models::Runtime;
    use turbo_pkg::models::PackageDefinition;

    if !runtimes_dir.exists() {
        tracing::warn!("Runtimes directory not found: {:?}", runtimes_dir);
        return Ok(());
    }

    let mut entries = fs::read_dir(runtimes_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let lang_path = entry.path();
        if lang_path.is_dir() {
            let lang = entry.file_name().to_string_lossy().to_string();
            let mut ver_entries = fs::read_dir(&lang_path).await?;
            while let Some(ver_entry) = ver_entries.next_entry().await? {
                let ver_path = ver_entry.path();
                if ver_path.is_dir() {
                    let version = ver_entry.file_name().to_string_lossy().to_string();

                    // Note: PackageDefinition::from_path uses std::fs (blocking)
                    match PackageDefinition::from_path(ver_path.clone()) {
                        Ok(pkg_def) => {
                            let runtime = Runtime {
                                language: lang.clone(),
                                version: version.clone(),
                                aliases: pkg_def.yaml.aliases.clone().unwrap_or_default(),
                                runtime: None,
                            };
                            if let Err(e) = db.metadata.add_runtime(&runtime).await {
                                tracing::error!("Failed to add runtime to Redis: {}", e);
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Skipping invalid runtime at {:?}: {}", ver_path, e);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
