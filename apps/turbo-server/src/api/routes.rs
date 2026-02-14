use crate::api::handlers;
use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;
use turbo_db::TurboDb;
pub struct AppState {
    pub db: TurboDb,
}

pub fn app(db: TurboDb) -> Router {
    let state = Arc::new(AppState { db });

    Router::new()
        .route("/api/v1/execute", post(handlers::execute))
        .route("/api/v1/runtimes", get(handlers::get_runtimes))
        .route("/health", get(handlers::health))
        .with_state(state)
}
