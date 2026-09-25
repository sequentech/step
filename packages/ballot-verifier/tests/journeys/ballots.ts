// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {loadCore} from "@sequentech/ui-test-kit/wasm/node"
import {electionFixture, IDS} from "@sequentech/ui-test-kit/fixtures"
import type {
    IAuditableSingleBallot,
    IAuditableMultiBallot,
    IDecodedVoteContest,
    IBallotStyle,
} from "@sequentech/ui-core"

/** Produces real encrypted and signed input independently of the verifier's wrappers. */
export async function signedBallot(multiple = false) {
    const core = await loadCore()
    const wire = electionFixture().ballot
    const style: IBallotStyle = {
        id: wire.id,
        tenant_id: wire.tenant_id,
        election_event_id: wire.election_event_id,
        election_id: wire.election_id,
        area_id: wire.area_id,
        description: wire.description,
        public_key: wire.public_key,
        contests: wire.contests.map(
            ({
                voting_type: _voting,
                counting_algorithm: _counting,
                presentation: _presentation,
                ...contest
            }) => contest
        ),
    }
    if (multiple) {
        style.contests.push({
            ...structuredClone(style.contests[0]),
            id: "school",
            name: "School representative",
            candidates: [
                {
                    ...style.contests[0].candidates[0],
                    id: "charlie",
                    contest_id: "school",
                    name: "Charlie Example",
                },
            ],
        })
    }
    const choices: IDecodedVoteContest[] = style.contests.map((contest) => ({
        contest_id: contest.id,
        is_explicit_invalid: false,
        is_decline_to_vote: false,
        is_blank_ballot: false,
        invalid_errors: [],
        invalid_alerts: [],
        choices: contest.candidates.map((candidate, index) => ({
            id: candidate.id,
            selected: index === 0 ? 0 : -1,
        })),
    }))
    const ballot: IAuditableSingleBallot | IAuditableMultiBallot = multiple
        ? core.encrypt_decoded_multi_contest_js(choices, style)
        : core.encrypt_decoded_contest_js(choices, style)
    const hash: string = multiple
        ? core.hash_auditable_multi_ballot_js(ballot)
        : core.hash_auditable_ballot_js(ballot)
    const signature: {public_key: string; signature: string} = multiple
        ? core.sign_hashable_multi_ballot_with_ephemeral_voter_signing_key_js(
              hash,
              IDS.election,
              ballot
          )
        : core.sign_hashable_ballot_with_ephemeral_voter_signing_key_js(hash, IDS.election, ballot)
    ballot.voter_signing_pk = signature.public_key
    ballot.voter_ballot_signature = signature.signature
    return {ballot, hash}
}
