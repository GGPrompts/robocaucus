use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

mod adapter;
mod agentmd;
mod context;
mod db;
mod mention;
mod orchestrate;
#[allow(dead_code)]
mod reconcile;
mod routes;
mod scaffold;
mod state;
mod templates;
#[allow(dead_code)]
mod tmux;

use state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let db = db::Database::new("robocaucus.db").expect("failed to open database");

    // Seed starter agents and scaffold their config folders on first launch.
    let seeded = templates::seed_starter_agents(&db).expect("failed to seed agents");
    if seeded > 0 {
        tracing::info!("seeded {} starter agents", seeded);
        for agent in db.list_agents(None).expect("failed to list agents") {
            let agent_home = db::agent_home_dir(&agent.name);
            if let Err(e) = scaffold::scaffold_agent_folder(
                &agent.provider,
                &agent_home,
                &agent.system_prompt,
            ) {
                tracing::warn!("failed to scaffold folder for {}: {e}", agent.name);
            }
            // Update agent_home in the database so adapters can find the config.
            db.update_agent(
                &agent.id,
                &agent.name,
                &agent.model,
                &agent.provider,
                &agent_home,
                &agent.color,
                &agent.scope,
                &agent.system_prompt,
                agent.workspace_path.as_deref(),
            )
            .expect("failed to update agent_home");
        }
    }

    let seeded = templates::seed_starter_playbooks(&db).expect("failed to seed playbooks");
    if seeded > 0 {
        tracing::info!("seeded {} starter playbooks", seeded);
    }

    let state = AppState::new(db);

    let cors = CorsLayer::new()
        .allow_origin([
            "http://localhost:7330".parse().unwrap(),
        ])
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .nest("/api", routes::api_routes())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "7331".to_string());
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
