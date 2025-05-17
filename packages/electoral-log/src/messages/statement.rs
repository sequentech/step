// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use strum_macros::Display;

use crate::messages::newtypes::*;
use tracing::info;

#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Debug)]
pub struct Statement {
    pub head: StatementHead,
    pub body: StatementBody,
}
impl Statement {
    pub fn new(head: StatementHead, body: StatementBody) -> Statement {
        Statement { head, body }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Debug, Clone)]
pub struct StatementHead {
    pub event: EventIdString,
    pub kind: StatementType,
    pub timestamp: Timestamp,
    pub event_type: StatementEventType,
    pub log_type: StatementLogType,
    pub description: String,
}
impl StatementHead {
    pub fn from_body(event: EventIdString, body: &StatementBody) -> Self {
        let timestamp = crate::timestamp();
        let default_head = StatementHead {
            event,
            kind: StatementType::Unknown,
            timestamp,
            event_type: StatementEventType::SYSTEM,
            log_type: StatementLogType::INFO,
            description: "".to_string(),
        };

        match body {
            StatementBody::CastVote(_, _, _, _, _) => StatementHead {
                kind: StatementType::CastVote,
                description: "Inserted cast vote.".to_string(),
                ..default_head
            },
            StatementBody::CastVoteError(_, _, _, _, _) => StatementHead {
                kind: StatementType::CastVoteError,
                log_type: StatementLogType::ERROR,
                description: "Error inserting cast vote.".to_string(),
                ..default_head
            },
            StatementBody::ElectionPublish(_, _) => StatementHead {
                kind: StatementType::ElectionPublish,
                description: "Election published.".to_string(),
                ..default_head
            },
            StatementBody::ElectionVotingPeriodOpen(_, channel) => StatementHead {
                kind: StatementType::ElectionVotingPeriodOpen,
                description: format!(
                    "Election voting period opened for {channel} channel.",
                    channel = channel.0
                ),
                ..default_head
            },
            StatementBody::ElectionVotingPeriodPause(_, channel) => StatementHead {
                kind: StatementType::ElectionVotingPeriodPause,
                description: format!(
                    "Election voting period paused for {channel} channel.",
                    channel = channel.0
                ),
                ..default_head
            },
            StatementBody::ElectionVotingPeriodClose(_, channel) => StatementHead {
                kind: StatementType::ElectionVotingPeriodClose,
                description: format!(
                    "Election voting period closed for {channel} channel.",
                    channel = channel.0
                ),
                ..default_head
            },
            StatementBody::ElectionEventVotingPeriodOpen(_, _, channel) => StatementHead {
                kind: StatementType::ElectionEventVotingPeriodOpen,
                description: format!(
                    "Election-event voting period opened for {channel} channel.",
                    channel = channel.0
                ),
                ..default_head
            },
            StatementBody::ElectionEventVotingPeriodPause(_, channel) => StatementHead {
                kind: StatementType::ElectionEventVotingPeriodPause,
                description: format!(
                    "Election-event voting period paused for {channel} channel.",
                    channel = channel.0
                ),
                ..default_head
            },
            StatementBody::ElectionEventVotingPeriodClose(_, _, channel) => StatementHead {
                kind: StatementType::ElectionEventVotingPeriodClose,
                description: format!(
                    "Election-event voting period closed for {channel} channel.",
                    channel = channel.0
                ),
                ..default_head
            },
            StatementBody::KeyGeneration => StatementHead {
                kind: StatementType::KeyGeneration,
                description: "Creating keys ceremony.".to_string(),
                ..default_head
            },
            StatementBody::KeyInsertionStart => StatementHead {
                kind: StatementType::KeyInsertionStart,
                description: "Tally ceremony created.".to_string(),
                ..default_head
            },
            StatementBody::KeyInsertionCeremony(_) => StatementHead {
                kind: StatementType::KeyInsertionCeremony,
                description: "Trustees key restored.".to_string(),
                ..default_head
            },
            StatementBody::TallyOpen(_) => StatementHead {
                kind: StatementType::TallyOpen,
                description: "Tally session openned.".to_string(),
                ..default_head
            },
            StatementBody::TallyClose(_) => StatementHead {
                kind: StatementType::TallyClose,
                description: "Tally closed, session completed.".to_string(),
                ..default_head
            },
            StatementBody::SendTemplate => StatementHead {
                kind: StatementType::SendTemplate,
                description: "Template sent to user.".to_string(),
                ..default_head
            },
            StatementBody::SendCommunications(_) => StatementHead {
                kind: StatementType::SendCommunications,
                description: "Communication sent to user.".to_string(),
                ..default_head
            },
            StatementBody::KeycloakUserEvent(error_message_string, error_message_type) => {
                let description = if (error_message_string.0.trim() == "null")
                    || (error_message_string.0.trim().is_empty())
                {
                    format!("{}", error_message_type.0)
                } else {
                    format!("{}: {}", error_message_type.0, error_message_string.0)
                };

                StatementHead {
                    kind: StatementType::KeycloakUserEvent,
                    event_type: StatementEventType::USER,
                    description,
                    ..default_head
                }
            }
            StatementBody::VoterPublicKey(_, _, _, _) => StatementHead {
                kind: StatementType::VoterPublicKey,
                event_type: StatementEventType::USER,
                description: "Voter has public key.".to_string(),
                ..default_head
            },
            StatementBody::AdminPublicKey(_, _, _) => StatementHead {
                kind: StatementType::AdminPublicKey,
                description: "Admin has public key.".to_string(),
                ..default_head
            },
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Debug)]
pub enum StatementBody {
    // NOT IMPLEMENTED YET, but please feel free
    // "Emisión de voto (sólo como registro que el sistema almacenó correctamente el voto)
    CastVote(
        ElectionIdString,
        PseudonymHash,
        CastVoteHash,
        VoterIpString,
        VoterCountryString,
    ),
    // NOT IMPLEMENTED YET, but please feel free
    // "Errores en la emisión del voto."
    CastVoteError(
        ElectionIdString,
        PseudonymHash,
        CastVoteErrorString,
        VoterIpString,
        VoterCountryString,
    ),
    // /workspaces/step/packages/harvest/src/main.rs
    //    routes::ballot_publication::publish_ballot
    //
    // "Publicación, apertura y cierre de las elecciones"
    ElectionPublish(ElectionIdString, BallotPublicationIdString),
    // /workspaces/step/packages/harvest/src/main.rs
    //    routes::voting_status::update_event_status,
    //    routes::voting_status::update_election_status,
    //
    // "Publicación, apertura y cierre de las elecciones"
    ElectionVotingPeriodOpen(ElectionIdString, VotingChannelString),
    ElectionVotingPeriodPause(ElectionIdString, VotingChannelString),
    ElectionVotingPeriodClose(ElectionIdString, VotingChannelString),
    ElectionEventVotingPeriodOpen(EventIdString, ElectionsIdsString, VotingChannelString),
    ElectionEventVotingPeriodPause(EventIdString, VotingChannelString),
    ElectionEventVotingPeriodClose(EventIdString, ElectionsIdsString, VotingChannelString),
    // /workspaces/step/packages/windmill/src/celery_app.rs
    // create_keys
    //
    // "Creación de llave criptográfica"
    KeyGeneration,
    // /workspaces/step/packages/harvest/src/main.rs
    // routes::tally_ceremony::restore_private_key
    //
    // "Ingreso de los fragmentos de la llave privada"
    KeyInsertionStart,
    KeyInsertionCeremony(TrusteeNameString),
    // /workspaces/step/packages/windmill/src/celery_app.rs
    // tally_election_event
    //
    // "Apertura y cierre de la bóveda de votos"
    TallyOpen(ElectionIdString),
    // /workspaces/step/packages/windmill/src/celery_app.rs
    // execute_tally_session: falta que Felix ponga SUCCESS cuando se termine, creo, hablar con felix
    //
    // "Apertura y cierre de la bóveda de votos"
    TallyClose(ElectionIdString),

    SendTemplate,
    SendCommunications(Option<String>),
    KeycloakUserEvent(ErrorMessageString, KeycloakEventTypeString),
    /// Represents the assertion that
    ///     within the given tenant
    ///     within the given election event
    ///     the given user pseudonym hash
    ///     has as their public key the given public key (in der_b64 format)
    VoterPublicKey(
        TenantIdString,
        EventIdString,
        PseudonymHash,
        PublicKeyDerB64,
    ),
    /// Represents the assertion that
    ///     within the given tenant
    ///     the given admin user
    ///     hash has as their public key the given public key (in der_b64 format)
    AdminPublicKey(TenantIdString, Option<String>, PublicKeyDerB64),
}

impl StatementBody {
    pub fn election_id_string(&self) -> Option<String> {
        match self {
            StatementBody::CastVote(election_id, _, _, _, _) => election_id.0.clone(),
            StatementBody::CastVoteError(election_id, _, _, _, _) => election_id.0.clone(),
            StatementBody::ElectionPublish(election_id, _) => election_id.0.clone(),
            StatementBody::ElectionVotingPeriodOpen(election_id, _) => election_id.0.clone(),
            StatementBody::ElectionVotingPeriodPause(election_id, _) => election_id.0.clone(),
            StatementBody::ElectionVotingPeriodClose(election_id, _) => election_id.0.clone(),
            StatementBody::TallyOpen(election_id) => election_id.0.clone(),
            StatementBody::TallyClose(election_id) => election_id.0.clone(),

            // Variants that do not contain a singular ElectionIdString
            StatementBody::ElectionEventVotingPeriodOpen(_, _, _)
            | StatementBody::ElectionEventVotingPeriodPause(_, _)
            | StatementBody::ElectionEventVotingPeriodClose(_, _, _)
            | StatementBody::KeyGeneration
            | StatementBody::KeyInsertionStart
            | StatementBody::KeyInsertionCeremony(_)
            | StatementBody::SendTemplate
            | StatementBody::SendCommunications(_)
            | StatementBody::KeycloakUserEvent(_, _)
            | StatementBody::VoterPublicKey(_, _, _, _)
            | StatementBody::AdminPublicKey(_, _, _) => None,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Display, Deserialize, Serialize, Debug, Clone)]
pub enum StatementType {
    Unknown,
    CastVote,
    CastVoteError,
    ElectionPublish,
    ElectionVotingPeriodOpen,
    ElectionVotingPeriodClose,
    ElectionVotingPeriodPause,
    ElectionEventVotingPeriodOpen,
    ElectionEventVotingPeriodClose,
    ElectionEventVotingPeriodPause,
    KeyGeneration,
    KeyInsertionStart,
    KeyInsertionCeremony,
    TallyOpen,
    TallyClose,
    SendTemplate,
    SendCommunications,
    KeycloakUserEvent,
    VoterPublicKey,
    AdminPublicKey,
}

#[derive(BorshSerialize, BorshDeserialize, Display, Deserialize, Serialize, Debug, Clone)]
pub enum StatementEventType {
    USER,
    SYSTEM,
}

#[derive(BorshSerialize, BorshDeserialize, Display, Deserialize, Serialize, Debug, Clone)]
pub enum StatementLogType {
    INFO,
    ERROR,
}
