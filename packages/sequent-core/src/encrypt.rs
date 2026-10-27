// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

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
use anyhow::{anyhow, Result as AnyhowResult};
use base64::engine::general_purpose;
use base64::Engine;
use cryptography::context::{Context, RistrettoCtx};
use cryptography::utils::serialization::{Deserializable, Serializable};
use std::collections::{HashMap, HashSet};
use voting_crypto::ballot::{
    encrypt_payload, verify_opening, BallotCiphertext,
};
use voting_crypto::hash::{self, Hash};
use voting_crypto::RecordSerialize;

pub const DEFAULT_PUBLIC_KEY_RISTRETTO_STR: &str =
    "ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4";

/// Displayed application hashes retain the first 32 of the 64 digest bytes.
/// The hash family is selected by the record type and ballot format version.
pub const SHORT_SHA512_HASH_LENGTH_BYTES: usize = 32;

pub type ShortHash = [u8; SHORT_SHA512_HASH_LENGTH_BYTES];

/// The demo public key, represented with wbraid's canonical element decoder.
pub fn default_public_key_ristretto(
) -> AnyhowResult<(String, <RistrettoCtx as Context>::Element)> {
    let bytes = general_purpose::STANDARD_NO_PAD
        .decode(DEFAULT_PUBLIC_KEY_RISTRETTO_STR)?;
    let key = <RistrettoCtx as Context>::Element::deser(&bytes)?;
    Ok((DEFAULT_PUBLIC_KEY_RISTRETTO_STR.to_string(), key))
}

/// Read the published joint key without changing its unpadded base64 encoding.
pub fn parse_public_key(
    config: &BallotStyle,
) -> Result<<RistrettoCtx as Context>::Element, BallotError> {
    let published = config.public_key.as_ref().ok_or_else(|| {
        BallotError::ConsistencyCheck("Missing Public Key".into())
    })?;
    let bytes = general_purpose::STANDARD_NO_PAD
        .decode(&published.public_key)
        .map_err(|e| BallotError::Serialization(e.to_string()))?;
    <RistrettoCtx as Context>::Element::deser(&bytes)
        .map_err(|e| BallotError::Serialization(e.to_string()))
}

/// The keys ceremony UUID is exactly the Configuration.id used by trustees.
pub fn ballot_configuration_id(
    config: &BallotStyle,
) -> Result<u128, BallotError> {
    let published = config.public_key.as_ref().ok_or_else(|| {
        BallotError::ConsistencyCheck("Missing Public Key".into())
    })?;
    if let Some(id) = &published.configuration_id {
        let parsed: u128 = id.parse().map_err(|e| {
            BallotError::ConsistencyCheck(format!(
                "Invalid cryptographic configuration ID: {e}"
            ))
        })?;
        if parsed.to_string() != *id {
            return Err(BallotError::ConsistencyCheck(
                "Noncanonical cryptographic configuration ID".into(),
            ));
        }
        return Ok(parsed);
    }
    match (&published.keys_ceremony_id, published.is_demo) {
        (Some(id), _) => uuid::Uuid::parse_str(id)
            .map(|id| id.as_u128())
            .map_err(|e| {
                BallotError::ConsistencyCheck(format!(
                    "Invalid keys ceremony UUID: {e}"
                ))
            }),
        (None, true) => Ok(0),
        (None, false) => Err(BallotError::ConsistencyCheck(
            "Missing keys ceremony UUID for ballot encryption".into(),
        )),
    }
}

/// Verify canonical ciphertext proofs and their complete ordered contest coverage
/// against a trusted published style. Client-supplied keys and contexts are never used.
pub fn verify_ballot_ciphertexts(
    entries: &[BallotCiphertext],
    style: &BallotStyle,
    policy: ContestEncryptionPolicy,
) -> Result<(), BallotError> {
    let expected: Vec<String> = style
        .votable_contests()
        .map(|contest| contest.id.clone())
        .collect();
    let actual: Vec<Vec<String>> = entries
        .iter()
        .map(|entry| entry.contest_ids.clone())
        .collect();
    let expected_groups = match policy {
        ContestEncryptionPolicy::SINGLE_CONTEST => {
            expected.into_iter().map(|id| vec![id]).collect()
        }
        ContestEncryptionPolicy::MULTIPLE_CONTESTS => vec![expected],
    };
    if actual != expected_groups {
        return Err(BallotError::ConsistencyCheck(
            "Ciphertext contest coverage differs from the published style"
                .into(),
        ));
    }
    let public_key = parse_public_key(style)?;
    let configuration_id = ballot_configuration_id(style)?;
    for entry in entries {
        entry
            .verify(configuration_id, &public_key)
            .map_err(|error| {
                BallotError::ConsistencyCheck(format!(
                    "Invalid ciphertext proof: {error}"
                ))
            })?;
    }
    Ok(())
}

/// Verify a revealed single-contest ballot using its native proofs and openings.
pub fn verify_auditable_ballot_ciphertexts(
    _ctx: &RistrettoCtx,
    ballot: &AuditableBallot,
) -> Result<(), BallotError> {
    check_types_version(ballot.version)?;
    let contests = ballot.deserialize_contests()?;
    let expected: Vec<String> = ballot
        .config
        .votable_contests()
        .map(|c| c.id.clone())
        .collect();
    let actual: Vec<String> =
        contests.iter().map(|c| c.contest_id.clone()).collect();
    if actual != expected {
        return Err(BallotError::ConsistencyCheck(
            "Auditable contest coverage differs from the ballot style".into(),
        ));
    }
    let public_key = parse_public_key(&ballot.config)?;
    let configuration_id = ballot_configuration_id(&ballot.config)?;
    for contest in contests {
        verify_auditable_choice(
            &contest.choice,
            configuration_id,
            &public_key,
        )?;
    }
    Ok(())
}

/// Verify a revealed multi-contest ballot using its native proof and opening.
pub fn verify_auditable_multi_ballot_ciphertext(
    _ctx: &RistrettoCtx,
    ballot: &AuditableMultiBallot,
) -> Result<(), BallotError> {
    check_types_version(ballot.version)?;
    let contests = ballot.deserialize_contests()?;
    let expected: Vec<String> = ballot
        .config
        .votable_contests()
        .map(|c| c.id.clone())
        .collect();
    if contests.contest_ids != expected {
        return Err(BallotError::ConsistencyCheck(
            "Auditable contest coverage differs from the ballot style".into(),
        ));
    }
    verify_auditable_choice(
        &contests.choice,
        ballot_configuration_id(&ballot.config)?,
        &parse_public_key(&ballot.config)?,
    )
}

fn verify_auditable_choice(
    choice: &ReplicationChoice,
    configuration_id: u128,
    public_key: &<RistrettoCtx as Context>::Element,
) -> Result<(), BallotError> {
    let entry = BallotCiphertext::new(Vec::new(), choice.ciphertext.clone());
    verify_opening(
        &entry,
        configuration_id,
        public_key,
        &choice.plaintext,
        &choice.randomness,
    )
    .map_err(|err| BallotError::CryptographicCheck(err.to_string()))
}

/// Encrypt the ballot codec's payload with wbraid's Naor–Yung tooling.
pub fn encrypt_plaintext_candidate(
    _ctx: &RistrettoCtx,
    public_key: <RistrettoCtx as Context>::Element,
    plaintext: [u8; 30],
    configuration_id: u128,
) -> Result<ReplicationChoice, BallotError> {
    let (ciphertext, randomness) =
        encrypt_payload(configuration_id, &public_key, &plaintext)
            .map_err(|e| BallotError::Serialization(e.to_string()))?;
    Ok(ReplicationChoice {
        ciphertext,
        plaintext,
        randomness,
    })
}

/// Audit the published ciphertext against the revealed plaintext and randomness.
pub fn recreate_encrypt_cyphertext(
    _ctx: &RistrettoCtx,
    ballot: &AuditableBallot,
) -> Result<Vec<ReplicationChoice>, BallotError> {
    let public_key = parse_public_key(&ballot.config)?;
    let configuration_id = ballot_configuration_id(&ballot.config)?;
    if ballot.contests.len() != ballot.config.votable_contests().count() {
        return Err(BallotError::ConsistencyCheck("Number of election contests should match number of candidates in the ballot".into()));
    }
    ballot
        .deserialize_contests()?
        .into_iter()
        .map(|contest| {
            recreate_encrypt_candidate(
                &public_key,
                configuration_id,
                &contest.choice,
            )
        })
        .collect()
}

pub(crate) fn recreate_encrypt_candidate(
    public_key: &<RistrettoCtx as Context>::Element,
    configuration_id: u128,
    choice: &ReplicationChoice,
) -> Result<ReplicationChoice, BallotError> {
    let entry = BallotCiphertext::new(Vec::new(), choice.ciphertext.clone());
    verify_opening(
        &entry,
        configuration_id,
        public_key,
        &choice.plaintext,
        &choice.randomness,
    )
    .map_err(|e| BallotError::ConsistencyCheck(e.to_string()))?;
    Ok(choice.clone())
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

pub fn encrypt_decoded_multi_contest(
    _ctx: &RistrettoCtx,
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

    encrypt_multi_ballot(_ctx, &ballot, config)
}

pub fn encrypt_decoded_contest(
    _ctx: &RistrettoCtx,
    decoded_contests: &Vec<DecodedVoteContest>,
    config: &BallotStyle,
) -> Result<AuditableBallot, BallotError> {
    let public_key: <RistrettoCtx as Context>::Element =
        parse_public_key(&config)?;

    let mut contests: Vec<AuditableBallotContest> = vec![];

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
        let choice = encrypt_plaintext_candidate(
            _ctx,
            public_key.clone(),
            plaintext,
            ballot_configuration_id(config)?,
        )?;
        contests.push(AuditableBallotContest {
            contest_id: contest.id.clone(),
            choice: choice,
        });
    }

    let mut auditable_ballot = AuditableBallot {
        version: TYPES_VERSION,
        issue_date: get_current_date(),
        contests: AuditableBallot::serialize_contests(&contests)?,
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
) -> AnyhowResult<Hash> {
    let mut bytes = ballot_style.record_serialize()?;
    if ballot_style.public_key.as_ref().is_some_and(|key| {
        key.configuration_id.is_some() || key.keys_ceremony_id.is_some()
    }) {
        bytes.extend_from_slice(b"sequent-wbraid-ballot-context-v3");
        bytes.extend_from_slice(
            &ballot_configuration_id(ballot_style)
                .map_err(|error| anyhow!("{error:?}"))?
                .to_be_bytes(),
        );
    }
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

pub fn hash_ballot_digest(
    hashable_ballot: &HashableBallot,
) -> AnyhowResult<Hash> {
    let bytes = match hashable_ballot.version {
        LEGACY_TYPES_VERSION => RawHashableBallot::try_from(hashable_ballot)
            .map_err(|error| anyhow!("{error:?}"))?
            .ser(),
        TYPES_VERSION | SHA512_TYPES_VERSION => StyleBoundHashableBallot {
            version: hashable_ballot.version,
            format: ContestEncryptionPolicy::SINGLE_CONTEST,
            issue_date: hashable_ballot.issue_date.clone(),
            config: hashable_ballot.config.clone(),
            ballot_style_hash: hashable_ballot.ballot_style_hash.clone(),
            contests: hashable_ballot
                .deserialize_contests()
                .map_err(|error| anyhow!("{error:?}"))?
                .iter()
                .map(Serializable::ser)
                .collect::<Vec<_>>(),
        }
        .record_serialize()?,
        version => return Err(anyhow!("Unexpected ballot version {version}")),
    };
    if hashable_ballot.version == TYPES_VERSION {
        Ok(hash::hash_sha3_512(&bytes))
    } else {
        hash::hash_to_array(&bytes)
    }
}

pub fn shorten_hash(hash: &Hash) -> ShortHash {
    let mut shortened: ShortHash = [0u8; SHORT_SHA512_HASH_LENGTH_BYTES];
    shortened.copy_from_slice(&hash[0..32]);
    shortened
}

// hash ballot:
// Hash the version-bound ballot bytes (SHA3-512 for version 4, historical
// SHA-512 for versions 2/3), truncate to 256 bits, and encode as hexadecimal.
pub fn hash_ballot(
    hashable_ballot: &HashableBallot,
) -> Result<String, BallotError> {
    let sha512_hash = hash_ballot_digest(hashable_ballot)
        .map_err(|error| BallotError::Serialization(error.to_string()))?;
    let short_hash = shorten_hash(&sha512_hash);
    Ok(hex::encode(short_hash))
}

////////////////////////////////////////////////////////////////
/// Multi ballots
////////////////////////////////////////////////////////////////

pub fn encrypt_multi_ballot(
    _ctx: &RistrettoCtx,
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

    let public_key: <RistrettoCtx as Context>::Element =
        parse_public_key(&config)?;
    let plaintext =
        ballot_choices.encode_to_30_bytes(&config).map_err(|err| {
            BallotError::Serialization(format!(
                "Error encrypting plaintext: {}",
                err
            ))
        })?;
    let contest_ids = ballot_choices.get_contest_ids();

    let choice = encrypt_plaintext_candidate(
        _ctx,
        public_key.clone(),
        plaintext,
        ballot_configuration_id(config)?,
    )?;

    let contests = AuditableMultiBallotContests {
        contest_ids,
        choice,
    };

    let mut auditable_ballot = AuditableMultiBallot {
        version: TYPES_VERSION,
        issue_date: get_current_date(),
        contests: AuditableMultiBallot::serialize_contests(&contests)?,
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
    let sha512_hash = hash_multi_ballot_digest(hashable_ballot)
        .map_err(|error| BallotError::Serialization(error.to_string()))?;
    let short_hash = shorten_hash(&sha512_hash);
    Ok(hex::encode(short_hash))
}

pub fn hash_multi_ballot_digest(
    hashable_ballot: &HashableMultiBallot,
) -> AnyhowResult<Hash> {
    let bytes = match hashable_ballot.version {
        LEGACY_TYPES_VERSION => {
            RawHashableMultiBallot::try_from(hashable_ballot)
                .map_err(|error| anyhow!("{error:?}"))?
                .ser()
        }
        TYPES_VERSION | SHA512_TYPES_VERSION => StyleBoundHashableBallot {
            version: hashable_ballot.version,
            format: ContestEncryptionPolicy::MULTIPLE_CONTESTS,
            issue_date: hashable_ballot.issue_date.clone(),
            config: hashable_ballot.config.clone(),
            ballot_style_hash: hashable_ballot.ballot_style_hash.clone(),
            contests: hashable_ballot
                .deserialize_contests()
                .map_err(|error| anyhow!("{error:?}"))?
                .ser(),
        }
        .record_serialize()?,
        version => return Err(anyhow!("Unexpected ballot version {version}")),
    };
    if hashable_ballot.version == TYPES_VERSION {
        Ok(hash::hash_sha3_512(&bytes))
    } else {
        hash::hash_to_array(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::{encrypt_decoded_contest, verify_ballot_ciphertexts};
    use crate::ballot::{ContestEncryptionPolicy, SignedHashableBallot};
    use crate::ballot_codec::bigint;
    use crate::ballot_codec::plaintext_contest::PlaintextCodec;
    use crate::ballot_codec::vec;
    use crate::encrypt;
    use crate::fixtures::ballot_codec::*;
    use crate::plaintext::DecodedVoteContest;
    use crate::serialization::deserialize_with_path::deserialize_value;
    use crate::util::normalize_vote::normalize_vote_contest;

    use cryptography::context::Context;
    use cryptography::context::RistrettoCtx;

    #[test]
    fn auditable_ciphertexts_reject_changed_openings_and_contest_coverage() {
        let style = get_writein_ballot_style();
        let selections = vec![get_writein_plaintext()];
        let mut ballot =
            encrypt_decoded_contest(&RistrettoCtx, &selections, &style)
                .unwrap();
        encrypt::verify_auditable_ballot_ciphertexts(&RistrettoCtx, &ballot)
            .unwrap();
        let mut contests = ballot.deserialize_contests().unwrap();
        contests[0].choice.plaintext[0] ^= 1;
        ballot.contests =
            crate::ballot::AuditableBallot::serialize_contests(&contests)
                .unwrap();
        assert!(matches!(
            encrypt::verify_auditable_ballot_ciphertexts(
                &RistrettoCtx,
                &ballot
            ),
            Err(crate::error::BallotError::CryptographicCheck(_))
        ));
        contests[0].choice.plaintext[0] ^= 1;
        contests[0].contest_id = "foreign-contest".into();
        ballot.contests =
            crate::ballot::AuditableBallot::serialize_contests(&contests)
                .unwrap();
        assert!(encrypt::verify_auditable_ballot_ciphertexts(
            &RistrettoCtx,
            &ballot
        )
        .is_err());
        ballot.contests.clear();
        assert!(encrypt::verify_auditable_ballot_ciphertexts(
            &RistrettoCtx,
            &ballot
        )
        .is_err());

        let mut multi = encrypt::encrypt_decoded_multi_contest(
            &RistrettoCtx,
            &selections,
            &style,
        )
        .unwrap();
        encrypt::verify_auditable_multi_ballot_ciphertext(
            &RistrettoCtx,
            &multi,
        )
        .unwrap();
        let mut contests = multi.deserialize_contests().unwrap();
        contests.choice.randomness = RistrettoCtx::random_scalar();
        multi.contests =
            crate::multi_ballot::AuditableMultiBallot::serialize_contests(
                &contests,
            )
            .unwrap();
        assert!(matches!(
            encrypt::verify_auditable_multi_ballot_ciphertext(
                &RistrettoCtx,
                &multi
            ),
            Err(crate::error::BallotError::CryptographicCheck(_))
        ));
        contests.contest_ids.push(contests.contest_ids[0].clone());
        multi.contests =
            crate::multi_ballot::AuditableMultiBallot::serialize_contests(
                &contests,
            )
            .unwrap();
        assert!(encrypt::verify_auditable_multi_ballot_ciphertext(
            &RistrettoCtx,
            &multi
        )
        .is_err());
    }

    #[test]
    fn published_style_binds_ciphertexts_to_ceremony_and_contest_order() {
        let mut style = get_writein_ballot_style();
        let first = get_writein_plaintext();
        let mut second_contest = style.contests[0].clone();
        second_contest.id = "87654321-4321-4321-9321-cba987654321".into();
        let mut second = first.clone();
        second.contest_id = second_contest.id.clone();
        style.contests.push(second_contest);
        let choices = vec![first, second];
        let published = style.public_key.as_mut().unwrap();
        published.is_demo = false;
        published.keys_ceremony_id =
            Some("12345678-1234-4234-9234-123456789abc".into());
        let ballot =
            encrypt_decoded_contest(&RistrettoCtx, &choices, &style).unwrap();
        let signed = SignedHashableBallot::try_from(&ballot).unwrap();
        let entries = signed.deserialize_contests().unwrap();
        verify_ballot_ciphertexts(
            &entries,
            &style,
            ContestEncryptionPolicy::SINGLE_CONTEST,
        )
        .unwrap();

        let mut foreign_style = style.clone();
        foreign_style.public_key.as_mut().unwrap().keys_ceremony_id =
            Some("87654321-4321-4321-9321-cba987654321".into());
        assert!(verify_ballot_ciphertexts(
            &entries,
            &foreign_style,
            ContestEncryptionPolicy::SINGLE_CONTEST
        )
        .is_err());
        let mut substituted = entries.clone();
        substituted[0].contest_ids = vec!["another-contest".into()];
        assert!(verify_ballot_ciphertexts(
            &substituted,
            &style,
            ContestEncryptionPolicy::SINGLE_CONTEST
        )
        .is_err());
        assert!(verify_ballot_ciphertexts(
            &entries[..entries.len() - 1],
            &style,
            ContestEncryptionPolicy::SINGLE_CONTEST
        )
        .is_err());
        let mut duplicated = entries.clone();
        duplicated.push(entries[0].clone());
        assert!(verify_ballot_ciphertexts(
            &duplicated,
            &style,
            ContestEncryptionPolicy::SINGLE_CONTEST
        )
        .is_err());
        assert!(verify_ballot_ciphertexts(
            &entries,
            &style,
            ContestEncryptionPolicy::MULTIPLE_CONTESTS
        )
        .is_err());
    }

    #[test]
    fn test_encrypt_plaintext_candidate() {
        let ctx = RistrettoCtx;

        let (pk_string, pk_element) =
            encrypt::default_public_key_ristretto().unwrap();

        let plaintext = [0x5a; 30];

        encrypt::encrypt_plaintext_candidate(&ctx, pk_element, plaintext, 0)
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
        let auditable_ballot = encrypt::encrypt_decoded_contest(
            &ctx,
            &vec![decoded_contest.clone()],
            &ballot_style,
        )
        .unwrap();
        let contests = auditable_ballot.deserialize_contests().unwrap();
        let plaintext = contests[0].choice.plaintext.clone();
        let plaintext_vec = vec::decode_array_to_vec(&plaintext).unwrap(); // compare
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
        use cryptography::context::RistrettoCtx;
        use cryptography::utils::serialization::Serializable;
        use voting_crypto::hash::hash_to_array;

        /// A plurality contest with three candidates under a ballot style
        /// that carries a public key, so both codecs can encrypt it.
        fn ballot_style() -> BallotStyle {
            BallotStyle {
                contests: vec![get_test_contest()],
                ..get_writein_ballot_style()
            }
        }

        fn single_ballot() -> AuditableBallot {
            encrypt_decoded_contest(
                &RistrettoCtx,
                &vec![get_test_decoded_vote_contest()],
                &ballot_style(),
            )
            .unwrap()
        }

        fn multi_ballot() -> AuditableMultiBallot {
            encrypt_decoded_multi_contest(
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
                keys_ceremony_id: None,
                configuration_id: None,
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
            let contest = ballot.deserialize_contests().unwrap().remove(0);
            let config = ballot.config.id.clone();
            let ballot_style_hash = hash_ballot_style(&ballot_style()).unwrap();

            let single = HashableBallot {
                version: TYPES_VERSION,
                issue_date: ballot.issue_date.clone(),
                contests: HashableBallot::serialize_contests(&vec![
                    HashableBallotContest {
                        contest_ids: vec![contest.contest_id.clone()],
                        ciphertext: contest.choice.ciphertext.clone(),
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
                    &HashableMultiBallotContests {
                        contest_ids: vec![contest.contest_id.clone()],
                        ciphertext: contest.choice.ciphertext.clone(),
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
            let raw = RawHashableBallot::try_from(&hashable).unwrap();

            assert_eq!(hashable.version, LEGACY_TYPES_VERSION);
            assert_eq!(
                hash_ballot(&hashable).unwrap(),
                short_hash_hex(&raw.ser())
            );
        }

        #[test]
        fn legacy_multi_ballot_keeps_its_tracker() {
            let mut ballot = multi_ballot();
            ballot.version = LEGACY_TYPES_VERSION;
            let hashable = HashableMultiBallot::try_from(&ballot).unwrap();
            let raw = RawHashableMultiBallot::try_from(&hashable).unwrap();

            assert_eq!(hashable.version, LEGACY_TYPES_VERSION);
            assert_eq!(
                hash_multi_ballot(&hashable).unwrap(),
                short_hash_hex(&raw.ser())
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
        let auditable_ballot = encrypt_decoded_contest(
            &ctx,
            &decoded_contests,
            &election,
        );
        assert_eq!(format!("{:?}", auditable_ballot.unwrap_err()), "".to_string());
        //assert!(auditable_ballot.is_ok());
    }*/
}
