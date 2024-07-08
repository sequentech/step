// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod immudb;
pub mod local;
pub mod pgsql;

use anyhow::Result;
use board_messages::braid::message::Message;

pub trait Board: Sized {
    type Factory: BoardFactory<Self>;

    async fn get_messages(&mut self, last_id: Option<i64>) -> Result<Vec<(Message, i64)>>;
    async fn insert_messages(&mut self, messages: Vec<Message>) -> Result<()>;
}

pub trait BoardFactory<B: Board>: Sized {
    async fn get_board(&self) -> Result<B>;
}
