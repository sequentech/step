// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use immu_board::ElectoralLogMessage;
use serde::{Deserialize, Serialize};

use immu_board::BoardMessage;
use strand::serialization::StrandSerialize;
use strand::signature::StrandSignature;
use strand::signature::StrandSignaturePk;
use strand::signature::StrandSignatureSk;

use crate::electoral_log::statement::Statement;
use crate::electoral_log::statement::StatementBody;
use crate::electoral_log::statement::StatementHead;

use crate::electoral_log::newtypes::EventIdString;

use super::newtypes::*;

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, std::fmt::Debug)]
pub struct Message {
    pub sender: Sender,
    pub sender_signature: StrandSignature,
    pub system_signature: StrandSignature,
    pub statement: Statement,
    pub artifact: Option<Vec<u8>>,
    pub user_id: Option<String>,
}
impl Message {
    pub fn cast_vote_message(
        event: EventIdString,
        election: ElectionIdString,
        pseudonym_h: PseudonymHash,
        vote_h: CastVoteHash,
        sd: &SigningData,
    ) -> Result<Self> {
        let body = StatementBody::CastVote(election, pseudonym_h, vote_h);
        Self::from_body(event, body, sd, None)
    }

    pub fn cast_vote_error_message(
        event: EventIdString,
        election: ElectionIdString,
        pseudonym_h: PseudonymHash,
        error: CastVoteErrorString,
        sd: &SigningData,
    ) -> Result<Self> {
        let body = StatementBody::CastVoteError(election, pseudonym_h, error);
        Self::from_body(event, body, sd, None)
    }

    pub fn election_published_message(
        event: EventIdString,
        election: ElectionIdString,
        ballot_pub_id: BallotPublicationIdString,
        sd: &SigningData,
    ) -> Result<Self> {
        let body = StatementBody::ElectionPublish(election, ballot_pub_id);
        Self::from_body(event, body, sd, None)
    }

    pub fn election_open_message(
        event: EventIdString,
        election: Option<ElectionIdString>,
        election_ids: Option<Vec<String>>,
        sd: &SigningData,
    ) -> Result<Self> {
        match election {
            Some(election) => {
                let body = StatementBody::ElectionVotingPeriodOpen(election);
                Self::from_body(event, body, sd, None)
            }
            None => {
                let body = StatementBody::ElectionEventVotingPeriodOpen(
                    event.clone(),
                    ElectionsIdsString(election_ids.clone()),
                );
                Self::from_body(event, body, sd, None)
            }
        }
    }

    pub fn election_pause_message(
        event: EventIdString,
        election: Option<ElectionIdString>,
        sd: &SigningData,
    ) -> Result<Self> {
        match election {
            Some(election) => {
                let body = StatementBody::ElectionVotingPeriodPause(election);
                Self::from_body(event, body, sd, None)
            }
            None => {
                let body = StatementBody::ElectionEventVotingPeriodPause(event.clone());
                Self::from_body(event, body, sd, None)
            }
        }
    }

    pub fn election_close_message(
        event: EventIdString,
        election: Option<ElectionIdString>,
        election_ids: Option<Vec<String>>,
        sd: &SigningData,
    ) -> Result<Self> {
        match election {
            Some(election) => {
                let body = StatementBody::ElectionVotingPeriodClose(election);
                Self::from_body(event, body, sd, None)
            }
            None => {
                let body = StatementBody::ElectionEventVotingPeriodClose(
                    event.clone(),
                    ElectionsIdsString(election_ids.clone()),
                );
                Self::from_body(event, body, sd, None)
            }
        }
    }

    pub fn keycloak_user_event(
        event: EventIdString,
        event_type: KeycloakEventTypeString,
        error: ErrorMessageString,
        user_id: Option<String>,
        sd: &SigningData,
    ) -> Result<Self> {
        let body = StatementBody::KeycloakUserEvent(error, event_type);
        Self::from_body(event, body, sd, user_id)
    }

    pub fn keygen_message(event: EventIdString, sd: &SigningData) -> Result<Self> {
        let body = StatementBody::KeyGeneration;
        Self::from_body(event, body, sd, None)
    }

    pub fn key_insertion_start(event: EventIdString, sd: &SigningData) -> Result<Self> {
        let body = StatementBody::KeyInsertionStart;
        Self::from_body(event, body, sd, None)
    }

    pub fn key_insertion_message(
        event: EventIdString,
        trustee_name: TrusteeNameString,
        sd: &SigningData,
    ) -> Result<Self> {
        let body = StatementBody::KeyInsertionCeremony(trustee_name);
        Self::from_body(event, body, sd, None)
    }

    pub fn tally_open_message(
        event: EventIdString,
        election: ElectionIdString,
        sd: &SigningData,
    ) -> Result<Self> {
        let body = StatementBody::TallyOpen(election);
        Self::from_body(event, body, sd, None)
    }

    pub fn tally_close_message(
        event: EventIdString,
        election: ElectionIdString,
        sd: &SigningData,
    ) -> Result<Self> {
        let body = StatementBody::TallyClose(election);
        Self::from_body(event, body, sd, None)
    }

    pub fn send_template(
        event: EventIdString,
        election: ElectionIdString,
        sd: &SigningData,
    ) -> Result<Self> {
        let body = StatementBody::SendTemplate;
        Self::from_body(event, body, sd, None)
    }

    fn from_body(
        event: EventIdString,
        body: StatementBody,
        sd: &SigningData,
        user_id: Option<String>,
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
        )
    }

    pub fn sign(
        statement: Statement,
        artifact: Option<Vec<u8>>,
        sender_sk: &StrandSignatureSk,
        sender_name: &str,
        system_sk: &StrandSignatureSk,
        user_id: Option<String>,
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
        })
    }

    pub fn verify(&self, system_pk: &StrandSignaturePk) -> Result<()> {
        let bytes = self.statement.strand_serialize()?;
        self.sender.pk.verify(&self.sender_signature, &bytes)?;
        system_pk.verify(&self.system_signature, &bytes)?;

        Ok(())
    }
}

impl TryFrom<Message> for BoardMessage {
    type Error = anyhow::Error;

    fn try_from(message: Message) -> Result<BoardMessage> {
        Ok(BoardMessage {
            id: 0,
            created: crate::timestamp() as i64,
            statement_timestamp: message.statement.head.timestamp as i64,
            statement_kind: message.statement.head.kind.to_string(),
            message: message.strand_serialize()?,
            sender_pk: message.sender.pk.to_der_b64_string()?,
            version: crate::getSchemaVersion(),
        })
    }
}

impl TryFrom<Message> for ElectoralLogMessage {
    type Error = anyhow::Error;

    fn try_from(message: Message) -> Result<ElectoralLogMessage> {
        Ok(ElectoralLogMessage {
            id: 0,
            created: crate::timestamp() as i64,
            statement_timestamp: message.statement.head.timestamp as i64,
            statement_kind: message.statement.head.kind.to_string(),
            message: message.strand_serialize()?,
            sender_pk: message.sender.pk.to_der_b64_string()?,
            version: crate::getSchemaVersion(),
            user_id: message.user_id,
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
