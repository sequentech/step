use borsh::{BorshSerialize, BorshDeserialize};
use strum::Display;

use crate::electoral_log::newtypes::*;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Statement {
    pub head: StatementHead,
    pub body: StatementBody,
}
impl Statement {
    pub fn new(head: StatementHead, body: StatementBody) -> Statement {
        Statement {
            head,
            body,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct StatementHead {
    pub context: ContextHash,
    pub kind: StatementType,
    pub timestamp: Timestamp
}
impl StatementHead {
    pub fn from_body(context: ContextHash, body: &StatementBody) -> Self {
        let kind = match body {
            StatementBody::CastVote(_, _, _) => StatementType::CastVote,
            StatementBody::CastVoteError(_, _) => StatementType::CastVoteError,
            StatementBody::ElectionPublish => StatementType::ElectionPublish,
            StatementBody::ElectionPeriodOpen => StatementType::ElectionPeriodOpen,
            StatementBody::ElectionPeriodClose => StatementType::ElectionPeriodClose,
            StatementBody::KeyGeneration => StatementType::KeyGeneration,
            StatementBody::KeyInsertionCeremony => StatementType::KeyInsertionCeremony,
            StatementBody::BallotBoxOpen => StatementType::BallotBoxOpen,
            StatementBody::BallotBoxClose => StatementType::BallotBoxClose,
        };
        let timestamp = instant::now() as u64;

        StatementHead {
            context,
            kind,
            timestamp,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum StatementBody {
    // "Emisión de voto (sólo como registro que el sistema almacenó correctamente el voto)"
    CastVote(ContestHash, PseudonymHash, CastVoteHash),
    // "Errores en la emisión del voto."
    CastVoteError(PseudonymHash, ContestHash),
    // "Publicación, apertura y cierre de las elecciones"
    ElectionPublish,
    // "Publicación, apertura y cierre de las elecciones"
    ElectionPeriodOpen,
    // "Publicación, apertura y cierre de las elecciones"
    ElectionPeriodClose,
    // "Creación de llave criptográfica"
    KeyGeneration,
    // "Ingreso de los fragmentos de la llave privada"
    KeyInsertionCeremony,
    // "Apertura y cierre de la bóveda de votos"
    BallotBoxOpen,
    // "Apertura y cierre de la bóveda de votos"
    BallotBoxClose,
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
    BallotBoxOpen,
    BallotBoxClose,
}