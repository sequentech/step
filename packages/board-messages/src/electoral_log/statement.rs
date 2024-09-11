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
}
impl StatementHead {
    pub fn from_body(event: EventIdString, body: &StatementBody) -> Self {
        let kind = match body {
            StatementBody::CastVote(_, _, _) => StatementType::CastVote,
            StatementBody::CastVoteError(_, _, _) => StatementType::CastVoteError,
            StatementBody::ElectionPublish(_, _) => StatementType::ElectionPublish,
            StatementBody::ElectionVotingPeriodOpen(_) => StatementType::ElectionVotingPeriodOpen,
            StatementBody::ElectionVotingPeriodPause(_) => StatementType::ElectionVotingPeriodPause,
            StatementBody::ElectionVotingPeriodClose(_) => StatementType::ElectionVotingPeriodClose,
            StatementBody::ElectionEventVotingPeriodOpen(_, _) => {
                StatementType::ElectionEventVotingPeriodOpen
            }
            StatementBody::ElectionEventVotingPeriodPause(_) => {
                StatementType::ElectionEventVotingPeriodPause
            }
            StatementBody::ElectionEventVotingPeriodClose(_, _) => {
                StatementType::ElectionEventVotingPeriodClose
            }
            StatementBody::KeyGeneration => StatementType::KeyGeneration,
            StatementBody::KeyInsertionStart => StatementType::KeyInsertionStart,
            StatementBody::KeyInsertionCeremony(_) => StatementType::KeyInsertionCeremony,
            StatementBody::TallyOpen(_) => StatementType::TallyOpen,
            StatementBody::TallyClose(_) => StatementType::TallyClose,
            StatementBody::SendCommunication => StatementType::SendCommunication,
            StatementBody::VoterRegistrationError(_) => StatementType::VoterRegistrationError,
        };
        let timestamp = crate::timestamp();

        StatementHead {
            event,
            kind,
            timestamp,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Debug)]
pub enum StatementBody {
    // NOT IMPLEMENTED YET, but please feel free
    // "Emisión de voto (sólo como registro que el sistema almacenó correctamente el voto)
    CastVote(ElectionIdString, PseudonymHash, CastVoteHash),
    // NOT IMPLEMENTED YET, but please feel free
    // "Errores en la emisión del voto."
    CastVoteError(ElectionIdString, PseudonymHash, CastVoteErrorString),
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

    SendCommunication,
    VoterRegistrationError(ErrorMessageString)
}

#[derive(BorshSerialize, BorshDeserialize, Display, Deserialize, Serialize, Debug)]
pub enum StatementType {
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
    SendCommunication,
    VoterRegistrationError,
}
