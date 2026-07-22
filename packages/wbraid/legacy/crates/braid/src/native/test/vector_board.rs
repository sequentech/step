// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use b4::{
    HttpB4Message,
    messages::{message::Message, statement::StatementType},
};
use cryptography::utils::serialization::variable::{VSerializable, VDeserializable};

/// VectorBoard
///
/// A vector backed dummy implementation for in memory testing.
pub struct VectorBoard {
    session_id: u128,
    pub(crate) messages: Vec<HttpB4Message>,
}

impl Clone for VectorBoard {
    fn clone(&self) -> Self {
        let mut ms = vec![];
        for m in &self.messages {
            ms.push(m.clone());
        }
        VectorBoard {
            session_id: self.session_id,
            messages: ms,
        }
    }
}

impl VectorBoard {
    pub fn new(session_id: u128) -> VectorBoard {
        let messages = Vec::new();

        VectorBoard {
            session_id,
            messages,
        }
    }

    pub fn add<C: cryptography::context::Context>(&mut self, message: Message<C>) {
        let last_id: i64 = self.messages.len() as i64;
        let m = message.ser();
        
        self.messages.push(HttpB4Message::new(
            last_id,
            m,
            "".to_string(),
        ));
    }

    pub fn get(&self, last_message: i64) -> Vec<HttpB4Message> {
        let next: usize = (last_message + 1) as usize;

        let mut ret = vec![];
        let slice = &self.messages[next..self.messages.len()];
        for m in slice {
            ret.push(m.clone());
        }

        ret

        // self.messages[next..self.messages.len()].to_vec()
    }
}

///////////////////////////////////////////////////////////////////////////
// Debug
///////////////////////////////////////////////////////////////////////////

impl std::fmt::Debug for VectorBoard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let types: Vec<(StatementType, bool)> = self
            .messages
            .iter()
            .map(|m| Message::<cryptography::context::RistrettoCtx>::deser(&m.message).unwrap())
            .map(|m| (m.statement.get_kind(), m.artifact.is_some()))
            .collect();
        write!(
            f,
            "VectorBoard{{session_id={} messages={:?} }}",
            self.session_id, types
        )
    }
}
