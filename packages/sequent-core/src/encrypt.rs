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
    HashableMultiBallotContests, RawHashableMultiBallot,
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

pub fn recreate_encrypt_cyphertext<C: Ctx>(
    ctx: &C,
    ballot: &AuditableBallot,
) -> Result<Vec<ReplicationChoice<C>>, BallotError> {
    let public_key = parse_public_key::<C>(&ballot.config)?;
    // check ballot version
    // sanity checks for number of candidates/choices
    if ballot.contests.len() != ballot.config.votable_contests().count() {
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

fn ballot_to_strand_error(error: BallotError) -> StrandError {
    StrandError::Generic(format!("{:?}", error))
}

fn unexpected_version_error(version: u32) -> StrandError {
    StrandError::Generic(format!("Unexpected ballot version {version}"))
}

pub fn hash_ballot_sha512(
    hashable_ballot: &HashableBallot,
) -> Result<Hash, StrandError> {
    let bytes = match hashable_ballot.version {
        LEGACY_TYPES_VERSION => {
            RawHashableBallot::<RistrettoCtx>::try_from(hashable_ballot)
                .map_err(ballot_to_strand_error)?
                .strand_serialize()?
        }
        TYPES_VERSION => StyleBoundHashableBallot::<
            Vec<HashableBallotContest<RistrettoCtx>>,
        >::try_from(hashable_ballot)
        .map_err(ballot_to_strand_error)?
        .strand_serialize()?,
        version => return Err(unexpected_version_error(version)),
    };
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
    let bytes = match hashable_ballot.version {
        LEGACY_TYPES_VERSION => {
            RawHashableMultiBallot::<RistrettoCtx>::try_from(hashable_ballot)
                .map_err(ballot_to_strand_error)?
                .strand_serialize()?
        }
        TYPES_VERSION => StyleBoundHashableBallot::<
            HashableMultiBallotContests<RistrettoCtx>,
        >::try_from(hashable_ballot)
        .map_err(ballot_to_strand_error)?
        .strand_serialize()?,
        version => return Err(unexpected_version_error(version)),
    };
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
        let plaintext_vec = vec::decode_array_to_vec(&plaintext).unwrap();
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

    mod ballot_tracker {
        use crate::ballot::BallotStyle;
        use crate::ballot::{
            AuditableBallot, HashableBallot, HashableBallotContest,
            PublicKeyConfig, RawHashableBallot, SignedHashableBallot,
            LEGACY_TYPES_VERSION, TYPES_VERSION,
        };
        use crate::encrypt::{
            encrypt_decoded_contest, encrypt_decoded_multi_contest,
        };
        use crate::encrypt::{
            hash_ballot, hash_ballot_style, hash_multi_ballot, shorten_hash,
        };
        use crate::fixtures::ballot_codec::{
            get_test_contest, get_test_decoded_vote_contest,
            get_writein_ballot_style,
        };
        use crate::multi_ballot::{
            AuditableMultiBallot, HashableMultiBallot,
            HashableMultiBallotContests, RawHashableMultiBallot,
        };
        use strand::backend::ristretto::RistrettoCtx;
        use strand::hash::hash_to_array;
        use strand::serialization::StrandSerialize;

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

        fn single_tracker(ballot: &AuditableBallot) -> String {
            let signed = SignedHashableBallot::try_from(ballot).unwrap();
            hash_ballot(&HashableBallot::try_from(&signed).unwrap()).unwrap()
        }

        fn multi_tracker(ballot: &AuditableMultiBallot) -> String {
            hash_multi_ballot(&HashableMultiBallot::try_from(ballot).unwrap())
                .unwrap()
        }

        fn other_public_key() -> Option<PublicKeyConfig> {
            Some(PublicKeyConfig {
                public_key: "4M8lEfQGWg8DtBIxXJrIYNKvsAx2RKEj8+Y1pncbH1U"
                    .to_string(),
                is_demo: false,
            })
        }

        fn short_hash_hex(bytes: &[u8]) -> String {
            hex::encode(shorten_hash(&hash_to_array(bytes).unwrap()))
        }

        #[test]
        fn single_ballot_tracker_matches_its_own_ballot_style() {
            let ballot = single_ballot();

            assert_eq!(single_tracker(&ballot), ballot.ballot_hash);
        }

        #[test]
        fn single_ballot_tracker_covers_the_public_key() {
            let ballot = single_ballot();
            let mut other_key = ballot.clone();
            other_key.config.public_key = other_public_key();

            assert_ne!(single_tracker(&other_key), ballot.ballot_hash);
        }

        #[test]
        fn single_ballot_tracker_covers_the_candidate_order() {
            let ballot = single_ballot();
            let mut reordered = ballot.clone();
            reordered.config.contests[0].candidates.reverse();

            assert_ne!(single_tracker(&reordered), ballot.ballot_hash);
        }

        #[test]
        fn single_ballot_tracker_covers_the_election() {
            let ballot = single_ballot();
            let mut other_election = ballot.clone();
            other_election.config.election_id = "other-election".to_string();

            assert_ne!(single_tracker(&other_election), ballot.ballot_hash);
        }

        #[test]
        fn multi_ballot_tracker_covers_the_public_key() {
            let ballot = multi_ballot();
            let mut other_key = ballot.clone();
            other_key.config.public_key = other_public_key();

            assert_eq!(multi_tracker(&ballot), ballot.ballot_hash);
            assert_ne!(multi_tracker(&other_key), ballot.ballot_hash);
        }

        #[test]
        fn multi_ballot_tracker_covers_the_candidate_order() {
            let ballot = multi_ballot();
            let mut reordered = ballot.clone();
            reordered.config.contests[0].candidates.reverse();

            assert_ne!(multi_tracker(&reordered), ballot.ballot_hash);
        }

        /// A one-contest single ballot and a multi ballot over the same
        /// contest, ciphertext and proof must not share a tracker.
        #[test]
        fn single_and_multi_ballot_trackers_differ() {
            let ballot = single_ballot();
            let contest = ballot
                .deserialize_contests::<RistrettoCtx>()
                .unwrap()
                .remove(0);
            let config = ballot.config.id.clone();
            let ballot_style_hash = hash_ballot_style(&ballot_style()).unwrap();

            let single = HashableBallot {
                version: TYPES_VERSION,
                issue_date: ballot.issue_date.clone(),
                contests: HashableBallot::serialize_contests(&vec![
                    HashableBallotContest::<RistrettoCtx> {
                        contest_id: contest.contest_id.clone(),
                        ciphertext: contest.choice.ciphertext.clone(),
                        proof: contest.proof.clone(),
                    },
                ])
                .unwrap(),
                config: config.clone(),
                ballot_style_hash: ballot_style_hash.clone(),
            };
            let multi = HashableMultiBallot {
                version: TYPES_VERSION,
                issue_date: ballot.issue_date.clone(),
                contests: HashableMultiBallot::serialize_contests(
                    &HashableMultiBallotContests::<RistrettoCtx> {
                        contest_ids: vec![contest.contest_id.clone()],
                        ciphertext: contest.choice.ciphertext.clone(),
                        proof: contest.proof.clone(),
                    },
                )
                .unwrap(),
                config,
                ballot_style_hash,
            };

            assert_ne!(
                hash_ballot(&single).unwrap(),
                hash_multi_ballot(&multi).unwrap()
            );
        }

        /// Ballots cast before the tracker covered the ballot style keep the
        /// tracker they were cast with.
        #[test]
        fn legacy_single_ballot_keeps_its_tracker() {
            let mut ballot = single_ballot();
            ballot.version = LEGACY_TYPES_VERSION;
            let signed = SignedHashableBallot::try_from(&ballot).unwrap();
            let hashable = HashableBallot::try_from(&signed).unwrap();
            let raw =
                RawHashableBallot::<RistrettoCtx>::try_from(&hashable).unwrap();

            assert_eq!(hashable.version, LEGACY_TYPES_VERSION);
            assert_eq!(
                hash_ballot(&hashable).unwrap(),
                short_hash_hex(&raw.strand_serialize().unwrap())
            );
        }

        #[test]
        fn legacy_multi_ballot_keeps_its_tracker() {
            let mut ballot = multi_ballot();
            ballot.version = LEGACY_TYPES_VERSION;
            let hashable = HashableMultiBallot::try_from(&ballot).unwrap();
            let raw =
                RawHashableMultiBallot::<RistrettoCtx>::try_from(&hashable)
                    .unwrap();

            assert_eq!(hashable.version, LEGACY_TYPES_VERSION);
            assert_eq!(
                hash_multi_ballot(&hashable).unwrap(),
                short_hash_hex(&raw.strand_serialize().unwrap())
            );
        }

        #[test]
        fn unknown_ballot_version_is_rejected() {
            let mut single = single_ballot();
            single.version = 99;
            let mut multi = multi_ballot();
            multi.version = 99;

            assert!(SignedHashableBallot::try_from(&single).is_err());
            assert!(HashableMultiBallot::try_from(&multi).is_err());

            let mut hashable_single = HashableBallot::try_from(
                &SignedHashableBallot::try_from(&single_ballot()).unwrap(),
            )
            .unwrap();
            hashable_single.version = 99;
            let mut hashable_multi =
                HashableMultiBallot::try_from(&multi_ballot()).unwrap();
            hashable_multi.version = 99;

            assert!(hash_ballot(&hashable_single).is_err());
            assert!(hash_multi_ballot(&hashable_multi).is_err());
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
