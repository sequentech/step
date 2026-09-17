// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The protocol manager: the other protocol participant besides a trustee
//! (`crate::trustee::Trustee`). It authors and signs `Configuration` and
//! `Ballots` (§4.3, §9.6) — the sender identity `verify` resolves via
//! `PROTOCOL_MANAGER_INDEX`.

use cryptography::context::Context;
use cryptography::utils::signatures::SignatureScheme;
use std::marker::PhantomData;

use crate::messages::wire::Signer;

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

impl<C: Context> Signer<C> for ProtocolManager<C> {
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
