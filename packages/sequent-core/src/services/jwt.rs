// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use base64::engine::general_purpose;
use base64::Engine;
use serde;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use tracing::{event, instrument, Level};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtRolesAccess {
    pub roles: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtHasuraClaims {
    #[serde(rename = "x-hasura-default-role")]
    pub default_role: String,
    #[serde(rename = "x-hasura-tenant-id")]
    pub tenant_id: String,
    #[serde(rename = "x-hasura-user-id")]
    pub user_id: String,
    #[serde(rename = "x-hasura-allowed-roles")]
    pub allowed_roles: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum StringOrVec {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims {
    pub exp: i64,
    pub iat: i64,
    pub auth_time: i64,
    pub jti: String,
    pub iss: String,
    //pub aud: StringOrVec,
    pub sub: String,
    pub typ: String,
    pub azp: String,
    pub nonce: String,
    pub session_state: String,
    pub acr: String,
    #[serde(rename = "allowed-origins")]
    pub allowed_origins: Vec<String>,
    pub realm_access: JwtRolesAccess,
    pub resource_access: HashMap<String, JwtRolesAccess>,
    pub scope: String,
    pub sid: String,
    pub email_verified: bool,
    #[serde(rename = "https://hasura.io/jwt/claims")]
    pub hasura_claims: JwtHasuraClaims,
    pub name: String,
    pub preferred_username: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
}

pub fn decode_jwt(token: &str) -> Result<JwtClaims> {
    let parts: Vec<&str> = token.split('.').collect();
    let bytes = general_purpose::STANDARD_NO_PAD
        .decode(parts[1])
        .map_err(|err| anyhow!("Error decoding string: {:?}", err))?;
    let json = String::from_utf8(bytes)
        .map_err(|err| anyhow!("Error decoding bytes to utf8: {:?}", err))?;

    event!(Level::INFO, "json: {:?}", json);
    let claims: JwtClaims = serde_json::from_str(&json).map_err(|err| {
        anyhow!("Error decoding string into formatted json: {:?}", err)
    })?;
    Ok(claims)
}

#[cfg(test)]
mod tests {
    use crate::services::jwt::*;
    use jsonwebtoken::TokenData;
    use jsonwebtoken::{
        decode, decode_header, Algorithm, DecodingKey, Validation,
    };
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {}

    #[test]
    fn test_jwt() {
        let token: &str = "eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICJJYXRJRWFNd004REEyVGFWa3ZEQklxbjcxQ2plRjhLeEx5VU9MbnBjRXlJIn0.eyJleHAiOjE3MDA0NDM1NzgsImlhdCI6MTcwMDQ0MzI3OCwiYXV0aF90aW1lIjoxNzAwNDQzMjc3LCJqdGkiOiI3NzRmNDY4OC03NjlhLTQyYzQtOGRiYS01YmJjNzdjYzFlOWEiLCJpc3MiOiJodHRwOi8vMTI3LjAuMC4xOjgwOTAvcmVhbG1zL2VsZWN0b3JhbC1wcm9jZXNzIiwiYXVkIjoiYWNjb3VudCIsInN1YiI6ImEyMDM3NmQ2LWU1M2EtNGVkZi1hZmMzLWNjMThiYjhlMjNlNSIsInR5cCI6IkJlYXJlciIsImF6cCI6ImFkbWluLXBvcnRhbCIsIm5vbmNlIjoiNmZmNDhhMDgtMTFmZC00MTUzLTgzZWUtYzljZWE1MGI1NTE1Iiwic2Vzc2lvbl9zdGF0ZSI6IjhhOWYwYzFjLTA2YzYtNDNkNS05OGY5LWZhYWFjZTNmYzZiYyIsImFjciI6IjEiLCJhbGxvd2VkLW9yaWdpbnMiOlsiKiJdLCJyZWFsbV9hY2Nlc3MiOnsicm9sZXMiOlsiZGVmYXVsdC1yb2xlcy1lbGVjdG9yYWwtcHJvY2VzcyIsIm9mZmxpbmVfYWNjZXNzIiwidW1hX2F1dGhvcml6YXRpb24iLCJteS1yZWFsbS1yb2xlIl19LCJyZXNvdXJjZV9hY2Nlc3MiOnsiYWRtaW4tcG9ydGFsIjp7InJvbGVzIjpbImFkbWluLXVzZXIiXX0sImFjY291bnQiOnsicm9sZXMiOlsibWFuYWdlLWFjY291bnQiLCJtYW5hZ2UtYWNjb3VudC1saW5rcyIsInZpZXctcHJvZmlsZSJdfX0sInNjb3BlIjoib3BlbmlkIHByb2ZpbGUgZW1haWwiLCJzaWQiOiI4YTlmMGMxYy0wNmM2LTQzZDUtOThmOS1mYWFhY2UzZmM2YmMiLCJlbWFpbF92ZXJpZmllZCI6ZmFsc2UsImh0dHBzOi8vaGFzdXJhLmlvL2p3dC9jbGFpbXMiOnsieC1oYXN1cmEtZGVmYXVsdC1yb2xlIjoiYWRtaW4tdXNlciIsIngtaGFzdXJhLXRlbmFudC1pZCI6IjkwNTA1YzhhLTIzYTktNGNkZi1hMjZiLTRlMTlmNmEwOTdkNSIsIngtaGFzdXJhLXVzZXItaWQiOiJhMjAzNzZkNi1lNTNhLTRlZGYtYWZjMy1jYzE4YmI4ZTIzZTUiLCJ4LWhhc3VyYS1hbGxvd2VkLXJvbGVzIjpbImRlZmF1bHQtcm9sZXMtZWxlY3RvcmFsLXByb2Nlc3MiLCJvZmZsaW5lX2FjY2VzcyIsInVtYV9hdXRob3JpemF0aW9uIiwibXktcmVhbG0tcm9sZSJdfSwibmFtZSI6ImFkbWluIiwicHJlZmVycmVkX3VzZXJuYW1lIjoiYWRtaW4iLCJnaXZlbl9uYW1lIjoiYWRtaW4iLCJmYW1pbHlfbmFtZSI6IiJ9.DDD4zfpUqYOuy_Q1jgF5Cw0uJV9LGiO10GhhE42S9f2xfjw8tnKaV6WzqJrMFjFKMvuixsoXyXKTGeNuW-XyAK7LD7wdPIM2rZPgyXTdzQhSnKBSFjQu87CU_is4ii9I5I3r3PGsVp821F_sWo9oG0_u3tphEl80FYYGINtHxZMvCc1BiikJIBP1tA_Dtl8qq2IfDuOeMO82vWgfNH6VmZwf92GmjkJfKMaAYf3nytRPtv-YvKc6HfWipYGwtt8kua4SlDavOw32Ec7osc_44gMhBL3FZrrikLhJPJ6n-iQBQstbMKTLLeowE4cqNkdhcnHjflmbGBEyY9242jkdqg";
        let n: &str = "q8-wv_ypwU2h2PjANU2_NNQT5NNVzI8au8Nzh7SD7Va2GJ41oef0zxa5SOIipBqcnbAcX9hCSaBqjg5CbSWy6XMWky54QzjDWdGhq9sqXIA6TAf2vKsrbL5daNQsRwBLwguB9QQKw52cL7T2NcIPS-y3H1oxPTfNUA92p2K8Eg7S3UN-kzxSyoytSJsO75HW20qrOcLls9u3_2wkMxy4Dn9ves4rcAEOR6IoMZVtOyZ8UBxiF552VhF1bC9yU6WSnDWaKdhACMf4mW2nJk5IvPF16iQ_cw2YJ5ro1KSVVra5WzPCZTIPuIu1IRWnZDgASnYJdrAu_pbNLO-JlwqGuQ";
        let e: &str = "AQAB";
        let _decoded = decode_jwt(token).unwrap();
        let header = decode_header(token);
        let token = decode::<Claims>(
            &token,
            &DecodingKey::from_rsa_components(n, e).unwrap(),
            &Validation::new(Algorithm::RS256),
        )
        .unwrap();
        println!("{:?}", header);
        println!("{:?}", token);
        assert!(false);
    }
}
