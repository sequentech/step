// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Independent public transcript verification. No trustee identity, secret,
//! persistence or protocol execution is needed. `Ok(None)` means incomplete,
//! never a successfully verified ceremony/tally. The configuration hash and
//! tally identifier must come from the caller's trusted election record.
//!
//! Public commitments and their knowledge proofs are checked here. Encrypted
//! DKG shares can only be checked against commitments by their recipients;
//! public verification does not claim to decrypt those private dealings.

use std::{collections::HashSet, marker::PhantomData};

use anyhow::{anyhow, ensure, Result};
use cryptography::context::Context;
use cryptography::cryptosystem::{elgamal, naoryung};
use cryptography::dkgd::recipient::{
    combine, AttributedDecryption, ParticipantPosition, Recipient,
};
use cryptography::traits::groups::CryptographicGroup;
use cryptography::utils::serialization::{Deserializable, Serializable};
use cryptography::zkp::shuffle::Shuffler;

use crate::messages::artifact::{
    Ballots, Configuration, DkgPublicKey, Mix, PartialDecryption, Plaintexts, Shares,
};
use crate::messages::newtypes::{hash_bytes, ConfigurationHash, PublicKeyHash};
use crate::messages::predicate::Predicate;
use crate::messages::wire::{MessageType, ProtocolMessage};
use crate::trustee::{
    ballot_encryption_context, domain_label, shuffle_generators_seed, shuffle_proof_label,
    tally_label, DKG_CHECKING_VALUE_PURPOSE,
};

/// Constructed only after verifying the complete public DKG transcript.
pub struct VerifiedDkg<C: Context> {
    configuration: Configuration<C>,
    configuration_hash: ConfigurationHash,
    public_key: DkgPublicKey<C>,
    public_key_hash: PublicKeyHash,
}

impl<C: Context> VerifiedDkg<C> {
    pub fn configuration(&self) -> &Configuration<C> {
        &self.configuration
    }
    pub fn public_key(&self) -> &DkgPublicKey<C> {
        &self.public_key
    }
    pub fn public_key_hash(&self) -> PublicKeyHash {
        self.public_key_hash
    }
}

struct Record {
    predicate: Predicate,
    body: Option<Vec<u8>>,
}

#[derive(PartialEq, Eq)]
enum TranscriptKind {
    Dkg,
    Tally,
}
impl Record {
    fn body(&self) -> Result<&[u8]> {
        self.body
            .as_deref()
            .ok_or_else(|| anyhow!("missing transcript body"))
    }
}

/// Validate every signature, configuration reference, role and equivocation
/// before interpreting any proof. Identical re-statements are idempotent.
fn records<C: Context>(
    messages: &[ProtocolMessage<C>],
    cfg: &Configuration<C>,
    anchor: ConfigurationHash,
    kind: TranscriptKind,
) -> Result<Vec<Record>> {
    let mut result: Vec<Record> = Vec::new();
    for message in messages {
        if message.message_type == MessageType::Configuration {
            ensure!(
                kind == TranscriptKind::Dkg,
                "configuration message on a tally board"
            );
            let configuration = message.verify_configuration()?;
            ensure!(
                ConfigurationHash::from_configuration(&configuration)? == anchor,
                "configuration does not match the trusted hash"
            );
            continue;
        }
        let (predicate, body) = crate::board::verify::verify(message, cfg)?;
        ensure!(
            predicate.get_configuration() == anchor,
            "message belongs to another configuration"
        );
        let sender = match &predicate {
            Predicate::Shares(p) if kind == TranscriptKind::Dkg => p.sender,
            Predicate::PublicKey(p) if kind == TranscriptKind::Dkg => p.sender,
            Predicate::Ballots(_) if kind == TranscriptKind::Tally => 0,
            Predicate::Mix(p) if kind == TranscriptKind::Tally => p.sender,
            Predicate::MixSignature(p) if kind == TranscriptKind::Tally => p.sender,
            Predicate::PartialDecryptions(p) if kind == TranscriptKind::Tally => p.sender,
            Predicate::Plaintexts(p) if kind == TranscriptKind::Tally => p.sender,
            _ => return Err(anyhow!("unexpected message type on this board")),
        };
        ensure!(
            sender <= cfg.trustees.len(),
            "message must be authored by a committee member"
        );
        ensure!(
            !result.iter().any(|r| r.predicate.collides(&predicate)),
            "equivocating transcript"
        );
        if !result.iter().any(|r| r.predicate == predicate) {
            result.push(Record { predicate, body });
        }
    }
    Ok(result)
}

/// Verify Schnorr proofs, derive the joint and every verification key from
/// the commitments, and require every trustee's published key to match them.
pub fn verify_dkg<C: Context>(
    messages: &[ProtocolMessage<C>],
    expected: ConfigurationHash,
) -> Result<Option<VerifiedDkg<C>>> {
    let Some(message) = messages
        .iter()
        .find(|m| m.message_type == MessageType::Configuration)
    else {
        return Ok(None);
    };
    let raw = message.verify_configuration()?;
    ensure!(
        ConfigurationHash::from_configuration(&raw)? == expected,
        "configuration does not match the trusted hash"
    );
    // Deserialization does not run Configuration::new's validity checks.
    let cfg = Configuration::new(
        raw.id,
        raw.protocol_manager,
        raw.trustees,
        raw.threshold,
        raw.ciphertext_width,
        raw.share_encryption_keys,
        PhantomData,
    )?;
    ensure!(
        !cfg.trustees.contains(&cfg.protocol_manager),
        "manager cannot also be a trustee"
    );
    let records = records(messages, &cfg, expected, TranscriptKind::Dkg)?;
    let mut dealings = Vec::new();
    let context = domain_label(&expected, DKG_CHECKING_VALUE_PURPOSE);
    for sender in 1..=cfg.trustees.len() {
        if let Some(record) = records
            .iter()
            .find(|r| matches!(&r.predicate, Predicate::Shares(p) if p.sender == sender))
        {
            let shares = Shares::<C>::deser(record.body()?)?;
            ensure!(
                shares.commitments.len() == cfg.threshold
                    && shares.encrypted_shares.len() == cfg.trustees.len(),
                "invalid DKG dealing dimensions"
            );
            for commitment in &shares.commitments {
                ensure!(
                    commitment.verify(&C::generator(), &context)?,
                    "invalid DKG commitment proof"
                );
            }
            dealings.push(
                shares
                    .commitments
                    .into_iter()
                    .map(|c| c.value)
                    .collect::<Vec<_>>(),
            );
        }
    }
    if dealings.len() != cfg.trustees.len() {
        return Ok(None);
    }
    let key = crate::dispatch_threshold_trustees!(
        cfg.threshold,
        cfg.trustees.len(),
        derive_key::<C, T, P>(dealings)
    )?;
    let key_hash = PublicKeyHash(hash_bytes(&key.ser()));
    let mut published = 0;
    for record in &records {
        if let Predicate::PublicKey(p) = &record.predicate {
            ensure!(
                p.public_key == key_hash && record.body()? == key.ser(),
                "published public key differs from the commitments"
            );
            published += 1;
        }
    }
    if published != cfg.trustees.len() {
        return Ok(None);
    }
    Ok(Some(VerifiedDkg {
        configuration: cfg,
        configuration_hash: expected,
        public_key: key,
        public_key_hash: key_hash,
    }))
}

#[inline(never)]
fn derive_key<C: Context, const T: usize, const P: usize>(
    values: Vec<Vec<C::Element>>,
) -> Result<DkgPublicKey<C>> {
    let values: [[C::Element; T]; P] = values
        .into_iter()
        .map(|v| v.try_into().map_err(|_| anyhow!("invalid commitments")))
        .collect::<Result<Vec<_>>>()?
        .try_into()
        .map_err(|_| anyhow!("invalid dealer count"))?;
    let pk = Recipient::<C, T, P>::joint_public_key(&values).y;
    let verification_keys = (1..=P)
        .map(|i| {
            Recipient::<C, T, P>::verification_key(&ParticipantPosition::from_usize(i), &values)
        })
        .collect();
    Ok(DkgPublicKey::new(pk, verification_keys))
}

/// Verify Naor-Yung ballots, every shuffle and endorsement, attributed
/// threshold-decryption proofs, and independently reproduce the plaintexts.
pub fn verify_tally<C: Context, const W: usize>(
    dkg: &VerifiedDkg<C>,
    messages: &[ProtocolMessage<C>],
    expected_tally_id: u128,
) -> Result<Option<Plaintexts<C, W>>> {
    let cfg = &dkg.configuration;
    ensure!(
        cfg.ciphertext_width == W,
        "ciphertext width does not match the verifier"
    );
    let records = records(messages, cfg, dkg.configuration_hash, TranscriptKind::Tally)?;
    let Some(input) = records
        .iter()
        .find(|r| matches!(r.predicate, Predicate::Ballots(_)))
    else {
        return Ok(None);
    };
    let Predicate::Ballots(head) = &input.predicate else {
        unreachable!()
    };
    ensure!(
        head.tally_id == expected_tally_id,
        "tally identifier does not match the trusted identifier"
    );
    let quorum = &head.trustees;
    ensure!(
        quorum.len() == cfg.threshold
            && quorum.iter().collect::<HashSet<_>>().len() == quorum.len()
            && quorum.iter().all(|i| (1..=cfg.trustees.len()).contains(i)),
        "invalid tally quorum"
    );
    for record in &records {
        let (pk, sender) = match &record.predicate {
            Predicate::Ballots(p) => (p.public_key, None),
            Predicate::Mix(p) => (p.public_key, Some(p.sender)),
            Predicate::MixSignature(p) => (p.public_key, Some(p.sender)),
            Predicate::PartialDecryptions(p) => (p.public_key, Some(p.sender)),
            Predicate::Plaintexts(p) => (p.public_key, Some(p.sender)),
            _ => unreachable!(),
        };
        ensure!(
            pk == dkg.public_key_hash,
            "message references another public key"
        );
        ensure!(
            sender.is_none_or(|s| quorum.contains(&s)),
            "message from outside the selected quorum"
        );
    }
    let ballots = Ballots::<C, W>::deser(input.body()?)?;
    let context = ballot_encryption_context::<C>(cfg.id, &dkg.public_key.pk);
    let key = elgamal::PublicKey::new(dkg.public_key.pk.clone());
    let ny_key = naoryung::PublicKey::augment(&key, &context)?;
    let mut ciphertexts = ballots
        .ciphertexts
        .into_iter()
        .map(|c| ny_key.strip(c, &context))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut current = head.ciphertexts;
    let mut links = Vec::new();
    let mut incomplete = false;
    for sender in quorum {
        let Some(record) = records
            .iter()
            .find(|r| matches!(&r.predicate, Predicate::Mix(p) if p.sender == *sender))
        else {
            return Ok(None);
        };
        let Predicate::Mix(m) = &record.predicate else {
            unreachable!()
        };
        ensure!(m.input == current, "mix does not extend the ordered chain");
        let mix = Mix::<C, W>::deser(record.body()?)?;
        ensure!(
            mix.ciphertexts.len() == ciphertexts.len(),
            "mix changes the ballot count"
        );
        if ciphertexts.is_empty() {
            ensure!(
                mix.proof.is_none() && mix.null_input == Some(current),
                "invalid empty shuffle"
            );
        } else {
            let proof = mix
                .proof
                .as_ref()
                .ok_or_else(|| anyhow!("missing shuffle proof"))?;
            let seed =
                shuffle_generators_seed(&dkg.configuration_hash, expected_tally_id, &current);
            let label = shuffle_proof_label(&dkg.configuration_hash, expected_tally_id, &current);
            ensure!(
                Shuffler::new(C::G::ind_generators(ciphertexts.len(), &seed)?, key.clone())
                    .verify(&ciphertexts, &mix.ciphertexts, proof, &label)?,
                "invalid shuffle proof"
            );
        }
        for signer in quorum.iter().filter(|s| *s != sender) {
            let endorsed = records.iter().any(|record| match &record.predicate {
                Predicate::MixSignature(signature) => {
                    signature.sender == *signer
                        && signature.input == current
                        && signature.output == m.output
                }
                _ => false,
            });
            incomplete |= !endorsed;
        }
        links.push((current, m.output, *sender));
        current = m.output;
        ciphertexts = mix.ciphertexts;
    }
    for record in &records {
        match &record.predicate {
            Predicate::MixSignature(p) => ensure!(
                links
                    .iter()
                    .any(|(i, o, s)| *i == p.input && *o == p.output && *s != p.sender),
                "signature does not endorse a chain link"
            ),
            Predicate::PartialDecryptions(p) => ensure!(
                p.ciphertexts == current,
                "partial decryption does not reference the final mix"
            ),
            Predicate::Plaintexts(p) => ensure!(
                p.ciphertexts == current,
                "plaintexts do not reference the final mix"
            ),
            _ => (),
        }
    }
    let mut contributions = Vec::with_capacity(quorum.len());
    for sender in quorum {
        if let Some(record) = records.iter().find(
            |r| matches!(&r.predicate, Predicate::PartialDecryptions(p) if p.sender == *sender),
        ) {
            contributions.push((*sender, PartialDecryption::<C, W>::deser(record.body()?)?));
        }
    }
    if contributions.len() != cfg.threshold {
        return Ok(None);
    }
    let label = tally_label(
        &dkg.configuration_hash,
        expected_tally_id,
        "decryption proof",
    );
    let plaintexts = crate::dispatch_threshold_trustees!(
        cfg.threshold,
        cfg.trustees.len(),
        decrypt::<C, T, P, W>(&ciphertexts, contributions, &dkg.public_key, &label)
    )?;
    let body = plaintexts.ser();
    let mut published = 0;
    for record in &records {
        if let Predicate::Plaintexts(_) = &record.predicate {
            ensure!(
                record.body()? == body,
                "published plaintexts differ from independently verified decryption"
            );
            published += 1;
        }
    }
    if incomplete || published != cfg.threshold {
        return Ok(None);
    }
    Ok(Some(plaintexts))
}

#[inline(never)]
fn decrypt<C: Context, const T: usize, const P: usize, const W: usize>(
    ciphertexts: &[elgamal::Ciphertext<C, W>],
    contributions: Vec<(usize, PartialDecryption<C, W>)>,
    key: &DkgPublicKey<C>,
    label: &[u8],
) -> Result<Plaintexts<C, W>> {
    // Positions have been range-checked and the quorum made distinct above.
    let contributions: [AttributedDecryption<C, W, P>; T] = contributions
        .into_iter()
        .map(|(sender, partial)| {
            AttributedDecryption::new(
                partial,
                ParticipantPosition::from_usize(sender),
                key.verification_keys[sender - 1].clone(),
            )
        })
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| anyhow!("invalid contribution count"))?;
    Ok(Plaintexts(combine::<C, T, P, W>(
        ciphertexts,
        &contributions,
        label,
    )?))
}
