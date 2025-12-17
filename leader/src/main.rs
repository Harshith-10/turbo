use axum::{
    routing::post,
    Router,
};
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::time::{self, Duration};
use tracing::{info, error};
use tracing_subscriber;

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

    // Spawn Discovery Broadcaster
    tokio::spawn(async move {
        let socket = UdpSocket::bind("0.0.0.0:0").await.expect("Failed to bind broadcast socket");
        socket.set_broadcast(true).expect("Failed to set broadcast");
        
        let discovery_msg = Message::Discovery { port: 8081 };
        let msg_bytes = serde_json::to_vec(&discovery_msg).expect("Failed to serialize discovery message");
        let broadcast_addr = "255.255.255.255:8082";
        let local_addr = "127.0.0.1:8082";

        let mut interval = time::interval(Duration::from_secs(3));
        loop {
            interval.tick().await;
            // Try Global Broadcast
            if let Err(e) = socket.send_to(&msg_bytes, broadcast_addr).await {
                error!("Failed to send global discovery broadcast: {}", e);
            }
            // Try Localhost (for local cluster simulation)
            if let Err(e) = socket.send_to(&msg_bytes, local_addr).await {
                 // Ignore errors on localhost if it's not reachable (e.g. true production env)
                 // But log debug if needed.
                 let _ = e;
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
