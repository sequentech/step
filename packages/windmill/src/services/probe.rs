// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::{hasura::tenant::get_tenant, services::celery_app::get_celery_app};
use sequent_core::services::keycloak::get_client_credentials;
use sequent_core::services::probe::ProbeHandler;
use std::net::SocketAddr;
use strum_macros::Display;
use tokio::join;
use tracing::{info, instrument, warn};
use uuid::Uuid;

use super::celery_app::get_is_app_active;

#[derive(Display, Debug, Eq, PartialEq, Clone)]
pub enum AppName {
    BEAT,
    HARVEST,
    WINDMILL,
}

const BROKER_CONNECTION_TIMEOUT: u32 = 2;

#[instrument]
async fn check_celery(_app_name: &AppName) -> Option<bool> {
    let celery_app = get_celery_app().await;

    let celery_result = celery_app.broker.reconnect(BROKER_CONNECTION_TIMEOUT).await;

    Some(celery_result.is_ok() && get_is_app_active())
}

#[instrument]
async fn check_hasura_db(app_name: &AppName) -> Option<bool> {
    if AppName::BEAT == *app_name {
        return None;
    }
    let hasura_db_result = get_hasura_pool().await;

    Some(hasura_db_result.get().await.is_ok())
}

#[instrument]
async fn check_keycloak_db(app_name: &AppName) -> Option<bool> {
    if AppName::BEAT == *app_name {
        return None;
    }

    let keycloak_db_result = get_keycloak_pool().await;

    Some(keycloak_db_result.get().await.is_ok())
}

#[instrument]
async fn check_hasura_graphql(app_name: &AppName) -> Option<bool> {
    if AppName::BEAT == *app_name {
        return None;
    }

    let keycloak_hasura_result = get_client_credentials().await;

    let hasura_query_ok = if let Ok(auth_headers) = keycloak_hasura_result {
        get_tenant(auth_headers, Uuid::new_v4().to_string())
            .await
            .is_ok()
    } else {
        info!("Can't connect to hasura graphql because can't authenticate to keycloak");
        return Some(false);
    };

    Some(hasura_query_ok)
}

#[instrument]
async fn readiness_test(app_name: &AppName) -> bool {
    // Use futures::join! to await multiple futures concurrently
    let (celery_ok, hasura_db_ok, keycloak_db_ok, hasura_graphql_ok) = join!(
        check_celery(app_name),
        check_hasura_db(app_name),
        check_keycloak_db(app_name),
        check_hasura_graphql(app_name),
    );

    info!(
        "celery: {:?}, hasura_db: {:?} , keycloak db: {:?}, hasura_graphql {:?}",
        celery_ok, hasura_db_ok, keycloak_db_ok, hasura_graphql_ok
    );

    let data = vec![celery_ok, hasura_db_ok, keycloak_db_ok, hasura_graphql_ok];

    data.iter().all(|&x| x.is_none() || x == Some(true))
}

pub async fn setup_probe(app_name: AppName) {
    let app = app_name.to_string();
    let addr_s = std::env::var(format!("{}_PROBE_ADDR", app)).unwrap_or("0.0.0.0:3030".to_string());
    let live_path = std::env::var(format!("{}_PROBE_LIVE_PATH", app)).unwrap_or("live".to_string());
    let ready_path =
        std::env::var(format!("{}_PROBE_READY_PATH", app)).unwrap_or("ready".to_string());

    let addr: Result<SocketAddr, _> = addr_s.parse();

    if let Ok(addr) = addr {
        let ph = ProbeHandler::new(&live_path, &ready_path, addr);
        let f = ph.future();
        let app_name_clone0 = app_name.clone();
        ph.set_live(move || {
            let app_name = app_name_clone0.clone();
            Box::pin(async move { readiness_test(&app_name).await })
        })
        .await;

        let app_name_clone = app_name.clone();
        ph.set_ready(move || {
            let app_name = app_name_clone.clone();
            Box::pin(async move { readiness_test(&app_name).await })
        })
        .await;
        tokio::spawn(f);
    } else {
        warn!("Could not parse address for probe '{}'", addr_s);
    }
}
