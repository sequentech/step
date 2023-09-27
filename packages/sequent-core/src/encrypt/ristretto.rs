// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

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

pub fn encrypt_plaintext_answer<C: Ctx>(
    ctx: &C,
    public_key_element: <C>::E,
    plaintext: <C>::P,
) -> Result<(ReplicationChoice<C>, Schnorr<C>), BallotError> {
    //let ctx = RistrettoCtx;
    //let ctx: BigintCtx<P2048> = Default::default();

    // construct a public key from a provided element
    let pk = PublicKey::from_element(&public_key_element, ctx);

    let encoded = ctx.encode(&plaintext).unwrap();

    // encrypt and prove knowledge of plaintext (enc + pok)
    let (ciphertext, proof, randomness) =
        pk.encrypt_and_pok(&encoded, &vec![]).unwrap();
    // verify
    let zkp = Zkp::new(ctx);
    let proof_ok = zkp
        .encryption_popk_verify(
            ciphertext.mhr(),
            ciphertext.gr(),
            &proof,
            &vec![],
        )
        .unwrap();
    assert!(proof_ok);

    Ok((
        ReplicationChoice {
            gr: ciphertext.gr,   // gr/alpha
            mhr: ciphertext.mhr, // mhr/beta
            plaintext: plaintext,
            randomness: randomness,
        },
        proof,
    ))
}

pub fn parse_public_key<C: Ctx>(
    election: &ElectionDTO,
) -> Result<DkgPublicKey<C>, BallotError> {
    let public_key_config =
        election
            .public_key
            .clone()
            .ok_or(BallotError::ConsistencyCheck(
                "Missing Public Key".to_string(),
            ))?;
    Base64Deserialize::deserialize(public_key_config.public_key)
}
/*
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
*/

pub fn encrypt_decoded_question<C: Ctx>(
    ctx: &C,
    decoded_questions: &Vec<DecodedVoteQuestion>,
    config: &ElectionDTO,
) -> Result<AuditableBallot<C>, BallotError> {
    if config.configuration.questions.len() != decoded_questions.len() {
        return Err(BallotError::ConsistencyCheck(format!(
            "Invalid number of decoded questions {} != {}",
            config.configuration.questions.len(),
            decoded_questions.len()
        )));
    }

    let public_key: DkgPublicKey<C> = parse_public_key(&config)?;

    let mut choices: Vec<ReplicationChoice<C>> = vec![];
    let mut proofs: Vec<Schnorr<C>> = vec![];
    for i in 0..decoded_questions.len() {
        let question = config.configuration.questions[i].clone();
        let decoded_question = decoded_questions[i].clone();
        let plaintext = question
            .encode_plaintext_question::<C>(&decoded_question)
            .map_err(|_err| {
                BallotError::Serialization(format!(
                    "Error encoding plaintext"
                ))
            })?;
        let (choice, proof) =
            encrypt_plaintext_answer(ctx, public_key.pk.clone(), plaintext)?;
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

    auditable_ballot.ballot_hash = hash_to(&auditable_ballot)?;

    Ok(auditable_ballot)
}

pub fn hash_to<C: Ctx>(auditable_ballot: AuditableBallot<C>) -> Result<String, BallotError> {
    let hashable_ballot = HashableBallot::from(auditable_ballot);
    Base64Serialize::serialize(&hashable_ballot)
}
