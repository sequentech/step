// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Windmill/Harvest/Beat probe wiring.
//!
//! This module owns two things:
//!
//! 1. **Individual health-check functions** — each checks exactly one
//!    dependency and returns `bool`. They take no `AppName` parameter; the
//!    decision of which checks apply to which app is made at composition time
//!    in [`setup_probe`].
//!
//! 2. **Per-app composition** — [`setup_probe`] builds two [`HealthCheck`]
//!    trees using the [`All`] combinator from `sequent_core`:
//!    - *Liveness*: a cheap broker-ping common to all apps. Kubernetes kills
//!      and restarts the pod only when this fails, so it must be fast and
//!      resilient to transient dependency outages.
//!    - *Readiness*: all checks relevant to the app's role. Kubernetes removes
//!      the pod from the load-balancer rotation when this fails, giving
//!      dependencies time to recover without causing a restart.

use crate::services::celery_app::{get_celery_app, get_celery_connection, get_queues, Queue};
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::jwks::get_jwks_secret_path;
use crate::services::metrics::{QUEUE_CONSUMERS, QUEUE_MESSAGES};
use crate::services::providers::sms_sender::{SmsSender, SmsTransport};
use crate::services::vault::check_master_secret;
use core::time::Duration;
use deadpool_postgres::Timeouts;
use sequent_core::services::probe::{All, FnCheck, HealthCheck, ProbeServer, ProbeState};
use sequent_core::services::s3;
use std::net::SocketAddr;
use std::sync::{Arc, LazyLock};
use strum_macros::Display;
use tracing::{error, info, instrument, warn};

use super::celery_app::get_is_app_active;

// ── AppName ───────────────────────────────────────────────────────────────────

/// Identifies which application role is running.
///
/// Controls which readiness checks are composed in [`setup_probe`] and which
/// environment-variable prefix is used for probe configuration
/// (e.g. `WINDMILL_PROBE_ADDR`).
#[derive(Display, Debug, Eq, PartialEq, Clone)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum AppName {
    Beat,
    Harvest,
    Windmill,
}

// ── Static configuration ──────────────────────────────────────────────────────

const BROKER_CONNECTION_TIMEOUT: u32 = 2;

/// Environment variable that identifies the deployment slug (e.g. `"dev"`).
const ENV_SLUG_VAR: &str = "ENV_SLUG";
const DEFAULT_ENV_SLUG: &str = "dev";

/// Cached deployment slug — read once at first access to avoid a syscall on
/// every liveness probe.
static ENV_SLUG: LazyLock<String> =
    LazyLock::new(|| std::env::var(ENV_SLUG_VAR).unwrap_or_else(|_| DEFAULT_ENV_SLUG.to_string()));

/// Timeouts used when acquiring a connection from the DB pool during readiness
/// checks. Shared by both the Hasura and Keycloak pool checks.
static DB_TIMEOUTS: LazyLock<Timeouts> = LazyLock::new(|| Timeouts {
    wait: Some(Duration::from_secs(5)),
    create: Some(Duration::from_secs(5)),
    recycle: Some(Duration::from_secs(5)),
});

// ── ProbeConfig ───────────────────────────────────────────────────────────────

/// Environment-derived HTTP server configuration for the probe endpoint.
struct ProbeConfig {
    address: SocketAddr,
    live_path: String,
    ready_path: String,
}

impl ProbeConfig {
    /// Reads probe configuration from environment variables prefixed by the
    /// app name (e.g. `WINDMILL_PROBE_ADDR`, `WINDMILL_PROBE_LIVE_PATH`).
    ///
    /// Returns `None` and logs a warning if the address string cannot be
    /// parsed as a [`SocketAddr`].
    fn from_env(app_name: &AppName) -> Option<Self> {
        let prefix = app_name.to_string();
        let addr_s = std::env::var(format!("{prefix}_PROBE_ADDR"))
            .unwrap_or_else(|_| "0.0.0.0:3030".to_string());

        let address = match addr_s.parse() {
            Ok(a) => a,
            Err(_) => {
                warn!("Could not parse probe address '{addr_s}' for {prefix}");
                return None;
            }
        };

        Some(Self {
            address,
            live_path: std::env::var(format!("{prefix}_PROBE_LIVE_PATH"))
                .unwrap_or_else(|_| "live".to_string()),
            ready_path: std::env::var(format!("{prefix}_PROBE_READY_PATH"))
                .unwrap_or_else(|_| "ready".to_string()),
        })
    }
}

// ── Individual health checks ──────────────────────────────────────────────────

/// Checks raw AMQP connectivity via the shared [`get_celery_connection`] pool.
///
/// Unlike checks that go through [`get_celery_app`], this never triggers
/// `generate_celery_app` and therefore never requires S3 or plugin loading.
/// Safe to call from any app role (Beat, Harvest, Windmill) at any point in
/// the startup lifecycle.
#[instrument(ret)]
async fn check_broker_connection() -> bool {
    match get_celery_connection().await {
        Ok(conn) => conn.status().connected(),
        Err(err) => {
            error!("AMQP connection check failed: {err:?}");
            false
        }
    }
}

/// Lightweight broker connectivity check used exclusively for **liveness**.
///
/// Delegates to [`check_broker_connection`] — no celery app initialisation,
/// no S3, no plugin loading. Safe for all app roles.
#[instrument(ret)]
async fn check_broker_alive() -> bool {
    check_broker_connection().await
}

/// Full Celery consumer readiness check for **Windmill only**.
///
/// Windmill is the sole task consumer; Beat and Harvest send tasks but do not
/// consume them and must not call this function (they would trigger
/// `generate_celery_app` / plugin loading / S3 before their own startup has
/// had a chance to initialise those resources).
///
/// Order of checks is significant:
/// 1. Raw broker connection — via [`check_broker_connection`], no celery app.
/// 2. App-active guard — returns `false` until `set_is_app_active(true)` is
///    called from `main`, ensuring `CELERY_APP` is initialised before we call
///    `get_celery_app()` below.
/// 3. Consumer health + queue-depth metric sampling.
///
/// TODO: extract metric sampling into a dedicated periodic background task.
#[instrument(ret)]
async fn check_celery() -> bool {
    if !check_broker_connection().await {
        return false;
    }

    // Guard: only proceed once main() has called set_is_app_active(true),
    // which happens after CELERY_APP is initialised. This prevents a race
    // where the probe fires before generate_celery_app() has completed.
    if !get_is_app_active() {
        return false;
    }

    // Safe to call: get_is_app_active() guard above ensures CELERY_APP is
    // already initialised by Windmill's main() before we reach this point.
    let celery_app = get_celery_app().await;
    let queues_to_check = get_queues();

    // The ElectoralLogBeat queue uses a dedicated standalone RabbitMQ
    // connection. If this app subscribes to that queue, verify it separately.
    let beat_queue = Queue::ElectoralLogBeat.queue_name(&ENV_SLUG);
    if queues_to_check.contains(&beat_queue) {
        if let Err(err) = get_celery_connection().await {
            error!("Standalone RabbitMQ connection failed: {err:?}");
            return false;
        }
    }

    match celery_app.check_consumer_health(&queues_to_check).await {
        Ok(health_info) => {
            for health in &health_info {
                info!(
                    "Queue '{}': {} consumers, {} messages",
                    health.queue_name, health.consumer_count, health.message_count,
                );
                QUEUE_MESSAGES
                    .with_label_values(&[&health.queue_name])
                    .set(health.message_count as f64);
                QUEUE_CONSUMERS
                    .with_label_values(&[&health.queue_name])
                    .set(health.consumer_count as f64);
            }
            true
        }
        Err(err) => {
            error!("Consumer health check failed: {err}");
            false
        }
    }
}

#[instrument(ret)]
async fn check_hasura_db() -> bool {
    let pool = get_hasura_pool().await;
    info!("hasura db pool status: {:?}", pool.status());
    match pool.timeout_get(&DB_TIMEOUTS).await {
        Ok(_) => true,
        Err(err) => {
            error!("Hasura DB pool error: {err:?}");
            false
        }
    }
}

#[instrument(ret)]
async fn check_keycloak_db() -> bool {
    let pool = get_keycloak_pool().await;
    info!("keycloak db pool status: {:?}", pool.status());
    match pool.timeout_get(&DB_TIMEOUTS).await {
        Ok(_) => true,
        Err(err) => {
            error!("Keycloak DB pool error: {err:?}");
            false
        }
    }
}

#[instrument(ret)]
async fn check_aws_secrets() -> bool {
    match check_master_secret().await {
        Ok(_) => true,
        Err(err) => {
            error!("AWS secrets vault error: {err:?}");
            false
        }
    }
}

#[instrument(ret)]
async fn check_s3() -> bool {
    let s3_bucket = match s3::get_public_bucket() {
        Ok(bucket) => bucket,
        Err(err) => {
            error!("S3 bucket configuration error: {err:?}");
            return false;
        }
    };
    match s3::get_file_from_s3(s3_bucket, get_jwks_secret_path()).await {
        Ok(_) => true,
        Err(err) => {
            error!("S3 file fetch error: {err:?}");
            false
        }
    }
}

/// Checks SNS connectivity by initialising a [`SmsSender`] and issuing a
/// lightweight `get_sms_attributes` call that does not send any message.
///
/// NOTE: `SmsSender::new()` runs on every probe call. If credential loading
/// proves expensive in production, consider caching the sender in a
/// `tokio::sync::OnceCell`.
#[instrument(ret)]
async fn check_sms_sender() -> bool {
    let sms_sender = match SmsSender::new().await {
        Ok(sender) => sender,
        Err(err) => {
            error!("SMS sender initialisation error: {err:?}");
            return false;
        }
    };
    match &sms_sender.transport {
        SmsTransport::AwsSns((aws_client, _)) => {
            match aws_client.get_sms_attributes().send().await {
                Ok(_) => true,
                Err(err) => {
                    error!("AWS SNS connectivity error: {err:?}");
                    false
                }
            }
        }
        // Console transport is always available.
        SmsTransport::Console => true,
    }
}

// ── Probe setup ───────────────────────────────────────────────────────────────

/// Starts the probe HTTP server for the given application role.
///
/// - **Liveness** (all apps): cheap broker ping via [`check_broker_alive`].
///   Kubernetes kills and restarts the pod only when this fails.
/// - **Readiness** (per app): all checks relevant to the role, composed with
///   [`All`]. Kubernetes removes the pod from load-balancer rotation when
///   this fails. Checks run in declaration order and short-circuit on the
///   first failure.
///
/// Configuration is read from environment variables prefixed by the app name
/// (e.g. `WINDMILL_PROBE_ADDR`). If the address cannot be parsed the probe
/// server is not started and a warning is logged.
pub async fn setup_probe(app_name: AppName) {
    let Some(config) = ProbeConfig::from_env(&app_name) else {
        return;
    };

    let state = Arc::new(ProbeState::new());

    // All roles share the same cheap liveness check.
    state.set_live(FnCheck(|| check_broker_alive())).await;

    // Readiness is composed per role — only the checks relevant to each
    // role are included. Adding a new check for a role means adding one
    // line here; the check function itself stays independent.
    // Only Windmill is a task consumer and uses CELERY_APP / plugin loading.
    // Beat and Harvest send tasks but never initialise CELERY_APP, so their
    // broker check must use check_broker_connection() directly — calling
    // check_celery() would trigger generate_celery_app() and panic if any
    // dependency (e.g. S3) is absent from that role's environment.
    let readiness = match app_name {
        AppName::Beat => All(vec![
            Arc::new(FnCheck(|| check_broker_connection())) as Arc<dyn HealthCheck>
        ]),
        AppName::Harvest => All(vec![
            Arc::new(FnCheck(|| check_celery())) as Arc<dyn HealthCheck>,
            Arc::new(FnCheck(|| check_hasura_db())),
            Arc::new(FnCheck(|| check_keycloak_db())),
            Arc::new(FnCheck(|| check_aws_secrets())),
            Arc::new(FnCheck(|| check_s3())),
        ]),
        AppName::Windmill => All(vec![
            Arc::new(FnCheck(|| check_celery())) as Arc<dyn HealthCheck>,
            Arc::new(FnCheck(|| check_hasura_db())),
            Arc::new(FnCheck(|| check_keycloak_db())),
            Arc::new(FnCheck(|| check_aws_secrets())),
            Arc::new(FnCheck(|| check_s3())),
            Arc::new(FnCheck(|| check_sms_sender())),
        ]),
    };
    state.set_ready(readiness).await;

    let server = ProbeServer::builder()
        .address(config.address)
        .live_path(config.live_path)
        .ready_path(config.ready_path)
        .state(state)
        .build();

    tokio::spawn(server.serve(std::future::pending()));
}
