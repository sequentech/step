// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[macro_use]
extern crate rocket;

use dotenv::dotenv;
use sequent_core::util::init_log::init_log;

mod pdf;
mod routes;
mod s3;
mod services;
mod types;

#[launch]
fn rocket() -> _ {
    dotenv().ok();
    init_log(true);
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
                routes::fetch_document::fetch_document,
                routes::scheduled_event::create_scheduled_event,
                routes::immudb_log_audit::list_pgaudit,
                routes::insert_election_event::insert_election_event_f,
                routes::insert_tenant::insert_tenant,
                routes::users::create_user,
                routes::users::delete_user,
                routes::users::get_users,
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
                routes::upload_document::upload_document,
                routes::key_ceremony::create_key_ceremony,
                routes::key_ceremony::get_private_key,
                routes::key_ceremony::check_private_key,
            ],
        )
}
