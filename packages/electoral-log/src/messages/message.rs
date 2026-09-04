// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ElectoralLogMessage;
use anyhow::Result;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use strand::hash::STRAND_HASH_LENGTH_BYTES;
use strand::serialization::StrandSerialize;
use strand::signature::StrandSignature;
use strand::signature::StrandSignaturePk;
use strand::signature::StrandSignatureSk;
use tracing::instrument;

use crate::messages::statement::Statement;
use crate::messages::statement::StatementBody;
use crate::messages::statement::StatementHead;

use super::newtypes::*;
use crate::messages::newtypes::{
    CertificateAuthEventAction, CertificateSubjectDnsString, EventIdString,
};
use std::fmt;

/// We use this when the statement is not related to any election event
/// For the moment the only case is admin_public_key_message, which is
/// a cross-event statement
pub const GENERIC_EVENT: &'static str = "Generic Event";

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, std::fmt::Debug)]
pub struct Message {
    pub sender: Sender,
    pub sender_signature: StrandSignature,
    pub system_signature: StrandSignature,
    pub statement: Statement,
    pub artifact: Option<Vec<u8>>,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub election_id: Option<String>,
    pub area_id: Option<String>,
    pub ballot_id: Option<String>,
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match serde_json::to_string(self) {
            Ok(json_str) => write!(f, "{}", json_str),
            Err(_) => Err(fmt::Error),
        }
    }
}

impl Message {
    #[instrument(skip_all, err)]
    pub fn external_api_request_message(
        event_id: EventIdString,
        election_id: ElectionIdString,
        sd: &SigningData,
        voter_id: Option<String>,
        voter_username: Option<String>,
        direction: ExtApiRequestDirection,
        api_name: ExtApiName,
        operation: String,
    ) -> Result<Self> {
        let subject = ExternalApiSubject {
            user_id: voter_id.clone(),
            username: voter_username.clone(),
        };
        let body = StatementBody::ExternalApiRequest(
            event_id.clone(),
            subject,
            direction,
            api_name,
            operation,
        );
        Self::from_body(
            event_id,
            body,
            sd,
            voter_id.clone(),
            voter_username.clone(), /* username */
            election_id.0,
            None,
            None,
        )
    }
    /// Records a third-party voter registry reconciliation run event (patch
    /// generation or applying the Sequent-side diff) — see
    /// `StatementBody::ExternalReconciliation`. Named for the general
    /// capability, not the specific integration (Datafix) that first needed
    /// it.
    #[instrument(skip_all, err)]
    pub fn external_reconciliation_message(
        event_id: EventIdString,
        kind: ExternalReconciliationKind,
        sequence: ExternalReconciliationSequenceString,
        generated_at: ExternalReconciliationGeneratedAtString,
        input_hash: ExternalReconciliationInputHashString,
        output_hash: ExternalReconciliationOutputHashString,
        sd: &SigningData,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<Self> {
        let body = StatementBody::ExternalReconciliation(
            event_id.clone(),
            kind,
            sequence,
            generated_at,
            input_hash,
            output_hash,
        );

        // A reconciliation run is event-wide, so no election, area or ballot
        // id is attached.
        Message::from_body(event_id, body, sd, user_id, username, None, None, None)
    }

    pub fn cast_vote_message(
        event: EventIdString,
        election: ElectionIdString,
        pseudonym_h: PseudonymHash,
        vote_h: CastVoteHash,
        sd: &SigningData,
        ip: VoterIpString,
        country: VoterCountryString,
        voter_id: Option<String>,
        voter_username: Option<String>,
        area_id: String,
    ) -> Result<Self> {
        let body =
            StatementBody::CastVote(election.clone(), pseudonym_h, vote_h.clone(), ip, country);
        Self::cast_vote_message_from_body(
            event,
            election,
            vote_h,
            body,
            sd,
            voter_id,
            voter_username,
            area_id,
        )
    }

    pub fn cast_vote_with_channel_message(
        event: EventIdString,
        election: ElectionIdString,
        pseudonym_h: PseudonymHash,
        vote_h: CastVoteHash,
        sd: &SigningData,
        ip: VoterIpString,
        country: VoterCountryString,
        voting_channel: VotingChannelString,
        voter_id: Option<String>,
        voter_username: Option<String>,
        area_id: String,
    ) -> Result<Self> {
        let body = StatementBody::CastVoteWithChannel(
            election.clone(),
            pseudonym_h,
            vote_h.clone(),
            ip,
            country,
            voting_channel,
        );
        Self::cast_vote_message_from_body(
            event,
            election,
            vote_h,
            body,
            sd,
            voter_id,
            voter_username,
            area_id,
        )
    }

    fn cast_vote_message_from_body(
        event: EventIdString,
        election: ElectionIdString,
        vote_h: CastVoteHash,
        body: StatementBody,
        sd: &SigningData,
        voter_id: Option<String>,
        voter_username: Option<String>,
        area_id: String,
    ) -> Result<Self> {
        let ballot_id: String = vote_h
            .0
            .into_inner()
            .iter()
            .take(STRAND_HASH_LENGTH_BYTES / 2)
            .map(|b| format!("{:02x}", b))
            .collect();

        Self::from_body(
            event,
            body,
            sd,
            voter_id.clone(),
            voter_username.clone(), /* username */
            election.0,
            Some(area_id),
            Some(ballot_id),
        )
    }

    pub fn cast_vote_error_message(
        event: EventIdString,
        election: ElectionIdString,
        pseudonym_h: PseudonymHash,
        error: CastVoteErrorString,
        sd: &SigningData,
        ip: VoterIpString,
        country: VoterCountryString,
        voter_id: Option<String>,
        area_id: String,
    ) -> Result<Self> {
        let body = StatementBody::CastVoteError(election.clone(), pseudonym_h, error, ip, country);
        Self::from_body(
            event,
            body,
            sd,
            voter_id,
            None, /* username */
            election.0,
            Some(area_id),
            None,
        )
    }

    pub fn election_published_message(
        event: EventIdString,
        election: ElectionIdString,
        ballot_pub_id: BallotPublicationIdString,
        sd: &SigningData,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<Self> {
        let body = StatementBody::ElectionPublish(election.clone(), ballot_pub_id);
        Self::from_body(event, body, sd, user_id, username, election.0, None, None)
    }

    pub fn election_open_message(
        event: EventIdString,
        election: Option<ElectionIdString>,
        election_ids: Option<Vec<String>>,
        voting_channel: VotingChannelString,
        sd: &SigningData,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<Self> {
        match election {
            Some(election) => {
                let body =
                    StatementBody::ElectionVotingPeriodOpen(election.clone(), voting_channel);
                Self::from_body(event, body, sd, user_id, username, election.0, None, None)
            }
            None => {
                let body = StatementBody::ElectionEventVotingPeriodOpen(
                    event.clone(),
                    ElectionsIdsString(election_ids.clone()),
                    voting_channel,
                );
                Self::from_body(event, body, sd, user_id, username, None, None, None)
            }
        }
    }

    pub fn election_pause_message(
        event: EventIdString,
        election: Option<ElectionIdString>,
        voting_channel: VotingChannelString,
        sd: &SigningData,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<Self> {
        match election {
            Some(election) => {
                let body =
                    StatementBody::ElectionVotingPeriodPause(election.clone(), voting_channel);
                Self::from_body(event, body, sd, user_id, username, election.0, None, None)
            }
            None => {
                let body =
                    StatementBody::ElectionEventVotingPeriodPause(event.clone(), voting_channel);
                Self::from_body(event, body, sd, user_id, username, None, None, None)
            }
        }
    }

    pub fn election_close_message(
        event: EventIdString,
        election: Option<ElectionIdString>,
        election_ids: Option<Vec<String>>,
        voting_channel: VotingChannelString,
        sd: &SigningData,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<Self> {
        match election {
            Some(election) => {
                let body =
                    StatementBody::ElectionVotingPeriodClose(election.clone(), voting_channel);
                Self::from_body(event, body, sd, user_id, username, election.0, None, None)
            }
            None => {
                let body = StatementBody::ElectionEventVotingPeriodClose(
                    event.clone(),
                    ElectionsIdsString(election_ids.clone()),
                    voting_channel,
                );
                Self::from_body(event, body, sd, user_id, username, None, None, None)
            }
        }
    }

    pub fn keycloak_user_event(
        event: EventIdString,
        event_type: KeycloakEventTypeString,
        error: ErrorMessageString,
        user_id: Option<String>,
        username: Option<String>,
        sd: &SigningData,
        area_id: Option<String>,
    ) -> Result<Self> {
        let body = StatementBody::KeycloakUserEvent(error, event_type);
        Self::from_body(event, body, sd, user_id, username, None, area_id, None)
    }

    pub fn keygen_message(
        event: EventIdString,
        sd: &SigningData,
        user_id: Option<String>,
        username: Option<String>,
        election_id: Option<String>,
    ) -> Result<Self> {
        let body = StatementBody::KeyGeneration;
        Self::from_body(event, body, sd, user_id, username, election_id, None, None)
    }

    pub fn key_insertion_start(
        event: EventIdString,
        sd: &SigningData,
        user_id: Option<String>,
        username: Option<String>,
        elections_ids: Option<String>,
    ) -> Result<Self> {
        let body = StatementBody::KeyInsertionStart;
        Self::from_body(
            event,
            body,
            sd,
            user_id,
            username,
            elections_ids,
            None,
            None,
        )
    }

    pub fn key_insertion_message(
        event: EventIdString,
        trustee_name: TrusteeNameString,
        sd: &SigningData,
        user_id: Option<String>,
        username: Option<String>,
        elections_ids: Option<String>,
    ) -> Result<Self> {
        let body = StatementBody::KeyInsertionCeremony(trustee_name);
        Self::from_body(
            event,
            body,
            sd,
            user_id,
            username,
            elections_ids,
            None,
            None,
        )
    }

    pub fn tally_open_message(
        event: EventIdString,
        election: ElectionIdString,
        sd: &SigningData,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<Self> {
        let body = StatementBody::TallyOpen(election.clone());
        Self::from_body(event, body, sd, user_id, username, election.0, None, None)
    }

    pub fn tally_close_message(
        event: EventIdString,
        election: ElectionIdString,
        sd: &SigningData,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<Self> {
        let body = StatementBody::TallyClose(election.clone());
        Self::from_body(event, body, sd, user_id, username, election.0, None, None)
    }

    pub fn phone_blacklist_entry_created_message(
        event: EventIdString,
        phone: PhoneE164String,
        sd: &SigningData,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<Self> {
        let body = StatementBody::PhoneBlacklistUpdated(phone, PhoneBlacklistAction::CreateEntry);
        Self::from_body(event, body, sd, user_id, username, None, None, None)
    }

    pub fn phone_blacklist_entry_deleted_message(
        event: EventIdString,
        phone: PhoneE164String,
        sd: &SigningData,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<Self> {
        let body = StatementBody::PhoneBlacklistUpdated(phone, PhoneBlacklistAction::DeleteEntry);
        Self::from_body(event, body, sd, user_id, username, None, None, None)
    }

    pub fn results_publication_action_message(
        event: EventIdString,
        details: ResultsPublicationDetails,
        sd: &SigningData,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<Self> {
        let election_id = details.route_election_id.0.clone();
        let body = StatementBody::ResultsPublicationAction(details);
        Self::from_body(event, body, sd, user_id, username, election_id, None, None)
    }

    pub fn send_template(
        event: EventIdString,
        _election: ElectionIdString,
        sd: &SigningData,
        user_id: Option<String>,
        username: Option<String>,
        message: Option<String>,
        area_id: Option<String>,
    ) -> Result<Self> {
        let body = StatementBody::SendCommunications(message);
        Self::from_body(event, body, sd, user_id, username, None, area_id, None)
    }

    pub fn voter_public_key_message(
        tenant_id: TenantIdString,
        event: EventIdString,
        user_hash: PseudonymHash,
        pk: PublicKeyDerB64,
        sd: &SigningData,
        user_id: Option<String>,
        username: Option<String>,
        area_id: Option<String>,
    ) -> Result<Self> {
        let body = StatementBody::VoterPublicKey(tenant_id, event.clone(), user_hash, pk);
        Self::from_body(event, body, sd, user_id, username, None, area_id, None)
    }

    pub fn tally_resumed_with_resolution(
        event: EventIdString,
        election: ElectionIdString,
        resolution_ids: Vec<String>,
        sd: &SigningData,
    ) -> Result<Self> {
        let body = StatementBody::TallyResumedWithResolution(
            election.clone(),
            ResolutionIdsString(resolution_ids),
        );
        Self::from_body(event, body, sd, None, None, election.0, None, None)
    }

    pub fn tally_paused_pending_resolutions(
        event: EventIdString,
        election: ElectionIdString,
        resolution_ids: Vec<String>,
        sd: &SigningData,
    ) -> Result<Self> {
        let body = StatementBody::TallyPausedPendingResolution(
            election.clone(),
            ResolutionIdsString(resolution_ids),
        );
        Self::from_body(event, body, sd, None, None, election.0, None, None)
    }

    pub fn tally_tie_resolved(
        event: EventIdString,
        election: ElectionIdString,
        contest: ContestIdString,
        resolution_id: String,
        sd: &SigningData,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<Self> {
        let body = StatementBody::TallyTieResolved(
            election.clone(),
            contest,
            ResolutionIdsString(vec![resolution_id]),
        );
        Self::from_body(event, body, sd, user_id, username, election.0, None, None)
    }

    pub fn tally_tie_resolution_updated(
        event: EventIdString,
        election: ElectionIdString,
        contest: ContestIdString,
        resolution_id: String,
        sd: &SigningData,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<Self> {
        let body = StatementBody::TallyTieResolutionUpdated(
            election.clone(),
            contest,
            ResolutionIdsString(vec![resolution_id]),
        );
        Self::from_body(event, body, sd, user_id, username, election.0, None, None)
    }

    pub fn certificate_auth_event_message(
        event: EventIdString,
        action: CertificateAuthEventAction,
        subject_dns: Vec<String>,
        sd: &SigningData,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<Self> {
        let subjects = CertificateSubjectDnsString(subject_dns);
        let body = StatementBody::CertificateAuthEvent(action, subjects);
        Self::from_body(event, body, sd, user_id, username, None, None, None)
    }

    pub fn admin_public_key_message(
        tenant_id: TenantIdString,
        user_id: Option<String>,
        username: Option<String>,
        pk: PublicKeyDerB64,
        sd: &SigningData,
        elections_ids: Option<String>,
        area_id: Option<String>,
    ) -> Result<Self> {
        let body = StatementBody::AdminPublicKey(tenant_id, user_id.clone(), pk);
        let event = EventIdString(GENERIC_EVENT.to_string());

        Self::from_body(
            event,
            body,
            sd,
            user_id,
            username,
            elections_ids,
            area_id,
            None,
        )
    }

    fn from_body(
        event: EventIdString,
        body: StatementBody,
        sd: &SigningData,
        user_id: Option<String>,
        username: Option<String>,
        election_id: Option<String>,
        area_id: Option<String>,
        ballot_id: Option<String>,
    ) -> Result<Self> {
        let head = StatementHead::from_body(event, &body);
        let statement = Statement::new(head, body);

        Message::sign(
            statement,
            None,
            &sd.sender_sk,
            &sd.sender_name,
            &sd.system_sk,
            user_id,
            username,
            election_id,
            area_id,
            ballot_id,
        )
    }

    pub fn sign(
        statement: Statement,
        artifact: Option<Vec<u8>>,
        sender_sk: &StrandSignatureSk,
        sender_name: &str,
        system_sk: &StrandSignatureSk,
        user_id: Option<String>,
        username: Option<String>,
        election_id: Option<String>,
        area_id: Option<String>,
        ballot_id: Option<String>,
    ) -> Result<Message> {
        let bytes = statement.strand_serialize()?;
        let sender_signature: StrandSignature = sender_sk.sign(&bytes)?;
        let system_signature: StrandSignature = system_sk.sign(&bytes)?;
        let sender_pk = StrandSignaturePk::from_sk(&sender_sk)?;
        let sender = Sender::new(sender_name.to_string(), sender_pk);

        Ok(Message {
            sender,
            sender_signature,
            system_signature,
            statement,
            artifact,
            user_id,
            username,
            election_id,
            area_id,
            ballot_id,
        })
    }

    pub fn verify(&self, system_pk: &StrandSignaturePk) -> Result<()> {
        let bytes = self.statement.strand_serialize()?;
        self.sender.pk.verify(&self.sender_signature, &bytes)?;
        system_pk.verify(&self.system_signature, &bytes)?;

        Ok(())
    }
}

impl TryFrom<&Message> for ElectoralLogMessage {
    type Error = anyhow::Error;

    fn try_from(message: &Message) -> Result<ElectoralLogMessage> {
        let version = match &message.statement.body {
            StatementBody::CastVoteWithChannel(_, _, _, _, _, _) => {
                crate::get_cast_vote_channel_schema_version()
            }
            _ => crate::get_schema_version(),
        };

        Ok(ElectoralLogMessage {
            id: 0,
            created: crate::timestamp() as i64,
            statement_timestamp: message.statement.head.timestamp as i64,
            statement_kind: message.statement.head.kind.to_string(),
            message: message.strand_serialize()?,
            sender_pk: message.sender.pk.to_der_b64_string()?,
            version,
            user_id: message.user_id.clone(),
            username: message.username.clone(),
            election_id: message.election_id.clone(),
            area_id: message.area_id.clone(),
            ballot_id: message.ballot_id.clone(),
        })
    }
}

#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, std::fmt::Debug)]
pub struct Sender {
    pub name: String,
    pub pk: StrandSignaturePk,
}
impl Sender {
    pub fn new(name: String, pk: StrandSignaturePk) -> Sender {
        Sender { name, pk }
    }
}

pub struct SigningData {
    sender_sk: StrandSignatureSk,
    sender_name: String,
    system_sk: StrandSignatureSk,
}
impl SigningData {
    pub fn new(
        sender_sk: StrandSignatureSk,
        sender_name: &str,
        system_sk: StrandSignatureSk,
    ) -> SigningData {
        SigningData {
            sender_sk,
            sender_name: sender_name.to_string(),
            system_sk,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results_publication_message_keeps_actor_and_action_details() -> Result<()> {
        let signing_data = SigningData::new(
            StrandSignatureSk::generate()?,
            "admin",
            StrandSignatureSk::generate()?,
        );
        let details = ResultsPublicationDetails {
            publication_id: ResultsPublicationIdString("publication-id".to_string()),
            action: ResultsPublicationAction::Revoke,
            route_scope: ResultsPublicationRouteScopeString("event".to_string()),
            route_election_id: ElectionIdString(None),
            access: ResultsPublicationAccessString("public".to_string()),
            visibility_scope: ResultsPublicationVisibilityScopeString("full_event".to_string()),
            contest_ids: vec![ContestIdString("contest-id".to_string())],
        };

        let message = Message::results_publication_action_message(
            EventIdString("event-id".to_string()),
            details,
            &signing_data,
            Some("user-id".to_string()),
            Some("username".to_string()),
        )?;

        assert_eq!(message.user_id.as_deref(), Some("user-id"));
        assert_eq!(message.username.as_deref(), Some("username"));
        assert!(matches!(
            message.statement.body,
            StatementBody::ResultsPublicationAction(ResultsPublicationDetails {
                action: ResultsPublicationAction::Revoke,
                ..
            })
        ));
        Ok(())
    }

    #[test]
    fn only_channel_aware_cast_votes_use_schema_version_two() -> Result<()> {
        let signing_data = SigningData::new(
            StrandSignatureSk::generate()?,
            "windmill",
            StrandSignatureSk::generate()?,
        );
        let legacy = Message::cast_vote_message(
            EventIdString("event-id".to_string()),
            ElectionIdString(Some("election-id".to_string())),
            PseudonymHash::new([1; 64]),
            CastVoteHash::new([2; 64]),
            &signing_data,
            VoterIpString("ip".to_string()),
            VoterCountryString("country".to_string()),
            Some("voter-id".to_string()),
            None,
            "area-id".to_string(),
        )?;
        let with_channel = Message::cast_vote_with_channel_message(
            EventIdString("event-id".to_string()),
            ElectionIdString(Some("election-id".to_string())),
            PseudonymHash::new([1; 64]),
            CastVoteHash::new([2; 64]),
            &signing_data,
            VoterIpString("ip".to_string()),
            VoterCountryString("country".to_string()),
            VotingChannelString("TELEPHONE".to_string()),
            Some("voter-id".to_string()),
            None,
            "area-id".to_string(),
        )?;

        let legacy_row: ElectoralLogMessage = (&legacy).try_into()?;
        let with_channel_row: ElectoralLogMessage = (&with_channel).try_into()?;
        assert_eq!(legacy_row.version, "1");
        assert_eq!(with_channel_row.version, "2");
        assert_eq!(
            with_channel.statement.head.description,
            "Inserted cast vote. Voting channel: TELEPHONE."
        );
        Ok(())
    }
}
