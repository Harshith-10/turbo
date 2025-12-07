use axum::{
    extract::{State, Json},
    routing::post,
    Router,
};
use bollard::Docker;
use tokio::net::UdpSocket;
use tokio::time::{self, Duration};
use tracing::{info, error};
use tracing_subscriber;
use uuid::Uuid;
use turbo_common::{Message, Job, JobResult};

mod executor;

#[derive(Clone)]
struct WorkerState {
    docker: Docker,
    worker_id: Uuid,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let worker_id = Uuid::new_v4();
    let leader_addr = "127.0.0.1:8081";
    
    // Connect to Docker
    let docker = Docker::connect_with_local_defaults().expect("Failed to connect to Docker");
    let state = WorkerState { docker, worker_id };

    // Setup HTTP Server for Jobs
    let app = Router::new()
        .route("/execute", post(execute_job_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    let worker_port = local_addr.port();
    
    info!("Worker {} listening on port {}", worker_id, worker_port);

    // Spawn HTTP Server
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Heartbeat Loop
    let socket = UdpSocket::bind("0.0.0.0:0").await.expect("Failed to bind UDP socket");
    socket.connect(leader_addr).await.expect("Failed to connect to leader");

    let heartbeat_msg = Message::Heartbeat { worker_id, port: worker_port };
    let msg_bytes = serde_json::to_vec(&heartbeat_msg).expect("Failed to serialize heartbeat");

    let mut interval = time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;
        match socket.send(&msg_bytes).await {
            Ok(_) => info!("Sent heartbeat to {}", leader_addr),
            Err(e) => error!("Failed to send heartbeat: {}", e),
        }
    }
}

async fn execute_job_handler(
    State(state): State<WorkerState>,
    Json(job): Json<Job>,
) -> Json<JobResult> {
    info!("Received job: {}", job.id);
    let result = executor::execute_job(&state.docker, &job, state.worker_id).await;
    info!("Completed job: {}", job.id);
    Json(result)
}
