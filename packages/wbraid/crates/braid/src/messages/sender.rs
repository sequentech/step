// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Defines the [`Signer`] trait and [`Sender`] identity type used by protocol
//! message construction.
//!
//! The v0.6 protocol message type is
//! [`ProtocolMessage`](super::wire::ProtocolMessage); it signs via its own
//! helper using [`Signer::get_signing_key`]. This module retains only the small
//! pieces still shared across the crate: the [`Signer`] trait (a trustee or the
//! protocol manager) and the [`Sender`] identity stamped into every message.
//! (The pre-v0.6 `Message<C>` model was removed in the M3 retirement pass.)

use cryptography::context::Context;
use cryptography::utils::signatures::SignatureScheme;
use cryptography::VSerializable as VSer;

///////////////////////////////////////////////////////////////////////////
// Signer (commonality to sign messages for Trustee and ProtocolManager)
///////////////////////////////////////////////////////////////////////////
pub trait Signer<C: Context> {
    fn get_signing_key(&self) -> &<C::SignatureScheme as SignatureScheme<C::Rng>>::Signer;
    fn get_name(&self) -> String;
}

#[derive(VSer)]
pub struct Sender<C: Context> {
    pub name: String,
    pub pk: <C::SignatureScheme as SignatureScheme<C::Rng>>::Verifier,
}

impl<C: Context> Clone for Sender<C> {
    fn clone(&self) -> Self {
        Sender {
            name: self.name.clone(),
            pk: self.pk.clone(),
        }
    }
}

impl<C: Context> Sender<C> {
    pub fn new(
        name: String,
        pk: <C::SignatureScheme as SignatureScheme<C::Rng>>::Verifier,
    ) -> Sender<C> {
        Sender { name, pk }
    }
}
