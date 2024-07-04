// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod immudb;
pub mod pgsql;
pub mod local;

use board_messages::braid::message::Message;
use anyhow::Result;

pub(crate) trait Board {
    type Params;

    async fn get_messages(&mut self, last_id: Option<i64>) -> Result<Vec<(Message, i64)>>;
    async fn insert_messages(&mut self, messages: Vec<Message>) -> Result<()>;
}

pub(crate) trait BoardIndex {
    async fn get_board_names(&mut self) -> Result<Vec<String>>;
}