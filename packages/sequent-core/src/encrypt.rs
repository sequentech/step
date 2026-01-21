// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::elgamal::*;
use strand::hash;
use strand::hash::Hash;
use strand::serialization::StrandDeserialize;
use strand::util::StrandError;
use strand::zkp::{Schnorr, Zkp};

use crate::ballot::*;
use crate::ballot_codec::multi_ballot::BallotChoices;
use crate::ballot_codec::multi_ballot::ContestChoices;
use crate::ballot_codec::PlaintextCodec;
use crate::error::BallotError;
use crate::multi_ballot::{
    AuditableMultiBallot, AuditableMultiBallotContests, HashableMultiBallot,
    RawHashableMultiBallot,
};
use crate::plaintext::map_decoded_ballot_choices_to_decoded_contests;
use crate::plaintext::DecodedVoteContest;
use crate::serialization::base64::Base64Deserialize;
use crate::util::date::get_current_date;
use crate::util::normalize_vote::normalize_election;
use base64::engine::general_purpose;
use base64::Engine;
use strand::serialization::StrandSerialize;

pub const DEFAULT_PUBLIC_KEY_RISTRETTO_STR: &str =
    "ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4";

/// Sha-512 hash are 64 bytes, this short hash uses
/// only the first 32 bytes.
pub const SHORT_SHA512_HASH_LENGTH_BYTES: usize = 32;

pub type ShortHash = [u8; SHORT_SHA512_HASH_LENGTH_BYTES];

// Labels are used to make the proof of knowledge unique.
// This is a constant for now but when we implement voter signatures this will
// be unique to the voter, and it will include the public key of the voter,
// election event id, election id, contest id etc.
pub const DEFAULT_PLAINTEXT_LABEL: [u8; 0] = [];

pub fn default_public_key_ristretto() -> (String, <RistrettoCtx as Ctx>::E) {
    let pk_str: String = DEFAULT_PUBLIC_KEY_RISTRETTO_STR.to_string();
    let pk_bytes = general_purpose::STANDARD_NO_PAD
        .decode(pk_str.clone())
        .unwrap();
    let pk = <RistrettoCtx as Ctx>::E::strand_deserialize(&pk_bytes).unwrap();
    (pk_str, pk)
}

pub fn encrypt_plaintext_candidate<C: Ctx>(
    ctx: &C,
    public_key_element: <C>::E,
    plaintext: <C>::P,
    label: &[u8],
) -> Result<(ReplicationChoice<C>, Schnorr<C>), BallotError> {
    // construct a public key from a provided element
    let pk = PublicKey::from_element(&public_key_element, ctx);

    let encoded = ctx.encode(&plaintext).unwrap();

    // encrypt and prove knowledge of plaintext (enc + pok)
    let (ciphertext, proof, randomness) =
        pk.encrypt_and_pok(&encoded, label).unwrap();
    // verify
    let zkp = Zkp::new(ctx);
    let proof_ok = zkp
        .encryption_popk_verify(&ciphertext.mhr, &ciphertext.gr, &proof, label)
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
    ballot: &AuditableBallot,
) -> Result<Vec<ReplicationChoice<C>>, BallotError> {
    let public_key = parse_public_key::<C>(&ballot.config)?;
    // check ballot version
    // sanity checks for number of candidates/choices
    if ballot.contests.len() != ballot.config.contests.len() {
        return Err(BallotError::ConsistencyCheck(String::from(
            "Number of election contests should match number of candidates in the ballot",
        )));
    }

    let contests: Vec<AuditableBallotContest<C>> =
        ballot.deserialize_contests::<C>()?;

    contests
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

pub fn encode_to_plaintext_decoded_multi_contest(
    decoded_contests: &Vec<DecodedVoteContest>,
    config: &BallotStyle,
) -> Result<([u8; 30], BallotChoices), BallotError> {
    if config.contests.len() != decoded_contests.len() {
        return Err(BallotError::ConsistencyCheck(format!(
            "Invalid number of decoded contests {} != {}",
            config.contests.len(),
            decoded_contests.len()
        )));
    }

    let contest_choices: Vec<_> = decoded_contests
        .iter()
        .map(ContestChoices::from_decoded_vote_contest)
        .collect();

    let is_explicit_invalid = decoded_contests
        .iter()
        .any(|choice| choice.is_explicit_invalid);

    let counting_algorithm = config.get_counting_algorithm()?;
    let ballot_choices = BallotChoices::new(
        is_explicit_invalid,
        contest_choices,
        counting_algorithm,
    );

    let plaintext =
        ballot_choices.encode_to_30_bytes(&config).map_err(|err| {
            BallotError::Serialization(format!(
                "Error encrypting plaintext: {}",
                err
            ))
        })?;

    Ok((plaintext, ballot_choices))
}

pub fn encrypt_decoded_multi_contest<C: Ctx<P = [u8; 30]>>(
    ctx: &C,
    decoded_contests: &Vec<DecodedVoteContest>,
    config: &BallotStyle,
) -> Result<AuditableMultiBallot, BallotError> {
    if config.contests.len() != decoded_contests.len() {
        return Err(BallotError::ConsistencyCheck(format!(
            "Invalid number of decoded contests {} != {}",
            config.contests.len(),
            decoded_contests.len()
        )));
    }

    let contest_choices = decoded_contests
        .iter()
        .map(ContestChoices::from_decoded_vote_contest)
        .collect();

    let is_explicit_invalid = decoded_contests
        .iter()
        .any(|choice| choice.is_explicit_invalid);

    let counting_algorithm = config.get_counting_algorithm()?;
    let ballot = BallotChoices::new(
        is_explicit_invalid,
        contest_choices,
        counting_algorithm,
    );

    encrypt_multi_ballot(ctx, &ballot, config)
}

pub fn encrypt_decoded_contest<C: Ctx<P = [u8; 30]>>(
    ctx: &C,
    decoded_contests: &Vec<DecodedVoteContest>,
    config: &BallotStyle,
) -> Result<AuditableBallot, BallotError> {
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
        let (choice, proof) = encrypt_plaintext_candidate(
            ctx,
            public_key.clone(),
            plaintext,
            &DEFAULT_PLAINTEXT_LABEL,
        )?;
        contests.push(AuditableBallotContest::<C> {
            contest_id: contest.id.clone(),
            choice: choice,
            proof: proof,
        });
    }

    let mut auditable_ballot = AuditableBallot {
        version: TYPES_VERSION,
        issue_date: get_current_date(),
        contests: AuditableBallot::serialize_contests::<C>(&contests)?,
        ballot_hash: String::from(""),
        config: config.clone(),
        voter_signing_pk: None,
        voter_ballot_signature: None,
    };

    let signed_hashable_ballot =
        SignedHashableBallot::try_from(&auditable_ballot)?;
    let hashable_ballot: HashableBallot =
        HashableBallot::try_from(&signed_hashable_ballot)?;
    auditable_ballot.ballot_hash = hash_ballot(&hashable_ballot)?;

    Ok(auditable_ballot)
}

pub fn hash_ballot_style_sha512(
    ballot_style: &BallotStyle,
) -> Result<Hash, StrandError> {
    let bytes = ballot_style.strand_serialize()?;
    hash::hash_to_array(&bytes)
}

pub fn hash_ballot_style(
    ballot_style: &BallotStyle,
) -> Result<String, BallotError> {
    let sha512_hash = hash_ballot_style_sha512(ballot_style)
        .map_err(|error| BallotError::Serialization(error.to_string()))?;
    let short_hash = shorten_hash(&sha512_hash);
    Ok(hex::encode(short_hash))
}

pub fn hash_ballot_sha512(
    hashable_ballot: &HashableBallot,
) -> Result<Hash, StrandError> {
    let raw_hashable_ballot =
        RawHashableBallot::<RistrettoCtx>::try_from(hashable_ballot)
            .map_err(|error| StrandError::Generic(format!("{:?}", error)))?;

    let bytes = raw_hashable_ballot.strand_serialize()?;
    hash::hash_to_array(&bytes)
}

pub fn shorten_hash(hash: &Hash) -> ShortHash {
    let mut shortened: ShortHash = [0u8; SHORT_SHA512_HASH_LENGTH_BYTES];
    shortened.copy_from_slice(&hash[0..32]);
    shortened
}

// hash ballot:
// serialize ballot into string, then hash to sha512, truncate to
// 256 bits and serialize to hexadecimal
pub fn hash_ballot(
    hashable_ballot: &HashableBallot,
) -> Result<String, BallotError> {
    let sha512_hash = hash_ballot_sha512(hashable_ballot)
        .map_err(|error| BallotError::Serialization(error.to_string()))?;
    let short_hash = shorten_hash(&sha512_hash);
    Ok(hex::encode(short_hash))
}

////////////////////////////////////////////////////////////////
/// Multi ballots
////////////////////////////////////////////////////////////////

pub fn encrypt_multi_ballot<C: Ctx<P = [u8; 30]>>(
    ctx: &C,
    ballot_choices: &BallotChoices,
    config: &BallotStyle,
) -> Result<AuditableMultiBallot, BallotError> {
    if config.contests.len() != ballot_choices.choices.len() {
        return Err(BallotError::ConsistencyCheck(format!(
            "Invalid number of decoded contests {} != {}",
            config.contests.len(),
            ballot_choices.choices.len()
        )));
    }

    let public_key: C::E = parse_public_key::<C>(&config)?;
    let plaintext =
        ballot_choices.encode_to_30_bytes(&config).map_err(|err| {
            BallotError::Serialization(format!(
                "Error encrypting plaintext: {}",
                err
            ))
        })?;
    let contest_ids = ballot_choices.get_contest_ids();

    let (choice, proof) = encrypt_plaintext_candidate(
        ctx,
        public_key.clone(),
        plaintext,
        &DEFAULT_PLAINTEXT_LABEL,
    )?;

    let contests = AuditableMultiBallotContests {
        contest_ids,
        choice,
        proof,
    };

    let mut auditable_ballot = AuditableMultiBallot {
        version: TYPES_VERSION,
        issue_date: get_current_date(),
        contests: AuditableMultiBallot::serialize_contests::<C>(&contests)?,
        ballot_hash: String::from(""),
        config: config.clone(),
        voter_signing_pk: None,
        voter_ballot_signature: None,
    };

    let hashable_ballot = HashableMultiBallot::try_from(&auditable_ballot)?;
    auditable_ballot.ballot_hash = hash_multi_ballot(&hashable_ballot)?;

    Ok(auditable_ballot)
}

pub fn hash_multi_ballot(
    hashable_ballot: &HashableMultiBallot,
) -> Result<String, BallotError> {
    let sha512_hash = hash_multi_ballot_sha512(hashable_ballot)
        .map_err(|error| BallotError::Serialization(error.to_string()))?;
    let short_hash = shorten_hash(&sha512_hash);
    Ok(hex::encode(short_hash))
}

pub fn hash_multi_ballot_sha512(
    hashable_ballot: &HashableMultiBallot,
) -> Result<Hash, StrandError> {
    let raw_hashable_ballot =
        RawHashableMultiBallot::<RistrettoCtx>::try_from(hashable_ballot)
            .map_err(|error| StrandError::Generic(format!("{:?}", error)))?;

    let bytes = raw_hashable_ballot.strand_serialize()?;
    hash::hash_to_array(&bytes)
}

#[cfg(test)]
mod tests {
    use crate::ballot_codec::bigint;
    use crate::ballot_codec::plaintext_contest::PlaintextCodec;
    use crate::ballot_codec::vec;
    use crate::encrypt;
    use crate::fixtures::ballot_codec::*;
    use crate::plaintext::DecodedVoteContest;
    use crate::serialization::deserialize_with_path::deserialize_value;
    use crate::util::normalize_vote::normalize_vote_contest;

    use strand::backend::ristretto::RistrettoCtx;
    use strand::context::Ctx;
    use strand::rng::StrandRng;

    #[test]
    fn test_encrypt_plaintext_candidate() {
        let mut csprng = StrandRng;
        let ctx = RistrettoCtx;

        let (pk_string, pk_element) = encrypt::default_public_key_ristretto();

        let plaintext = ctx.rnd_plaintext(&mut csprng);

        encrypt::encrypt_plaintext_candidate(
            &ctx,
            pk_element,
            plaintext,
            &encrypt::DEFAULT_PLAINTEXT_LABEL,
        )
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
        let invalid_candidate_ids = contest.get_invalid_candidate_ids();
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
        let contests = auditable_ballot
            .deserialize_contests::<RistrettoCtx>()
            .unwrap();
        let plaintext = contests[0].choice.plaintext.clone();
        let plaintext_vec = vec::decode_array_to_vec(&plaintext); // compare
        assert_eq!(plaintext_vec, plaintext_bytes_vec);
        assert_eq!(plaintext_vec, vec![198, 20, 150, 48]);
        let decoded_plaintext =
            contest.decode_plaintext_contest(&plaintext).unwrap();
        assert_eq!(
            normalize_vote_contest(
                &decoded_plaintext,
                contest.get_counting_algorithm(),
                false,
                &invalid_candidate_ids,
            ),
            normalize_vote_contest(
                &decoded_contest,
                contest.get_counting_algorithm(),
                false,
                &invalid_candidate_ids,
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
        let invalid_candidate_ids = contest.get_invalid_candidate_ids();
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
                contest.get_counting_algorithm(),
                false,
                &invalid_candidate_ids,
            ),
            normalize_vote_contest(
                &decoded_contest2,
                contest.get_counting_algorithm(),
                false,
                &invalid_candidate_ids,
            )
        );
    }

    /*
    #[test]
    fn test_encrypt_default_voting_portal_fixture() {
        use crate::encrypt::encrypt_decoded_contest;
        use crate::fixtures::encrypt::*;

        let (decoded_contests, election) = default_voting_portal_fixture();
        //get_encrypt_decoded_test_fixture(); //default_voting_portal_fixture();
        let ctx = RistrettoCtx;

        // encrypt ballot
        let auditable_ballot = encrypt_decoded_contest::<RistrettoCtx>(
            &ctx,
            &decoded_contests,
            &election,
        );
        assert_eq!(format!("{:?}", auditable_ballot.unwrap_err()), "".to_string());
        //assert!(auditable_ballot.is_ok());
    }*/
}
