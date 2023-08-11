use crate::protocol2::message::Message;
use crate::protocol2::trustee::Trustee;
use crate::test::vector_board::VectorBoard;
use log::{info, warn};
use std::sync::{Arc, Mutex};
use strand::context::Ctx;

use crate::protocol2::{
    artifact::{DkgPublicKey, Plaintexts},
    datalog::BatchNumber,
};

// Implements cross-session parallelism as well as simulates cross-trustee parallelism
#[derive(Debug)]
pub struct VectorSession<C: Ctx> {
    trustee: Trustee<C>,
    remote: Arc<Mutex<VectorBoard>>,
}

impl<C: Ctx> VectorSession<C> {
    pub fn new(trustee: Trustee<C>, remote: Arc<Mutex<VectorBoard>>) -> VectorSession<C> {
        VectorSession { trustee, remote }
    }

    pub fn step(&mut self) {
        info!("Trustee {:?} step..", self.trustee.get_pk());
        let remote = self.remote.lock().unwrap();

        // Equivalent of getting all messages
        let messages = remote.get(0);
        drop(remote);

        // let (send_messages, _actions) = self.trustee.step(messages);
        let result = self.trustee.step(messages);
        if let Ok((send_messages, _actions)) = result {
            let mut remote = self.remote.lock().unwrap();
            send(send_messages, &mut remote);
        } else {
            warn!("Trustee step returned err");
        }
    }

    pub(crate) fn get_plaintexts_nohash(&self, batch: BatchNumber) -> Option<Plaintexts<C>> {
        self.trustee.get_plaintexts_nohash(batch)
    }
    pub(crate) fn get_dkg_public_key_nohash(&self) -> Option<DkgPublicKey<C>> {
        self.trustee.get_dkg_public_key_nohash()
    }
}

fn send(messages: Vec<Message>, remote: &mut VectorBoard) {
    for m in messages.iter() {
        info!("Adding message {:?} to remote", m);
        remote.add(m.clone());
    }
}
