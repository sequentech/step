// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

/// The contests of a cast ballot have to be exactly those of a ballot style
/// published for the voter's area and election.
mod ballot_style_contests {
    use super::*;
    use sequent_core::ballot::Contest;
    use sequent_core::encrypt::{encrypt_decoded_contest, encrypt_decoded_multi_contest};
    use sequent_core::fixtures::ballot_codec::{
        get_test_contest, get_test_decoded_vote_contest, get_writein_ballot_style,
    };

    const VOTER_ID: &str = "voter";
    const OTHER_CONTEST_ID: &str = "8d5b1c4e-2a3f-4e6b-9c7d-0e1f2a3b4c5d";

    /// The ballot style of the fixture ballots, which has a single contest.
    fn ballot_style() -> BallotStyle {
        BallotStyle {
            contests: vec![get_test_contest()],
            ..get_writein_ballot_style()
        }
    }

    /// A set of contest ids, as the lookup of a published style returns them.
    fn contest_ids(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    /// The contests of the fixture ballot style.
    fn style_contest_ids() -> HashSet<String> {
        contest_ids(&[get_test_contest().id.as_str()])
    }

    /// The published styles of an area that has only the fixture ballot style.
    fn published_styles() -> Vec<HashSet<String>> {
        vec![style_contest_ids()]
    }

    /// A ballot that encrypts each contest of the fixture ballot style on its own.
    fn single_ballot() -> HashableBallot {
        let auditable = encrypt_decoded_contest::<RistrettoCtx>(
            &RistrettoCtx,
            &vec![get_test_decoded_vote_contest()],
            &ballot_style(),
        )
        .unwrap();
        HashableBallot::try_from(&auditable).unwrap()
    }

    /// The fixture ballot as it arrives in a cast request.
    fn single_content() -> String {
        serde_json::to_string(&single_ballot()).unwrap()
    }

    /// A ballot that carries one copy of the encrypted contest for each of
    /// the given ids.
    fn single_content_with_contest_ids(ids: &[&str]) -> String {
        let ballot = single_ballot();
        let contest = ballot
            .deserialize_contests::<RistrettoCtx>()
            .unwrap()
            .remove(0);
        let contests: Vec<HashableBallotContest<RistrettoCtx>> = ids
            .iter()
            .map(|id| HashableBallotContest {
                contest_id: id.to_string(),
                ..contest.clone()
            })
            .collect();
        serde_json::to_string(&HashableBallot {
            contests: HashableBallot::serialize_contests::<RistrettoCtx>(&contests).unwrap(),
            ..ballot
        })
        .unwrap()
    }

    /// A ballot that encrypts all the contests of the fixture ballot style together.
    fn multi_ballot() -> HashableMultiBallot {
        let auditable = encrypt_decoded_multi_contest::<RistrettoCtx>(
            &RistrettoCtx,
            &vec![get_test_decoded_vote_contest()],
            &ballot_style(),
        )
        .unwrap();
        HashableMultiBallot::try_from(&auditable).unwrap()
    }

    /// The fixture multi ballot as it arrives in a cast request.
    fn multi_content() -> String {
        serde_json::to_string(&multi_ballot()).unwrap()
    }

    /// A multi ballot that lists exactly the given contest ids.
    fn multi_content_with_contest_ids(ids: &[&str]) -> String {
        let ballot = multi_ballot();
        let mut contests = ballot.deserialize_contests::<RistrettoCtx>().unwrap();
        contests.contest_ids = ids.iter().map(|id| id.to_string()).collect();
        serde_json::to_string(&HashableMultiBallot {
            contests: HashableMultiBallot::serialize_contests::<RistrettoCtx>(&contests).unwrap(),
            ..ballot
        })
        .unwrap()
    }

    /// A ballot with each contest of the published style is accepted.
    #[test]
    fn ballot_with_every_style_contest_is_accepted() {
        assert!(
            deserialize_and_check_ballot(&single_content(), VOTER_ID, &published_styles()).is_ok()
        );
    }

    /// A ballot that lacks a contest of the style is rejected, and the error names it.
    #[test]
    fn ballot_missing_a_style_contest_is_rejected() {
        let published = vec![contest_ids(&[&get_test_contest().id, OTHER_CONTEST_ID])];

        assert!(matches!(
            deserialize_and_check_ballot(&single_content(), VOTER_ID, &published),
            Err(CastVoteError::BallotStyleMismatch(message)) if message.contains(OTHER_CONTEST_ID)
        ));
    }

    /// A ballot with no contests is rejected when the style has one.
    #[test]
    fn ballot_without_contests_is_rejected() {
        let content = single_content_with_contest_ids(&[]);

        assert!(matches!(
            deserialize_and_check_ballot(&content, VOTER_ID, &published_styles()),
            Err(CastVoteError::BallotStyleMismatch(_))
        ));
    }

    /// A ballot with a contest that the style does not have is rejected, and the error
    /// names it.
    #[test]
    fn ballot_with_a_contest_outside_the_style_is_rejected() {
        let content = single_content_with_contest_ids(&[&get_test_contest().id, OTHER_CONTEST_ID]);

        assert!(matches!(
            deserialize_and_check_ballot(&content, VOTER_ID, &published_styles()),
            Err(CastVoteError::BallotStyleMismatch(message)) if message.contains(OTHER_CONTEST_ID)
        ));
    }

    /// A ballot that repeats a contest is rejected, even when it has every contest of
    /// the style.
    #[test]
    fn ballot_repeating_a_style_contest_is_rejected() {
        let contest_id = get_test_contest().id;
        let content = single_content_with_contest_ids(&[&contest_id, &contest_id]);

        assert!(matches!(
            deserialize_and_check_ballot(&content, VOTER_ID, &published_styles()),
            Err(CastVoteError::BallotStyleMismatch(message)) if message.contains(&contest_id)
        ));
    }

    /// No ballot is accepted for an area and election without a published ballot style.
    #[test]
    fn ballot_is_rejected_when_no_ballot_style_is_published() {
        assert!(matches!(
            deserialize_and_check_ballot(&single_content(), VOTER_ID, &[]),
            Err(CastVoteError::BallotStyleMismatch(_))
        ));
    }

    /// A ballot with no contests does not match an empty list of published styles
    /// either.
    #[test]
    fn ballot_without_contests_is_rejected_when_no_ballot_style_is_published() {
        let content = single_content_with_contest_ids(&[]);

        assert!(matches!(
            deserialize_and_check_ballot(&content, VOTER_ID, &[]),
            Err(CastVoteError::BallotStyleMismatch(_))
        ));
    }

    /// A style without contests has nothing to encrypt, so it matches a ballot with
    /// no contests.
    #[test]
    fn ballot_without_contests_is_accepted_for_a_style_without_contests() {
        let content = single_content_with_contest_ids(&[]);

        assert!(deserialize_and_check_ballot(&content, VOTER_ID, &[HashSet::new()]).is_ok());
    }

    /// A ballot is accepted when it matches any one of the published styles.
    #[test]
    fn ballot_matching_any_published_style_is_accepted() {
        let published = vec![contest_ids(&[OTHER_CONTEST_ID]), style_contest_ids()];

        assert!(deserialize_and_check_ballot(&single_content(), VOTER_ID, &published).is_ok());
    }

    /// The contests of two published styles cannot be combined in one ballot.
    #[test]
    fn ballot_cannot_combine_the_contests_of_different_published_styles() {
        let content = single_content_with_contest_ids(&[&get_test_contest().id, OTHER_CONTEST_ID]);
        let published = vec![style_contest_ids(), contest_ids(&[OTHER_CONTEST_ID])];

        assert!(matches!(
            deserialize_and_check_ballot(&content, VOTER_ID, &published),
            Err(CastVoteError::BallotStyleMismatch(_))
        ));
    }

    /// A multi ballot with each contest of the published style is accepted.
    #[test]
    fn multi_ballot_with_every_style_contest_is_accepted() {
        assert!(deserialize_and_check_multi_ballot(
            &multi_content(),
            VOTER_ID,
            &published_styles()
        )
        .is_ok());
    }

    /// A multi ballot that lacks a contest of the style is rejected, and the error names
    /// it.
    #[test]
    fn multi_ballot_missing_a_style_contest_is_rejected() {
        let published = vec![contest_ids(&[&get_test_contest().id, OTHER_CONTEST_ID])];

        assert!(matches!(
            deserialize_and_check_multi_ballot(&multi_content(), VOTER_ID, &published),
            Err(CastVoteError::BallotStyleMismatch(message)) if message.contains(OTHER_CONTEST_ID)
        ));
    }

    /// A multi ballot that lists a contest the style does not have is rejected, and the
    /// error names it.
    #[test]
    fn multi_ballot_with_a_contest_outside_the_style_is_rejected() {
        let content = multi_content_with_contest_ids(&[&get_test_contest().id, OTHER_CONTEST_ID]);

        assert!(matches!(
            deserialize_and_check_multi_ballot(&content, VOTER_ID, &published_styles()),
            Err(CastVoteError::BallotStyleMismatch(message)) if message.contains(OTHER_CONTEST_ID)
        ));
    }

    /// A multi ballot that lists a contest twice is rejected, even when it has every
    /// contest of the style.
    #[test]
    fn multi_ballot_repeating_a_style_contest_is_rejected() {
        let contest_id = get_test_contest().id;
        let content = multi_content_with_contest_ids(&[&contest_id, &contest_id]);

        assert!(matches!(
            deserialize_and_check_multi_ballot(&content, VOTER_ID, &published_styles()),
            Err(CastVoteError::BallotStyleMismatch(message)) if message.contains(&contest_id)
        ));
    }

    /// No multi ballot is accepted for an area and election without a published ballot
    /// style.
    #[test]
    fn multi_ballot_is_rejected_when_no_ballot_style_is_published() {
        assert!(matches!(
            deserialize_and_check_multi_ballot(&multi_content(), VOTER_ID, &[]),
            Err(CastVoteError::BallotStyleMismatch(_))
        ));
    }

    /// Every contest of a ballot style is required from the ballot.
    #[test]
    fn every_contest_of_a_published_style_is_required_from_the_ballot() {
        let other_contest = Contest {
            id: OTHER_CONTEST_ID.to_string(),
            ..get_test_contest()
        };
        let style = BallotStyle {
            contests: vec![get_test_contest(), other_contest],
            ..ballot_style()
        };

        assert_eq!(
            votable_contest_ids(&serde_json::to_string(&style).unwrap()).unwrap(),
            contest_ids(&[&get_test_contest().id, OTHER_CONTEST_ID])
        );
    }

    /// A published style that cannot be parsed is an internal error, not a mismatch of
    /// the ballot.
    #[test]
    fn malformed_published_ballot_style_is_an_internal_error() {
        assert!(matches!(
            votable_contest_ids("{\"contests\": 1}"),
            Err(CastVoteError::CheckStatusInternalFailed(_))
        ));
    }

    /// A published style that cannot be parsed fails the same way on every attempt, so
    /// it is final.
    #[test]
    fn malformed_published_ballot_style_is_not_retried() {
        let style = PublishedBallotStyle {
            id: Uuid::new_v4(),
            ballot_eml: Some("{\"contests\": 1}".to_string()),
        };

        assert!(matches!(
            read_style_contest_ids(&style),
            Err(PublishedStylesError::Unreadable(
                CastVoteError::CheckStatusInternalFailed(_)
            ))
        ));
    }

    /// A published style stored without its EML cannot be read on another attempt
    /// either, so it is final.
    #[test]
    fn published_ballot_style_without_eml_is_not_retried() {
        let style = PublishedBallotStyle {
            id: Uuid::new_v4(),
            ballot_eml: None,
        };

        assert!(matches!(
            read_style_contest_ids(&style),
            Err(PublishedStylesError::Unreadable(
                CastVoteError::CheckStatusInternalFailed(_)
            ))
        ));
    }

    /// The contests of a readable published style are the contests of its EML.
    #[test]
    fn published_ballot_style_is_read_from_its_eml() {
        let style = PublishedBallotStyle {
            id: Uuid::new_v4(),
            ballot_eml: Some(serde_json::to_string(&ballot_style()).unwrap()),
        };

        assert_eq!(read_style_contest_ids(&style).unwrap(), style_contest_ids());
    }

    /// An unreadable published style is returned as a final result, so the cast is not
    /// retried.
    #[test]
    fn unreadable_published_style_ends_the_cast_without_a_retry() {
        let error = PublishedStylesError::Unreadable(CastVoteError::CheckStatusInternalFailed(
            "unreadable".to_string(),
        ));

        assert!(matches!(
            error.into_cast_vote_result(),
            Ok(InsertCastVoteResult::SkipRetryFailure(
                CastVoteError::CheckStatusInternalFailed(_)
            ))
        ));
    }

    /// A failed lookup of the published styles is returned as an error, so the cast is
    /// retried.
    #[test]
    fn failed_published_styles_lookup_is_returned_for_a_retry() {
        let error = PublishedStylesError::lookup_failed(anyhow!("connection reset"));

        assert!(matches!(
            error.into_cast_vote_result(),
            Err(CastVoteError::CheckStatusInternalFailed(_))
        ));
    }
}
