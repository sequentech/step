// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use b4::{app, db, s3, state::AppState};

/// The listen address.
const DEFAULT_BIND: &str = "127.0.0.1:3005";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "b4=info,b4v6=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = db::open_from_env().await?;
    let s3_client = s3::init_s3_client().await;
    let state = AppState::from_env(db, s3_client);
    tracing::info!("S3 bucket {:?}", state.bucket_name,);

    let app = app::router(state);

    let bind = std::env::var("WBRAID_B4_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_string());
    tracing::info!("Binding to {bind}");

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!(
        "Bulletin board service listening on {}",
        listener.local_addr()?
    );

    axum::serve(listener, app).await?;

    Ok(())
}
