// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

mod ballot_style_contests {
    use super::*;
    use sequent_core::ballot::Contest;
    use sequent_core::encrypt::{encrypt_decoded_contest, encrypt_decoded_multi_contest};
    use sequent_core::fixtures::ballot_codec::{
        get_test_contest, get_test_decoded_vote_contest, get_writein_ballot_style,
    };

    const VOTER_ID: &str = "voter";
    const OTHER_CONTEST_ID: &str = "8d5b1c4e-2a3f-4e6b-9c7d-0e1f2a3b4c5d";

    fn ballot_style() -> BallotStyle {
        BallotStyle {
            contests: vec![get_test_contest()],
            ..get_writein_ballot_style()
        }
    }

    fn contest_ids(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    fn style_contest_ids() -> HashSet<String> {
        contest_ids(&[get_test_contest().id.as_str()])
    }

    fn published_styles() -> Vec<HashSet<String>> {
        vec![style_contest_ids()]
    }

    fn single_ballot() -> HashableBallot {
        let auditable = encrypt_decoded_contest::<RistrettoCtx>(
            &RistrettoCtx,
            &vec![get_test_decoded_vote_contest()],
            &ballot_style(),
        )
        .unwrap();
        HashableBallot::try_from(&auditable).unwrap()
    }

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

    fn multi_ballot() -> HashableMultiBallot {
        let auditable = encrypt_decoded_multi_contest::<RistrettoCtx>(
            &RistrettoCtx,
            &vec![get_test_decoded_vote_contest()],
            &ballot_style(),
        )
        .unwrap();
        HashableMultiBallot::try_from(&auditable).unwrap()
    }

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

    #[test]
    fn ballot_with_every_style_contest_is_accepted() {
        assert!(
            deserialize_and_check_ballot(&single_content(), VOTER_ID, &published_styles()).is_ok()
        );
    }

    #[test]
    fn ballot_missing_a_style_contest_is_rejected() {
        let published = vec![contest_ids(&[&get_test_contest().id, OTHER_CONTEST_ID])];

        assert!(matches!(
            deserialize_and_check_ballot(&single_content(), VOTER_ID, &published),
            Err(CastVoteError::BallotStyleMismatch(message)) if message.contains(OTHER_CONTEST_ID)
        ));
    }

    #[test]
    fn ballot_without_contests_is_rejected() {
        let content = single_content_with_contest_ids(&[]);

        assert!(matches!(
            deserialize_and_check_ballot(&content, VOTER_ID, &published_styles()),
            Err(CastVoteError::BallotStyleMismatch(_))
        ));
    }

    #[test]
    fn ballot_with_a_contest_outside_the_style_is_rejected() {
        let content = single_content_with_contest_ids(&[&get_test_contest().id, OTHER_CONTEST_ID]);

        assert!(matches!(
            deserialize_and_check_ballot(&content, VOTER_ID, &published_styles()),
            Err(CastVoteError::BallotStyleMismatch(message)) if message.contains(OTHER_CONTEST_ID)
        ));
    }

    #[test]
    fn ballot_repeating_a_style_contest_is_rejected() {
        let contest_id = get_test_contest().id;
        let content = single_content_with_contest_ids(&[&contest_id, &contest_id]);

        assert!(matches!(
            deserialize_and_check_ballot(&content, VOTER_ID, &published_styles()),
            Err(CastVoteError::BallotStyleMismatch(message)) if message.contains(&contest_id)
        ));
    }

    #[test]
    fn ballot_is_rejected_when_no_ballot_style_is_published() {
        assert!(matches!(
            deserialize_and_check_ballot(&single_content(), VOTER_ID, &[]),
            Err(CastVoteError::BallotStyleMismatch(_))
        ));
    }

    #[test]
    fn ballot_without_contests_is_rejected_when_no_ballot_style_is_published() {
        let content = single_content_with_contest_ids(&[]);

        assert!(matches!(
            deserialize_and_check_ballot(&content, VOTER_ID, &[]),
            Err(CastVoteError::BallotStyleMismatch(_))
        ));
    }

    #[test]
    fn ballot_without_contests_is_accepted_for_a_style_without_contests() {
        let content = single_content_with_contest_ids(&[]);

        assert!(deserialize_and_check_ballot(&content, VOTER_ID, &[HashSet::new()]).is_ok());
    }

    #[test]
    fn ballot_matching_any_published_style_is_accepted() {
        let published = vec![contest_ids(&[OTHER_CONTEST_ID]), style_contest_ids()];

        assert!(deserialize_and_check_ballot(&single_content(), VOTER_ID, &published).is_ok());
    }

    #[test]
    fn ballot_cannot_combine_the_contests_of_different_published_styles() {
        let content = single_content_with_contest_ids(&[&get_test_contest().id, OTHER_CONTEST_ID]);
        let published = vec![style_contest_ids(), contest_ids(&[OTHER_CONTEST_ID])];

        assert!(matches!(
            deserialize_and_check_ballot(&content, VOTER_ID, &published),
            Err(CastVoteError::BallotStyleMismatch(_))
        ));
    }

    #[test]
    fn multi_ballot_with_every_style_contest_is_accepted() {
        assert!(deserialize_and_check_multi_ballot(
            &multi_content(),
            VOTER_ID,
            &published_styles()
        )
        .is_ok());
    }

    #[test]
    fn multi_ballot_missing_a_style_contest_is_rejected() {
        let published = vec![contest_ids(&[&get_test_contest().id, OTHER_CONTEST_ID])];

        assert!(matches!(
            deserialize_and_check_multi_ballot(&multi_content(), VOTER_ID, &published),
            Err(CastVoteError::BallotStyleMismatch(message)) if message.contains(OTHER_CONTEST_ID)
        ));
    }

    #[test]
    fn multi_ballot_with_a_contest_outside_the_style_is_rejected() {
        let content = multi_content_with_contest_ids(&[&get_test_contest().id, OTHER_CONTEST_ID]);

        assert!(matches!(
            deserialize_and_check_multi_ballot(&content, VOTER_ID, &published_styles()),
            Err(CastVoteError::BallotStyleMismatch(message)) if message.contains(OTHER_CONTEST_ID)
        ));
    }

    #[test]
    fn multi_ballot_repeating_a_style_contest_is_rejected() {
        let contest_id = get_test_contest().id;
        let content = multi_content_with_contest_ids(&[&contest_id, &contest_id]);

        assert!(matches!(
            deserialize_and_check_multi_ballot(&content, VOTER_ID, &published_styles()),
            Err(CastVoteError::BallotStyleMismatch(message)) if message.contains(&contest_id)
        ));
    }

    #[test]
    fn multi_ballot_is_rejected_when_no_ballot_style_is_published() {
        assert!(matches!(
            deserialize_and_check_multi_ballot(&multi_content(), VOTER_ID, &[]),
            Err(CastVoteError::BallotStyleMismatch(_))
        ));
    }

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

    #[test]
    fn malformed_published_ballot_style_is_an_internal_error() {
        assert!(matches!(
            votable_contest_ids("{\"contests\": 1}"),
            Err(CastVoteError::CheckStatusInternalFailed(_))
        ));
    }
}
