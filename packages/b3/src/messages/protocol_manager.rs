// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::messages::message;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use strand::context::Ctx;
use strand::signature::StrandSignatureSk;

///////////////////////////////////////////////////////////////////////////
// ProtocolManager
///////////////////////////////////////////////////////////////////////////

pub struct ProtocolManager<C: Ctx> {
    pub signing_key: StrandSignatureSk,
    pub phantom: PhantomData<C>,
}

impl<C: Ctx> ProtocolManager<C> {
    pub fn new(pmkey: StrandSignatureSk) -> Self {
        ProtocolManager {
            signing_key: pmkey,
            phantom: PhantomData,
        }
    }
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
    // base64 encoding of a der encoded pkcs#8 v1
    pub signing_key: String,
}
impl ProtocolManagerConfig {
    pub fn from<C: Ctx>(pm: &ProtocolManager<C>) -> ProtocolManagerConfig {
        let sk_string = pm.signing_key.to_der_b64_string().unwrap();

        ProtocolManagerConfig {
            signing_key: sk_string,
        }
    }
    pub fn get_signing_key(&self) -> anyhow::Result<StrandSignatureSk> {
        let sk = StrandSignatureSk::from_der_b64_string(&self.signing_key)?;

        Ok(sk)
    }
}
