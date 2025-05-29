// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::date::ISO8601;
use crate::types::permissions::Permissions;
use anyhow::{anyhow, Result};
use base64::engine::general_purpose;
use base64::Engine;
use chrono::{DateTime, Duration, Local};
use serde;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use tracing::{debug, info, instrument, warn};

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
    #[serde(rename = "x-hasura-area-id")]
    pub area_id: Option<String>,
    #[serde(rename = "authorized-election-ids")]
    pub authorized_election_ids: Option<Vec<String>>,
    #[serde(rename = "x-hasura-allowed-roles")]
    pub allowed_roles: Vec<String>,
    #[serde(rename = "x-hasura-permission-labels")]
    pub permission_labels: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum StringOrVec {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims {
    pub exp: i64,
    pub iat: i64,
    pub auth_time: Option<i64>,
    pub jti: String,
    pub iss: String,
    pub aud: Option<StringOrVec>,
    pub sub: String,
    pub typ: String,
    pub azp: String,
    pub nonce: Option<String>,
    pub session_state: Option<String>,
    pub acr: String,
    #[serde(rename = "allowed-origins")]
    pub allowed_origins: Vec<String>,
    pub realm_access: Option<JwtRolesAccess>,
    pub resource_access: Option<HashMap<String, JwtRolesAccess>>,
    pub scope: String,
    pub sid: Option<String>,
    pub email_verified: bool,
    #[serde(rename = "https://hasura.io/jwt/claims")]
    pub hasura_claims: JwtHasuraClaims,
    pub name: Option<String>,
    pub preferred_username: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub trustee: Option<String>,
}

#[instrument(err, skip_all)]
pub fn decode_jwt(token: &str) -> Result<JwtClaims> {
    let parts: Vec<&str> = token.split('.').collect();
    let part = parts.get(1).ok_or(anyhow::anyhow!("Bad token (no '.')"))?;
    let bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(part)
        .map_err(|err| anyhow!("Error decoding string: {:?}", err))?;
    let json = String::from_utf8(bytes)
        .map_err(|err| anyhow!("Error decoding bytes to utf8: {:?}", err))?;
    debug!("json: {:?}", json);
    let claims: JwtClaims = serde_json::from_str(&json).map_err(|err| {
        anyhow!("Error decoding string into formatted json: {:?}", err)
    })?;

    Ok(claims)
}

#[instrument(skip_all, ret)]
pub fn decode_permission_labels(claims: &JwtClaims) -> Vec<String> {
    let Some(label_str) = claims.hasura_claims.permission_labels.clone() else {
        return vec![];
    };

    let s = label_str.trim();
    let s = if s.starts_with('{') && s.ends_with('}') {
        &s[1..s.len() - 1]
    } else {
        s
    };

    // Split the string into items
    let items = s.split(',');

    // Process each item: trim whitespace and surrounding quotes
    let keys: Vec<String> = items
        .map(|item| item.trim().trim_matches('"').to_string())
        .filter(|item| item.len() > 0)
        .collect();
    keys
}

/**
 * Returns true only if the JWT has gold permissions and the JWT
 * authentication is fresh, i.e. performed less than 60 seconds ago.
 */
#[instrument(skip_all)]
pub fn has_gold_permission(claims: &JwtClaims) -> bool {
    let auth_time_local: DateTime<Local> =
        if let Some(auth_time_int) = claims.auth_time {
            if let Ok(auth_time_parsed) =
                ISO8601::timestamp_ms_utc_to_date_opt(auth_time_int * 1000)
            {
                auth_time_parsed
            } else {
                // Try to use iat if auth_time parsing failed
                if let Ok(iat_parsed) =
                    ISO8601::timestamp_ms_utc_to_date_opt(claims.iat * 1000)
                {
                    iat_parsed
                } else {
                    warn!("Failed to parse both auth_time and iat");
                    return false;
                }
            }
        } else {
            warn!("claims.auth_time is None");
            return false;
        };
    // Let's asume fresh means token has at most 1 minute since authentication
    let freshness_limit = ISO8601::now() - Duration::seconds(60);
    let is_fresh = auth_time_local > freshness_limit;
    warn!("is_fresh={is_fresh:?}, auth_time_local={auth_time_local:?}, freshness_limit={freshness_limit:?}");
    let is_gold = claims.acr == Permissions::GOLD.to_string();
    is_fresh && is_gold
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
        );
        println!("{:?}", header);
        println!("{:?}", token);
        assert!(format!("{:?}", token).contains("ExpiredSignature"));
    }
    #[test]
    fn test_jwt2() {
        let token: &str = "eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICJodi00Tk4zdVZXb3Z5clBTOXFkcVUwYWZUQWQtTmZmdENySW56SzRibktNIn0.eyJleHAiOjE3MTU3Njk1MTEsImlhdCI6MTcxNTc2OTIxMSwiYXV0aF90aW1lIjoxNzE1NzY5MjEwLCJqdGkiOiJjOGM4NTljZC01ODg4LTRjZTctYjUzZS0yMjQzODNiNDhjNWUiLCJpc3MiOiJodHRwczovL2tleWNsb2FrLXVhLnNlcXVlbnQudm90ZS9hdXRoL3JlYWxtcy90ZW5hbnQtOTA1MDVjOGEtMjNhOS00Y2RmLWEyNmItNGUxOWY2YTA5N2Q1LWV2ZW50LTM4OGIzZWZmLWU1ODMtNGE1Ni04MmI3LTBhZDE1ZWFhNDA5YSIsInN1YiI6IjY2OTcxYWIwLTFlNmQtNGZjNS1iNzI2LTliZmRkNzgyNDAyMSIsInR5cCI6IkJlYXJlciIsImF6cCI6InZvdGluZy1wb3J0YWwiLCJub25jZSI6ImQzNThjZmQ4LTFlYWQtNDlkYy04NWJjLThlMWYwMTE4MjIxMCIsInNlc3Npb25fc3RhdGUiOiI2Y2RmN2I1ZC1lYmMwLTRjOTUtODFlMC00MWE2NjE5NDlhMWIiLCJhY3IiOiIxIiwiYWxsb3dlZC1vcmlnaW5zIjpbIioiXSwicmVhbG1fYWNjZXNzIjp7InJvbGVzIjpbInVzZXIiXX0sInNjb3BlIjoib3BlbmlkIGVtYWlsIHByb2ZpbGUiLCJzaWQiOiI2Y2RmN2I1ZC1lYmMwLTRjOTUtODFlMC00MWE2NjE5NDlhMWIiLCJlbWFpbF92ZXJpZmllZCI6dHJ1ZSwiaHR0cHM6Ly9oYXN1cmEuaW8vand0L2NsYWltcyI6eyJ4LWhhc3VyYS1kZWZhdWx0LXJvbGUiOiJ1c2VyIiwieC1oYXN1cmEtYXJlYS1pZCI6IjI4YmRmOWIyLTA2YmMtNDRkZC1iMzJkLTc1OTIwNGJlMTlkNSIsIngtaGFzdXJhLXRlbmFudC1pZCI6IjkwNTA1YzhhLTIzYTktNGNkZi1hMjZiLTRlMTlmNmEwOTdkNSIsIngtaGFzdXJhLXVzZXItaWQiOiI2Njk3MWFiMC0xZTZkLTRmYzUtYjcyNi05YmZkZDc4MjQwMjEiLCJ4LWhhc3VyYS1hbGxvd2VkLXJvbGVzIjpbInVzZXIiXX0sIm5hbWUiOiJCRUdPw5FBIENFQkFMTE9TIENBTUFSRVJPIC0iLCJwcmVmZXJyZWRfdXNlcm5hbWUiOiJiLmNlYmFsbG9zQHVhLmVzIiwiZ2l2ZW5fbmFtZSI6IkJFR0_DkUEgQ0VCQUxMT1MgQ0FNQVJFUk8iLCJmYW1pbHlfbmFtZSI6Ii0iLCJlbWFpbCI6ImIuY2ViYWxsb3NAdWEuZXMifQ.G6IwhFvxrCFxKicbcoVcwuOVSWTRjcy31RWqHWha8E5Kz7tNn8KpUsADy595rZEsLeG5tGvyJ98PMoN2L7RInGKssxx6zRGo5kwa5qHW2sndTwyvjrjPlaZlTB3tNPa2uqTOn6ztfFfqFmAlKB2ig29NThUyqIHnMq502jPPK3a3LZcPBrAvrOUtlEJPxO3MKz2ItQy-YfAdIajR1T1BJmo5b-nzMqpEPFFae1rxKAV5SgyOhhSL2R-K5rKfmoJiUhudB132cgioWAcsw8L1LzA6esIPSn5apN_4y13LcqtbCPXJi0PxDwkKx4r5M-LUwWCcJMneI3aLOGs-cxo8pQ";

        let decoded = decode_jwt(token);
        println!("{:?}", decoded);

        let header = decode_header(token);
        assert!(true);
    }
}
