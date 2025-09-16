// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![recursion_limit = "256"]
#[macro_use]
extern crate rocket;

use dotenv::dotenv;
use sequent_core::services::connection::LastDatafixAccessToken;
use sequent_core::util::init_log::init_log;
use windmill::services::{
    celery_app::set_is_app_active,
    plugins_manager::plugin_manager::init_plugin_manager,
    probe::{setup_probe, AppName},
};

mod routes;
mod services;
mod types;

#[launch]
async fn rocket() -> _ {
    dotenv().ok();
    init_log(true);

    setup_probe(AppName::HARVEST).await;
    set_is_app_active(true);
    init_plugin_manager().await.unwrap();

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
            "/api/datafix",
            routes![
                routes::api_datafix::add_voter,
                routes::api_datafix::update_voter,
                routes::api_datafix::delete_voter,
                routes::api_datafix::unmark_voted,
                routes::api_datafix::mark_voted,
                routes::api_datafix::replace_pin,
            ],
        )
        .mount(
            "/",
            routes![
                routes::ballot_publication::get_ballot_publication_changes,
                routes::ballot_publication::generate_ballot_publication,
                routes::ballot_publication::publish_ballot,
                routes::ballot_publication_prepare_preview::prepare_ballot_publication_preview,
                routes::voter_electoral_log::list_cast_vote_messages,
                routes::insert_cast_vote::insert_cast_vote,
                routes::fetch_document::fetch_document,
                routes::elections::create_election,
                routes::areas::upsert_area,
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
                routes::delete_election_event::delete_election_event_f,
                routes::import_tenant_config::import_tenant_config_route,
                routes::export_tenant_config::export_tenant_config_route,
                routes::insert_tenant::insert_tenant,
                routes::users::create_user,
                routes::users::import_users_f,
                routes::users::export_users_f,
                routes::users::export_tenant_users_f,
                routes::users::delete_user,
                routes::users::delete_users,
                routes::users::get_users,
                routes::users::count_users,
                routes::users::get_user,
                routes::users::edit_user,
                routes::users::get_user_profile_attributes,
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
                routes::keys_ceremony::list_keys_ceremonies,
                routes::tally_ceremony::create_tally_ceremony,
                routes::tally_ceremony::restore_private_key,
                routes::voting_status::update_event_status,
                routes::voting_status::update_election_status,
                routes::tally_ceremony::update_tally_ceremony,
                routes::manual_verification_pdf::get_manual_verification_pdf,
                routes::tally_sheets::publish_tally_sheet,
                routes::create_ballot_receipt::create_ballot_receipt,
                routes::election_dates::manage_election_dates,
                routes::custom_urls::update_custom_url,
                routes::custom_urls::get_custom_url,
                routes::miru_plugin::create_transmission_package,
                routes::miru_plugin::send_transmission_package,
                routes::miru_plugin::upload_signature,
                routes::export_tasks_execution::export_tasks_execution_route,
                routes::limit_access_by_countries::limit_access_by_countries,
                routes::export_template::export_template,
                routes::import_templates::import_templates_route,
                routes::election_event_stats::get_election_event_top_votes_by_ip,
                routes::export_ballot_publication::export_ballot_publication_route,
                routes::reports::render_document_pdf,
                routes::reports::generate_template,
                routes::reports::generate_report,
                routes::reports::encrypt_report_route,
                routes::reports::generate_transmission_report,
                routes::templates::get_user_template,
                routes::applications::verify_user_application,
                routes::applications::change_application_status,
                routes::election_event_monitoring::get_election_event_monitoring_f,
                routes::election_monitoring::get_election_monitoring_f,
                routes::export_application::export_application_route,
                routes::import_application::import_application_route,
                routes::trustees::export_trustees_route,
                routes::set_voter_authentication::set_voter_authentication,
                routes::export_tally_results::export_tally_results_route,
            ],
        )
        .mount("/", routes![routes::plugins::plugin_routes])
        .manage(LastDatafixAccessToken::init())
}
