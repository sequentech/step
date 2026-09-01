// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Protocol message: the signed, typed unit of protocol communication.
//!
//! §3.1 of `crates/braid/v0.6_spec.md`: a [`ProtocolMessage`] carries `sender`,
//! `signature`, a `message_type` discriminant, a serialized **head**, and an
//! optional serialized **body**. The head holds the protocol-message-only
//! metadata (`date`) plus the message's **in** hashes (references to other
//! messages' bodies); the body is the bulk artifact. The **out** hash (`H(body)`)
//! is NOT on the protocol message — the verifier recomputes it from the received
//! body bytes and assembles the signed Statement as `head + H(body)` (§3.4).
//!
//! Head layout (the §4.4 pin) follows one rule for every type:
//!
//! > `head = predicate − sender − body_hash + date`
//!
//! i.e. a head carries exactly the predicate's context fields (its **in** hashes
//! and any parameters such as `trustees`), MINUS the `sender` (bound by the
//! verifying key) and MINUS the `body_hash` (recomputed as `H(body)`), PLUS the
//! protocol-message-only `date`. Reconstructing the predicate
//! (`into_predicate`) reverses this:
//! `predicate = head (drop date) + sender + body_hash`.
//!
//! [`MessageType::MixSignature`] is the one **bodyless** type: its content is the
//! signature itself, so it has no body and no out hash — both of its endpoint
//! hashes (`input`, `output`) are in-hashes carried by the head.

/// The schema version of the [`ProtocolMessage`] wire format (§10.1).
///
/// Stamped on every outgoing message and checked on every fetch; a mismatch
/// between a b4 instance and a trustee is a hard error — no backward
/// compatibility. One bump per format change.
pub fn schema_version() -> String {
    "1".to_string()
}

use anyhow::{anyhow, Result};
use cryptography::context::Context;
use cryptography::utils::error::Error;
use cryptography::utils::serialization::{Deserializable, Serializable};
use cryptography::utils::signatures::SignatureScheme;
use cryptography::Canonical;

use super::artifact::Configuration;
use super::newtypes::{
    CiphertextsHash, ConfigurationHash, Hash, PublicKeyHash, Timestamp, TrusteeIndex,
};

///////////////////////////////////////////////////////////////////////////
// Message type discriminant
///////////////////////////////////////////////////////////////////////////

/// The protocol message `type` field (§3.1): selects the concrete head/body
/// structs.
///
/// Note this is the protocol message type, distinct from the datalog
/// [`Predicate`]: a `Configuration` message maps to the derived
/// `ConfigurationValid` predicate (§9.8), while the other seven map one-to-one.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum MessageType {
    Configuration = 0,
    Shares = 1,
    PublicKey = 2,
    Ballots = 3,
    Mix = 4,
    MixSignature = 5,
    PartialDecryptions = 6,
    Plaintexts = 7,
}

impl MessageType {
    /// Whether messages of this type carry a body. Everything but
    /// [`MessageType::MixSignature`] (whose content is the signature itself).
    pub fn has_body(&self) -> bool {
        !matches!(self, MessageType::MixSignature)
    }
}

impl Serializable for MessageType {
    fn write(&self, out: &mut Vec<u8>) {
        (*self as u8).write(out);
    }
}

impl Deserializable for MessageType {
    fn read(input: &mut &[u8]) -> Result<Self, Error> {
        let discriminant = u8::read(input)?;
        match discriminant {
            0 => Ok(MessageType::Configuration),
            1 => Ok(MessageType::Shares),
            2 => Ok(MessageType::PublicKey),
            3 => Ok(MessageType::Ballots),
            4 => Ok(MessageType::Mix),
            5 => Ok(MessageType::MixSignature),
            6 => Ok(MessageType::PartialDecryptions),
            7 => Ok(MessageType::Plaintexts),
            other => Err(Error::DeserializationError(format!(
                "unknown MessageType discriminant {other}"
            ))),
        }
    }
}

///////////////////////////////////////////////////////////////////////////
// Heads
//
// Each head = the predicate's context fields (in-hashes + params) + `date`,
// with the body hash (H(body)) and the sender omitted. `into_predicate` rebuilds
// the predicate from (head − date) + sender + body_hash.
///////////////////////////////////////////////////////////////////////////

/// Head of a `Configuration` message. The body is the `Configuration` artifact
/// and its `H(body)` is the configuration hash; the derived predicate
/// (`ConfigurationValid`) also needs the trustee count / threshold (from the
/// body) and this trustee's `self_index`, so it is assembled by the board client
/// at construction (§9.8), not by the generic verify path.
#[derive(Clone, Debug, Canonical)]
pub struct ConfigurationHead {
    pub date: Timestamp,
}

/// Head of a `Shares` message. In: `configuration`. Out: `H(body)` = shares hash.
#[derive(Clone, Debug, Canonical)]
pub struct SharesHead {
    pub date: Timestamp,
    pub configuration: ConfigurationHash,
}

/// Head of a `PublicKey` message. In: `configuration`. Out: `H(body)` = public
/// key hash. (Lean: the justifying shares are separate `Shares` predicates the
/// datalog joins from the EDB — they are not carried here.)
#[derive(Clone, Debug, Canonical)]
pub struct PublicKeyHead {
    pub date: Timestamp,
    pub configuration: ConfigurationHash,
}

/// Head of a `Ballots` message (manager-authored). In: `configuration`,
/// `public_key`; param: `trustees` (the active mixing set) and `tally_id`
/// (manager-assigned identifier of this tally execution, separating the
/// Fiat-Shamir transcript domains of sibling tallies over one DKG, §8.2).
/// Out: `H(body)` = ciphertexts hash. Ballots has no sender field (single
/// manager slot).
#[derive(Clone, Debug, Canonical)]
pub struct BallotsHead {
    pub date: Timestamp,
    pub configuration: ConfigurationHash,
    pub public_key: PublicKeyHash,
    pub trustees: Vec<TrusteeIndex>,
    pub tally_id: u128,
}

/// Head of a `Mix` message. In: `configuration`, `public_key`, `input` (the
/// consumed ciphertexts). Out: `H(body)` = output ciphertexts hash.
#[derive(Clone, Debug, Canonical)]
pub struct MixHead {
    pub date: Timestamp,
    pub configuration: ConfigurationHash,
    pub public_key: PublicKeyHash,
    pub input: CiphertextsHash,
}

/// Head of a `MixSignature` message. BODYLESS: both `input` and `output` are
/// in-hashes carried by the head; there is no body and no out hash.
#[derive(Clone, Debug, Canonical)]
pub struct MixSignatureHead {
    pub date: Timestamp,
    pub configuration: ConfigurationHash,
    pub public_key: PublicKeyHash,
    pub input: CiphertextsHash,
    pub output: CiphertextsHash,
}

/// Head of a `PartialDecryptions` message. In: `configuration`, `public_key`,
/// `ciphertexts` (the ciphertexts being decrypted). Out: `H(body)` = decryption
/// factors hash.
#[derive(Clone, Debug, Canonical)]
pub struct PartialDecryptionsHead {
    pub date: Timestamp,
    pub configuration: ConfigurationHash,
    pub public_key: PublicKeyHash,
    pub ciphertexts: CiphertextsHash,
}

/// Head of a `Plaintexts` message. In: `configuration`, `public_key`,
/// `ciphertexts`. Out: `H(body)` = plaintexts hash.
#[derive(Clone, Debug, Canonical)]
pub struct PlaintextsHead {
    pub date: Timestamp,
    pub configuration: ConfigurationHash,
    pub public_key: PublicKeyHash,
    pub ciphertexts: CiphertextsHash,
}

///////////////////////////////////////////////////////////////////////////
// Sender & Signer
//
// Sender is the identity stamped into every ProtocolMessage; Signer is the
// commonality that lets anything sign one — a trustee (`crate::trustee::Trustee`)
// or the protocol manager (`crate::protocol_manager::ProtocolManager`).
///////////////////////////////////////////////////////////////////////////

pub trait Signer<C: Context> {
    fn get_signing_key(&self) -> &<C::SignatureScheme as SignatureScheme<C::Rng>>::Signer;
    fn get_name(&self) -> String;
}

#[derive(Canonical)]
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

///////////////////////////////////////////////////////////////////////////
// ProtocolMessage
///////////////////////////////////////////////////////////////////////////

/// The single protocol message structure (§3.1). `head` is the serialized head
/// struct selected by `message_type`; `body` is the serialized bulk artifact
/// (absent for the bodyless [`MessageType::MixSignature`]). Both `head` and
/// `body` are length-delimited fields, so `body` is a clean slice hashed
/// directly.
#[derive(Canonical)]
pub struct ProtocolMessage<C: Context> {
    pub sender: Sender<C>,
    pub signature: <C::SignatureScheme as SignatureScheme<C::Rng>>::Signature,
    pub message_type: MessageType,
    pub head: Vec<u8>,
    pub body: Option<Vec<u8>>,
}

impl<C: Context> Clone for ProtocolMessage<C> {
    fn clone(&self) -> Self {
        ProtocolMessage {
            sender: self.sender.clone(),
            signature: self.signature.clone(),
            message_type: self.message_type.clone(),
            head: self.head.clone(),
            body: self.body.clone(),
        }
    }
}

impl<C: Context> std::fmt::Debug for ProtocolMessage<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ProtocolMessage{{ sender={:?} type={:?} has_body={} }}",
            self.sender.name,
            self.message_type,
            self.body.is_some()
        )
    }
}

///////////////////////////////////////////////////////////////////////////
// Statement bytes (§3.3) — the transient signed object
//
// The Statement is NOT a materialized type (§3.3): `ser(Statement)` is realized
// as `head ++ H(body)` (or `head` alone for bodyless messages). This single
// helper is the ONE place that pins that byte layout, used by both the `sign_*`
// constructors and `verify`, so signer and verifier agree by construction.
///////////////////////////////////////////////////////////////////////////

/// Serialize the transient Statement that is signed and verified (§3.3): the
/// serialized `head` followed by `H(body)` when present. For bodyless messages
/// (`MixSignature`) there is no `body_hash`, so the statement is the head alone.
pub fn statement_bytes<H: Serializable>(head: &H, body_hash: Option<&Hash>) -> Vec<u8> {
    let mut bytes = head.ser();
    if let Some(body_hash) = body_hash {
        bytes.extend(body_hash.ser());
    }
    bytes
}

/// `H(bytes)` under the protocol hash (§3.4), over the given (received, or
/// freshly serialized) bytes.
fn hash(bytes: &[u8]) -> Hash {
    super::newtypes::hash_bytes(bytes)
}

///////////////////////////////////////////////////////////////////////////
// Signing constructors
//
// One per message type. Each builds the head, serializes and hashes the body,
// signs `statement_bytes(head, H(body))` (or `statement_bytes(head, None)` for
// the bodyless MixSignature), and returns the ProtocolMessage. The signer
// supplies the signing key + name; its public side becomes `sender.pk`, which
// binds the sender (§3.3), so `sender` is never in the signed bytes.
///////////////////////////////////////////////////////////////////////////

impl<C: Context> ProtocolMessage<C> {
    /// Low-level: sign `signed_bytes` with `signer` and assemble the ProtocolMessage.
    fn sign_wire<S: Signer<C>>(
        signer: &S,
        message_type: MessageType,
        head: Vec<u8>,
        signed_bytes: &[u8],
        body: Option<Vec<u8>>,
    ) -> ProtocolMessage<C> {
        use cryptography::utils::signatures::Signer as CryptoSigner;

        let sk = signer.get_signing_key();
        let signature = sk.sign(signed_bytes);
        let pk = C::SignatureScheme::verifying_key(sk);
        let sender = Sender::new(signer.get_name(), pk);

        ProtocolMessage {
            sender,
            signature,
            message_type,
            head,
            body,
        }
    }

    /// `Configuration` (manager self-signed). Special: it is accepted and
    /// verified at construction (§9.8), so there is no matching `verify` arm —
    /// [`ProtocolMessage::verify`] rejects [`MessageType::Configuration`].
    pub fn configuration<S: Signer<C>, B: Serializable>(
        manager: &S,
        date: Timestamp,
        body: &B,
    ) -> ProtocolMessage<C> {
        let body_bytes = body.ser();
        let body_hash = hash(&body_bytes);
        let head = ConfigurationHead { date };
        let signed = statement_bytes(&head, Some(&body_hash));
        Self::sign_wire(
            manager,
            MessageType::Configuration,
            head.ser(),
            &signed,
            Some(body_bytes),
        )
    }

    /// `Shares`. In: `configuration`. Out: `H(body)` = shares hash.
    pub fn shares<S: Signer<C>, B: Serializable>(
        signer: &S,
        date: Timestamp,
        configuration: ConfigurationHash,
        body: &B,
    ) -> ProtocolMessage<C> {
        let body_bytes = body.ser();
        let body_hash = hash(&body_bytes);
        let head = SharesHead {
            date,
            configuration,
        };
        let signed = statement_bytes(&head, Some(&body_hash));
        Self::sign_wire(
            signer,
            MessageType::Shares,
            head.ser(),
            &signed,
            Some(body_bytes),
        )
    }

    /// `PublicKey`. In: `configuration`. Out: `H(body)` = public key hash.
    pub fn public_key<S: Signer<C>, B: Serializable>(
        signer: &S,
        date: Timestamp,
        configuration: ConfigurationHash,
        body: &B,
    ) -> ProtocolMessage<C> {
        let body_bytes = body.ser();
        let body_hash = hash(&body_bytes);
        let head = PublicKeyHead {
            date,
            configuration,
        };
        let signed = statement_bytes(&head, Some(&body_hash));
        Self::sign_wire(
            signer,
            MessageType::PublicKey,
            head.ser(),
            &signed,
            Some(body_bytes),
        )
    }

    /// `Ballots` (manager-authored). In: `configuration`, `public_key`; param:
    /// `trustees`, `tally_id`. Out: `H(body)` = ciphertexts hash.
    pub fn ballots<S: Signer<C>, B: Serializable>(
        manager: &S,
        date: Timestamp,
        configuration: ConfigurationHash,
        public_key: PublicKeyHash,
        trustees: Vec<TrusteeIndex>,
        tally_id: u128,
        body: &B,
    ) -> ProtocolMessage<C> {
        let body_bytes = body.ser();
        let body_hash = hash(&body_bytes);
        let head = BallotsHead {
            date,
            configuration,
            public_key,
            trustees,
            tally_id,
        };
        let signed = statement_bytes(&head, Some(&body_hash));
        Self::sign_wire(
            manager,
            MessageType::Ballots,
            head.ser(),
            &signed,
            Some(body_bytes),
        )
    }

    /// `Mix`. In: `configuration`, `public_key`, `input`. Out: `H(body)` =
    /// output ciphertexts hash.
    pub fn mix<S: Signer<C>, B: Serializable>(
        signer: &S,
        date: Timestamp,
        configuration: ConfigurationHash,
        public_key: PublicKeyHash,
        input: CiphertextsHash,
        body: &B,
    ) -> ProtocolMessage<C> {
        let body_bytes = body.ser();
        let body_hash = hash(&body_bytes);
        let head = MixHead {
            date,
            configuration,
            public_key,
            input,
        };
        let signed = statement_bytes(&head, Some(&body_hash));
        Self::sign_wire(
            signer,
            MessageType::Mix,
            head.ser(),
            &signed,
            Some(body_bytes),
        )
    }

    /// `MixSignature` (BODYLESS): both `input` and `output` are in-hashes carried
    /// by the head; there is no body and the statement is the head alone.
    pub fn mix_signature<S: Signer<C>>(
        signer: &S,
        date: Timestamp,
        configuration: ConfigurationHash,
        public_key: PublicKeyHash,
        input: CiphertextsHash,
        output: CiphertextsHash,
    ) -> ProtocolMessage<C> {
        let head = MixSignatureHead {
            date,
            configuration,
            public_key,
            input,
            output,
        };
        let signed = statement_bytes(&head, None);
        Self::sign_wire(signer, MessageType::MixSignature, head.ser(), &signed, None)
    }

    /// `PartialDecryptions`. In: `configuration`, `public_key`, `ciphertexts`.
    /// Out: `H(body)` = decryption factors hash.
    pub fn partial_decryptions<S: Signer<C>, B: Serializable>(
        signer: &S,
        date: Timestamp,
        configuration: ConfigurationHash,
        public_key: PublicKeyHash,
        ciphertexts: CiphertextsHash,
        body: &B,
    ) -> ProtocolMessage<C> {
        let body_bytes = body.ser();
        let body_hash = hash(&body_bytes);
        let head = PartialDecryptionsHead {
            date,
            configuration,
            public_key,
            ciphertexts,
        };
        let signed = statement_bytes(&head, Some(&body_hash));
        Self::sign_wire(
            signer,
            MessageType::PartialDecryptions,
            head.ser(),
            &signed,
            Some(body_bytes),
        )
    }

    /// `Plaintexts`. In: `configuration`, `public_key`, `ciphertexts`. Out:
    /// `H(body)` = plaintexts hash.
    pub fn plaintexts<S: Signer<C>, B: Serializable>(
        signer: &S,
        date: Timestamp,
        configuration: ConfigurationHash,
        public_key: PublicKeyHash,
        ciphertexts: CiphertextsHash,
        body: &B,
    ) -> ProtocolMessage<C> {
        let body_bytes = body.ser();
        let body_hash = hash(&body_bytes);
        let head = PlaintextsHead {
            date,
            configuration,
            public_key,
            ciphertexts,
        };
        let signed = statement_bytes(&head, Some(&body_hash));
        Self::sign_wire(
            signer,
            MessageType::Plaintexts,
            head.ser(),
            &signed,
            Some(body_bytes),
        )
    }

    /// Verify a `Configuration` message's manager **self-signature** and return
    /// the accepted [`Configuration`] (§9.8).
    ///
    /// This is the special path [`verify`](Self::verify) rejects: a
    /// `Configuration` carries its own protocol-manager verifying key *inside*
    /// the body, so authenticity here is self-referential (provenance only — the
    /// operator anchors the manager key out of band, §9.3). The board client
    /// calls this at construction; the derived `ConfigurationValid` predicate is
    /// assembled there (it also needs this trustee's `self_index`), not here.
    pub fn verify_configuration(&self) -> Result<Configuration<C>> {
        if self.message_type != MessageType::Configuration {
            return Err(anyhow!(
                "verify_configuration called on a {:?} message",
                self.message_type
            ));
        }
        let body = self.require_body()?;
        let configuration = Configuration::<C>::deser(body)
            .map_err(|e| anyhow!("Configuration body deserialization failed: {e:?}"))?;

        // Self-signed: the signer must be the protocol manager named inside.
        if self.sender.pk != configuration.protocol_manager {
            return Err(anyhow!(
                "Configuration is not self-signed by its own protocol manager"
            ));
        }

        let body_hash = hash(body);
        let head = ConfigurationHead::deser(&self.head)
            .map_err(|e| anyhow!("Configuration head deserialization failed: {e:?}"))?;
        self.check_signature(
            &configuration.protocol_manager,
            &statement_bytes(&head, Some(&body_hash)),
        )?;

        Ok(configuration)
    }

    /// Body bytes for a bodied message, or an error if absent.
    fn require_body(&self) -> Result<&Vec<u8>> {
        self.body
            .as_ref()
            .ok_or_else(|| anyhow!("{:?} message is missing its body", self.message_type))
    }

    /// Verify `self.signature` over `signed_bytes` under `verifier`.
    pub fn check_signature(
        &self,
        verifier: &<C::SignatureScheme as SignatureScheme<C::Rng>>::Verifier,
        signed_bytes: &[u8],
    ) -> Result<()> {
        use cryptography::utils::signatures::Verifier;

        if verifier.verify(signed_bytes, &self.signature).is_err() {
            return Err(anyhow!("signature verification failed for {:?}", self));
        }
        Ok(())
    }
}
