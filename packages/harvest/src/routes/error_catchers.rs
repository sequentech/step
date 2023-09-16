// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use tracing::instrument;
use rocket::Request;
use rocket::http::Status;

#[instrument(skip_all)]
#[catch(500)]
pub fn internal_error() -> &'static str {
    "Whoops! Looks like we messed up."
}

#[instrument(skip_all)]
#[catch(404)]
pub fn not_found(req: &Request) -> String {
    format!("I couldn't find '{}'. Try something else?", req.uri())
}

#[instrument(skip_all)]
#[catch(default)]
pub fn default(status: Status, req: &Request) -> String {
    format!("{} ({})", status, req.uri())
}
