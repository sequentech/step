// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Authorization tokens for the internal shared-page proxy. The digest conveys
//! no identity and is valid only after the current bearer and event gates pass.
use sequent_core::services::jwt::JwtClaims;
use sha2::{Digest, Sha256};

pub fn scope(
    tenant: &str,
    event: &str,
    area: &str,
    elections: &[String],
) -> String {
    hex::encode(Sha256::digest(
        serde_json::json!(["ballot-log-v1", tenant, event, area, elections])
            .to_string()
            .as_bytes(),
    ))
}

pub fn search_session(claims: &JwtClaims) -> Vec<u8> {
    let session = claims
        .sid
        .as_deref()
        .or(claims.session_state.as_deref())
        .unwrap_or(&claims.jti);
    Sha256::digest(
        serde_json::json!([
            claims.iss,
            claims.hasura_claims.tenant_id,
            claims.hasura_claims.election_event_id,
            claims.hasura_claims.user_id,
            session,
        ])
        .to_string()
        .as_bytes(),
    )
    .to_vec()
}

/// The limiter key of lookups made with no token, such as a hub's ballot
/// verifier asking for a published audited ballot: one per client address, and
/// one shared by every client whose address is unknown. Only the digest is kept.
pub fn public_lookup_session(ip: Option<std::net::IpAddr>) -> Vec<u8> {
    Sha256::digest(
        serde_json::json!([
            "public-ballot-lookup-v1",
            ip.map(|ip| ip.to_string())
        ])
        .to_string()
        .as_bytes(),
    )
    .to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_claims::Claims;

    #[test]
    fn shared_scope_excludes_identity_but_isolates_authorized_populations() {
        let elections = vec!["e1".to_owned(), "e2".to_owned()];
        let base = scope("tenant", "event", "area", &elections);
        assert_eq!(base.len(), 64);
        for other in [
            scope("other", "event", "area", &elections),
            scope("tenant", "other", "area", &elections),
            scope("tenant", "event", "other", &elections),
            scope("tenant", "event", "area", &elections[..1]),
        ] {
            assert_ne!(base, other);
        }
    }

    #[test]
    fn refreshed_tokens_share_a_session_limit_and_other_sessions_do_not() {
        let mut first = Claims::new("tenant", "voter").build();
        first.sid = Some("session-a".into());
        let mut refreshed = first.clone();
        refreshed.jti = "new-access-token".into();
        assert_eq!(search_session(&first), search_session(&refreshed));
        refreshed.sid = Some("session-b".into());
        assert_ne!(search_session(&first), search_session(&refreshed));
        refreshed.sid = first.sid.clone();
        refreshed.hasura_claims.tenant_id = "other".into();
        assert_ne!(search_session(&first), search_session(&refreshed));
    }

    #[test]
    fn public_lookups_are_limited_per_address_apart_from_voters() {
        let one = public_lookup_session(Some("192.0.2.1".parse().unwrap()));
        assert_eq!(one.len(), 32);
        assert_eq!(
            one,
            public_lookup_session(Some("192.0.2.1".parse().unwrap()))
        );
        assert_ne!(
            one,
            public_lookup_session(Some("192.0.2.2".parse().unwrap()))
        );
        assert_ne!(one, public_lookup_session(None));
        assert_eq!(public_lookup_session(None), public_lookup_session(None));
        assert_ne!(
            public_lookup_session(None),
            search_session(&Claims::new("tenant", "voter").build())
        );
    }
}
