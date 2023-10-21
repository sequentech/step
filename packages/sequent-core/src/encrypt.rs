// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::elgamal::*;
use strand::hash;
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

pub fn encrypt_plaintext_candidate<C: Ctx<P = [u8; 30]>>(
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
    // sanity checks for number of candidates/choices
    if ballot.contests.len() != ballot.config.contests.len() {
        return Err(BallotError::ConsistencyCheck(String::from(
            "Number of election contests should match number of candidates in the ballot",
        )));
    }

    ballot
        .contests
        .clone()
        .into_iter()
        .map(|contests| {
            recreate_encrypt_candidate(ctx, &public_key, &contests.choice)
        })
        .collect::<Vec<Result<ReplicationChoice<C>, BallotError>>>()
        .into_iter()
        .collect()
}

fn recreate_encrypt_candidate<C: Ctx>(
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

pub fn encrypt_decoded_contest<C: Ctx<P = [u8; 30]>>(
    ctx: &C,
    decoded_contests: &Vec<DecodedVoteContest>,
    config: &BallotStyle,
) -> Result<AuditableBallot<C>, BallotError> {
    if config.contests.len() != decoded_contests.len() {
        return Err(BallotError::ConsistencyCheck(format!(
            "Invalid number of decoded contests {} != {}",
            config.contests.len(),
            decoded_contests.len()
        )));
    }

    let public_key: C::E = parse_public_key::<C>(&config)?;

    let mut contests: Vec<AuditableBallotContest<C>> = vec![];

    for decoded_contest in decoded_contests {
        let contest = config
            .contests
            .iter()
            .find(|contest| contest.id == decoded_contest.contest_id)
            .ok_or_else(|| {
                BallotError::Serialization(format!(
                    "Can't find contest with id {} on ballot style",
                    decoded_contest.contest_id
                ))
            })?;
        let plaintext = contest
            .encode_plaintext_contest(&decoded_contest)
            .map_err(|err| {
                BallotError::Serialization(format!(
                    "Error encrypting plaintext: {}",
                    err
                ))
            })?;
        let (choice, proof) =
            encrypt_plaintext_candidate(ctx, public_key.clone(), plaintext)?;
        contests.push(AuditableBallotContest::<C> {
            contest_id: contest.id.clone(),
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
    let hash_bytes = hash::hash(bytes.as_slice())
        .map_err(|error| BallotError::Serialization(error.to_string()))?;
    let hash_256bits_slice = &hash_bytes[0..32];
    Ok(hex::encode(hash_256bits_slice))
}

#[cfg(test)]
mod tests {
    use crate::ballot_codec::bigint;
    use crate::ballot_codec::plaintext_contest::PlaintextCodec;
    use crate::ballot_codec::vec;
    use crate::encrypt;
    use crate::fixtures::ballot_codec::*;
    use crate::util::normalize_vote_contest;

    use strand::backend::ristretto::RistrettoCtx;
    use strand::context::Ctx;
    use strand::rng::StrandRng;

    #[test]
    fn test_encrypt_plaintext_candidate() {
        let mut csprng = StrandRng;
        let ctx = RistrettoCtx;

        let (pk_string, pk_element) = encrypt::default_public_key_ristretto();

        let plaintext = ctx.rnd_plaintext(&mut csprng);

        encrypt::encrypt_plaintext_candidate(&ctx, pk_element, plaintext)
            .unwrap();
        assert_eq!(
            pk_string.as_str(),
            encrypt::DEFAULT_PUBLIC_KEY_RISTRETTO_STR
        );
    }

    #[test]
    fn test_encrypt_writein_candidate() {
        let ctx = RistrettoCtx;
        let ballot_style = get_writein_ballot_style();
        let contest = ballot_style.contests[0].clone();
        let decoded_contest = get_writein_plaintext();
        let plaintext_bytes_vec = contest
            .encode_plaintext_contest_to_bytes(&decoded_contest)
            .unwrap(); // compare
        let auditable_ballot =
            encrypt::encrypt_decoded_contest::<RistrettoCtx>(
                &ctx,
                &vec![decoded_contest.clone()],
                &ballot_style,
            )
            .unwrap();
        let plaintext = auditable_ballot.contests[0].choice.plaintext.clone();
        let plaintext_vec = vec::decode_array_to_vec(&plaintext); // compare
        assert_eq!(plaintext_vec, plaintext_bytes_vec);
        assert_eq!(plaintext_vec, vec![198, 20, 150, 48]);
        let decoded_plaintext =
            contest.decode_plaintext_contest(&plaintext).unwrap();
        assert_eq!(
            normalize_vote_contest(
                &decoded_plaintext,
                contest.get_counting_algorithm().as_str(),
                false
            ),
            normalize_vote_contest(
                &decoded_contest,
                contest.get_counting_algorithm().as_str(),
                false
            )
        );
    }

    #[test]
    fn test_encrypt_writein_candidate2() {
        use crate::ballot_codec::bigint::BigUIntCodec;
        use crate::ballot_codec::raw_ballot::RawBallotCodec;

        let ctx = RistrettoCtx;
        let ballot_style = get_writein_ballot_style();
        let contest = ballot_style.contests[0].clone();
        let bigint_vec2: Vec<u8> = vec![198, 20, 150, 48];
        let bigint2 =
            bigint::decode_bigint_from_bytes(bigint_vec2.as_slice()).unwrap();
        assert_eq!(bigint2.to_str_radix(10), "815142086");

        let decoded_contest = get_writein_plaintext();

        let raw_ballot =
            contest.encode_to_raw_ballot(&decoded_contest).unwrap();
        let bigint = contest
            .encode_plaintext_contest_bigint(&decoded_contest)
            .unwrap();
        let raw_ballot2 = contest.bigint_to_raw_ballot(&bigint).unwrap();
        //assert_eq!(raw_ballot, raw_ballot2);

        assert_eq!(bigint2.to_str_radix(10), bigint.to_str_radix(10));
        let decoded_contest2 =
            contest.decode_plaintext_contest_bigint(&bigint).unwrap();
        assert_eq!(
            normalize_vote_contest(
                &decoded_contest,
                contest.get_counting_algorithm().as_str(),
                false
            ),
            normalize_vote_contest(
                &decoded_contest2,
                contest.get_counting_algorithm().as_str(),
                false
            )
        );
    }
}
