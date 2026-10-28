// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What a voter's booth holds before it encrypts, and the voter routes of a
//! private event's `ballots` log, which Harvest forwards for the voter's own
//! ballots only (step-integration.md, section 2.6).

use super::client::{SidecarClient, SidecarError};
use super::statements::{config_key, portal_release, VOTING_OPENED_KEY};
use super::transparency::transparency_mode;
use super::{canonical_id, LogName};
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::vitrine::{
    get_vitrine_event, get_vitrine_log, get_vitrine_statement, VitrineStatement,
};
use crate::services::protocol_manager::get_event_store;
use crate::types::vitrine::{
    VitrineForwardOutput, VitrineVoterContext, VitrineVoterForwardInput, VoterRoute,
};
use anyhow::{anyhow, Result};
use deadpool_postgres::Client as DbClient;
use electoral_log::adapters::ballot_box_reads::StoredBallot;
use electoral_log::vitrine::{ballot_id_hash, ElectionEntry, Submitted};
use sequent_core::ballot::TransparencyMode;
use serde_json::{json, Value};
use std::str::FromStr;

const MICROS_PER_SEC: i64 = 1_000_000;

/// `{statement, entry, chain}` of a receipted statement.
fn receipted(statement: Option<VitrineStatement>) -> Option<VitrineStatement> {
    statement.filter(|s| s.chain.is_some())
}

fn statement_json(statement: &VitrineStatement) -> Value {
    json!({
        "statement": statement.statement,
        "entry": statement.entry,
        "chain": statement.chain,
    })
}

/// `vitrine_voter_context`: the policy, the `ballots` log's acknowledgement
/// of it, and, once windmill has them receipted, the election's config
/// statement with its ballot-style digests and the event's `voting-opened`
/// milestone.
pub async fn voter_context(
    db: &mut DbClient,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<VitrineVoterContext> {
    let tx = db.transaction().await?;
    let event = get_election_event_by_id(&tx, tenant_id, election_event_id).await?;
    let mode = transparency_mode(&event)?;
    if !mode.is_enabled() {
        return Ok(VitrineVoterContext::default());
    }
    let vitrine = get_vitrine_event(&tx, tenant_id, election_event_id).await?;
    let ballots = get_vitrine_log(&tx, tenant_id, election_event_id, LogName::Ballots).await?;
    let config = receipted(
        get_vitrine_statement(&tx, tenant_id, election_event_id, &config_key(election_id)?).await?,
    );
    let opened = receipted(
        get_vitrine_statement(&tx, tenant_id, election_event_id, VOTING_OPENED_KEY).await?,
    );
    tx.commit().await?;
    Ok(VitrineVoterContext {
        mode,
        policy: vitrine.and_then(|v| v.policy),
        policy_ack: ballots.and_then(|l| l.policy_ack),
        style_hashes: config.as_ref().and_then(|c| c.style_hashes.clone()),
        release: config.as_ref().and_then(|c| portal_release(&c.statement)),
        config: config.as_ref().map(statement_json),
        opened: opened.as_ref().map(statement_json),
    })
}

/// The `vitrine/election/ballots/v1` entries of a voter's ballots, as the ballot box
/// queued them. Ballots whose ID isn't a hash have none.
pub fn ballot_entries(election_event_id: &str, ballots: &[StoredBallot]) -> Result<Vec<Vec<u8>>> {
    let event_id = canonical_id(election_event_id)?;
    Ok(ballots
        .iter()
        .filter_map(|ballot| {
            let accepted_at = u64::try_from(ballot.accepted_at.div_euclid(MICROS_PER_SEC)).ok()?;
            ElectionEntry {
                event_id: event_id.clone(),
                election_id: canonical_id(&ballot.election_id).ok()?,
                ballot_id: ballot_id_hash(&ballot.ballot_id).ok()?,
                accepted_at,
            }
            .encode()
            .ok()
        })
        .collect())
}

#[derive(Debug, thiserror::Error)]
pub enum ForwardError {
    /// Not a request Harvest forwards.
    #[error("{0}")]
    BadRequest(String),
    /// Not this voter's to make.
    #[error("{0}")]
    Refused(String),
    #[error(transparent)]
    Sidecar(#[from] SidecarError),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// The route of a forward, if its body is a receipt or a promise for one of
/// the voter's own entries (IT-40).
pub fn check_forward(
    route: &str,
    body: &str,
    own_entries: &[Vec<u8>],
) -> Result<VoterRoute, ForwardError> {
    let route = VoterRoute::from_str(route)
        .map_err(|_| ForwardError::BadRequest(format!("Unknown voter route {route:?}")))?;
    if route == VoterRoute::Policies {
        if !body.is_empty() {
            return Err(ForwardError::BadRequest(
                "Policy discovery takes no body".into(),
            ));
        }
        return Ok(route);
    }
    let submitted = Submitted::parse(body)
        .map_err(|e| ForwardError::BadRequest(format!("Not a receipt or a promise: {e}")))?;
    if !submitted.is_for_one_of(own_entries) {
        return Err(ForwardError::Refused(
            "This isn't a receipt for one of your ballots".into(),
        ));
    }
    Ok(route)
}

/// `vitrine_voter_forward`: carries a private event's voter request to the
/// sidecar's internal listener and its answer back, as the sidecar gave it.
pub async fn voter_forward(
    db: &mut DbClient,
    sidecar: Option<&SidecarClient>,
    tenant_id: &str,
    voter_id: &str,
    input: &VitrineVoterForwardInput,
) -> Result<VitrineForwardOutput, ForwardError> {
    let election_event_id = input.election_event_id.as_str();
    let tx = db.transaction().await.map_err(anyhow::Error::from)?;
    let event = get_election_event_by_id(&tx, tenant_id, election_event_id).await?;
    if transparency_mode(&event)? != TransparencyMode::PRIVATE {
        return Err(ForwardError::Refused(
            "Only the voters of a private event ask Harvest for their receipts".into(),
        ));
    }
    let log = get_vitrine_log(&tx, tenant_id, election_event_id, LogName::Ballots)
        .await?
        .ok_or_else(|| ForwardError::Refused("The event has no ballots log".into()))?;
    tx.commit().await.map_err(anyhow::Error::from)?;

    let ballots = if input.route == "policies" {
        if input.hub.is_some() {
            return Err(ForwardError::BadRequest(
                "Policy discovery takes no hub".into(),
            ));
        }
        Vec::new()
    } else {
        get_event_store(election_event_id)
            .await?
            .voter_ballots(election_event_id, voter_id)
            .await?
    };
    let route = check_forward(
        &input.route,
        &input.body,
        &ballot_entries(election_event_id, &ballots)?,
    )?;
    let sidecar = sidecar.ok_or_else(|| anyhow!("HARVEST_VITRINE_INTERNAL_URL isn't set"))?;
    let answer = sidecar
        .voter(
            &log.sidecar_name,
            route.as_ref(),
            &input.body,
            input.hub.as_deref(),
            &format!("{tenant_id}/{voter_id}"),
        )
        .await?;
    Ok(VitrineForwardOutput {
        status: answer.status,
        body: Some(answer.body),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use electoral_log::adapters::ballot_box::BallotStatus;
    use electoral_log::vitrine::leaf_hash;

    const EVENT: &str = "e7a2b3c4-0000-4000-8000-000000000001";
    const MAYOR: &str = "E7A2B3C4-0000-4000-8000-000000000002";

    fn ballot(ballot_id: &str, accepted_at: i64) -> StoredBallot {
        StoredBallot {
            id: "b".into(),
            ballot_id: ballot_id.into(),
            election_id: MAYOR.into(),
            area_id: "a".into(),
            voter_id: "v".into(),
            status: BallotStatus::Valid,
            voting_channel: "ONLINE".into(),
            accepted_at,
        }
    }

    fn receipt(entry: &[u8]) -> String {
        format!(
            "c2sp.org/tlog-proof@v1\nextra {}\nindex 3\n",
            STANDARD.encode(entry)
        )
    }

    fn promise(entry: &[u8]) -> String {
        format!(
            "vitrine/promise/v1\nleaf {}\n",
            STANDARD.encode(leaf_hash(entry))
        )
    }

    #[test]
    fn authenticated_policy_discovery_needs_no_ballot_and_refuses_a_body() {
        assert_eq!(
            check_forward("policies", "", &[]).unwrap(),
            VoterRoute::Policies
        );
        assert!(matches!(
            check_forward("policies", "payload", &[]),
            Err(ForwardError::BadRequest(_))
        ));
        assert!(check_forward("../policies", "", &[]).is_err());
        assert!(check_forward("upgrade", "", &[]).is_err());
    }

    #[test]
    fn a_voters_entries_are_those_the_ballot_box_queued() {
        let mine = "ab".repeat(32);
        let entries = ballot_entries(
            EVENT,
            &[
                ballot(&mine, 1_793_520_123_000_000),
                ballot("not-a-hash", 1),
            ],
        )
        .unwrap();
        let expected = ElectionEntry {
            event_id: EVENT.into(),
            election_id: MAYOR.to_lowercase(),
            ballot_id: [0xab; 32],
            accepted_at: 1_793_520_123,
        }
        .encode()
        .unwrap();
        assert_eq!(entries, [expected]);
    }

    #[test]
    fn harvest_forwards_receipts_and_promises_for_the_voters_own_ballots() {
        let own = ballot_entries(EVENT, &[ballot(&"ab".repeat(32), 10_000_000)]).unwrap();
        let other = ballot_entries(EVENT, &[ballot(&"cd".repeat(32), 10_000_000)]).unwrap();
        assert_eq!(
            check_forward("upgrade", &receipt(&own[0]), &own).unwrap(),
            VoterRoute::Upgrade
        );
        assert_eq!(
            check_forward("opening", &promise(&own[0]), &own).unwrap(),
            VoterRoute::Opening
        );
        assert!(matches!(
            check_forward("upgrade", &receipt(&other[0]), &own),
            Err(ForwardError::Refused(_))
        ));
        assert!(matches!(
            check_forward("upgrade", &promise(&other[0]), &own),
            Err(ForwardError::Refused(_))
        ));
        assert!(matches!(
            check_forward("upgrade", &receipt(&own[0]), &[]),
            Err(ForwardError::Refused(_))
        ));
    }

    #[test]
    fn harvest_forwards_nothing_else() {
        let own = ballot_entries(EVENT, &[ballot(&"ab".repeat(32), 10_000_000)]).unwrap();
        for (route, body) in [
            ("policies", receipt(&own[0])),
            ("../admin/logs", receipt(&own[0])),
            ("upgrade", "garbage".to_string()),
            ("upgrade", "c2sp.org/tlog-proof@v1\nextra !!\n".to_string()),
            ("upgrade", String::new()),
        ] {
            assert!(
                matches!(
                    check_forward(route, &body, &own),
                    Err(ForwardError::BadRequest(_))
                ),
                "{route} {body:?}"
            );
        }
    }
}
