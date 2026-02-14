use crate::api::routes::AppState;
use axum::{Json, extract::State, http::StatusCode};
use std::sync::Arc;
use turbo_core::models::{Job, JobRequest, JobResult, Runtime};
use uuid::Uuid;

pub async fn execute(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<JobRequest>,
) -> Result<Json<JobResult>, (StatusCode, String)> {
    let job_id = Uuid::new_v4().to_string();
    let job = Job {
        id: job_id.clone(),
        request: payload,
    };

    state.db.queue.push_job(job).await.map_err(|e| {
        tracing::error!("Failed to queue job: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Queue error: {}", e),
        )
    })?;

    let result = state.db.queue.wait_for_result(&job_id).await.map_err(|e| {
        tracing::error!("Failed to wait for result: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Execution timeout or error: {}", e),
        )
    })?;

    Ok(Json(result))
}

pub async fn get_runtimes(State(state): State<Arc<AppState>>) -> Json<Vec<Runtime>> {
    match state.db.metadata.get_runtimes().await {
        Ok(runtimes) => Json(runtimes),
        Err(e) => {
            tracing::error!("Failed to get runtimes: {}", e);
            Json(vec![])
        }
    }
}

pub async fn health() -> StatusCode {
    StatusCode::OK
}

