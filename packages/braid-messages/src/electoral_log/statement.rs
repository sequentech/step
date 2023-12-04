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
            StatementBody::CastVote(_) => StatementType::CastVote,
        };
        let timestamp = instant::now() as u64;

        StatementHead {
            context,
            kind,
            timestamp,
        }
    }
}

/*
• Emisión de voto (sólo como registro que el sistema almacenó correctamente el voto).
• Errores en la emisión del voto.
• Publicación, apertura y cierre de las elecciones.
• Creación de llave criptográfica.
• Ingreso de los fragmentos de la llave privada
• Apertura y cierre de la bóveda de votos
*/

#[derive(BorshSerialize, BorshDeserialize)]
pub enum StatementBody {
    CastVote(PseudonymHash)
}

#[derive(BorshSerialize, BorshDeserialize, Display)]
pub enum StatementType {
    CastVote
}