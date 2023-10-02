// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use num_bigint::BigUint;
use num_traits::Num;
use sha2::{Digest, Sha256};

use chrono::prelude::*;
use strand::backend::num_bigint::{
    BigUintP, BigintCtx, DeserializeNumber, SerializeNumber, P2048,
};
use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::elgamal::*;
use strand::hashing::rustcrypto;
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
    // Possible contexts:
    // let ctx = RistrettoCtx;
    // let ctx: BigintCtx<P2048> = Default::default();

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
            &ciphertext.mhr,
            &ciphertext.gr,
            &proof,
            &vec![],
        )
        .unwrap();
    assert!(proof_ok);

    Ok((
        ReplicationChoice {
            ciphertext: ciphertext,
            plaintext: plaintext,
            randomness: randomness,
        },
        proof,
    ))
}

pub fn parse_public_key<C: Ctx>(
    election: &ElectionDTO,
) -> Result<C::E, BallotError> {
    let public_key_config: PublicKeyConfig = election
        .public_key
        .clone()
        .ok_or(BallotError::ConsistencyCheck(
            "Missing Public Key".to_string(),
        ))?;
    Base64Deserialize::deserialize(public_key_config.public_key)
}

pub fn recreate_encrypt_cyphertext<C: Ctx>(
    ctx: &C,
    ballot: &AuditableBallot<C>,
) -> Result<Vec<ReplicationChoice<C>>, BallotError> {
    let public_key = parse_public_key::<C>(&ballot.config)?;
    // check ballot version
    // sanity checks for number of answers/choices
    if ballot.choices.len() != ballot.config.configuration.questions.len() {
        return Err(BallotError::ConsistencyCheck(String::from(
            "Number of election questions should match number of answers in the ballot",
        )));
    }

    ballot
        .choices
        .clone()
        .into_iter()
        .map(|choice| recreate_encrypt_answer(ctx, &public_key, &choice))
        .collect::<Vec<Result<ReplicationChoice<C>, BallotError>>>()
        .into_iter()
        .collect()
}

fn recreate_encrypt_answer<C: Ctx>(
    ctx: &C,
    public_key_element: &C::E,
    choice: &ReplicationChoice<C>,
) -> Result<ReplicationChoice<C>, BallotError> {
    // construct a public key from a provided element
    let public_key = PublicKey::from_element(public_key_element, ctx);

    let encoded = ctx.encode(&choice.plaintext).unwrap();

    // encrypt / create ciphertext
    let ciphertext =
        public_key.encrypt_with_randomness(&encoded, &choice.randomness);

    // convert to output format
    Ok(ReplicationChoice {
        ciphertext: ciphertext,
        plaintext: choice.plaintext.clone(),
        randomness: choice.randomness.clone(),
    })
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

    let public_key: C::E = parse_public_key::<C>(&config)?;

    let mut choices: Vec<ReplicationChoice<C>> = vec![];
    let mut proofs: Vec<Schnorr<C>> = vec![];
    for i in 0..decoded_questions.len() {
        let question = config.configuration.questions[i].clone();
        let decoded_question = decoded_questions[i].clone();
        let plaintext = question
            .encode_plaintext_question::<C>(&decoded_question)
            .map_err(|_err| {
                BallotError::Serialization(format!("Error encoding plaintext"))
            })?;
        let (choice, proof) =
            encrypt_plaintext_answer(ctx, public_key.clone(), plaintext)?;
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

    let hashable_ballot = HashableBallot::from(&auditable_ballot);
    auditable_ballot.ballot_hash = hash_to(&hashable_ballot)?;

    Ok(auditable_ballot)
}

pub fn hash_to<C: Ctx>(
    hashable_ballot: &HashableBallot<C>,
) -> Result<String, BallotError> {
    let bytes = hashable_ballot
        .strand_serialize()
        .map_err(|error| BallotError::Serialization(error.to_string()))?;
    let hash_bytes = rustcrypto::hash(bytes.as_slice())
        .map_err(|error| BallotError::Serialization(error.to_string()))?;
    Base64Serialize::serialize(&hash_bytes)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_elfelix() {
        assert!(false, "false wasn't true! hehe");
    }
}
