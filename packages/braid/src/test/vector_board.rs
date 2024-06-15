// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use board_messages::braid::{message::Message, statement::StatementType};

///////////////////////////////////////////////////////////////////////////
// VectorBoard
//
// A vector backed dummy implementation for in memory testing.
///////////////////////////////////////////////////////////////////////////

/*
Persistence implementation:

* Add a message counter, starts at zero when constructing

* Step procedure

1) Retrieve messages starting at counter
2) Verify messages
3) Save messages in kv
4) Trustee update
5) Increment counter

* When constructing:
1) Read all persistent messages and store them in memory
2) Return persistent messages + updated messages in next Trustee update
3) Clear persistent messages from memory

*/

// #[derive(Clone)]
pub struct VectorBoard {
    session_id: u128,
    pub(crate) messages: Vec<(Message, i64)>,
}

impl Clone for VectorBoard {
    fn clone(&self) -> Self {
        let mut ms = vec![];
        for (m, id) in &self.messages {
            ms.push((m.try_clone().unwrap(), id.clone()));
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

    pub fn add(&mut self, message: Message) {
        let last_id: i64= self.messages.len() as i64;
        self.messages.push((message, last_id + 1));
    }

    pub fn get(&self, last_message: i64) -> Vec<(Message, i64)> {
        let next: usize = (last_message + 1) as usize;

        let mut ret = vec![];
        let slice = &self.messages[next..self.messages.len()];
        for (m, id) in slice {
            ret.push((m.try_clone().unwrap(), id.clone()));
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
            .map(|m| (m.0.statement.get_kind(), m.0.artifact.is_some()))
            .collect();
        write!(
            f,
            "VectorBoard{{session_id={} messages={:?} }}",
            self.session_id, types
        )
    }
}
