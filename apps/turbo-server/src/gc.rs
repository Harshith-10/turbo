use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;
use tracing::{error, info};

const CACHE_DIR: &str = "/tmp/turbo-cache";
const MAX_CACHE_ENTRIES: usize = 500;
const GC_INTERVAL: u64 = 300; // 5 minutes

pub async fn start_gc() {
    info!("Garbage Collector started. Max entries: {}, Interval: {}s", MAX_CACHE_ENTRIES, GC_INTERVAL);
    let cache_path = PathBuf::from(CACHE_DIR);

    // Create cache dir if it doesn't exist, to avoid errors
    if !cache_path.exists() {
        let _ = fs::create_dir_all(&cache_path).await;
    }

    loop {
        tokio::time::sleep(Duration::from_secs(GC_INTERVAL)).await;
        if let Err(e) = run_gc_pass(&cache_path).await {
            error!("GC Pass failed: {}", e);
        }
    }
}

async fn run_gc_pass(path: &PathBuf) -> std::io::Result<()> {
    let mut entries = fs::read_dir(path).await?;
    let mut cache_items = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        if let Ok(metadata) = entry.metadata().await {
            if metadata.is_dir() {
                // Use modified time of the directory itself
                if let Ok(modified) = metadata.modified() {
                    cache_items.push((entry.path(), modified));
                }
            }
        }
    }

    if cache_items.len() <= MAX_CACHE_ENTRIES {
        return Ok(());
    }

    // Sort by modified time (oldest first)
    cache_items.sort_by(|a, b| a.1.cmp(&b.1));

    let to_remove = cache_items.len() - MAX_CACHE_ENTRIES;
    info!("GC: Cleaning up {} items", to_remove);

    for (dir_path, _) in cache_items.iter().take(to_remove) {
        if let Err(e) = fs::remove_dir_all(dir_path).await {
            error!("Failed to remove cache entry {:?}: {}", dir_path, e);
        }
    }

    Ok(())
}
