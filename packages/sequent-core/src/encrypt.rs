// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::elgamal::*;
use strand::hashing::rustcrypto;
use strand::serialization::{StrandDeserialize, StrandSerialize};
use strand::zkp::{Schnorr, Zkp};

use base64::engine::general_purpose;
use base64::Engine;
use hex;

use crate::ballot::*;
use crate::ballot_codec::PlaintextCodec;
use crate::error::BallotError;
use crate::plaintext::DecodedVoteContest;
use crate::serialization::base64::Base64Deserialize;
use crate::util::get_current_date;

pub const DEFAULT_PUBLIC_KEY_RISTRETTO_STR: &str =
    "ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4";

pub fn default_public_key_ristretto() -> (String, <RistrettoCtx as Ctx>::E) {
    let pk_str: String = DEFAULT_PUBLIC_KEY_RISTRETTO_STR.to_string();
    let pk_bytes = general_purpose::STANDARD_NO_PAD
        .decode(pk_str.clone())
        .unwrap();
    let pk = <RistrettoCtx as Ctx>::E::strand_deserialize(&pk_bytes).unwrap();
    (pk_str, pk)
}

pub fn encrypt_plaintext_answer<C: Ctx<P = [u8; 30]>>(
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
    election: &BallotStyle,
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
    if ballot.contests.len() != ballot.config.configuration.questions.len() {
        return Err(BallotError::ConsistencyCheck(String::from(
            "Number of election questions should match number of answers in the ballot",
        )));
    }

    ballot
        .contests
        .clone()
        .into_iter()
        .map(|contests| {
            recreate_encrypt_answer(ctx, &public_key, &contests.choice)
        })
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

pub fn encrypt_decoded_question<C: Ctx<P = [u8; 30]>>(
    ctx: &C,
    decoded_questions: &Vec<DecodedVoteContest>,
    config: &BallotStyle,
) -> Result<AuditableBallot<C>, BallotError> {
    if config.configuration.questions.len() != decoded_questions.len() {
        return Err(BallotError::ConsistencyCheck(format!(
            "Invalid number of decoded questions {} != {}",
            config.configuration.questions.len(),
            decoded_questions.len()
        )));
    }

    let public_key: C::E = parse_public_key::<C>(&config)?;

    let mut contests: Vec<AuditableBallotContest<C>> = vec![];

    for decoded_question in decoded_questions {
        let question = config
            .configuration
            .questions
            .iter()
            .find(|question| question.id == decoded_question.contest_id)
            .ok_or_else(|| {
                BallotError::Serialization(format!(
                    "Can't find contest with id {} on ballot style",
                    decoded_question.contest_id
                ))
            })?;
        let plaintext = question
            .encode_plaintext_question(&decoded_question)
            .map_err(|err| {
            BallotError::Serialization(format!(
                "Error encrypting plaintext: {}",
                err
            ))
        })?;
        let (choice, proof) =
            encrypt_plaintext_answer(ctx, public_key.clone(), plaintext)?;
        contests.push(AuditableBallotContest::<C> {
            contest_id: question.id.clone(),
            choice: choice,
            proof: proof,
        });
    }

    let mut auditable_ballot = AuditableBallot {
        version: TYPES_VERSION,
        issue_date: get_current_date(),
        contests: contests,
        ballot_hash: String::from(""),
        config: config.clone(),
    };

    let hashable_ballot = HashableBallot::from(&auditable_ballot);
    auditable_ballot.ballot_hash = hash_ballot(&hashable_ballot)?;

    Ok(auditable_ballot)
}

// hash ballot:
// serialize ballot into string, then hash to sha512, truncate to
// 256 bits and serialize to hexadecimal
pub fn hash_ballot<C: Ctx>(
    hashable_ballot: &HashableBallot<C>,
) -> Result<String, BallotError> {
    let bytes = hashable_ballot
        .strand_serialize()
        .map_err(|error| BallotError::Serialization(error.to_string()))?;
    let hash_bytes = rustcrypto::hash(bytes.as_slice())
        .map_err(|error| BallotError::Serialization(error.to_string()))?;
    let hash_256bits_slice = &hash_bytes[0..32];
    Ok(hex::encode(hash_256bits_slice))
}

#[cfg(test)]
mod tests {
    use crate::encrypt;

    use strand::backend::ristretto::RistrettoCtx;
    use strand::context::Ctx;
    use strand::rng::StrandRng;

    #[test]
    fn test_encrypt_plaintext_answer() {
        let mut csprng = StrandRng;
        let ctx = RistrettoCtx;

        let (pk_string, pk_element) = encrypt::default_public_key_ristretto();

        let plaintext = ctx.rnd_plaintext(&mut csprng);

        encrypt::encrypt_plaintext_answer(&ctx, pk_element, plaintext).unwrap();
        assert_eq!(
            pk_string.as_str(),
            encrypt::DEFAULT_PUBLIC_KEY_RISTRETTO_STR
        );
    }
}
