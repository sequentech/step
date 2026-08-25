// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Phase 2 — voting: login, fetch the ballot style, encrypt/hash/(sign) a
//! synthetic ballot, and cast it. See `LOAD_TEST_DESIGN.md` § Phase 2.

pub mod ballot;
pub mod ballot_style;
pub mod cast;

pub use ballot::{build_synthetic_contests, prepare_ballot, PreparedVote};
pub use ballot_style::fetch_ballot_style;
pub use cast::{cast_vote, CastOutcome};

use crate::auth::login;
use crate::hasura::HasuraClient;

/// The only client id `authorize_voter_election` maps to the `ONLINE`
/// voting channel
/// (`packages/sequent-core/src/services/authorization.rs:108-113`) — and
/// public, so password grant needs no `client_secret`.
pub const VOTING_PORTAL_CLIENT_ID: &str = "voting-portal";

/// `packages/sequent-core/src/services/keycloak/realm.rs:76-78`.
fn event_realm(tenant_id: &str, election_event_id: &str) -> String {
    format!("tenant-{tenant_id}-event-{election_event_id}")
}

#[derive(Debug, Clone)]
pub enum VoteOutcome {
    Cast(CastOutcome),
    LoginFailed(String),
    BallotStyleUnavailable(String),
    BallotPreparationFailed(String),
}

/// Logs in as `username`/`password` against the tenant/event realm using
/// the public `voting-portal` client, fetches the ballot style for
/// `election_id`, builds and encrypts a synthetic "first candidate in
/// every contest" ballot, and casts it.
///
/// Never returns `Err` for expected failure modes (bad credentials, a
/// rejected cast, ...) — those come back as a classified `VoteOutcome`
/// variant so a caller running many of these concurrently can tally
/// outcomes without a single voter's failure aborting the run.
#[allow(clippy::too_many_arguments)]
pub async fn cast_one_vote(
    http: &reqwest::Client,
    keycloak_url: &str,
    endpoint_url: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    username: &str,
    password: &str,
) -> VoteOutcome {
    let realm = event_realm(tenant_id, election_event_id);
    let token = match login(
        http,
        keycloak_url,
        &realm,
        VOTING_PORTAL_CLIENT_ID,
        None,
        username,
        password,
    )
    .await
    {
        Ok(token) => token,
        Err(err) => return VoteOutcome::LoginFailed(err.to_string()),
    };

    let client = HasuraClient::new(http.clone(), endpoint_url, token.access_token);

    let style = match fetch_ballot_style(&client, election_id).await {
        Ok(style) => style,
        Err(err) => return VoteOutcome::BallotStyleUnavailable(err.to_string()),
    };

    let contests = build_synthetic_contests(&style);
    let prepared = match prepare_ballot(&style, contests) {
        Ok(prepared) => prepared,
        Err(err) => return VoteOutcome::BallotPreparationFailed(err.to_string()),
    };

    let outcome = cast_vote(&client, election_id, &prepared.ballot_id, &prepared.content).await;
    VoteOutcome::Cast(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_realm_matches_the_sequent_core_convention() {
        assert_eq!(
            event_realm("90505c8a-23a9-4cdf-a26b-4e19f6a097d5", "7d7f840a"),
            "tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-7d7f840a"
        );
    }
}
