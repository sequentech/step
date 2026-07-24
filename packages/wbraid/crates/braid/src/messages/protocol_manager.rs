// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::sender;
use cryptography::context::Context;
use cryptography::utils::signatures::SignatureScheme;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

///////////////////////////////////////////////////////////////////////////
// ProtocolManager
///////////////////////////////////////////////////////////////////////////

pub struct ProtocolManager<C: Context> {
    pub signing_key: <C::SignatureScheme as SignatureScheme<C::Rng>>::Signer,
    pub phantom: PhantomData<C>,
}

impl<C: Context> ProtocolManager<C> {
    pub fn new(pmkey: <C::SignatureScheme as SignatureScheme<C::Rng>>::Signer) -> Self {
        ProtocolManager {
            signing_key: pmkey,
            phantom: PhantomData,
        }
    }
}

impl<C: Context> sender::Signer<C> for ProtocolManager<C> {
    fn get_signing_key(&self) -> &<C::SignatureScheme as SignatureScheme<C::Rng>>::Signer {
        &self.signing_key
    }
    fn get_name(&self) -> String {
        "Protocol Manager".to_string()
    }
}

impl<C: Context> std::fmt::Debug for ProtocolManager<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProtcolManager()")
    }
}

///////////////////////////////////////////////////////////////////////////
// ProtocolManagerConfig
///////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize)]
pub struct ProtocolManagerConfig {
    // base64 encoding of serialized signing key bytes
    pub signing_key: String,
}
impl ProtocolManagerConfig {
    pub fn from<C: Context>(pm: &ProtocolManager<C>) -> ProtocolManagerConfig {
        use base64::{engine::general_purpose, Engine as _};
        use cryptography::utils::serialization::VSerializable;
        let sk_bytes = pm.signing_key.ser();
        let sk_string = general_purpose::STANDARD.encode(&sk_bytes);

        ProtocolManagerConfig {
            signing_key: sk_string,
        }
    }
    pub fn get_signing_key<C: Context>(
        &self,
    ) -> anyhow::Result<<C::SignatureScheme as SignatureScheme<C::Rng>>::Signer> {
        use base64::{engine::general_purpose, Engine as _};
        use cryptography::utils::serialization::VDeserializable;
        let sk_bytes = general_purpose::STANDARD.decode(&self.signing_key)?;
        let sk = <<C::SignatureScheme as SignatureScheme<C::Rng>>::Signer>::deser(&sk_bytes)?;

        Ok(sk)
    }
}
