// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// pub mod grpc;
pub mod grpc_m;
// pub mod local;
pub mod local2;
// pub mod pgsql;

use anyhow::Result;
use board_messages::grpc::KeyedMessages;
use board_messages::{braid::message::Message, grpc::GrpcB3Message};

pub trait Board: Sized {
    type Factory: BoardFactory<Self>;

    fn get_messages(
        &mut self,
        board: &str,
        last_id: i64,
    ) -> impl std::future::Future<Output = Result<Vec<GrpcB3Message>>> + Send;
    fn insert_messages(
        &mut self,
        board: &str,
        messages: Vec<Message>,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

pub trait BoardFactory<B: Board>: Sized {
    fn get_board(&self) -> B;
}

pub trait BoardMulti: Sized {
    type Factory: BoardFactoryMulti<Self>;

    fn get_messages_multi(
        &self,
        requests: &Vec<(String, i64)>,
    ) -> impl std::future::Future<Output = Result<(Vec<KeyedMessages>, bool)>> + Send;

    fn insert_messages_multi(
        &self,
        requests: Vec<(String, Vec<Message>)>,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

pub trait BoardFactoryMulti<B: BoardMulti>: Sized {
    fn get_board(&self) -> B;
}
