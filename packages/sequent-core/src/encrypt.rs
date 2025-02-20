// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;

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
use crate::plaintext::DecodedVoteContest;
use crate::serialization::base64::Base64Deserialize;
use crate::util::date::get_current_date;
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

    let contests_map: HashMap<String, Contest> = config
        .contests
        .iter()
        .map(|contest| (contest.id.clone(), contest.clone()))
        .collect();

    let contest_choices = decoded_contests
        .iter()
        .map(|choice| -> Result<ContestChoices, BallotError> {
            let contest = contests_map.get(&choice.contest_id).ok_or(
                BallotError::ConsistencyCheck(format!("Can't find contest")),
            )?;
            Ok(ContestChoices::from_decoded_vote_contest(choice, contest))
        })
        .collect::<Result<Vec<_>, BallotError>>()?;

    let ballot_choices = BallotChoices::new(false, contest_choices);

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

    let contests_map: HashMap<String, Contest> = config
        .contests
        .iter()
        .map(|contest| (contest.id.clone(), contest.clone()))
        .collect();

    let contest_choices = decoded_contests
        .iter()
        .map(|choice| -> Result<ContestChoices, BallotError> {
            let contest = contests_map.get(&choice.contest_id).ok_or(
                BallotError::ConsistencyCheck(format!("Can't find contest")),
            )?;
            Ok(ContestChoices::from_decoded_vote_contest(choice, contest))
        })
        .collect::<Result<Vec<_>, BallotError>>()?;

    let ballot = BallotChoices::new(false, contest_choices);

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
    };

    let hashable_ballot = HashableBallot::try_from(&auditable_ballot)?;
    auditable_ballot.ballot_hash = hash_ballot(&hashable_ballot)?;

    Ok(auditable_ballot)
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
                contest.get_counting_algorithm().as_str(),
                false,
                &invalid_candidate_ids,
            ),
            normalize_vote_contest(
                &decoded_contest,
                contest.get_counting_algorithm().as_str(),
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
                contest.get_counting_algorithm().as_str(),
                false,
                &invalid_candidate_ids,
            ),
            normalize_vote_contest(
                &decoded_contest2,
                contest.get_counting_algorithm().as_str(),
                false,
                &invalid_candidate_ids,
            )
        );
    }

    fn get_multi_reencoding_fixture() -> Result<()> {
        let ballot_selection_str = "[{\"contest_id\":\"3708000c-333b-4f5b-ae9d-eed052af9aff\",\"is_explicit_invalid\":false,\"is_explicit_blank\":true,\"invalid_errors\":[],\"invalid_alerts\":[],\"choices\":[{\"id\":\"53afa394-910b-444a-b119-696ad77d5b22\",\"selected\":-1},{\"id\":\"83590954-a4ab-47e3-bda2-dda883e5840f\",\"selected\":-1},{\"id\":\"b1ba31fa-be0c-49fa-b5fc-fa5c94c959c5\",\"selected\":-1},{\"id\":\"c6010d28-b7de-41d2-af01-63b386833dc2\",\"selected\":-1}]}]";
        let ballot_eml_str = "{\"area_id\":\"c555c0ab-ee22-4b22-8df6-c8ffff032054\",\"contests\":[{\"alias\":\"\",\"alias_i18n\":{},\"annotations\":null,\"candidates\":[{\"alias\":null,\"alias_i18n\":{},\"annotations\":null,\"candidate_type\":null,\"contest_id\":\"3708000c-333b-4f5b-ae9d-eed052af9aff\",\"description\":null,\"description_i18n\":{},\"election_event_id\":\"5fca3d60-86de-4fee-8d39-f6622a6d1f87\",\"election_id\":\"17745b23-469d-4c21-a396-74bff608fd85\",\"id\":\"53afa394-910b-444a-b119-696ad77d5b22\",\"name\":\"A\",\"name_i18n\":{\"cat\":\"A\",\"en\":\"A\",\"es\":\"A\",\"fr\":\"A\",\"tl\":\"A\"},\"presentation\":{\"i18n\":{\"cat\":{\"name\":\"A\"},\"en\":{\"name\":\"A\"},\"es\":{\"name\":\"A\"},\"fr\":{\"name\":\"A\"},\"tl\":{\"name\":\"A\"}},\"invalid_vote_position\":null,\"is_category_list\":null,\"is_disabled\":null,\"is_explicit_blank\":null,\"is_explicit_invalid\":null,\"is_write_in\":null,\"sort_order\":null,\"subtype\":null,\"urls\":null},\"tenant_id\":\"90505c8a-23a9-4cdf-a26b-4e19f6a097d5\"},{\"alias\":\"\",\"alias_i18n\":{},\"annotations\":null,\"candidate_type\":null,\"contest_id\":\"3708000c-333b-4f5b-ae9d-eed052af9aff\",\"description\":\"\",\"description_i18n\":{},\"election_event_id\":\"5fca3d60-86de-4fee-8d39-f6622a6d1f87\",\"election_id\":\"17745b23-469d-4c21-a396-74bff608fd85\",\"id\":\"83590954-a4ab-47e3-bda2-dda883e5840f\",\"name\":\"Blank\",\"name_i18n\":{\"cat\":\"Blank\",\"en\":\"Blank\",\"es\":\"Blank\",\"fr\":\"Blank\",\"tl\":\"Blank\"},\"presentation\":{\"i18n\":{\"cat\":{\"name\":\"Blank\"},\"en\":{\"name\":\"Blank\"},\"es\":{\"name\":\"Blank\"},\"fr\":{\"name\":\"Blank\"},\"tl\":{\"name\":\"Blank\"}},\"invalid_vote_position\":null,\"is_category_list\":false,\"is_disabled\":false,\"is_explicit_blank\":true,\"is_explicit_invalid\":false,\"is_write_in\":false,\"sort_order\":null,\"subtype\":null,\"urls\":null},\"tenant_id\":\"90505c8a-23a9-4cdf-a26b-4e19f6a097d5\"},{\"alias\":null,\"alias_i18n\":{},\"annotations\":null,\"candidate_type\":null,\"contest_id\":\"3708000c-333b-4f5b-ae9d-eed052af9aff\",\"description\":null,\"description_i18n\":{},\"election_event_id\":\"5fca3d60-86de-4fee-8d39-f6622a6d1f87\",\"election_id\":\"17745b23-469d-4c21-a396-74bff608fd85\",\"id\":\"b1ba31fa-be0c-49fa-b5fc-fa5c94c959c5\",\"name\":\"B\",\"name_i18n\":{\"cat\":\"B\",\"en\":\"B\",\"es\":\"B\",\"fr\":\"B\",\"tl\":\"B\"},\"presentation\":{\"i18n\":{\"cat\":{\"name\":\"B\"},\"en\":{\"name\":\"B\"},\"es\":{\"name\":\"B\"},\"fr\":{\"name\":\"B\"},\"tl\":{\"name\":\"B\"}},\"invalid_vote_position\":null,\"is_category_list\":null,\"is_disabled\":null,\"is_explicit_blank\":null,\"is_explicit_invalid\":null,\"is_write_in\":null,\"sort_order\":null,\"subtype\":null,\"urls\":null},\"tenant_id\":\"90505c8a-23a9-4cdf-a26b-4e19f6a097d5\"},{\"alias\":\"\",\"alias_i18n\":{},\"annotations\":null,\"candidate_type\":null,\"contest_id\":\"3708000c-333b-4f5b-ae9d-eed052af9aff\",\"description\":\"\",\"description_i18n\":{},\"election_event_id\":\"5fca3d60-86de-4fee-8d39-f6622a6d1f87\",\"election_id\":\"17745b23-469d-4c21-a396-74bff608fd85\",\"id\":\"c6010d28-b7de-41d2-af01-63b386833dc2\",\"name\":\"Invalid\",\"name_i18n\":{\"cat\":\"Invalid\",\"en\":\"Invalid\",\"es\":\"Invalid\",\"fr\":\"Invalid\",\"tl\":\"Invalid\"},\"presentation\":{\"i18n\":{\"cat\":{\"name\":\"Invalid\"},\"en\":{\"name\":\"Invalid\"},\"es\":{\"name\":\"Invalid\"},\"fr\":{\"name\":\"Invalid\"},\"tl\":{\"name\":\"Invalid\"}},\"invalid_vote_position\":null,\"is_category_list\":false,\"is_disabled\":false,\"is_explicit_blank\":false,\"is_explicit_invalid\":true,\"is_write_in\":false,\"sort_order\":null,\"subtype\":null,\"urls\":null},\"tenant_id\":\"90505c8a-23a9-4cdf-a26b-4e19f6a097d5\"}],\"counting_algorithm\":\"plurality-at-large\",\"created_at\":\"2025-02-15T02:59:07.646934+00:00\",\"description\":\"\",\"description_i18n\":{},\"election_event_id\":\"5fca3d60-86de-4fee-8d39-f6622a6d1f87\",\"election_id\":\"17745b23-469d-4c21-a396-74bff608fd85\",\"id\":\"3708000c-333b-4f5b-ae9d-eed052af9aff\",\"is_encrypted\":true,\"max_votes\":1,\"min_votes\":1,\"name\":\"Contest\",\"name_i18n\":{\"cat\":\"Contest\",\"en\":\"Contest\",\"es\":\"Contest\",\"fr\":\"Contest\",\"tl\":\"Contest\"},\"presentation\":{\"allow_writeins\":null,\"base32_writeins\":null,\"blank_vote_policy\":\"not-allowed\",\"candidates_icon_checkbox_policy\":\"square-checkbox\",\"candidates_order\":\"alphabetical\",\"candidates_selection_policy\":null,\"columns\":null,\"cumulative_number_of_checkboxes\":null,\"enable_checkable_lists\":\"allow-selecting-candidates-and-lists\",\"i18n\":{\"cat\":{\"name\":\"Contest\"},\"en\":{\"name\":\"Contest\"},\"es\":{\"name\":\"Contest\"},\"fr\":{\"name\":\"Contest\"},\"tl\":{\"name\":\"Contest\"}},\"invalid_vote_policy\":\"warn-invalid-implicit-and-explicit\",\"max_selections_per_type\":null,\"over_vote_policy\":\"not-allowed-with-msg-and-disable\",\"pagination_policy\":\"\",\"show_points\":null,\"shuffle_categories\":null,\"shuffle_category_list\":null,\"sort_order\":null,\"types_presentation\":null,\"under_vote_policy\":\"warn-and-alert\"},\"tenant_id\":\"90505c8a-23a9-4cdf-a26b-4e19f6a097d5\",\"voting_type\":\"non-preferential\",\"winning_candidates_num\":1}],\"description\":null,\"election_annotations\":{},\"election_dates\":{\"first_paused_at\":null,\"first_started_at\":\"2025-02-15T03:10:41.494840572+00:00\",\"first_stopped_at\":null,\"last_paused_at\":null,\"last_started_at\":\"2025-02-15T03:10:41.494840572+00:00\",\"last_stopped_at\":null,\"scheduled_event_dates\":{}},\"election_event_annotations\":{},\"election_event_id\":\"5fca3d60-86de-4fee-8d39-f6622a6d1f87\",\"election_event_presentation\":{\"contest_encryption_policy\":null,\"css\":null,\"custom_urls\":null,\"elections_order\":null,\"enrollment\":null,\"i18n\":{\"cat\":{\"name\":\"Event\"},\"en\":{\"name\":\"Event\"},\"es\":{\"name\":\"Event\"},\"fr\":{\"name\":\"Event\"},\"tl\":{\"name\":\"Event\"}},\"keys_ceremony_policy\":null,\"language_conf\":{\"default_language_code\":\"en\",\"enabled_language_codes\":[\"en\"]},\"locked_down\":null,\"logo_url\":null,\"materials\":null,\"otp\":null,\"publish_policy\":null,\"redirect_finish_url\":null,\"show_user_profile\":null,\"skip_election_list\":null,\"voter_signing_policy\":null,\"voting_portal_countdown_policy\":null},\"election_id\":\"17745b23-469d-4c21-a396-74bff608fd85\",\"election_presentation\":{\"audit_button_cfg\":null,\"cast_vote_confirm\":null,\"contests_order\":null,\"dates\":null,\"grace_period_policy\":null,\"grace_period_secs\":null,\"i18n\":{\"cat\":{\"name\":\"Election\"},\"en\":{\"name\":\"Election\"},\"es\":{\"name\":\"Election\"},\"fr\":{\"name\":\"Election\"},\"tl\":{\"name\":\"Election\"}},\"init_report\":null,\"initialization_report_policy\":null,\"is_grace_priod\":null,\"language_conf\":{\"default_language_code\":\"en\",\"enabled_language_codes\":[\"en\"]},\"manual_start_voting_period\":null,\"sort_order\":null,\"tally\":null,\"voting_period_end\":null},\"id\":\"fd715a44-9803-46e2-9a5a-34325db1d2ae\",\"num_allowed_revotes\":null,\"public_key\":{\"is_demo\":false,\"public_key\":\"jIFQinnN0cYj3Q0H70DViHDS1AvaL/v3Y1VZGoTmhi8\"},\"tenant_id\":\"90505c8a-23a9-4cdf-a26b-4e19f6a097d5\"}";

        Ok(())
    }

    #[test]
    fn test_test_multi_contest_reencoding_js() {
        let fixture = get_multi_reencoding_fixture().unwrap();
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
