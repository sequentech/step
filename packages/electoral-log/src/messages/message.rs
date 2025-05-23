// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ElectoralLogMessage;
use anyhow::Result;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use strand::serialization::StrandSerialize;
use strand::signature::StrandSignature;
use strand::signature::StrandSignaturePk;
use strand::signature::StrandSignatureSk;

use crate::messages::statement::Statement;
use crate::messages::statement::StatementBody;
use crate::messages::statement::StatementHead;

use super::newtypes::*;
use crate::messages::newtypes::EventIdString;
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
        let ballot_id: String = vote_h
            .0
            .into_inner()
            .iter()
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
        let body = StatementBody::TallyClose(election);
        Self::from_body(event, body, sd, user_id, username, None, None, None)
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
        Ok(ElectoralLogMessage {
            id: 0,
            created: crate::timestamp() as i64,
            statement_timestamp: message.statement.head.timestamp as i64,
            statement_kind: message.statement.head.kind.to_string(),
            message: message.strand_serialize()?,
            sender_pk: message.sender.pk.to_der_b64_string()?,
            version: crate::get_schema_version(),
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
