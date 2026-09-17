// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Bijection property tests for braid's wire boundaries over adversarial
//! bytes (`SERIALIZATION.md` phase 4).
//!
//! The oracle is the strictness property P2: any *accepted* byte string must
//! re-serialize to exactly itself. It runs over three distributions — valid
//! encodings (where it reduces to the round trip), mutations of valid
//! encodings, and raw random bytes — for the two braid-specific boundaries:
//!
//! - [`ProtocolMessage`]: the outermost adversarial surface (bytes from the
//!   untrusted board, before signature checks);
//! - [`Predicate`]: the persistence surface (bytes reloading the anti-rewrite
//!   commitments across restarts).
//!
//! These complement the coverage-guided fuzz targets in `crates/braid/fuzz`,
//! which carry the same oracle but cannot link on Windows/MSVC (braid's wasm
//! `cdylib` crate-type conflicts with libFuzzer's `/include:main`); on Linux
//! they run as-is.

#![cfg(feature = "native")]

use proptest::prelude::*;

use braid::messages::newtypes::{
    hash_bytes, CiphertextsHash, ConfigurationHash, PartialDecryptionHash, PlaintextsHash,
    PublicKeyHash, SharesHash,
};
use braid::messages::predicate::{
    Ballots, ConfigurationValid, Mix, MixSignature, PartialDecryptions, Plaintexts, Predicate,
    PublicKey, Shares,
};
use braid::messages::wire::{MessageType, ProtocolMessage, Sender};
use cryptography::context::{Context, RistrettoCtx};
use cryptography::utils::serialization::{Deserializable, Serializable};
use cryptography::utils::signatures::{SignatureScheme, Signer as _};

type Sig = <RistrettoCtx as Context>::SignatureScheme;

// -- Generators ---------------------------------------------------------------

fn cfg_hash() -> impl Strategy<Value = ConfigurationHash> {
    any::<[u8; 8]>().prop_map(|b| ConfigurationHash(hash_bytes(&b)))
}

fn message_type() -> impl Strategy<Value = MessageType> {
    prop_oneof![
        Just(MessageType::Configuration),
        Just(MessageType::Shares),
        Just(MessageType::PublicKey),
        Just(MessageType::Ballots),
        Just(MessageType::Mix),
        Just(MessageType::MixSignature),
        Just(MessageType::PartialDecryptions),
        Just(MessageType::Plaintexts),
    ]
}

/// A structurally arbitrary `ProtocolMessage`: the signature is a real
/// signature over arbitrary bytes (its validity is irrelevant to
/// serialization), the head and body are arbitrary byte strings — exactly the
/// latitude a hostile board relay has.
fn protocol_message() -> impl Strategy<Value = ProtocolMessage<RistrettoCtx>> {
    (
        ".{0,8}",
        any::<[u8; 8]>(),
        message_type(),
        proptest::collection::vec(any::<u8>(), 0..64),
        proptest::option::of(proptest::collection::vec(any::<u8>(), 0..64)),
    )
        .prop_map(|(name, seed, message_type, head, body)| {
            let mut rng = RistrettoCtx::get_rng();
            let sk = Sig::gen_signing_key(&mut rng);
            let pk = Sig::verifying_key(&sk);
            let signature = sk.sign(&seed);
            ProtocolMessage {
                sender: Sender::new(name, pk),
                signature,
                message_type,
                head,
                body,
            }
        })
}

fn predicate() -> impl Strategy<Value = Predicate> {
    let h = any::<[u8; 8]>();
    prop_oneof![
        (cfg_hash(), 1usize..8, 1usize..8, 1usize..8).prop_map(|(c, t, n, i)| {
            Predicate::ConfigurationValid(ConfigurationValid {
                configuration: c,
                threshold: t,
                trustee_count: n,
                self_index: i,
            })
        }),
        (cfg_hash(), h, 1usize..8).prop_map(|(c, s, i)| {
            Predicate::Shares(Shares {
                configuration: c,
                shares: SharesHash(hash_bytes(&s)),
                sender: i,
            })
        }),
        (cfg_hash(), h, 1usize..8).prop_map(|(c, p, i)| {
            Predicate::PublicKey(PublicKey {
                configuration: c,
                public_key: PublicKeyHash(hash_bytes(&p)),
                sender: i,
            })
        }),
        (
            cfg_hash(),
            h,
            h,
            proptest::collection::vec(1usize..8, 0..4),
            any::<u128>()
        )
            .prop_map(|(c, p, ct, trustees, tally_id)| {
                Predicate::Ballots(Ballots {
                    configuration: c,
                    public_key: PublicKeyHash(hash_bytes(&p)),
                    ciphertexts: CiphertextsHash(hash_bytes(&ct)),
                    trustees,
                    tally_id,
                })
            }),
        (cfg_hash(), h, h, h, 1usize..8).prop_map(|(c, p, input, output, i)| {
            Predicate::Mix(Mix {
                configuration: c,
                public_key: PublicKeyHash(hash_bytes(&p)),
                input: CiphertextsHash(hash_bytes(&input)),
                output: CiphertextsHash(hash_bytes(&output)),
                sender: i,
            })
        }),
        (cfg_hash(), h, h, h, 1usize..8).prop_map(|(c, p, input, output, i)| {
            Predicate::MixSignature(MixSignature {
                configuration: c,
                public_key: PublicKeyHash(hash_bytes(&p)),
                input: CiphertextsHash(hash_bytes(&input)),
                output: CiphertextsHash(hash_bytes(&output)),
                sender: i,
            })
        }),
        (cfg_hash(), h, h, h, 1usize..8).prop_map(|(c, p, ct, d, i)| {
            Predicate::PartialDecryptions(PartialDecryptions {
                configuration: c,
                public_key: PublicKeyHash(hash_bytes(&p)),
                ciphertexts: CiphertextsHash(hash_bytes(&ct)),
                decryptions: PartialDecryptionHash(hash_bytes(&d)),
                sender: i,
            })
        }),
        (cfg_hash(), h, h, h, 1usize..8).prop_map(|(c, p, ct, pt, i)| {
            Predicate::Plaintexts(Plaintexts {
                configuration: c,
                public_key: PublicKeyHash(hash_bytes(&p)),
                ciphertexts: CiphertextsHash(hash_bytes(&ct)),
                plaintexts: PlaintextsHash(hash_bytes(&pt)),
                sender: i,
            })
        }),
    ]
}

// -- Mutations (as in vsc's properties harness) --------------------------------

#[derive(Debug, Clone)]
enum Mutation {
    Truncate(usize),
    Extend(Vec<u8>),
    Edit { index: usize, xor: u8 },
}

fn mutation() -> impl Strategy<Value = Mutation> {
    prop_oneof![
        (1usize..64).prop_map(Mutation::Truncate),
        proptest::collection::vec(any::<u8>(), 1..16).prop_map(Mutation::Extend),
        (any::<usize>(), 1u8..=255).prop_map(|(index, xor)| Mutation::Edit { index, xor }),
    ]
}

fn apply(mutation: &Mutation, mut bytes: Vec<u8>) -> Vec<u8> {
    match mutation {
        Mutation::Truncate(n) => {
            let keep = bytes.len().saturating_sub(*n);
            bytes.truncate(keep);
        }
        Mutation::Extend(tail) => bytes.extend_from_slice(tail),
        Mutation::Edit { index, xor } => {
            if let Some(byte) = index
                .checked_rem(bytes.len())
                .and_then(|i| bytes.get_mut(i))
            {
                *byte ^= xor;
            }
        }
    }
    bytes
}

// -- The properties -------------------------------------------------------------

proptest! {
    #[test]
    fn protocol_message_p1_roundtrip(m in protocol_message()) {
        let bytes = m.ser();
        let back = ProtocolMessage::<RistrettoCtx>::deser(&bytes).unwrap();
        prop_assert_eq!(back.ser(), bytes);
    }

    #[test]
    fn protocol_message_p2_strict_mutated(m in protocol_message(), mu in mutation()) {
        let bytes = apply(&mu, m.ser());
        if let Ok(v) = ProtocolMessage::<RistrettoCtx>::deser(&bytes) {
            prop_assert_eq!(v.ser(), bytes);
        }
    }

    #[test]
    fn protocol_message_p2_strict_random(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
        if let Ok(v) = ProtocolMessage::<RistrettoCtx>::deser(&bytes) {
            prop_assert_eq!(v.ser(), bytes);
        }
    }

    #[test]
    fn predicate_p1_roundtrip(p in predicate()) {
        let bytes = p.ser();
        prop_assert_eq!(Predicate::deser(&bytes).unwrap(), p);
    }

    #[test]
    fn predicate_p2_strict_mutated(p in predicate(), mu in mutation()) {
        let bytes = apply(&mu, p.ser());
        if let Ok(v) = Predicate::deser(&bytes) {
            prop_assert_eq!(v.ser(), bytes);
        }
    }

    #[test]
    fn predicate_p2_strict_random(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
        if let Ok(v) = Predicate::deser(&bytes) {
            prop_assert_eq!(v.ser(), bytes);
        }
    }
}
