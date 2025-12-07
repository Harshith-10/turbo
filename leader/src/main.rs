use axum::{
    routing::post,
    Router,
};
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tracing::{info, error};
use tracing_subscriber;
use std::sync::Arc;

mod state;
mod api;

use state::{AppState, WorkerInfo};
use turbo_common::Message;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = AppState::new();
    let state_clone = state.clone();

    // Spawn UDP Heartbeat Listener
    tokio::spawn(async move {
        let socket = UdpSocket::bind("0.0.0.0:8081").await.expect("Failed to bind UDP socket");
        info!("Heartbeat listener running on 0.0.0.0:8081");
        
        let mut buf = [0u8; 1024];
        loop {
            match socket.recv_from(&mut buf).await {
                Ok((size, addr)) => {
                    if let Ok(msg) = serde_json::from_slice::<Message>(&buf[..size]) {
                        match msg {
                            Message::Heartbeat { worker_id, port } => {
                                info!("Received heartbeat from {} on port {}", worker_id, port);
                                state_clone.workers.insert(worker_id, WorkerInfo {
                                    id: worker_id,
                                    address: addr,
                                    http_port: port,
                                    last_seen: std::time::SystemTime::now(),
                                });
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => error!("UDP receive error: {}", e),
            }
        }
    });

    // HTTP API
    let app = Router::new()
        .route("/submit", post(api::submit_request))
        .route("/health", axum::routing::get(|| async { "OK" }))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    info!("API server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
