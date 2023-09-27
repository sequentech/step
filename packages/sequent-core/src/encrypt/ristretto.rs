// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
/*
use num_bigint::BigUint;
use num_traits::Num;
use sha2::{Digest, Sha256};

use braid::protocol2::artifact::DkgPublicKey;
use chrono::prelude::*;
use strand::backend::num_bigint::{
    BigUintP, BigintCtx, DeserializeNumber, SerializeNumber, P2048,
};
use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::elgamal::*;
use strand::serialization::{StrandDeserialize, StrandSerialize};
use strand::signature::StrandSignaturePk;
use strand::util::StrandError;
use strand::zkp::{Schnorr, Zkp};

use base64::engine::general_purpose;
use base64::DecodeError;
use base64::Engine;
use std::error::Error;

use crate::ballot::*;
use crate::ballot_codec::BallotCodec;
use crate::base64::{Base64Deserialize, Base64Serialize};
use crate::error::BallotError;
use crate::plaintext::DecodedVoteQuestion;
use crate::util::get_current_date;

pub fn encrypt_plaintext_answer_ecc(
    public_key_element: <RistrettoCtx as Ctx>::E,
    plaintext: <RistrettoCtx as Ctx>::P,
) -> Result<
    (
        Ciphertext<RistrettoCtx>,
        Schnorr<RistrettoCtx>,
        <RistrettoCtx as Ctx>::X,
    ),
    BallotError,
> {
    let ctx = RistrettoCtx;

    // construct a public key from a provided element
    let pk = PublicKey::from_element(&public_key_element, &ctx);

    let encoded = ctx.encode(&plaintext).unwrap();

    // encrypt and prove knowledge of plaintext (enc + pok)
    let (c, proof, randomness) = pk.encrypt_and_pok(&encoded, &vec![]).unwrap();
    // verify
    let zkp = Zkp::new(&ctx);
    let proof_ok = zkp
        .encryption_popk_verify(c.mhr(), c.gr(), &proof, &vec![])
        .unwrap();
    assert!(proof_ok);

    Ok((c, proof, randomness))
}

pub fn encrypt_plaintext_answer_ecc_wrapper(
    public_key_element: <RistrettoCtx as Ctx>::E,
    plaintext: <RistrettoCtx as Ctx>::P,
) -> Result<(ReplicationChoice<RistrettoCtx>, Schnorr<RistrettoCtx>), BallotError>
{
    let (ciphertext, proof, randomness) =
        encrypt_plaintext_answer_ecc(public_key_element, plaintext)?;
    // convert to output format
    Ok((
        ReplicationChoice {
            gr: Base64Serialize::serialize(&ciphertext)?, // gr/alpha
            mhr: "".to_string(),                          // mhr/beta
            plaintext: Base64Serialize::serialize(&plaintext)?,
            randomness: Base64Serialize::serialize(&randomness)?,
        },
        proof,
    ))
}

pub fn parse_public_key_ecc(
    election: &ElectionDTO,
) -> Result<DkgPublicKey<RistrettoCtx>, BallotError> {
    let public_key_config =
        election
            .public_key
            .clone()
            .ok_or(BallotError::ConsistencyCheck(
                "Missing Public Key".to_string(),
            ))?;
    Base64Deserialize::deserialize(public_key_config.public_key)
}

pub fn to_30bytes(plaintext: Vec<u8>) -> Result<[u8; 30], BallotError> {
    let len = plaintext.len();

    if len > 30 {
        return Err(BallotError::Serialization(format!(
            "Plaintext too long, length {} is longer than 30 bytes",
            len
        )));
    }
    let mut array: [u8; 30] = [0; 30];

    // Copy the elements from the vector to the array
    array[..len].copy_from_slice(&plaintext);

    Ok(array)
}

pub fn encrypt_decoded_question_ecc(
    decoded_questions: &Vec<DecodedVoteQuestion>,
    config: &ElectionDTO,
) -> Result<AuditableBallot<RistrettoCtx>, BallotError> {
    if config.configuration.questions.len() != decoded_questions.len() {
        return Err(BallotError::ConsistencyCheck(format!(
            "Invalid number of decoded questions {} != {}",
            config.configuration.questions.len(),
            decoded_questions.len()
        )));
    }

    let public_key = parse_public_key_ecc(&config)?;

    let mut choices: Vec<ReplicationChoice> = vec![];
    let mut proofs: Vec<Schnorr<RistrettoCtx>> = vec![];
    for i in 0..decoded_questions.len() {
        let question = config.configuration.questions[i].clone();
        let decoded_question = decoded_questions[i].clone();
        let plaintext = question
            .encode_plaintext_question_to_bytes(&decoded_question)
            .map_err(|_err| {
                BallotError::Serialization(format!(
                    "Error encoding vote choice"
                ))
            })?;
        let plaintext_30b = to_30bytes(plaintext)?;

        let (choice, proof) = encrypt_plaintext_answer_ecc_wrapper(
            public_key.pk.clone(),
            plaintext_30b,
        )?;
        choices.push(choice);
        proofs.push(proof);
    }

    let mut auditable_ballot = AuditableBallot {
        version: TYPES_VERSION,
        issue_date: get_current_date(),
        choices: choices,
        proofs: proofs,
        ballot_hash: String::from(""),
        config: config.clone(),
    };

    // TODO
    // auditable_ballot.ballot_hash = hash_to(&auditable_ballot)?;

    Ok(auditable_ballot)
}
*/
