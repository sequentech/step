// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use strum::Display;

use crate::electoral_log::newtypes::*;

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

#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Debug)]
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
            event_type: StatementEventType::USER,
            log_type: StatementLogType::INFO,
            description: "".to_string(),
        };

        match body {
            StatementBody::CastVote(_, _, _, _, _) => StatementHead {
                kind: StatementType::CastVote,
                description: "Cast vote INFO event".to_string(),
                ..default_head
            },
            StatementBody::CastVoteError(_, _, _, _, _) => StatementHead {
                kind: StatementType::CastVoteError,
                log_type: StatementLogType::ERROR,
                description: "Cast vote ERROR event".to_string(),
                ..default_head
            },
            StatementBody::ElectionPublish(_, _) => StatementHead {
                kind: StatementType::ElectionPublish,
                description: "Election publish INFO event".to_string(),
                ..default_head
            },
            StatementBody::ElectionVotingPeriodOpen(_) => StatementHead {
                kind: StatementType::ElectionVotingPeriodOpen,
                description: "Election voting period open INFO event".to_string(),
                ..default_head
            },
            StatementBody::ElectionVotingPeriodPause(_) => StatementHead {
                kind: StatementType::ElectionVotingPeriodPause,
                description: "Election voting period pause INFO event".to_string(),
                ..default_head
            },
            StatementBody::ElectionVotingPeriodClose(_) => StatementHead {
                kind: StatementType::ElectionVotingPeriodClose,
                description: "Election voting period close INFO event".to_string(),
                ..default_head
            },
            StatementBody::ElectionEventVotingPeriodOpen(_, _) => StatementHead {
                kind: StatementType::ElectionEventVotingPeriodOpen,
                description: "Election event voting period open INFO event".to_string(),
                ..default_head
            },
            StatementBody::ElectionEventVotingPeriodPause(_) => StatementHead {
                kind: StatementType::ElectionEventVotingPeriodPause,
                description: "Election event voting period pause INFO event".to_string(),
                ..default_head
            },
            StatementBody::ElectionEventVotingPeriodClose(_, _) => StatementHead {
                kind: StatementType::ElectionEventVotingPeriodClose,
                description: "Election event voting period close INFO event".to_string(),
                ..default_head
            },
            StatementBody::KeyGeneration => StatementHead {
                kind: StatementType::KeyGeneration,
                description: "Key generation INFO event".to_string(),
                ..default_head
            },
            StatementBody::KeyInsertionStart => StatementHead {
                kind: StatementType::KeyInsertionStart,
                description: "Key insertion start INFO event".to_string(),
                ..default_head
            },
            StatementBody::KeyInsertionCeremony(_) => StatementHead {
                kind: StatementType::KeyInsertionCeremony,
                description: "Key insertion ceremony INFO event".to_string(),
                ..default_head
            },
            StatementBody::TallyOpen(_) => StatementHead {
                kind: StatementType::TallyOpen,
                description: "Tally open INFO event".to_string(),
                ..default_head
            },
            StatementBody::TallyClose(_) => StatementHead {
                kind: StatementType::TallyClose,
                description: "Tally close INFO event".to_string(),
                ..default_head
            },
            StatementBody::SendTemplate => StatementHead {
                kind: StatementType::SendTemplate,
                description: "Send template INFO event".to_string(),
                ..default_head
            },
            StatementBody::KeycloakUserEvent(_, _) => StatementHead {
                kind: StatementType::KeycloakUserEvent,
                description: "Keycloak user INFO event".to_string(),
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
    ElectionVotingPeriodOpen(ElectionIdString),
    ElectionVotingPeriodPause(ElectionIdString),
    ElectionVotingPeriodClose(ElectionIdString),
    ElectionEventVotingPeriodOpen(EventIdString, ElectionsIdsString),
    ElectionEventVotingPeriodPause(EventIdString),
    ElectionEventVotingPeriodClose(EventIdString, ElectionsIdsString),
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
    KeycloakUserEvent(ErrorMessageString, KeycloakEventTypeString),
}

#[derive(BorshSerialize, BorshDeserialize, Display, Deserialize, Serialize, Debug)]
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
    KeycloakUserEvent,
}

#[derive(BorshSerialize, BorshDeserialize, Display, Deserialize, Serialize, Debug)]
pub enum StatementEventType {
    USER,
    SYSTEM,
}

#[derive(BorshSerialize, BorshDeserialize, Display, Deserialize, Serialize, Debug)]
pub enum StatementLogType {
    INFO,
    ERROR,
}
