use crate::api::handlers;
use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;
use turbo_db::TurboDb;
use turbo_pkg::PackageCache;

pub struct AppState {
    pub db: TurboDb,
    pub packages: PackageCache,
}

pub fn app(db: TurboDb, packages: PackageCache) -> Router {
    let state = Arc::new(AppState { db, packages });

    Router::new()
        .route("/api/v1/execute", post(handlers::execute))
        .route("/api/v1/runtimes", get(handlers::get_runtimes))
        .route("/api/v1/packages", get(handlers::get_packages))
        .with_state(state)
}
