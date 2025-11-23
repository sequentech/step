// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::protocol::trustee2::Trustee;
use crate::test::vector_board::VectorBoard;
use b3::messages::artifact::{DkgPublicKey, Plaintexts};
use b3::messages::message::Message;
use log::{error, info};
use std::sync::{Arc, Mutex};
use strand::context::Ctx;

use b3::messages::newtypes::{BatchNumber, TrusteePosition};

// Implements cross-session parallelism as well as simulates cross-trustee parallelism
#[derive(Debug)]
pub struct VectorSession<C: Ctx> {
    trustee: Trustee<C>,
    remote: Arc<Mutex<VectorBoard>>,
    last_message: i64,
}

impl<C: Ctx> VectorSession<C> {
    pub fn new(trustee: Trustee<C>, remote: Arc<Mutex<VectorBoard>>) -> VectorSession<C> {
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

    pub(crate) fn get_plaintexts_nohash(
        &self,
        batch: BatchNumber,
        signer_position: TrusteePosition,
    ) -> Option<Plaintexts<C>> {
        self.trustee._get_plaintexts_nohash(batch, signer_position)
    }
    pub(crate) fn get_dkg_public_key_nohash(&self) -> Option<DkgPublicKey<C>> {
        self.trustee._get_dkg_public_key_nohash()
    }
}

fn send(messages: Vec<Message>, remote: &mut VectorBoard) {
    for m in messages.iter() {
        info!("Sending message to vector board {:?}", m);
        remote.add(m.try_clone().unwrap());
    }
}
