// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The one public entrypoint for messaging provider callbacks.
//!
//! A thin relay: no database, no vault, no provider secrets and no
//! verification. Each callback is wrapped in a
//! [`messaging::relay::CallbackEnvelope`], signed with the relay secret and
//! posted to harvest's internal `POST /messaging/callbacks`; harvest's
//! answer goes back to the provider unchanged. Any other path is a 404 and
//! is never forwarded.

#[macro_use]
extern crate rocket;

use rocket::data::Data;
use rocket::http::Status;

/// Meta's subscription handshake (`hub.mode`, `hub.verify_token`,
/// `hub.challenge`).
#[get("/webhooks/meta/<key>")]
fn meta_subscription(key: &str) -> Status {
    let _ = key;
    Status::NotImplemented
}

/// WhatsApp statuses, Messenger delivery and Get Started.
#[post("/webhooks/meta/<key>", data = "<body>")]
fn meta_callback(key: &str, body: Data<'_>) -> Status {
    let _ = (key, body);
    Status::NotImplemented
}

/// Infobip Viber delivery reports.
#[post("/webhooks/viber/<key>", data = "<body>")]
fn viber_callback(key: &str, body: Data<'_>) -> Status {
    let _ = (key, body);
    Status::NotImplemented
}

/// SNS-wrapped SES events and SNS subscription confirmation.
#[post("/webhooks/aws/<key>", data = "<body>")]
fn aws_callback(key: &str, body: Data<'_>) -> Status {
    let _ = (key, body);
    Status::NotImplemented
}

/// Liveness. Never forwards.
#[get("/healthz")]
fn healthz() -> Status {
    Status::Ok
}

#[launch]
fn rocket() -> _ {
    sequent_core::util::init_log::init_log(true);
    rocket::build().mount(
        "/",
        routes![
            meta_subscription,
            meta_callback,
            viber_callback,
            aws_callback,
            healthz
        ],
    )
}
