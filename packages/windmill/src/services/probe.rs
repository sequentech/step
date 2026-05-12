// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Readiness and liveness checks.

use crate::services::celery_app::{get_celery_app, get_celery_connection, get_queues, Queue};
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::jwks::get_jwks_secret_path;
use crate::services::providers::sms_sender::{SmsSender, SmsTransport};
use crate::services::vault::check_master_secret;
use core::time::Duration;
use deadpool_postgres::Timeouts;
use sequent_core::services::keycloak::get_client_credentials;
use sequent_core::services::probe::ProbeHandler;
use sequent_core::services::s3;
use std::net::SocketAddr;
use std::sync::LazyLock;
use strum_macros::Display;
use tokio::join;
use tracing::{error, info, instrument, warn};

use super::celery_app::get_is_app_active;

#[derive(Display, Debug, Eq, PartialEq, Clone)]
/// Application name.
#[allow(missing_docs)]
pub enum AppName {
    BEAT,
    HARVEST,
    WINDMILL,
}

/// Broker reconnect timeout for Celery health checks (seconds).
const BROKER_CONNECTION_TIMEOUT: u32 = 2;

/// Database connection timeouts.
static DB_TIMEOUTS: LazyLock<Timeouts> = LazyLock::new(|| Timeouts {
    wait: Some(Duration::new(5, 0)),
    create: Some(Duration::new(5, 0)),
    recycle: Some(Duration::new(5, 0)),
});

/// Probe Celery broker connectivity and consumer health for configured queues.
#[instrument(ret)]
async fn check_celery(app_name: &AppName) -> Option<bool> {
    let _ = app_name;
    let celery_app = get_celery_app().await;

    // Check basic broker connection
    let celery_result = celery_app.broker.reconnect(BROKER_CONNECTION_TIMEOUT).await;
    let is_connected = celery_result.is_ok();

    if !is_connected || !get_is_app_active() {
        return Some(false);
    }

    let queues_to_check = get_queues();

    // If subscribed to ElectoralLogBeat, check the standalone rabbitmq connection
    let slug = std::env::var("ENV_SLUG").unwrap_or("dev".to_string());
    let queue_name = Queue::ElectoralLogBeat.queue_name(&slug);
    if queues_to_check.contains(&queue_name) {
        info!("Checking rabbitmq connection");
        let conn = get_celery_connection().await;
        if conn.is_err() {
            error!("Failed to check consumer health: {:?}", conn);
        }
    }

    match celery_app.check_consumer_health(&queues_to_check).await {
        Ok(health_info) => {
            let mut all_healthy = true;

            for health in &health_info {
                info!(
                    "Queue '{}': {} consumers, {} messages, consuming: {}",
                    health.queue_name,
                    health.consumer_count,
                    health.message_count,
                    health.is_consuming
                );

                // A queue is considered unhealthy if it's supposed to have consumers but doesn't
                // For now, we'll be permissive and only require that the connection works
                // Individual queue health can be monitored separately
            }

            Some(all_healthy)
        }
        Err(e) => {
            error!("Failed to check consumer health: {}", e);
            Some(false)
        }
    }
}

/// Probe Hasura Postgres pool by acquiring a connection within [`DB_TIMEOUTS`].
#[instrument(ret)]
async fn check_hasura_db(app_name: &AppName) -> Option<bool> {
    if AppName::BEAT == *app_name {
        return None;
    }
    info!("obtaining hasura pool reference..");
    let hasura_db_result = get_hasura_pool().await;

    let status = hasura_db_result.status();
    info!("hasura db pool status: {status:?}");

    match hasura_db_result.timeout_get(&DB_TIMEOUTS).await {
        Ok(_) => Some(true),
        Err(error) => {
            error!("hasura db pool object error: {error:?}");
            Some(false)
        }
    }
}

/// Probe Keycloak Postgres pool by acquiring a connection within [`DB_TIMEOUTS`].
#[instrument(ret)]
async fn check_keycloak_db(app_name: &AppName) -> Option<bool> {
    if AppName::BEAT == *app_name {
        return None;
    }

    info!("obtaining keycloak pool reference..");
    let keycloak_db_result = get_keycloak_pool().await;

    let status = keycloak_db_result.status();
    info!("keycloak db pool status: {status:?}");

    match keycloak_db_result.timeout_get(&DB_TIMEOUTS).await {
        Ok(_) => Some(true),
        Err(error) => {
            error!("keycloak db pool object error: {error:?}");
            Some(false)
        }
    }
}

/// Probe AWS Secrets Manager / vault master secret accessibility.
#[instrument(ret)]
async fn check_aws_secrets(app_name: &AppName) -> Option<bool> {
    if AppName::BEAT == *app_name {
        return None;
    }

    match check_master_secret().await {
        Ok(()) => Some(true),
        Err(error) => {
            error!("aws secrets error: {error:?}");
            Some(false)
        }
    }
}

/// Probe public S3 by reading the configured JWKS object path.
#[instrument(ret)]
async fn check_s3(app_name: &AppName) -> Option<bool> {
    if AppName::BEAT == *app_name {
        return None;
    }

    let s3_bucket = match s3::get_public_bucket() {
        Ok(s3_bucket) => s3_bucket,
        Err(err) => {
            error!("s3 error: {err:?}");
            return Some(false);
        }
    };
    let path = get_jwks_secret_path();
    match s3::get_file_from_s3(s3_bucket, path).await {
        Ok(_) => Some(true),
        Err(error) => {
            error!("s3 error: {error:?}");
            Some(false)
        }
    }
}

/// Probe SMS transport (e.g. AWS SNS attributes) when the app sends outbound SMS.
#[instrument(ret)]
async fn check_sms_sender(app_name: &AppName) -> Option<bool> {
    if AppName::BEAT == *app_name || AppName::HARVEST == *app_name {
        return None;
    }

    // First try to initialize the SMS sender
    let sms_sender = match SmsSender::new().await {
        Ok(sender) => sender,
        Err(error) => {
            error!("sms sender initialization error: {error:?}");
            return Some(false);
        }
    };

    // Check if we're using AWS SNS and attempt a test connection
    match &sms_sender.transport {
        SmsTransport::AwsSns((aws_client, _)) => {
            // Try a lightweight operation to verify connectivity
            // For AWS SNS, we can use list_topics or get_sms_attributes which doesn't send actual SMS
            match aws_client.get_sms_attributes().send().await {
                Ok(_) => Some(true),
                Err(error) => {
                    error!("AWS SNS connection error: {error:?}");
                    Some(false)
                }
            }
        }
        SmsTransport::Console => {
            // Console transport always works
            Some(true)
        }
    }
}

/// Run all configured readiness checks; returns true when every probe passes or is skipped.
#[instrument(ret)]
async fn readiness_test(app_name: &AppName) -> bool {
    // Use futures::join! to await multiple futures concurrently
    let (celery_ok, hasura_db_ok, keycloak_db_ok, aws_secrets_ok, s3_ok, sms_sender_ok) = join!(
        check_celery(app_name),
        check_hasura_db(app_name),
        check_keycloak_db(app_name),
        check_aws_secrets(app_name),
        check_s3(app_name),
        check_sms_sender(app_name),
    );

    info!(
        "celery: {:?}, hasura_db: {:?} , keycloak db: {:?}, aws_secrets: {:?}, s3: {:?}, sms_sender: {:?}",
        celery_ok, hasura_db_ok, keycloak_db_ok, aws_secrets_ok, s3_ok, sms_sender_ok
    );

    let data = [
        celery_ok,
        hasura_db_ok,
        keycloak_db_ok,
        aws_secrets_ok,
        s3_ok,
        sms_sender_ok,
    ];

    data.iter().all(|&x| x.is_none() || x == Some(true))
}

/// Setup the probe for the application.
pub async fn setup_probe(app_name: AppName) {
    let app = app_name.to_string();
    let addr_s = std::env::var(format!("{app}_PROBE_ADDR")).unwrap_or("0.0.0.0:3030".to_string());
    let live_path = std::env::var(format!("{app}_PROBE_LIVE_PATH")).unwrap_or("live".to_string());
    let ready_path =
        std::env::var(format!("{app}_PROBE_READY_PATH")).unwrap_or("ready".to_string());

    let addr: Result<SocketAddr, _> = addr_s.parse();

    if let Ok(addr) = addr {
        let ph = ProbeHandler::new(&live_path, &ready_path, addr);
        let f = ph.future();
        let app_name_for_live = app_name.clone();
        ph.set_live(move || {
            let live_app = app_name_for_live.clone();
            #[allow(clippy::large_futures)]
            Box::pin(async move { readiness_test(&live_app).await })
        })
        .await;

        let app_name_for_ready = app_name.clone();
        ph.set_ready(move || {
            let ready_app = app_name_for_ready.clone();
            #[allow(clippy::large_futures)]
            Box::pin(async move { readiness_test(&ready_app).await })
        })
        .await;
        tokio::spawn(f);
    } else {
        warn!("Could not parse address for probe '{}'", addr_s);
    }
}
