// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::jwt::*;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthHeaders {
    pub key: String,
    pub value: String,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthHeaders {
    type Error = ();

    async fn from_request(
        request: &'r Request<'_>,
    ) -> Outcome<Self, Self::Error> {
        let headers = request.headers().clone();
        if headers.contains("X-Hasura-Admin-Secret") {
            Outcome::Success(AuthHeaders {
                key: "X-Hasura-Admin-Secret".to_string(),
                value: headers
                    .get_one("X-Hasura-Admin-Secret")
                    .unwrap()
                    .to_string(),
            })
        } else if headers.contains("authorization") {
            Outcome::Success(AuthHeaders {
                key: "authorization".to_string(),
                value: headers.get_one("authorization").unwrap().to_string(),
            })
        } else {
            Outcome::Error((Status::Unauthorized, ()))
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for JwtClaims {
    type Error = ();

    async fn from_request(
        request: &'r Request<'_>,
    ) -> Outcome<Self, Self::Error> {
        let headers = request.headers().clone();
        match headers.get_one("authorization") {
            Some(authorization) => {
                match authorization.strip_prefix("Bearer ") {
                    Some(token) => match decode_jwt(token) {
                        Ok(jwt) => Outcome::Success(jwt),
                        Err(err) => {
                            event!(Level::WARN, "decode_jwt error {:?}", err);
                            Outcome::Error((Status::Unauthorized, ()))
                        }
                    },
                    None => Outcome::Error((Status::Unauthorized, ())),
                }
            }
            None => Outcome::Error((Status::Unauthorized, ())),
        }
    }
}
