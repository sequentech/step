// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::jwt::*;
// use crate::services::keycloak::get
use rocket::http::HeaderMap;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
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
    pub election_event_id: String,
}

#[derive(Debug)]
struct DatafixAuth {
    client_id: String,
    client_secret: String,
}

#[derive(Debug)]
struct DatafixHeaders {
    tenant_id: String,
    event_id: String,
    authorization: DatafixAuth,
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
        authorization: DatafixAuth {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
        },
    })
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for DatafixClaims {
    type Error = ();
    #[instrument]
    async fn from_request(
        request: &'r Request<'_>,
    ) -> Outcome<Self, Self::Error> {
        const MOCK_ACCESS_TOKEN: &str = "eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICJJYXRJRWFNd004REEyVGFWa3ZEQklxbjcxQ2plRjhLeEx5VU9MbnBjRXlJIn0.eyJleHAiOjE3MzgwNTgxMTMsImlhdCI6MTczODA1NzgxMywianRpIjoiYTVkOGJmZDEtNTRkNy00NGI4LWIxMTgtYzJkMWEyYTQzNGI0IiwiaXNzIjoiaHR0cDovLzEyNy4wLjAuMTo4MDkwL3JlYWxtcy90ZW5hbnQtOTA1MDVjOGEtMjNhOS00Y2RmLWEyNmItNGUxOWY2YTA5N2Q1IiwiYXVkIjoiYWNjb3VudCIsInN1YiI6IjQ4N2VhNDk2LTlmMDYtNDE0OC1hZDA2LTRhMTJmOWJiNjk4MCIsInR5cCI6IkJlYXJlciIsImF6cCI6InNlcnZpY2UtYWNjb3VudCIsImFjciI6InNpbHZlciIsImFsbG93ZWQtb3JpZ2lucyI6WyIvKiJdLCJyZXNvdXJjZV9hY2Nlc3MiOnsic2VydmljZS1hY2NvdW50Ijp7InJvbGVzIjpbInVtYV9wcm90ZWN0aW9uIiwic2VydmljZS1yb2xlIl19LCJhY2NvdW50Ijp7InJvbGVzIjpbIm1hbmFnZS1hY2NvdW50IiwibWFuYWdlLWFjY291bnQtbGlua3MiLCJ2aWV3LXByb2ZpbGUiXX19LCJzY29wZSI6Im9wZW5pZCBwcm9maWxlIGVtYWlsIiwiY2xpZW50SG9zdCI6IjE3Mi4xOC4wLjEiLCJlbWFpbF92ZXJpZmllZCI6ZmFsc2UsImh0dHBzOi8vaGFzdXJhLmlvL2p3dC9jbGFpbXMiOnsieC1oYXN1cmEtZGVmYXVsdC1yb2xlIjoic2VydmljZS1hY2NvdW50IiwieC1oYXN1cmEtdGVuYW50LWlkIjoiOTA1MDVjOGEtMjNhOS00Y2RmLWEyNmItNGUxOWY2YTA5N2Q1IiwieC1oYXN1cmEtdXNlci1pZCI6IjQ4N2VhNDk2LTlmMDYtNDE0OC1hZDA2LTRhMTJmOWJiNjk4MCIsIngtaGFzdXJhLWFsbG93ZWQtcm9sZXMiOlsic2VydmljZS1hY2NvdW50Il19LCJwcmVmZXJyZWRfdXNlcm5hbWUiOiJzZXJ2aWNlLWFjY291bnQtc2VydmljZS1hY2NvdW50IiwiY2xpZW50QWRkcmVzcyI6IjE3Mi4xOC4wLjEiLCJjbGllbnRfaWQiOiJzZXJ2aWNlLWFjY291bnQifQ.g_Y1WbS0C_w-6HyahoDeNDihCIwr-5f-fJ0K2HVwbC1JHCZ50SdiWnc8Mge1DZd889fHzK7zPPdY0sWtD06dIJRKipXun6p9oW54j1lSdRt6SCZR1GZZwXH-tMumZwWff9qoniKCKHGlOXVxTY1_Tr00snRI6JbpYEKNt65ZKu7wVcXYiNsbLDbr349umF1g1UH2rYq8DGc49PuwraX4xho1RBxrkOYM3bR1cqTMOEFq2fHXhH9fTTyNrN2vmyWfQs0Znez8che2ZRxCr_UQi7drnLOkmd-jU884_LojiHbkVKreiCFgXNd8Ypu8Qn9BgpwgL8mp9WUCG_rJ81hg3w";
        let (tenant_id, election_event_id, authorization) =
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

        let access_token: &str = MOCK_ACCESS_TOKEN; // get access_token in similar way as get_client_credentials
                                                    // .... WIP
                                                    // Do the DB query to get the tenant ID if it´s not directly
                                                    // provided from the client.

        match decode_jwt(access_token) {
            Ok(jwt_claims) => Outcome::Success(DatafixClaims {
                jwt_claims,
                tenant_id,
                election_event_id,
            }),
            Err(err) => {
                warn!("JwtClaims guard: decode_jwt error {err:?}");
                Outcome::Error((Status::Unauthorized, ()))
            }
        }
    }
}
