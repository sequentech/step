// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
    #[serde(rename = "x-hasura-election-event-id")]
    pub election_event_id: Option<String>,
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
        .filter(|item| !item.is_empty())
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
                warn!("Failed to parse auth_time");
                return false;
            }
        } else {
            info!("fallback to iat, claims.auth_time is None");
            if let Ok(iat_parsed) =
                ISO8601::timestamp_ms_utc_to_date_opt(claims.iat * 1000)
            {
                iat_parsed
            } else {
                warn!("Failed to parse iat");
                return false;
            }
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

    use jsonwebtoken::{
        decode, decode_header, errors::ErrorKind, Algorithm, DecodingKey,
        Validation,
    };
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {}

    // Signed once with a disposable test key. Only its public modulus and the
    // signature remain; the claims are synthetic and expired since Unix time 1.
    const RSA_MODULUS: &str = "pT-Q5Noy87kgcU8Mhtjf5dZNOEoBh6O4RX1Vi426dDRoXwzMJyyl6eL8MPu2Ck89QPX1YL2mzYPOh7oz6asY928mgB_tYVmVlqZq2Foga_W0STDoGH_r8RdIVnMkI2ZuV8gkJlBzAP0uXxPDfMLecAWz6fYt3rU12NhX_HeNYkNSbZxOTX5k1mGLhwnYdIxFs5WQfgXOkK45EdbS5ccLcjN51g20JDjEPtLerX5dOc9sSf0NGS7RlYdl4sVFQi5upyn1dhSbg2Z0tbPMwIE7cQqNVfQsCIoT6qJGKC674fwTW4x-KAesbbthrYxtP34pxMlNw_DeLJ5XTcdM-enxWw";

    fn synthetic_rs256_token() -> String {
        let header = r#"{"alg":"RS256","typ":"JWT"}"#;
        let claims = r#"{"acr":"1","allowed-origins":[],"aud":"synthetic-client","azp":"synthetic-client","email_verified":false,"exp":1,"https://hasura.io/jwt/claims":{"x-hasura-allowed-roles":["voter"],"x-hasura-default-role":"voter","x-hasura-tenant-id":"synthetic-tenant","x-hasura-user-id":"synthetic-user"},"iat":0,"iss":"https://issuer.example.invalid","jti":"synthetic-jwt-id","scope":"openid","sub":"synthetic-subject","typ":"Bearer"}"#;
        let signature = "Ezs-VLRQqn3IoWJSl00nrDKpYDXGYaVyaKVFCMdeLQY6nCSlbCoM9_PtT4hSWlRc5VvnIVppDxRbdJKfZmLDTPJyvpOqKv9mRJB8maNAWne9Bcivjlv7w3hityUfD93XSNFBIaJ_rCO34qAOjRB3g4HfflMvHPlg8FZYmHNE_LIy0g4TcO4CRK8Gc-YP8n5X0_GyNissug927Pbi5iYkxz0buNElFGON_rTrmBnY34EivABPJdzYI1dNFeVmC5lMAd-ZspabMQySgc4jrIslY_-LRAd4OriUPSpT9J6wQy0eKG17rvAbxdTru1tIijPxeNmTujCTC7TW9cDs4cUOjg";
        format!(
            "{}.{}.{}",
            general_purpose::URL_SAFE_NO_PAD.encode(header),
            general_purpose::URL_SAFE_NO_PAD.encode(claims),
            signature
        )
    }

    #[test]
    fn expired_synthetic_rs256_token_is_rejected() {
        let token = synthetic_rs256_token();
        assert!(decode_jwt(&token).is_ok());
        let result = decode::<Claims>(
            &token,
            &DecodingKey::from_rsa_components(RSA_MODULUS, "AQAB").unwrap(),
            &Validation::new(Algorithm::RS256),
        );
        assert_eq!(result.unwrap_err().kind(), &ErrorKind::ExpiredSignature);
    }

    #[test]
    fn synthetic_rs256_token_decodes_claims_and_header() {
        let token = synthetic_rs256_token();
        let claims = decode_jwt(&token).unwrap();
        assert_eq!(claims.iss, "https://issuer.example.invalid");
        assert_eq!(claims.sub, "synthetic-subject");
        assert_eq!(decode_header(&token).unwrap().alg, Algorithm::RS256);
    }
}
