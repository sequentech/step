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
use crate::plaintext::DecodedVoteQuestion;

quick_error! {
    #[derive(Debug, PartialEq, Eq)]
    pub enum BallotError {
        ParseBigUint(uint_str: String, message: String) {}
        CryptographicCheck(message: String) {}
        ConsistencyCheck(message: String) {}
        Serialization(message: String) {}
    }
}

pub trait Base64Serialize {
    fn serialize(&self) -> Result<String, BallotError>;
}

pub trait Base64Deserialize {
    fn deserialize(value: String) -> Result<Self, BallotError>
    where
        Self: Sized;
}
/*
impl Base64Serialize for <RistrettoCtx as Ctx>::P {
    fn serialize(&self) -> String {
        general_purpose::STANDARD_NO_PAD.encode(self)
    }
}
impl Base64Deserialize for <RistrettoCtx as Ctx>::P {
    fn deserialize(value: String) -> Result<Self, DecodeError> where Self: Sized {
        let vec = general_purpose::STANDARD_NO_PAD.decode(value)?;
        if vec.len() > 30 {
            return Err(DecodeError::InvalidLength);
        }
        let mut array: [u8; 30] = [0; 30];
        array[..vec.len()].copy_from_slice(&vec);

        Ok(array)
    }
}
*/

impl<T: StrandSerialize> Base64Serialize for T {
    fn serialize(&self) -> Result<String, BallotError> {
        let bytes = self
            .strand_serialize()
            .map_err(|error| BallotError::Serialization(error.to_string()))?;
        Ok(general_purpose::STANDARD_NO_PAD.encode(bytes))
    }
}

impl<T: StrandDeserialize> Base64Deserialize for T {
    fn deserialize(value: String) -> Result<Self, BallotError>
    where
        Self: Sized,
    {
        let bytes_vec = general_purpose::STANDARD_NO_PAD.decode(value).unwrap();
        StrandDeserialize::strand_deserialize(&bytes_vec.as_slice())
            .map_err(|error| BallotError::Serialization(error.to_string()))
    }
}

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
) -> Result<(ReplicationChoice, CyphertextProof), BallotError> {
    let (ciphertext, proof, randomness) =
        encrypt_plaintext_answer_ecc(public_key_element, plaintext)?;
    // convert to output format
    Ok((
        ReplicationChoice {
            alpha: Base64Serialize::serialize(&ciphertext)?,
            beta: "".to_string(),
            plaintext: Base64Serialize::serialize(&plaintext)?,
            randomness: Base64Serialize::serialize(&randomness)?,
        },
        CyphertextProof {
            challenge: Base64Serialize::serialize(&proof.challenge)?,
            commitment: Base64Serialize::serialize(&proof.commitment)?,
            response: Base64Serialize::serialize(&proof.response)?,
        },
    ))
}

pub fn encrypt_plaintext_answer(
    public_key: &Pk,
    plaintext_str: String,
) -> Result<(ReplicationChoice, CyphertextProof), BallotError> {
    // create P2048 context
    let ctx: BigintCtx<P2048> = Default::default();

    // create public key
    let pk_bigint =
        BigUint::from_str_radix(&public_key.y, 10).map_err(|_| {
            BallotError::ParseBigUint(
                public_key.y.clone(),
                String::from("Error parsing public key"),
            )
        })?;
    let pk_bigint_e = ctx
        .element_from_bytes(&pk_bigint.to_bytes_le())
        .map_err(|_| {
            BallotError::CryptographicCheck(String::from(
                "Error parsing public key as an element",
            ))
        })?;
    let pk = PublicKey::from_element(&pk_bigint_e, &ctx);

    // parse plaintext
    let plaintext =
        BigUintP::from_str_radix(&plaintext_str, 10).map_err(|_| {
            BallotError::ParseBigUint(
                plaintext_str.clone(),
                String::from("Error parsing plaintext"),
            )
        })?;
    let plaintext_e = ctx.encode(&plaintext).map_err(|error| {
        BallotError::CryptographicCheck(format!(
            "Error parsing plaintext as an element: {}, error: {}",
            plaintext_str, error
        ))
    })?;

    /*
    if KeyType::P2048 != public_key.key_type {
        return Err(BallotError::ConsistencyCheck(String::from(
            "Invalid key type",
        )));
    }
    */

    // encrypt / create cyphertext
    let label = vec![];
    let (cyphertext, proof, randomness) =
        pk.encrypt_and_pok(&plaintext_e, &label).map_err(|_| {
            BallotError::CryptographicCheck(String::from(
                "Error encrypting plaintext",
            ))
        })?;

    // convert to output format
    Ok((
        ReplicationChoice {
            alpha: cyphertext.gr().to_str_radix(10),
            beta: cyphertext.mhr().to_str_radix(10),
            plaintext: plaintext_str.clone(),
            randomness: randomness.to_str_radix(10),
        },
        CyphertextProof {
            challenge: proof.challenge.to_str_radix(10),
            commitment: proof.commitment.to_str_radix(10),
            response: proof.response.to_str_radix(10),
        },
    ))
}

fn recreate_encrypt_answer(
    public_key: &Pk,
    choice: &ReplicationChoice,
) -> Result<ReplicationChoice, BallotError> {
    // create P2048 context
    let ctx: BigintCtx<P2048> = Default::default();

    // create public key
    let pk_bigint =
        BigUint::from_str_radix(&public_key.y, 10).map_err(|_| {
            BallotError::ParseBigUint(
                public_key.y.clone(),
                String::from("Error parsing public key"),
            )
        })?;
    let pk_bigint_e = ctx
        .element_from_bytes(&pk_bigint.to_bytes_le())
        .map_err(|_| {
            BallotError::CryptographicCheck(String::from(
                "Error parsing public key as an element",
            ))
        })?;
    let pk = PublicKey::from_element(&pk_bigint_e, &ctx);

    // parse plaintext
    let plaintext =
        BigUintP::from_str_radix(&choice.plaintext, 10).map_err(|_| {
            BallotError::ParseBigUint(
                choice.plaintext.clone(),
                String::from("Error parsing plaintext"),
            )
        })?;
    let plaintext_e = ctx.encode(&plaintext).map_err(|_| {
        BallotError::CryptographicCheck(String::from(
            "Error parsing plaintext as an element",
        ))
    })?;

    // parse randomness
    let randomness =
        BigUint::from_str_radix(&choice.randomness, 10).map_err(|_| {
            BallotError::ParseBigUint(
                choice.randomness.clone(),
                String::from("Error parsing randomness"),
            )
        })?;
    let randomness_e =
        ctx.exp_from_bytes(&randomness.to_bytes_le()).map_err(|_| {
            BallotError::CryptographicCheck(String::from(
                "Error parsing randomness as an element",
            ))
        })?;

    /*
    if KeyType::P2048 != public_key.key_type {
        return Err(BallotError::ConsistencyCheck(String::from(
            "Invalid key type",
        )));
    }
    */

    // encrypt / create cyphertext
    let cyphertext = pk.encrypt_with_randomness(&plaintext_e, &randomness_e);

    // convert to output format
    Ok(ReplicationChoice {
        alpha: cyphertext.gr().to_str_radix(10),
        beta: cyphertext.mhr().to_str_radix(10),
        plaintext: choice.plaintext.clone(),
        randomness: choice.randomness.clone(),
    })
}

pub fn parse_public_keys(
    election: &ElectionDTO,
) -> Result<Vec<Pk>, BallotError> {
    serde_json::from_str(&election.pks.clone().expect("Public Keys required"))
        .map_err(|err| (BallotError::Serialization(err.to_string())))
}

pub fn recreate_encrypt_cyphertext(
    ballot: &AuditableBallot,
) -> Result<Vec<ReplicationChoice>, BallotError> {
    let pks = parse_public_keys(&ballot.config)?;
    // check ballot version
    // sanity checks for number of answers/choices
    if ballot.choices.len() != pks.len() {
        return Err(BallotError::ConsistencyCheck(String::from(
            "Number of public keys should match number of answers in the ballot",
        )));
    }
    if pks.len() != ballot.config.configuration.questions.len() {
        return Err(BallotError::ConsistencyCheck(String::from(
            "Number of public keys should match number of election questions",
        )));
    }
    let mut choices = vec![];

    #[allow(clippy::needless_range_loop)]
    for i in 0..ballot.choices.len() {
        let cyphertext_answer =
            recreate_encrypt_answer(&pks[i], &ballot.choices[i])?;
        choices.push(cyphertext_answer);
    }
    Ok(choices)
}

pub fn hash_cyphertext(
    cyphertext: &HashableBallot,
) -> Result<String, BallotError> {
    let ballot_str = serde_json::to_string(&cyphertext).map_err(|_| {
        BallotError::Serialization(String::from("Error serializing cyphertext"))
    })?;
    let mut hasher = Sha256::new();
    hasher.update(ballot_str.as_bytes());
    let hashed = hasher.finalize();
    Ok(hex::encode(hashed))
}

pub fn hash_to(ballot: &AuditableBallot) -> Result<String, BallotError> {
    let cyphertext = recreate_encrypt_cyphertext(ballot)?;
    let mut ballot_clone = ballot.clone();
    ballot_clone.choices = cyphertext;
    let hashable_ballot = HashableBallot::from(ballot_clone);
    hash_cyphertext(&hashable_ballot)
}

fn get_current_date() -> String {
    let local: DateTime<Local> = Local::now();
    local.format("%-d/%-m/%Y").to_string()
}

pub fn encrypt_decoded_question(
    decoded_questions: &Vec<DecodedVoteQuestion>,
    config: &ElectionDTO,
) -> Result<AuditableBallot, BallotError> {
    if config.configuration.questions.len() != decoded_questions.len() {
        return Err(BallotError::ConsistencyCheck(format!(
            "Invalid number of decoded questions {} != {}",
            config.configuration.questions.len(),
            decoded_questions.len()
        )));
    }

    let pks = parse_public_keys(&config)?;

    let mut choices: Vec<ReplicationChoice> = vec![];
    let mut proofs: Vec<CyphertextProof> = vec![];
    for i in 0..decoded_questions.len() {
        let question = config.configuration.questions[i].clone();
        let decoded_question = decoded_questions[i].clone();
        let plaintext = question
            .encode_plaintext_question(&decoded_question)
            .map_err(|_err| {
            BallotError::Serialization(format!("Error encoding vote choice"))
        })?;

        let (choice, proof) = encrypt_plaintext_answer(&pks[i], plaintext)?;
        choices.push(choice);
        proofs.push(proof);
    }

    let mut auditable_ballot = AuditableBallot {
        issue_date: get_current_date(),
        choices: choices,
        proofs: proofs,
        ballot_hash: String::from(""),
        config: config.clone(),
    };

    auditable_ballot.ballot_hash = hash_to(&auditable_ballot)?;

    Ok(auditable_ballot)
}

pub fn parse_public_key_ecc(
    election: &ElectionDTO,
) -> Result<DkgPublicKey<RistrettoCtx>, BallotError> {
    let public_key_config = election.public_key.ok_or(
        BallotError::ConsistencyCheck("Missing Public Key".to_string()),
    )?;
    let pk_bytes =
        general_purpose::STANDARD_NO_PAD.decode(public_key_config.public_key);

    pk_bytes.strand_deserialize()
}

pub fn encrypt_decoded_question_ecc(
    decoded_questions: &Vec<DecodedVoteQuestion>,
    config: &ElectionDTO,
) -> Result<AuditableBallot, BallotError> {
    if config.configuration.questions.len() != decoded_questions.len() {
        return Err(BallotError::ConsistencyCheck(format!(
            "Invalid number of decoded questions {} != {}",
            config.configuration.questions.len(),
            decoded_questions.len()
        )));
    }

    let public_key = parse_public_key_ecc(&config)?;

    let mut choices: Vec<ReplicationChoice> = vec![];
    let mut proofs: Vec<CyphertextProof> = vec![];
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

        let (choice, proof) =
            encrypt_plaintext_answer_ecc_wrapper(&public_key, plaintext)?;
        choices.push(choice);
        proofs.push(proof);
    }

    let mut auditable_ballot = AuditableBallot {
        issue_date: get_current_date(),
        choices: choices,
        proofs: proofs,
        ballot_hash: String::from(""),
        config: config.clone(),
    };

    auditable_ballot.ballot_hash = hash_to(&auditable_ballot)?;

    Ok(auditable_ballot)
}

#[cfg(test)]
mod tests {
    use crate::encrypt::*;
    use crate::fixtures::encrypt::get_encrypt_decoded_test_fixture;
    use crate::util::*;

    #[test]
    fn test_parse_ballot() {
        let ballot = read_ballot_fixture();
        let sha256_ballot = hash_to(&ballot).unwrap();
        assert_eq!(&sha256_ballot, &ballot.ballot_hash);
        let recreated_cyphertext =
            recreate_encrypt_cyphertext(&ballot).unwrap();
        assert_eq!(recreated_cyphertext, ballot.choices);
    }

    #[test]
    fn test_recreate_encrypt_answer() {
        let ballot = read_ballot_fixture();
        let pk = Pk {
            y: "invalid_key".to_string(),
            p: "p".to_string(),
            q: "q".to_string(),
            g: "g".to_string(),
        };
        let call_result = recreate_encrypt_answer(&pk, &ballot.choices[0]);
        assert_eq!(
            call_result,
            Err(BallotError::ParseBigUint(
                pk.y,
                String::from("Error parsing public key"),
            ))
        );
    }

    #[test]
    fn test_encrypt_decoded_question() {
        let (decoded_questions, election) = get_encrypt_decoded_test_fixture();
        encrypt_decoded_question(&decoded_questions, &election).unwrap();
    }
}
