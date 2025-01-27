// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::jwt::*;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use tracing::{instrument, warn};

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
            warn!("AuthHeaders guard: headers: {headers:?}");
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
                            warn!("JwtClaims guard: decode_jwt error {err:?}");
                            Outcome::Error((Status::Unauthorized, ()))
                        }
                    },
                    None => {
                        warn!("JwtClaims guard: not a bearer token: {authorization:?}");
                        Outcome::Error((Status::Unauthorized, ()))
                    }
                }
            }
            None => {
                warn!("JwtClaims guard: headers: {headers:?}");
                Outcome::Error((Status::Unauthorized, ()))
            }
        }
    }
}

#[derive(Debug)]
pub struct UserLocation {
    pub ip: Option<IpAddr>,
    pub country_code: Option<String>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for UserLocation {
    type Error = ();

    async fn from_request(
        request: &'r Request<'_>,
    ) -> Outcome<Self, Self::Error> {
        let ip = request
            .headers()
            .get_one("CF-Connecting-IP")
            .or_else(|| request.headers().get_one("X-Forwarded-For"))
            .and_then(|ip_str| ip_str.parse().ok());

        let country_code = request
            .headers()
            .get_one("CF-IPCountry")
            .map(|s| s.to_string());

        Outcome::Success(UserLocation { ip, country_code })
    }
}

#[derive(Debug)]
pub struct DatafixClaims(pub JwtClaims);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for DatafixClaims {
    type Error = ();
    async fn from_request(
        request: &'r Request<'_>,
    ) -> Outcome<Self, Self::Error> {
        let headers = request.headers().clone();
        match headers.get_one("authorization") {
            Some(authorization) => {
                let mut data = authorization.split(":");
                let client_id = data.nth(0);
                let client_secret = data.nth(1);
                // .... WIP
                // Do the DB query to get the tenant ID if it´s not directly
                // provided from the client.
                let access_token: &str = "access_token"; // get access_token in similar way as get_client_credentials
                match decode_jwt(access_token) {
                    Ok(jwt) => Outcome::Success(DatafixClaims(jwt)),
                    Err(err) => {
                        warn!("JwtClaims guard: decode_jwt error {err:?}");
                        Outcome::Error((Status::Unauthorized, ()))
                    }
                }
            }
            None => {
                warn!("DatafixClaims guard: headers: {headers:?}");
                Outcome::Error((Status::Unauthorized, ()))
            }
        }
    }
}
