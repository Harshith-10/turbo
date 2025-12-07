use axum::{
    extract::{State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use turbo_common::{Job, JobResult, JobData, JobResultData, TestCase, TestCaseResult};
use crate::state::AppState;
use uuid::Uuid;
use tracing::{info, error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct UserRequest {
    pub user_id: String,
    pub language: String,
    pub code: String,
    pub testcases: Vec<TestCase>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub passed: bool,
    pub results: Vec<TestCaseResult>,
    pub error: Option<String>,
}

async fn dispatch_job(state: &AppState, job: Job) -> Result<JobResult, String> {
    let workers_len = state.workers.len();
    if workers_len == 0 {
        return Err("No workers available".to_string());
    }
    let index = state.rr_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % workers_len;
    let worker_info = {
        state.workers.iter().nth(index).map(|w| w.value().clone())
    };

    if let Some(worker_info) = worker_info {
        let worker_url = format!("http://{}:{}/execute", worker_info.address.ip(), worker_info.http_port);
        info!("Sending job to worker: {}", worker_url);
        match state.http_client.post(&worker_url).json(&job).send().await {
            Ok(resp) => {
                info!("Received response from worker: {}", worker_url);
                resp.json::<JobResult>().await.map_err(|e| e.to_string())
            },
            Err(e) => {
                error!("Failed to send job to worker {}: {}", worker_url, e);
                Err(e.to_string())
            },
        }
    } else {
        Err("Worker selection failed".to_string())
    }
}

pub async fn submit_request(
    State(state): State<AppState>,
    Json(req): Json<UserRequest>,
) -> impl IntoResponse {
    info!("Received request from user: {}", req.user_id);

    let mut binary = None;

    // Compilation
    if req.language == "java" || req.language == "rust" {
        let compile_job = Job {
            id: Uuid::new_v4(),
            data: JobData::Compile {
                language: req.language.clone(),
                code: req.code.clone(),
            },
            timeout_seconds: 60,
        };

        match dispatch_job(&state, compile_job).await {
            Ok(result) => {
                match result.data {
                    JobResultData::Compile { success, message, binary: bin } => {
                        if !success {
                            return (StatusCode::OK, Json(UserResponse {
                                passed: false,
                                results: vec![],
                                error: Some(format!("Compilation failed: {}", message)),
                            })).into_response();
                        }
                        binary = bin;
                    },
                    JobResultData::Error(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Compilation worker error: {}", e))).into_response(),
                    _ => return (StatusCode::INTERNAL_SERVER_ERROR, Json("Unexpected worker response")).into_response(),
                }
            },
            Err(e) => return (StatusCode::BAD_GATEWAY, Json(format!("Compilation worker failed: {}", e))).into_response(),
        }
    }

    // Execution
    let mut all_results = Vec::new();
    let chunks = req.testcases.chunks(5);
    let mut futures = Vec::new();

    for chunk in chunks {
        let job = Job {
            id: Uuid::new_v4(),
            data: JobData::Execute {
                language: req.language.clone(),
                binary: binary.clone(),
                code: if binary.is_none() { Some(req.code.clone()) } else { None },
                testcases: chunk.to_vec(),
            },
            timeout_seconds: 60,
        };
        futures.push(dispatch_job(&state, job));
    }

    let results = futures_util::future::join_all(futures).await;

    for res in results {
        match res {
            Ok(job_res) => {
                match job_res.data {
                    JobResultData::Execute { results } => all_results.extend(results),
                    JobResultData::Error(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(format!("Execution error: {}", e))).into_response(),
                    _ => return (StatusCode::INTERNAL_SERVER_ERROR, Json("Unexpected worker response")).into_response(),
                }
            },
            Err(e) => return (StatusCode::BAD_GATEWAY, Json(format!("Execution worker failed: {}", e))).into_response(),
        }
    }

    (StatusCode::OK, Json(UserResponse {
        passed: all_results.iter().all(|r| r.passed),
        results: all_results,
        error: None,
    })).into_response()
}
