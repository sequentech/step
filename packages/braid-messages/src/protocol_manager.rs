// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use serde::{Deserialize, Serialize};
use strand::context::Ctx;
use strand::serialization::{StrandDeserialize, StrandSerialize};
use strand::signature::StrandSignatureSk;
use base64::{engine::general_purpose, Engine as _};
use std::marker::PhantomData;
use crate::message;

///////////////////////////////////////////////////////////////////////////
// ProtocolManager
///////////////////////////////////////////////////////////////////////////

pub struct ProtocolManager<C: Ctx> {
    pub signing_key: StrandSignatureSk,
    pub phantom: PhantomData<C>,
}

impl<C: Ctx> message::Signer for ProtocolManager<C> {
    fn get_signing_key(&self) -> &StrandSignatureSk {
        &self.signing_key
    }
    fn get_name(&self) -> String {
        "Protocol Manager".to_string()
    }
}

impl<C: Ctx> std::fmt::Debug for ProtocolManager<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProtcolManager()")
    }
}

///////////////////////////////////////////////////////////////////////////
// ProtocolManagerConfig
///////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize)]
pub struct ProtocolManagerConfig {
    // base64 encoding of a StrandSignatureSk serialization
    pub signing_key: String,
}
impl ProtocolManagerConfig {
    pub fn from<C: Ctx>(pm: &ProtocolManager<C>) -> ProtocolManagerConfig {
        let sk_bytes = pm.signing_key.strand_serialize().unwrap();

        let sk_string: String = general_purpose::STANDARD_NO_PAD.encode(sk_bytes);

        ProtocolManagerConfig {
            signing_key: sk_string,
        }
    }
    pub fn get_signing_key(&self) -> anyhow::Result<StrandSignatureSk> {
        let bytes = general_purpose::STANDARD_NO_PAD.decode(&self.signing_key)?;

        Ok(StrandSignatureSk::strand_deserialize(&bytes)?)
    }
}
