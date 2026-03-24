use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

mod adapter;
mod agentmd;
mod context;
mod db;
mod mention;
mod orchestrate;
mod reconcile;
mod routes;
mod state;
mod templates;
mod tmux;

use state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let db = db::Database::new("robocaucus.db").expect("failed to open database");
    let state = AppState::new(db);

    let cors = CorsLayer::new()
        .allow_origin([
            "http://localhost:5173".parse().unwrap(),
        ])
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .nest("/api", routes::api_routes())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3001".to_string());
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
