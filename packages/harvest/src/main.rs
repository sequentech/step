// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[macro_use]
extern crate rocket;

use dotenv::dotenv;
use sequent_core::services::keycloak::get_client_credentials;
use sequent_core::services::probe::ProbeHandler;
use sequent_core::util::init_log::init_log;
use std::net::SocketAddr;
use tokio::join;
use tracing::{instrument, warn};
use uuid::Uuid;
use windmill::services::database::get_hasura_pool;
use windmill::{
    hasura::tenant::get_tenant, services::celery_app::get_celery_app,
};

mod pdf;
mod routes;
mod services;
mod types;

#[launch]
async fn rocket() -> _ {
    dotenv().ok();
    init_log(true);

    setup_probe().await;

    rocket::build()
        .register(
            "/",
            catchers![
                routes::error_catchers::internal_error,
                routes::error_catchers::not_found,
                routes::error_catchers::default
            ],
        )
        .mount(
            "/",
            routes![
                routes::ballot_publication::get_ballot_publication_changes,
                routes::ballot_publication::generate_ballot_publication,
                routes::ballot_publication::publish_ballot,
                routes::insert_cast_vote::insert_cast_vote,
                routes::fetch_document::fetch_document,
                routes::election_event_stats::get_election_event_stats,
                routes::election_stats::get_election_stats,
                routes::scheduled_event::create_scheduled_event,
                routes::immudb_log_audit::list_pgaudit,
                routes::import_areas::import_areas_route,
                routes::import_areas::upsert_areas_route,
                routes::electoral_log::list_electoral_log,
                routes::export_election_event::export_election_event_route,
                routes::export_election_event_logs::export_election_event_logs_route,
                routes::insert_election_event::insert_election_event_f,
                routes::import_candidates::import_candidates_route,
                routes::insert_election_event::import_election_event_f,
                routes::insert_tenant::insert_tenant,
                routes::users::create_user,
                routes::users::import_users_f,
                routes::users::export_users_f,
                routes::users::delete_user,
                routes::users::get_users,
                routes::users::get_user,
                routes::users::edit_user,
                routes::roles::list_user_roles,
                routes::roles::set_user_role,
                routes::roles::delete_user_role,
                routes::roles::create_role,
                routes::roles::delete_role,
                routes::roles::get_roles,
                routes::permissions::create_permission,
                routes::permissions::delete_permission,
                routes::permissions::get_permissions,
                routes::permissions::set_role_permission,
                routes::permissions::delete_role_permission,
                routes::upload_document::get_upload_url,
                routes::keys_ceremony::create_keys_ceremony,
                routes::keys_ceremony::get_private_key,
                routes::keys_ceremony::check_private_key,
                routes::tally_ceremony::create_tally_ceremony,
                routes::tally_ceremony::restore_private_key,
                routes::voting_status::update_event_status,
                routes::voting_status::update_election_status,
                routes::tally_ceremony::update_tally_ceremony,
                routes::manual_verification_pdf::get_manual_verification_pdf,
                routes::tally_sheets::publish_tally_sheet,
                routes::create_vote_receipt::create_vote_receipt,
                routes::election_dates::manage_election_dates,
            ],
        )
}

#[instrument]
async fn readiness_test() -> bool {
    let celery_app = get_celery_app().await;

    let broker_connection_timeout = 2;

    // Use futures::join! to await multiple futures concurrently
    let (celery_result, hasura_db_result, keycloak_hasura_result) = join!(
        celery_app.broker.reconnect(broker_connection_timeout),
        get_hasura_pool(),
        get_client_credentials()
    );

    let celery_ok = celery_result.is_ok();
    let hasura_db_client_ok = hasura_db_result.get().await.is_ok();
    let hasura_graphql_client_ok = keycloak_hasura_result.is_ok();

    let hasura_query_ok = if let Ok(auth_headers) = keycloak_hasura_result {
        get_tenant(auth_headers, Uuid::new_v4().to_string())
            .await
            .is_ok()
    } else {
        false
    };
    warn!(
        "celery: {}, hasura_db: {} , keycloak_hasura {}, hasura_query: {}",
        celery_ok,
        hasura_db_client_ok,
        hasura_graphql_client_ok,
        hasura_query_ok
    );

    celery_ok
        && hasura_db_client_ok
        && hasura_graphql_client_ok
        && hasura_query_ok
}

async fn setup_probe() {
    let addr_s = std::env::var("HARVEST_PROBE_ADDR")
        .unwrap_or("0.0.0.0:3030".to_string());
    let live_path =
        std::env::var("HARVEST_PROBE_LIVE_PATH").unwrap_or("live".to_string());
    let ready_path = std::env::var("HARVEST_PROBE_READY_PATH")
        .unwrap_or("ready".to_string());

    let addr: Result<SocketAddr, _> = addr_s.parse();

    if let Ok(addr) = addr {
        let ph = ProbeHandler::new(&live_path, &ready_path, addr);
        let f = ph.future();
        ph.set_live(move || true).await;
        ph.set_ready(move || Box::pin(async { readiness_test().await }))
            .await;
        tokio::spawn(f);
    } else {
        warn!("Could not parse address for probe '{}'", addr_s);
    }
}
