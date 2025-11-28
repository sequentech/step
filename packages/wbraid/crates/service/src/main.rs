mod db;
mod handlers;
mod s3;
mod state;

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wbraid_service=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize database
    let db = db::init_db().await?;
    
    // Initialize S3 client
    let s3_client = s3::init_s3_client().await;
    
    let state = AppState::new(db, s3_client);

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/messages/initiate", post(handlers::initiate_message))
        .route("/messages/:id/confirm", post(handlers::confirm_message))
        .route("/messages", get(handlers::list_messages))
        .route("/messages/:id", get(handlers::get_message))
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    tracing::info!("Bulletin board service listening on {}", listener.local_addr()?);
    
    axum::serve(listener, app).await?;

    Ok(())
}
