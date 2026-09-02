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
use std::collections::{HashMap, HashSet};
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

/// Re-encrypts every contest of a single-contest auditable ballot from the
/// plaintext and randomness it carries, using the public key of its ballot
/// style. The result is what the ciphertexts must be if the ballot is honest;
/// [`verify_auditable_ballot_ciphertexts`] compares it with what the ballot
/// actually contains.
pub fn recreate_encrypt_cyphertext<C: Ctx>(
    ctx: &C,
    ballot: &AuditableBallot,
) -> Result<Vec<ReplicationChoice<C>>, BallotError> {
    let public_key = parse_public_key::<C>(&ballot.config)?;
    let contests: Vec<AuditableBallotContest<C>> =
        ballot.deserialize_contests::<C>()?;
    let contest_ids: Vec<&str> = contests
        .iter()
        .map(|contest| contest.contest_id.as_str())
        .collect();
    check_contest_ids_match_style(&contest_ids, &ballot.config)?;

    contests
        .iter()
        .map(|contest| {
            recreate_encrypt_candidate(ctx, &public_key, &contest.choice)
        })
        .collect()
}

/// Multi-contest counterpart of [`recreate_encrypt_cyphertext`]: a
/// multi-ballot holds a single ciphertext covering every votable contest.
pub fn recreate_encrypt_multi_ballot_cyphertext<C: Ctx>(
    ctx: &C,
    ballot: &AuditableMultiBallot,
) -> Result<ReplicationChoice<C>, BallotError> {
    let public_key = parse_public_key::<C>(&ballot.config)?;
    let contests: AuditableMultiBallotContests<C> =
        ballot.deserialize_contests::<C>()?;
    let contest_ids: Vec<&str> = contests
        .contest_ids
        .iter()
        .map(|contest_id| contest_id.as_str())
        .collect();
    check_contest_ids_match_style(&contest_ids, &ballot.config)?;

    recreate_encrypt_candidate(ctx, &public_key, &contests.choice)
}

/// Checks that the plaintext and randomness of every contest in a
/// single-contest auditable ballot reproduce exactly the ciphertext the ballot
/// carries for that contest. Any structural problem (unparseable key or
/// contests, contests that do not match the ballot style) is an error too, so
/// a verifier fails closed.
pub fn verify_auditable_ballot_ciphertexts<C: Ctx>(
    ctx: &C,
    ballot: &AuditableBallot,
) -> Result<(), BallotError> {
    let contests: Vec<AuditableBallotContest<C>> =
        ballot.deserialize_contests::<C>()?;
    let recreated = recreate_encrypt_cyphertext(ctx, ballot)?;

    for (contest, recreated_choice) in contests.iter().zip(recreated.iter()) {
        if !ciphertexts_match(
            &contest.choice.ciphertext,
            &recreated_choice.ciphertext,
        ) {
            return Err(BallotError::CryptographicCheck(format!(
                "The ciphertext of contest {} is not the encryption of the \
                 plaintext and randomness included in the ballot",
                contest.contest_id
            )));
        }
    }
    Ok(())
}

/// Multi-contest counterpart of [`verify_auditable_ballot_ciphertexts`].
pub fn verify_auditable_multi_ballot_ciphertext<C: Ctx>(
    ctx: &C,
    ballot: &AuditableMultiBallot,
) -> Result<(), BallotError> {
    let contests: AuditableMultiBallotContests<C> =
        ballot.deserialize_contests::<C>()?;
    let recreated = recreate_encrypt_multi_ballot_cyphertext(ctx, ballot)?;

    if !ciphertexts_match(&contests.choice.ciphertext, &recreated.ciphertext) {
        return Err(BallotError::CryptographicCheck(String::from(
            "The ballot ciphertext is not the encryption of the plaintext \
             and randomness included in the ballot",
        )));
    }
    Ok(())
}

fn ciphertexts_match<C: Ctx>(
    expected: &Ciphertext<C>,
    actual: &Ciphertext<C>,
) -> bool {
    expected.mhr == actual.mhr && expected.gr == actual.gr
}

/// The contests named by a ballot must be exactly the votable contests of its
/// ballot style, each named once. Acclaimed contests are never encoded, so
/// they must not appear.
fn check_contest_ids_match_style(
    contest_ids: &[&str],
    config: &BallotStyle,
) -> Result<(), BallotError> {
    let votable_contest_ids: HashSet<&str> = config
        .votable_contests()
        .map(|contest| contest.id.as_str())
        .collect();
    let named_contest_ids: HashSet<&str> =
        contest_ids.iter().copied().collect();

    if named_contest_ids.len() != contest_ids.len() {
        return Err(BallotError::ConsistencyCheck(String::from(
            "Ballot names the same contest more than once",
        )));
    }
    if named_contest_ids != votable_contest_ids {
        return Err(BallotError::ConsistencyCheck(format!(
            "Ballot was cast over {} contests, but this ballot style encodes {}",
            named_contest_ids.len(),
            votable_contest_ids.len()
        )));
    }
    Ok(())
}

fn recreate_encrypt_candidate<C: Ctx>(
    ctx: &C,
    public_key_element: &C::E,
    choice: &ReplicationChoice<C>,
) -> Result<ReplicationChoice<C>, BallotError> {
    // construct a public key from a provided element
    let public_key = PublicKey::from_element(public_key_element, ctx);

    let encoded = ctx.encode(&choice.plaintext).map_err(|err| {
        BallotError::Serialization(format!(
            "Error encoding the ballot plaintext: {}",
            err
        ))
    })?;

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

/// The selections that are actually encoded into the ballot, one per votable
/// contest, in ballot style order.
///
/// This is also the consistency check that the plain length comparison used
/// to be: every votable contest must have exactly one selection. Selections
/// for acclaimed contests are ignored rather than rejected, because both
/// shapes legitimately reach here. The voting portal holds one selection per
/// displayed contest, but re-encrypting a ballot feeds back the decoded one,
/// which only covers the contests that were actually encoded.
fn votable_decoded_contests<'a>(
    decoded_contests: &'a [DecodedVoteContest],
    config: &BallotStyle,
) -> Result<Vec<&'a DecodedVoteContest>, BallotError> {
    let selections_by_contest_id: HashMap<&str, &DecodedVoteContest> =
        decoded_contests
            .iter()
            .map(|decoded_contest| {
                (decoded_contest.contest_id.as_str(), decoded_contest)
            })
            .collect();

    // Collecting into a map would otherwise swallow a repeated contest, and a
    // selection for a contest outside the ballot style would go unnoticed now
    // that the lengths are no longer compared.
    if selections_by_contest_id.len() != decoded_contests.len() {
        return Err(BallotError::ConsistencyCheck(
            "Repeated decoded contest".to_string(),
        ));
    }
    let contest_ids: HashSet<&str> =
        config.contests.iter().map(|c| c.id.as_str()).collect();
    if let Some(unknown) = decoded_contests
        .iter()
        .find(|decoded| !contest_ids.contains(decoded.contest_id.as_str()))
    {
        return Err(BallotError::ConsistencyCheck(format!(
            "Can't find contest with id {} on ballot style",
            unknown.contest_id
        )));
    }

    config
        .votable_contests()
        .map(|contest| {
            selections_by_contest_id
                .get(contest.id.as_str())
                .copied()
                .ok_or_else(|| {
                    BallotError::ConsistencyCheck(format!(
                        "Missing decoded contest for contest {}",
                        contest.id
                    ))
                })
        })
        .collect()
}

/// Derives and validates the ballot-level `is_explicit_invalid`/
/// `is_blank_ballot` flags from a ballot's decoded contests: every contest
/// must agree with the flag, the flag must be enabled by the ballot style,
/// and blank content must actually be blank. Shared by
/// [`encode_to_plaintext_decoded_multi_contest`] and
/// [`encrypt_decoded_multi_contest`], which otherwise only differ in how
/// they consume the resulting [`BallotChoices`].
///
/// Only the encoded contests are considered, so "every contest is declined"
/// means every contest whose selection reaches the ballot.
fn validate_ballot_level_flags(
    decoded_contests: &[&DecodedVoteContest],
    config: &BallotStyle,
) -> Result<(bool, bool), BallotError> {
    // is_explicit_invalid is true if any of the contests is a decline to vote contest
    let is_explicit_invalid = decoded_contests
        .iter()
        .any(|choice| choice.is_decline_to_vote);

    if is_explicit_invalid && !config.decline_to_vote_enabled() {
        return Err(BallotError::ConsistencyCheck(
            "Decline to vote is not enabled for this election".to_string(),
        ));
    }

    if is_explicit_invalid {
        let number_of_contests_decline_to_vote = decoded_contests
            .iter()
            .filter(|choice| choice.is_decline_to_vote)
            .count();

        if number_of_contests_decline_to_vote != decoded_contests.len() {
            return Err(BallotError::ConsistencyCheck(format!(
                "Invalid number of contests with decline to vote {} != {}",
                number_of_contests_decline_to_vote,
                decoded_contests.len()
            )));
        }
    }

    // is_blank_ballot is true if any of the contests is marked as a blank
    // ballot contest; every contest must agree, and the flag must agree
    // with the actual content.
    let is_blank_ballot =
        decoded_contests.iter().any(|choice| choice.is_blank_ballot);

    if is_blank_ballot && !config.blank_ballots_enabled() {
        return Err(BallotError::ConsistencyCheck(
            "Blank ballots is not enabled for this election".to_string(),
        ));
    }

    if is_blank_ballot && is_explicit_invalid {
        return Err(BallotError::ConsistencyCheck(
            "A ballot cannot be both declined and blank".to_string(),
        ));
    }

    if is_blank_ballot {
        let number_of_contests_blank_ballot = decoded_contests
            .iter()
            .filter(|choice| choice.is_blank_ballot)
            .count();

        if number_of_contests_blank_ballot != decoded_contests.len() {
            return Err(BallotError::ConsistencyCheck(format!(
                "Invalid number of contests marked blank ballot {} != {}",
                number_of_contests_blank_ballot,
                decoded_contests.len()
            )));
        }

        // Unlike decline, "blank" has content-derived meaning: reject a
        // flag that disagrees with what was actually selected.
        if decoded_contests.iter().any(|choice| !choice.is_blank()) {
            return Err(BallotError::ConsistencyCheck(
                "Blank ballot flag disagrees with contest content".to_string(),
            ));
        }
    }

    Ok((is_explicit_invalid, is_blank_ballot))
}

pub fn encode_to_plaintext_decoded_multi_contest(
    decoded_contests: &Vec<DecodedVoteContest>,
    config: &BallotStyle,
) -> Result<([u8; 30], BallotChoices), BallotError> {
    let votable_contests = votable_decoded_contests(decoded_contests, config)?;

    let contest_choices: Vec<_> = votable_contests
        .iter()
        .map(|decoded_contest| {
            ContestChoices::from_decoded_vote_contest(decoded_contest)
        })
        .collect();

    let (is_explicit_invalid, is_blank_ballot) =
        validate_ballot_level_flags(&votable_contests, config)?;

    let counting_algorithm = config.get_counting_algorithm()?;
    let ballot_choices = BallotChoices::new(
        is_explicit_invalid,
        is_blank_ballot,
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
    let votable_contests = votable_decoded_contests(decoded_contests, config)?;

    let contest_choices = votable_contests
        .iter()
        .map(|decoded_contest| {
            ContestChoices::from_decoded_vote_contest(decoded_contest)
        })
        .collect();

    let (is_explicit_invalid, is_blank_ballot) =
        validate_ballot_level_flags(&votable_contests, config)?;

    let counting_algorithm = config.get_counting_algorithm()?;
    let ballot = BallotChoices::new(
        is_explicit_invalid,
        is_blank_ballot,
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
    let public_key: C::E = parse_public_key::<C>(&config)?;

    let mut contests: Vec<AuditableBallotContest<C>> = vec![];

    // An acclaimed contest has no ciphertext of its own, so it contributes
    // nothing to the ballot.
    for decoded_contest in votable_decoded_contests(decoded_contests, config)? {
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
    // `ballot_choices` must cover exactly the contests that get encoded, one
    // entry each. Comparing ids rather than counts matters once a contest can
    // be acclaimed: a selection for an acclaimed contest would keep the count
    // right while shifting which contests the bases cover.
    let votable_contest_ids: HashSet<&str> =
        config.votable_contests().map(|c| c.id.as_str()).collect();
    let choice_contest_ids: HashSet<&str> = ballot_choices
        .choices
        .iter()
        .map(|choices| choices.contest_id.as_str())
        .collect();
    if choice_contest_ids.len() != ballot_choices.choices.len() {
        return Err(BallotError::ConsistencyCheck(
            "Repeated contest in the ballot choices".to_string(),
        ));
    }
    if choice_contest_ids != votable_contest_ids {
        return Err(BallotError::ConsistencyCheck(format!(
            "Ballot choices cover {} contests, but this ballot style encodes \
             {}",
            choice_contest_ids.len(),
            votable_contest_ids.len()
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

    mod ciphertext_reconstruction {
        use crate::ballot::BallotStyle;
        use crate::ballot::{AuditableBallot, PublicKeyConfig};
        use crate::encrypt::{
            encrypt_decoded_contest, encrypt_decoded_multi_contest,
            encrypt_plaintext_candidate, recreate_encrypt_cyphertext,
            recreate_encrypt_multi_ballot_cyphertext,
            verify_auditable_ballot_ciphertexts,
            verify_auditable_multi_ballot_ciphertext, DEFAULT_PLAINTEXT_LABEL,
        };
        use crate::error::BallotError;
        use crate::fixtures::ballot_codec::{
            get_test_contest, get_test_decoded_vote_contest,
            get_writein_ballot_style,
        };
        use crate::multi_ballot::AuditableMultiBallot;
        use strand::backend::ristretto::RistrettoCtx;
        use strand::context::Ctx;
        use strand::rng::StrandRng;

        /// A plurality contest with three candidates under a ballot style
        /// that carries a public key, so both codecs can encrypt it.
        fn ballot_style() -> BallotStyle {
            BallotStyle {
                contests: vec![get_test_contest()],
                ..get_writein_ballot_style()
            }
        }

        fn single_ballot() -> AuditableBallot {
            encrypt_decoded_contest::<RistrettoCtx>(
                &RistrettoCtx,
                &vec![get_test_decoded_vote_contest()],
                &ballot_style(),
            )
            .unwrap()
        }

        fn multi_ballot() -> AuditableMultiBallot {
            encrypt_decoded_multi_contest::<RistrettoCtx>(
                &RistrettoCtx,
                &vec![get_test_decoded_vote_contest()],
                &ballot_style(),
            )
            .unwrap()
        }

        fn assert_cryptographic_check(result: Result<(), BallotError>) {
            match result {
                Err(BallotError::CryptographicCheck(_)) => {}
                other => {
                    panic!("expected a ciphertext mismatch, got {other:?}")
                }
            }
        }

        fn assert_consistency_check(result: Result<(), BallotError>) {
            match result {
                Err(BallotError::ConsistencyCheck(_)) => {}
                other => panic!("expected a consistency error, got {other:?}"),
            }
        }

        #[test]
        fn valid_single_ballot_reproduces_its_ciphertexts() {
            let ballot = single_ballot();
            let contests =
                ballot.deserialize_contests::<RistrettoCtx>().unwrap();

            let recreated =
                recreate_encrypt_cyphertext(&RistrettoCtx, &ballot).unwrap();

            assert_eq!(recreated.len(), contests.len());
            for (contest, choice) in contests.iter().zip(recreated.iter()) {
                assert_eq!(contest.choice, *choice);
            }
            assert!(verify_auditable_ballot_ciphertexts(
                &RistrettoCtx,
                &ballot
            )
            .is_ok());
        }

        #[test]
        fn single_ballot_with_modified_plaintext_is_rejected() {
            let ballot = single_ballot();
            let mut contests =
                ballot.deserialize_contests::<RistrettoCtx>().unwrap();
            contests[0].choice.plaintext[0] ^= 0x01;
            let tampered = AuditableBallot {
                contests: AuditableBallot::serialize_contests(&contests)
                    .unwrap(),
                ..ballot
            };

            assert_cryptographic_check(verify_auditable_ballot_ciphertexts(
                &RistrettoCtx,
                &tampered,
            ));
        }

        #[test]
        fn single_ballot_with_modified_randomness_is_rejected() {
            let ballot = single_ballot();
            let mut contests =
                ballot.deserialize_contests::<RistrettoCtx>().unwrap();
            contests[0].choice.randomness =
                RistrettoCtx.rnd_exp(&mut StrandRng);
            let tampered = AuditableBallot {
                contests: AuditableBallot::serialize_contests(&contests)
                    .unwrap(),
                ..ballot
            };

            assert_cryptographic_check(verify_auditable_ballot_ciphertexts(
                &RistrettoCtx,
                &tampered,
            ));
        }

        #[test]
        fn single_ballot_with_modified_ciphertext_is_rejected() {
            let ballot = single_ballot();
            let mut contests =
                ballot.deserialize_contests::<RistrettoCtx>().unwrap();
            let public_key = crate::encrypt::parse_public_key::<RistrettoCtx>(
                &ballot.config,
            )
            .unwrap();
            // A fresh encryption of the same plaintext uses new randomness,
            // so its ciphertext cannot match the recorded randomness.
            let (other_choice, _) = encrypt_plaintext_candidate(
                &RistrettoCtx,
                public_key,
                contests[0].choice.plaintext,
                &DEFAULT_PLAINTEXT_LABEL,
            )
            .unwrap();
            contests[0].choice.ciphertext = other_choice.ciphertext;
            let tampered = AuditableBallot {
                contests: AuditableBallot::serialize_contests(&contests)
                    .unwrap(),
                ..ballot
            };

            assert_cryptographic_check(verify_auditable_ballot_ciphertexts(
                &RistrettoCtx,
                &tampered,
            ));
        }

        #[test]
        fn single_ballot_with_malformed_public_key_is_an_error() {
            let mut ballot = single_ballot();
            ballot.config.public_key = Some(PublicKeyConfig {
                public_key: "not a public key".to_string(),
                is_demo: true,
            });

            assert!(
                recreate_encrypt_cyphertext(&RistrettoCtx, &ballot).is_err()
            );
            assert!(verify_auditable_ballot_ciphertexts(
                &RistrettoCtx,
                &ballot
            )
            .is_err());
        }

        #[test]
        fn single_ballot_without_public_key_is_an_error() {
            let mut ballot = single_ballot();
            ballot.config.public_key = None;

            assert_consistency_check(verify_auditable_ballot_ciphertexts(
                &RistrettoCtx,
                &ballot,
            ));
        }

        #[test]
        fn single_ballot_with_inconsistent_contest_count_is_an_error() {
            let ballot = single_ballot();
            let mut contests =
                ballot.deserialize_contests::<RistrettoCtx>().unwrap();
            contests.push(contests[0].clone());
            let duplicated = AuditableBallot {
                contests: AuditableBallot::serialize_contests(&contests)
                    .unwrap(),
                ..ballot.clone()
            };
            let empty = AuditableBallot {
                contests: vec![],
                ..ballot
            };

            assert_consistency_check(verify_auditable_ballot_ciphertexts(
                &RistrettoCtx,
                &duplicated,
            ));
            assert_consistency_check(verify_auditable_ballot_ciphertexts(
                &RistrettoCtx,
                &empty,
            ));
        }

        #[test]
        fn valid_multi_ballot_reproduces_its_ciphertext() {
            let ballot = multi_ballot();
            let contests =
                ballot.deserialize_contests::<RistrettoCtx>().unwrap();

            let recreated = recreate_encrypt_multi_ballot_cyphertext(
                &RistrettoCtx,
                &ballot,
            )
            .unwrap();

            assert_eq!(contests.choice, recreated);
            assert!(verify_auditable_multi_ballot_ciphertext(
                &RistrettoCtx,
                &ballot
            )
            .is_ok());
        }

        #[test]
        fn multi_ballot_with_modified_plaintext_is_rejected() {
            let ballot = multi_ballot();
            let mut contests =
                ballot.deserialize_contests::<RistrettoCtx>().unwrap();
            contests.choice.plaintext[0] ^= 0x01;
            let tampered = AuditableMultiBallot {
                contests: AuditableMultiBallot::serialize_contests(&contests)
                    .unwrap(),
                ..ballot
            };

            assert_cryptographic_check(
                verify_auditable_multi_ballot_ciphertext(
                    &RistrettoCtx,
                    &tampered,
                ),
            );
        }

        #[test]
        fn multi_ballot_with_modified_randomness_is_rejected() {
            let ballot = multi_ballot();
            let mut contests =
                ballot.deserialize_contests::<RistrettoCtx>().unwrap();
            contests.choice.randomness = RistrettoCtx.rnd_exp(&mut StrandRng);
            let tampered = AuditableMultiBallot {
                contests: AuditableMultiBallot::serialize_contests(&contests)
                    .unwrap(),
                ..ballot
            };

            assert_cryptographic_check(
                verify_auditable_multi_ballot_ciphertext(
                    &RistrettoCtx,
                    &tampered,
                ),
            );
        }

        #[test]
        fn multi_ballot_with_modified_ciphertext_is_rejected() {
            let ballot = multi_ballot();
            let mut contests =
                ballot.deserialize_contests::<RistrettoCtx>().unwrap();
            let public_key = crate::encrypt::parse_public_key::<RistrettoCtx>(
                &ballot.config,
            )
            .unwrap();
            let (other_choice, _) = encrypt_plaintext_candidate(
                &RistrettoCtx,
                public_key,
                contests.choice.plaintext,
                &DEFAULT_PLAINTEXT_LABEL,
            )
            .unwrap();
            contests.choice.ciphertext = other_choice.ciphertext;
            let tampered = AuditableMultiBallot {
                contests: AuditableMultiBallot::serialize_contests(&contests)
                    .unwrap(),
                ..ballot
            };

            assert_cryptographic_check(
                verify_auditable_multi_ballot_ciphertext(
                    &RistrettoCtx,
                    &tampered,
                ),
            );
        }

        #[test]
        fn multi_ballot_with_malformed_public_key_is_an_error() {
            let mut ballot = multi_ballot();
            ballot.config.public_key = Some(PublicKeyConfig {
                public_key: "not a public key".to_string(),
                is_demo: true,
            });

            assert!(recreate_encrypt_multi_ballot_cyphertext(
                &RistrettoCtx,
                &ballot
            )
            .is_err());
        }

        #[test]
        fn multi_ballot_with_inconsistent_contest_ids_is_an_error() {
            let ballot = multi_ballot();
            let mut contests =
                ballot.deserialize_contests::<RistrettoCtx>().unwrap();
            contests.contest_ids.push(contests.contest_ids[0].clone());
            let tampered = AuditableMultiBallot {
                contests: AuditableMultiBallot::serialize_contests(&contests)
                    .unwrap(),
                ..ballot
            };

            assert_consistency_check(verify_auditable_multi_ballot_ciphertext(
                &RistrettoCtx,
                &tampered,
            ));
        }
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
