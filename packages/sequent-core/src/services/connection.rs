// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::jwt::*;
use crate::services::keycloak::{
    get_third_party_client_access_token, KeycloakAdminClient,
    PubKeycloakAdminToken,
};
use anyhow::{anyhow, Result as AnyhowResult};
use keycloak::{KeycloakAdmin, KeycloakAdminToken};
use rocket::http::HeaderMap;
use rocket::http::Status;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::State;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use tracing::{error, info, instrument, warn};

const TENANT_ID_HEADER: &str = "tenant-id";
const EVENT_ID_HEADER: &str = "event-id";
const AUTHORIZATION_HEADER: &str = "authorization";
pub const PRE_EXPIRATION_SECS: i64 = 5;
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
#[instrument(skip_all)]
fn parse_datafix_headers(headers: &HeaderMap) -> Option<DatafixHeaders> {
    let required_headers =
        [TENANT_ID_HEADER, EVENT_ID_HEADER, AUTHORIZATION_HEADER];
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
        headers.get_one(TENANT_ID_HEADER).unwrap_or_default(),
        headers.get_one(EVENT_ID_HEADER).unwrap_or_default(),
        headers.get_one(AUTHORIZATION_HEADER).unwrap_or_default(),
    );

    info!(
        "tenant-id: {:?} event-id: {:?} authorization: {:?}",
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

/// TokenResponse, timestamp before sending the request and the credentials to
/// make sure the requester is the same.
#[derive(Debug, Clone)]
struct TokenResponseExtended {
    token_resp: PubKeycloakAdminToken,
    stamp: Instant,
    client_id: String,
    client_secret: String,
    tenant_id: String,
}

/// Last access token can be reused if it´s not expired, this is to avoid
/// Keycloak having to hold one token per Api request which could lead quickly
/// to many thousands of tokens.
///
/// Keycloak can hold multiple tokens for the same client, so we do not care
/// about using the previous token if one thread read it and while didn´t send
/// it yet other thread wrote it. As long as it is not expired, we can reuse it.
/// The same goes for scalability as each container can hold a different
/// token being all valid.
#[derive(Debug, Default)]
pub struct LastDatafixAccessToken(RwLock<Option<TokenResponseExtended>>);

impl LastDatafixAccessToken {
    pub fn init() -> Self {
        LastDatafixAccessToken(RwLock::new(None))
    }
}

/// Reads the access token if it has been requested successfully before and it
/// is not expired.
#[instrument(skip(lst_acc_tkn))]
async fn read_access_token(
    client_id: &str,
    client_secret: &str,
    tenant_id: &str,
    lst_acc_tkn: &LastDatafixAccessToken,
) -> Option<PubKeycloakAdminToken> {
    let token_resp_ext_opt = match lst_acc_tkn.0.read() {
        Ok(read) => read.clone(),
        Err(err) => {
            warn!("Error acquiring read lock {err:?}");
            return None;
        }
    };

    if let Some(data) = token_resp_ext_opt {
        let pre_expiration_time: i64 =
            data.token_resp.expires_in as i64 - PRE_EXPIRATION_SECS; // Renew the token 5 seconds before it expires

        if data.client_id.eq(client_id)
            && data.client_secret.eq(client_secret)
            && data.tenant_id.eq(tenant_id)
            && pre_expiration_time.is_positive()
            && data.stamp.elapsed()
                < Duration::from_secs(pre_expiration_time as u64)
        {
            return Some(data.token_resp);
        }
    }
    return None;
}

/// Request a new access token and writes it to the cache
#[instrument(err, skip(lst_acc_tkn))]
async fn request_access_token(
    client_id: String,
    client_secret: String,
    tenant_id: String,
    lst_acc_tkn: &LastDatafixAccessToken,
) -> AnyhowResult<PubKeycloakAdminToken> {
    let stamp: Instant = Instant::now(); // Capture the stamp before sending the request
    info!("Requesting access token");
    let keycloak_adm_tkn = get_third_party_client_access_token(
        client_id.clone(),
        client_secret.clone(),
        tenant_id.clone(),
    )
    .await?;

    let token_resp: PubKeycloakAdminToken = keycloak_adm_tkn.try_into()?;
    let mut write = match lst_acc_tkn.0.write() {
        Ok(write) => write,
        Err(err) => {
            return Err(anyhow!("Error acquiring write lock: {err:?}"));
        }
    };
    *write = Some(TokenResponseExtended {
        token_resp: token_resp.clone(),
        stamp,
        client_id,
        client_secret,
        tenant_id,
    });

    Ok(token_resp)
} // release the lock

#[rocket::async_trait]
impl<'r> FromRequest<'r> for DatafixClaims {
    type Error = ();
    #[instrument(skip_all)]
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
                    error!("DatafixClaims guard: Missing headers!");
                    return Outcome::Error((Status::BadRequest, ()));
                }
            };

        // Try to read the access token from the cache, if it´s not there or
        // expired request a new one and write it to the cache.
        let lst_acc_tkn = try_outcome!(
            request.guard::<&State<LastDatafixAccessToken>>().await
        );
        let token_resp = match read_access_token(
            &authorization.client_id,
            &authorization.client_secret,
            &tenant_id,
            &lst_acc_tkn,
        )
        .await
        {
            Some(token_resp) => token_resp,
            None => {
                match request_access_token(
                    authorization.client_id,
                    authorization.client_secret,
                    tenant_id.clone(),
                    &lst_acc_tkn,
                )
                .await
                {
                    Ok(token_resp) => token_resp,
                    Err(err) => {
                        error!("DatafixClaims guard: request_access_token error {err:?}");
                        return Outcome::Error((Status::Unauthorized, ()));
                    }
                }
            }
        };

        match decode_jwt(&token_resp.access_token) {
            Ok(jwt_claims) => Outcome::Success(DatafixClaims {
                jwt_claims,
                tenant_id,
                datafix_event_id,
            }),
            Err(err) => {
                warn!("DatafixClaims guard: decode_jwt error {err:?}");
                Outcome::Error((Status::Unauthorized, ()))
            }
        }
    }
}
