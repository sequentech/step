// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::protocol::trustee::Trustee;
use crate::native::test::vector_board::VectorBoard;
use b4::messages::artifact::DkgPublicKey;
use b4::messages::message::Message;
use log::{error, info};
use std::sync::{Arc, Mutex};
use cryptography::context::Context;

use b4::messages::newtypes::{BatchNumber, TrusteePosition};

// Implements cross-session parallelism as well as simulates cross-trustee parallelism
#[derive(Debug)]
pub struct VectorSession<C: Context, S: crate::protocol::board::LocalBoardStorage> {
    trustee: Trustee<C, S>,
    remote: Arc<Mutex<VectorBoard>>,
    last_message: i64,
}

impl<C: Context, S: crate::protocol::board::LocalBoardStorage> VectorSession<C, S> {
    pub fn new(trustee: Trustee<C, S>, remote: Arc<Mutex<VectorBoard>>) -> VectorSession<C, S> {
        VectorSession {
            trustee,
            remote,
            last_message: -1,
        }
    }

    pub fn step(&mut self) {
        info!("Trustee {:?} step..", self.trustee.name);
        let remote = self.remote.lock().unwrap();

        // Equivalent of getting all messages
        let messages = remote.get(self.last_message);
        drop(remote);

        // let (send_messages, _actions) = self.trustee.step(messages);
        let count = messages.len() as i64;
        let result = self.trustee.step(&messages);
        self.last_message += count;
        // if let Ok((send_messages, _actions, _last_id)) = result {
        if let Ok(step_result) = result {
            let mut remote = self.remote.lock().unwrap();
            send(step_result.messages, &mut remote);
        } else {
            error!(
                "VectorSession: Trustee step returned err {:?}",
                result.err().unwrap()
            );
        }
    }

    pub(crate) fn get_plaintexts_nohash<const W: usize>(
        &self,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Option<b4::messages::artifact::Plaintexts<C, W>> {
        self.trustee._get_plaintexts_nohash::<W>(batch, signer_position)
    }
    pub(crate) fn get_dkg_public_key_nohash(&self) -> Option<DkgPublicKey<C>> {
        self.trustee._get_dkg_public_key_nohash()
    }
}

fn send<C: cryptography::context::Context>(messages: Vec<Message<C>>, remote: &mut VectorBoard) {
    for m in messages.iter() {
        info!("Sending message to vector board {:?}", m);
        remote.add(m.try_clone().unwrap());
    }
}
