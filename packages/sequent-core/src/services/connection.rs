// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::jwt::*;
use crate::services::keycloak::{
    get_third_party_client_access_token, TokenResponse,
};
use anyhow::Result as AnyhowResult;
use rocket::http::HeaderMap;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::RwLock;
use std::thread::sleep;
use std::time::{Duration, Instant};
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

/// TokenResponse, tiemstamp before sending the request and the credentials used
#[derive(Debug, Clone)]
struct TokenResponseExtended {
    token_resp: TokenResponse,
    stamp: Instant,
    client_id: String,
    client_secret: String,
    tenant_id: String,
}

#[derive(Debug, Clone, Default)]
struct LastAccessToken {
    token_ext: Option<TokenResponseExtended>,
    is_in_request: bool,
}

/// Last access token can be reused if it´s not expired, to avoid concurrent
/// requests to get the token
static LAST_DATAFIX_ACCESS_TKN: RwLock<LastAccessToken> =
    RwLock::new(LastAccessToken {
        token_ext: None,
        is_in_request: false,
    });

/// Reads the access token if it has been requested successfully before and it
/// is not expired.
#[instrument]
async fn read_access_token(
    client_id: &str,
    client_secret: &str,
    tenant_id: &str,
) -> Option<TokenResponse> {
    let read = LAST_DATAFIX_ACCESS_TKN.read().unwrap();
    if read.is_in_request {
        sleep(Duration::from_millis(10)); // Shall we use Tokio sleep?
        return None; // TODO: Handle this from the caller to try read again.
    }
    if let Some(data) = read.token_ext.clone() {
        let pre_expriration_time: i64 = data.token_resp.expires_in - 1; // Renew the token 1 second before it expires

        if data.client_id.eq(client_id)
            && data.client_secret.eq(client_secret)
            && data.tenant_id.eq(tenant_id)
            && pre_expriration_time.is_positive()
            && data.stamp.elapsed()
                < Duration::from_secs(pre_expriration_time as u64)
        {
            return Some(data.token_resp);
        }
    }
    return None;
}

/// Request a new access token and writes it to the cache
#[instrument(err)]
pub async fn request_access_token(
    client_id: String,
    client_secret: String,
    tenant_id: String,
) -> AnyhowResult<TokenResponse> {
    {
        // Write the lock and set the flag to avoid multiple requests
        let mut write = LAST_DATAFIX_ACCESS_TKN.write().unwrap();
        *write = LastAccessToken {
            token_ext: None,
            is_in_request: true,
        };
    } // release the lock

    let stamp: Instant = Instant::now(); // Capture the stamp before sending the request
    let token_resp = match get_third_party_client_access_token(
        client_id.clone(),
        client_secret.clone(),
        tenant_id.clone(),
    )
    .await
    {
        Ok(token_resp) => token_resp,
        Err(err) => {
            let mut write = LAST_DATAFIX_ACCESS_TKN.write().unwrap();
            *write = LastAccessToken {
                token_ext: None,
                is_in_request: false, // Undo the flag before returning
            };
            // release the lock
            return Err(err);
        }
    };

    let mut write = LAST_DATAFIX_ACCESS_TKN.write().unwrap();
    *write = LastAccessToken {
        token_ext: Some(TokenResponseExtended {
            token_resp: token_resp.clone(),
            stamp,
            client_id,
            client_secret,
            tenant_id,
        }),
        is_in_request: false, // Undo the flag before returning
    };

    Ok(token_resp)
} // release the lock

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

        let token_resp = match read_access_token(
            &authorization.client_id,
            &authorization.client_secret,
            &tenant_id,
        )
        .await
        {
            Some(token_resp) => token_resp,
            None => {
                match request_access_token(
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
                }
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
