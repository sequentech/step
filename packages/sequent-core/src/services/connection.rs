// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::jwt::*;
use crate::services::keycloak::{
    get_third_party_client_access_token, TokenResponse,
};
use rocket::http::HeaderMap;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::RwLock;
use std::time::Instant;
use tracing::{info, instrument, warn};

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
pub struct DatafixClaims {
    pub jwt_claims: JwtClaims,
    pub tenant_id: String,
    /// Event ID matching the election event Datafix:id in annotations
    pub datafix_event_id: String,
}

#[derive(Debug)]
struct DatafixCredentials {
    client_id: String,
    client_secret: String,
}

#[derive(Debug)]
struct DatafixHeaders {
    tenant_id: String,
    event_id: String,
    authorization: DatafixCredentials,
}

/// Returns None if any of the required headers are missing or is incomplete
#[instrument]
fn parse_datafix_headers(headers: &HeaderMap) -> Option<DatafixHeaders> {
    let required_headers = ["tenant_id", "event_id", "authorization"];
    let mut missing_headers = vec![];
    for header in required_headers {
        if !headers.contains(header) {
            warn!("DatafixClaims guard: No {header} in headers: {headers:?}");
            missing_headers.push(header);
        }
    }

    if !missing_headers.is_empty() {
        return None;
    }

    let (tenant_id, event_id, authorization) = (
        headers.get_one("tenant_id").unwrap_or_default(),
        headers.get_one("event_id").unwrap_or_default(),
        headers.get_one("authorization").unwrap_or_default(),
    );

    info!(
        "tenant_id: {:?} event_id: {:?} authorization: {:?}",
        tenant_id, event_id, authorization
    );

    let mut auth_collection = authorization.split(":");
    let client_id = auth_collection.nth(0); // get the first item and consumes it
    let client_secret = auth_collection.nth(0);
    info!("{:?}:{:?}", client_id, client_secret);
    let (client_id, client_secret) =
        if let (Some(client_id), Some(client_secret)) =
            (client_id, client_secret)
        {
            (client_id, client_secret)
        } else {
            return None;
        };

    Some(DatafixHeaders {
        tenant_id: tenant_id.to_string(),
        event_id: event_id.to_string(),
        authorization: DatafixCredentials {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
        },
    })
}

/// Last access token can be reused if not expired
struct LastAccessToken {
    tkn_resp: TokenResponse,
    stamp: Instant,
}
static mut LAST_DATAFIX_ACCESS_TKN: RwLock<Option<LastAccessToken>> =
    RwLock::new(None);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for DatafixClaims {
    type Error = ();
    #[instrument]
    async fn from_request(
        request: &'r Request<'_>,
    ) -> Outcome<Self, Self::Error> {
        let (tenant_id, datafix_event_id, authorization) =
            match parse_datafix_headers(request.headers()) {
                Some(datafix_headers) => (
                    datafix_headers.tenant_id,
                    datafix_headers.event_id,
                    datafix_headers.authorization,
                ),
                None => {
                    return Outcome::Error((Status::BadRequest, ()));
                }
            };

        let token_resp = match get_third_party_client_access_token(
            authorization.client_id,
            authorization.client_secret,
            tenant_id.clone(),
        )
        .await
        {
            Ok(token_resp) => token_resp,
            Err(err) => {
                warn!("JwtClaims guard: decode_jwt error {err:?}");
                return Outcome::Error((Status::Unauthorized, ()));
            }
        };

        info!("JwtClaims guard: token_resp: {token_resp:?}");
        match decode_jwt(&token_resp.access_token) {
            Ok(jwt_claims) => Outcome::Success(DatafixClaims {
                jwt_claims,
                tenant_id,
                datafix_event_id,
            }),
            Err(err) => {
                warn!("JwtClaims guard: decode_jwt error {err:?}");
                Outcome::Error((Status::Unauthorized, ()))
            }
        }
    }
}
