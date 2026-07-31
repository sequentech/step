// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Verification (§3.4 of `crates/braid/v0.6_spec.md`): the trustee-side
//! projection of a signed [`ProtocolMessage`] onto a datalog [`Predicate`].
//!
//! This is the trust boundary. The wire layer owns the *format* (the
//! [`ProtocolMessage`] structure, the per-type heads, signing, and the
//! `statement_bytes` byte layout); this module owns the *interpretation* —
//! checking the signature against the configuration and reconstructing the
//! [`Predicate`] that feeds the datalog. `Configuration` messages are the
//! exception: they are accepted at construction (§9.8) via
//! [`ProtocolMessage::verify_configuration`], which stays in the wire layer
//! because it yields a [`Configuration`], not a predicate.

use anyhow::{anyhow, Result};
use cryptography::context::Context;
use cryptography::utils::serialization::VDeserializable;

use crate::messages::artifact::Configuration;
use crate::messages::newtypes::{
    hash_bytes, CiphertextsHash, DecryptionFactorsHash, PlaintextsHash, PublicKeyHash, SharesHash,
    TrusteeIndex, PROTOCOL_MANAGER_INDEX,
};
use crate::messages::wire::{
    statement_bytes, BallotsHead, MessageType, MixHead, MixSignatureHead, PartialDecryptionsHead,
    PlaintextsHead, ProtocolMessage, PublicKeyHead, SharesHead,
};

use crate::messages::predicate::{
    Ballots, Mix, MixSignature, PartialDecryptions, Plaintexts, Predicate, PublicKey, Shares,
};

/// Verify the signature and reconstruct the datalog predicate (plus the body for
/// bodied types). See §3.4:
/// 1. `body_hash = H(body)` over the received body bytes;
/// 2. deserialize the head selected by `message_type`;
/// 3. re-assemble `signed_bytes = statement_bytes(head, body_hash)`;
/// 4. verify the signature under the sender's key from `configuration`;
/// 5. project head + sender + body_hash to the predicate.
///
/// Covers the 7 message-derived predicate types. `Configuration` is NOT handled
/// here — its predicate (`ConfigurationValid`) additionally needs the body's
/// threshold/trustee_count and this trustee's `self_index`, so it is assembled by
/// the board client / Trustee at construction (§9.8), via
/// [`ProtocolMessage::verify_configuration`].
///
/// The cfg-domain check (`predicate.configuration == our cfg_hash`) is NOT done
/// here — it is the datalog's single enforcement point (§7.3), so this function
/// stays purely signature + reconstruction.
pub fn verify<C: Context>(
    message: &ProtocolMessage<C>,
    configuration: &Configuration<C>,
) -> Result<(Predicate, Option<Vec<u8>>)> {
    let position = configuration
        .get_trustee_position(&message.sender.pk)
        .ok_or_else(|| {
            anyhow!(
                "message from a sender that is not part of the configuration: {:?}",
                message.sender.pk
            )
        })?;
    let is_manager = position == PROTOCOL_MANAGER_INDEX as usize;
    let verifier = if is_manager {
        &configuration.protocol_manager
    } else {
        &configuration.trustees[position]
    };
    // 0-based trustee position -> 1-based TrusteeIndex (§4.3); the manager keeps
    // its sentinel. Only the sender-carrying predicate types use this.
    let sender: TrusteeIndex = if is_manager {
        PROTOCOL_MANAGER_INDEX as TrusteeIndex
    } else {
        position + 1
    };

    match message.message_type {
        MessageType::Configuration => Err(anyhow!(
            "Configuration is accepted and verified at construction (§9.8), \
             not via verify"
        )),
        MessageType::Shares => {
            let body = require_body(message)?;
            let body_hash = hash_bytes(body);
            let head = SharesHead::deser(&message.head)
                .map_err(|e| anyhow!("Shares head deserialization failed: {e:?}"))?;
            message.check_signature(verifier, &statement_bytes(&head, Some(&body_hash)))?;
            let predicate = Shares {
                configuration: head.configuration,
                shares: SharesHash(body_hash),
                sender,
            }
            .into();
            Ok((predicate, Some(body.clone())))
        }
        MessageType::PublicKey => {
            let body = require_body(message)?;
            let body_hash = hash_bytes(body);
            let head = PublicKeyHead::deser(&message.head)
                .map_err(|e| anyhow!("PublicKey head deserialization failed: {e:?}"))?;
            message.check_signature(verifier, &statement_bytes(&head, Some(&body_hash)))?;
            let predicate = PublicKey {
                configuration: head.configuration,
                public_key: PublicKeyHash(body_hash),
                sender,
            }
            .into();
            Ok((predicate, Some(body.clone())))
        }
        MessageType::Ballots => {
            if !is_manager {
                return Err(anyhow!("Ballots must be signed by the protocol manager"));
            }
            let body = require_body(message)?;
            let body_hash = hash_bytes(body);
            let head = BallotsHead::deser(&message.head)
                .map_err(|e| anyhow!("Ballots head deserialization failed: {e:?}"))?;
            message.check_signature(verifier, &statement_bytes(&head, Some(&body_hash)))?;
            let predicate = Ballots {
                configuration: head.configuration,
                public_key: head.public_key,
                ciphertexts: CiphertextsHash(body_hash),
                trustees: head.trustees,
            }
            .into();
            Ok((predicate, Some(body.clone())))
        }
        MessageType::Mix => {
            let body = require_body(message)?;
            let body_hash = hash_bytes(body);
            let head = MixHead::deser(&message.head)
                .map_err(|e| anyhow!("Mix head deserialization failed: {e:?}"))?;
            message.check_signature(verifier, &statement_bytes(&head, Some(&body_hash)))?;
            let predicate = Mix {
                configuration: head.configuration,
                public_key: head.public_key,
                input: head.input,
                output: CiphertextsHash(body_hash),
                sender,
            }
            .into();
            Ok((predicate, Some(body.clone())))
        }
        MessageType::MixSignature => {
            if message.body.is_some() {
                return Err(anyhow!("MixSignature must not carry a body"));
            }
            let head = MixSignatureHead::deser(&message.head)
                .map_err(|e| anyhow!("MixSignature head deserialization failed: {e:?}"))?;
            message.check_signature(verifier, &statement_bytes(&head, None))?;
            let predicate = MixSignature {
                configuration: head.configuration,
                public_key: head.public_key,
                input: head.input,
                output: head.output,
                sender,
            }
            .into();
            Ok((predicate, None))
        }
        MessageType::PartialDecryptions => {
            let body = require_body(message)?;
            let body_hash = hash_bytes(body);
            let head = PartialDecryptionsHead::deser(&message.head)
                .map_err(|e| anyhow!("PartialDecryptions head deserialization failed: {e:?}"))?;
            message.check_signature(verifier, &statement_bytes(&head, Some(&body_hash)))?;
            let predicate = PartialDecryptions {
                configuration: head.configuration,
                public_key: head.public_key,
                ciphertexts: head.ciphertexts,
                decryptions: DecryptionFactorsHash(body_hash),
                sender,
            }
            .into();
            Ok((predicate, Some(body.clone())))
        }
        MessageType::Plaintexts => {
            let body = require_body(message)?;
            let body_hash = hash_bytes(body);
            let head = PlaintextsHead::deser(&message.head)
                .map_err(|e| anyhow!("Plaintexts head deserialization failed: {e:?}"))?;
            message.check_signature(verifier, &statement_bytes(&head, Some(&body_hash)))?;
            let predicate = Plaintexts {
                configuration: head.configuration,
                public_key: head.public_key,
                ciphertexts: head.ciphertexts,
                plaintexts: PlaintextsHash(body_hash),
                sender,
            }
            .into();
            Ok((predicate, Some(body.clone())))
        }
    }
}

/// Body bytes for a bodied message, or an error if absent.
fn require_body<C: Context>(message: &ProtocolMessage<C>) -> Result<&Vec<u8>> {
    message
        .body
        .as_ref()
        .ok_or_else(|| anyhow!("{:?} message is missing its body", message.message_type))
}
