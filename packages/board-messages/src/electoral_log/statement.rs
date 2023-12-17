use borsh::{BorshDeserialize, BorshSerialize};
use strum::Display;

use crate::electoral_log::newtypes::*;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Statement {
    pub head: StatementHead,
    pub body: StatementBody,
}
impl Statement {
    pub fn new(head: StatementHead, body: StatementBody) -> Statement {
        Statement { head, body }
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct StatementHead {
    pub event: EventIdString,
    pub kind: StatementType,
    pub timestamp: Timestamp,
}
impl StatementHead {
    pub fn from_body(event: EventIdString, body: &StatementBody) -> Self {
        let kind = match body {
            StatementBody::CastVote(_, _, _) => StatementType::CastVote,
            StatementBody::CastVoteError(_, _) => StatementType::CastVoteError,
            StatementBody::ElectionPublish(_, _) => StatementType::ElectionPublish,
            StatementBody::ElectionPeriodOpen(_) => StatementType::ElectionPeriodOpen,
            StatementBody::ElectionPeriodClose(_) => StatementType::ElectionPeriodClose,
            StatementBody::KeyGeneration => StatementType::KeyGeneration,
            StatementBody::KeyInsertionCeremony => StatementType::KeyInsertionCeremony,
            StatementBody::TallyOpen(_) => StatementType::TallyOpen,
            StatementBody::TallyClose(_) => StatementType::TallyClose,
        };
        let timestamp = instant::now() as u64;

        StatementHead {
            event,
            kind,
            timestamp,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum StatementBody {
    // NOT IMPLEMENTED YET, but please feel free
    // "Emisión de voto (sólo como registro que el sistema almacenó correctamente el voto)
    CastVote(ElectionIdString, PseudonymHash, CastVoteHash),
    // NOT IMPLEMENTED YET, but please feel free
    // "Errores en la emisión del voto."
    CastVoteError(PseudonymHash, ElectionIdString),
    // /workspaces/backend-services/packages/harvest/src/main.rs
    //    routes::ballot_publication::publish_ballot
    //
    // "Publicación, apertura y cierre de las elecciones"
    ElectionPublish(ElectionIdString, BallotPublicationIdString),
    // /workspaces/backend-services/packages/harvest/src/main.rs
    //    routes::voting_status::update_event_status,
    //    routes::voting_status::update_election_status,
    //
    // "Publicación, apertura y cierre de las elecciones"
    ElectionPeriodOpen(ElectionIdString),
    // /workspaces/backend-services/packages/harvest/src/main.rs
    //    routes::voting_status::update_event_status,
    //    routes::voting_status::update_election_status,
    //
    // "Publicación, apertura y cierre de las elecciones"
    ElectionPeriodClose(ElectionIdString),
    // /workspaces/backend-services/packages/windmill/src/celery_app.rs
    // create_keys
    //
    // "Creación de llave criptográfica"
    KeyGeneration,
    // /workspaces/backend-services/packages/harvest/src/main.rs
    // routes::tally_ceremony::restore_private_key
    //
    // "Ingreso de los fragmentos de la llave privada"
    KeyInsertionCeremony,
    // /workspaces/backend-services/packages/windmill/src/celery_app.rs
    // tally_election_event
    //
    // "Apertura y cierre de la bóveda de votos"
    TallyOpen(ElectionIdString),
    // /workspaces/backend-services/packages/windmill/src/celery_app.rs
    // execute_tally_session: falta que Felix ponga SUCCESS cuando se termine, creo, hablar con felix
    //
    // "Apertura y cierre de la bóveda de votos"
    TallyClose(ElectionIdString),
}

#[derive(BorshSerialize, BorshDeserialize, Display)]
pub enum StatementType {
    CastVote,
    CastVoteError,
    ElectionPublish,
    ElectionPeriodOpen,
    ElectionPeriodClose,
    KeyGeneration,
    KeyInsertionCeremony,
    TallyOpen,
    TallyClose,
}
