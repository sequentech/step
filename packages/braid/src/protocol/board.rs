// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// pub mod grpc;
pub mod grpc_m;
// pub mod local;
#[cfg(feature = "sqlite")]
pub mod local2;
// pub mod pgsql;

use anyhow::Result;
use board_messages::{braid::message::Message, grpc::GrpcB3Message};

pub trait Board: Sized {
    type Factory: BoardFactory<Self>;

    // async fn get_messages(&mut self, last_id: Option<i64>) -> Result<Vec<(Message, i64)>>;
    // async fn insert_messages(&mut self, messages: Vec<Message>) -> Result<()>;
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
    // async fn get_board(&self) -> Result<B>;
    // fn get_board(&self) -> impl std::future::Future<Output = Result<B>> + Send;
    fn get_board(&self) -> B;
}

pub enum ArtifactRef<'a, T> {
    Ref(&'a T),
    Owned(T)
}
impl<'a, T> ArtifactRef<'a, T> {
    pub fn get_ref(&'a self) -> &'a T { 
        match self {
            ArtifactRef::Ref(ref v) => { *v },
            ArtifactRef::Owned(v) => { v },
        }
     }
     pub fn transform<U, F: FnOnce(&'a T) -> &'a U, G: FnOnce(T) -> U>(self, f: F, g: G) -> ArtifactRef<'a, U> {
        let ret = match self {
            ArtifactRef::Ref(ref v) => { ArtifactRef::Ref(f(*v)) },
            ArtifactRef::Owned(v) => { ArtifactRef::Owned(g(v)) },
        };
        
        ret
    }
}