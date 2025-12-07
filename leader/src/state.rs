use dashmap::DashMap;
use std::sync::Arc;
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct WorkerInfo {
    pub id: Uuid,
    pub address: std::net::SocketAddr, // UDP source address
    pub http_port: u16,                // HTTP listening port
    pub last_seen: SystemTime,
}

use std::sync::atomic::AtomicUsize;

#[derive(Clone)]
pub struct AppState {
    pub workers: Arc<DashMap<Uuid, WorkerInfo>>,
    pub rr_counter: Arc<AtomicUsize>,
    pub http_client: reqwest::Client,
}

impl AppState {
    pub fn new() -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .pool_max_idle_per_host(10)
            .build()
            .expect("Failed to create HTTP client");
            
        Self {
            workers: Arc::new(DashMap::new()),
            rr_counter: Arc::new(AtomicUsize::new(0)),
            http_client,
        }
    }
}
