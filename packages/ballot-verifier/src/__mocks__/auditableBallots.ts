// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {
    IAuditableMultiBallot,
    IAuditableSingleBallot,
    IBallotStyle,
    IDecodedVoteContest,
} from "@sequentech/ui-core"
import {electionFixture, IDS} from "@sequentech/ui-test-kit/fixtures"
import {sequentCore} from "./sequentCore"

export {IDS}

/** The ballot style that tests/journeys/ballots.ts encrypts and signs with the real WASM. */
export function ballotStyle(multiple = false): IBallotStyle {
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
            ...style.contests[0],
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
    return style
}

/** The journeys' literal votes: the first candidate of each contest (Alice, Charlie), not Bob. */
export const firstCandidateChosen = (style: IBallotStyle): IDecodedVoteContest[] =>
    style.contests.map((contest) => ({
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

type AuditableBallot = IAuditableSingleBallot | IAuditableMultiBallot

export interface RecordedBallot<Ballot extends AuditableBallot = AuditableBallot> {
    ballot: Ballot
    ballotId: string
    decoded: IDecodedVoteContest[]
}

const signature = btoa(String.fromCharCode(...Array.from({length: 64}, (_, index) => index)))

export const singleContestBallot = (): RecordedBallot<IAuditableSingleBallot> => {
    const config = ballotStyle()
    return {
        ballot: {
            version: 1,
            issue_date: "15/01/2026",
            config,
            contests: ["council-ciphertext"],
            ballot_hash: "5c".repeat(32),
            voter_signing_pk: "voter-signing-public-key",
            voter_ballot_signature: signature,
        },
        ballotId: "5c".repeat(32),
        decoded: firstCandidateChosen(config),
    }
}

export const multiContestBallot = (): RecordedBallot<IAuditableMultiBallot> => {
    const config = ballotStyle(true)
    return {
        ballot: {
            version: 1,
            issue_date: "15/01/2026",
            config,
            contests: "council-and-school-ciphertext",
            ballot_hash: "3e".repeat(32),
            voter_signing_pk: "voter-signing-public-key",
            voter_ballot_signature: signature,
        },
        ballotId: "3e".repeat(32),
        decoded: firstCandidateChosen(config),
    }
}

/** What the WASM sample generator returns: a single-contest ballot without a voter signature. */
export const sampleBallot = (): RecordedBallot<IAuditableSingleBallot> => {
    const config = ballotStyle()
    return {
        ballot: {
            version: 1,
            issue_date: "15/01/2026",
            config,
            contests: ["sample-ciphertext"],
            ballot_hash: "a7".repeat(32),
        },
        ballotId: "a7".repeat(32),
        decoded: firstCandidateChosen(config),
    }
}

/** Flips one bit of the decoded signature, keeping valid base64, as the journeys do. */
export function withModifiedSignature<Ballot extends AuditableBallot>(ballot: Ballot): Ballot {
    const bytes = atob(ballot.voter_ballot_signature ?? "")
    const last = bytes.charCodeAt(bytes.length - 1) ^ 1
    return {
        ...ballot,
        voter_ballot_signature: btoa(bytes.slice(0, -1) + String.fromCharCode(last)),
    }
}

/**
 * Answers the sequent-core ballot bindings for the given ballots as the real
 * module does (src/wasm/wasm.rs, ballot.rs): single- and multi-contest
 * bindings reject each other's format, and a signature check returns false
 * without any signature, but throws for an incomplete or non-matching one.
 */
export function recordSequentCore(...records: RecordedBallot[]) {
    const recordOf = (ballot: AuditableBallot, multiple: boolean) => {
        if (Array.isArray(ballot?.contests) === multiple) {
            throw `Error parsing auditable ballot: expected ${multiple ? "a string" : "a sequence"}`
        }
        const record = records.find(
            (candidate) =>
                JSON.stringify(candidate.ballot.contests) === JSON.stringify(ballot.contests)
        )
        if (!record) throw "Error decoding the ballot contests"
        return record
    }
    const verify =
        (multiple: boolean) => (ballotId: string, electionId: string, ballot: AuditableBallot) => {
            const record = recordOf(ballot, multiple)
            const {voter_signing_pk: publicKey, voter_ballot_signature: signed} = ballot
            if (publicKey == null && signed == null) return false
            if (publicKey == null || signed == null) {
                throw "Incomplete ballot signature: public key and signature must both be present"
            }
            if (
                ballotId !== record.ballotId ||
                electionId !== record.ballot.config.election_id ||
                publicKey !== record.ballot.voter_signing_pk ||
                signed !== record.ballot.voter_ballot_signature
            ) {
                throw "Error verifying the ballot: Failed to verify signature"
            }
            return true
        }
    for (const multiple of [false, true]) {
        const format = multiple ? "multi_ballot" : "ballot"
        sequentCore[`decode_auditable_${format}_js`].mockImplementation(
            (ballot: AuditableBallot) => recordOf(ballot, multiple).decoded
        )
        sequentCore[`hash_auditable_${format}_js`].mockImplementation(
            (ballot: AuditableBallot) => recordOf(ballot, multiple).ballotId
        )
        sequentCore[`verify_${format}_signature_js`].mockImplementation(verify(multiple))
    }
}
