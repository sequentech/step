// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {
    ICandidate,
    IContest,
    IBallotStyle,
    IAuditableSingleBallot,
} from "../src/types/CoreTypes"
import type {IDecodedVoteContest} from "../src/services/wasm"

/** Complete synthetic identities keep display tests independent of real elections. */
export function candidate(id: string, overrides: Partial<ICandidate> = {}): ICandidate {
    return {
        id,
        tenant_id: "tenant-1",
        election_event_id: "event-1",
        election_id: "election-1",
        contest_id: "contest-1",
        ...overrides,
    }
}

export function contest(overrides: Partial<IContest> = {}): IContest {
    return {
        id: "contest-1",
        tenant_id: "tenant-1",
        election_event_id: "event-1",
        election_id: "election-1",
        max_votes: 1,
        min_votes: 0,
        winning_candidates_num: 1,
        is_encrypted: true,
        candidates: [],
        ...overrides,
    }
}

/** Adapter inputs are deliberately synthetic; browser tests exercise real WASM ballots. */
export const ballotStyle: IBallotStyle = {
    id: "style-1",
    tenant_id: "tenant-1",
    election_event_id: "event-1",
    election_id: "election-1",
    area_id: "area-1",
    contests: [contest()],
}

export const auditableBallot: IAuditableSingleBallot = {
    version: 1,
    issue_date: "2026-01-01T00:00:00Z",
    config: ballotStyle,
    ballot_hash: "synthetic-hash",
    contests: ["synthetic-encoded-contest"],
}

export function decodedContest(overrides: Partial<IDecodedVoteContest> = {}): IDecodedVoteContest {
    return {
        contest_id: "contest-1",
        is_explicit_invalid: false,
        is_decline_to_vote: false,
        is_blank_ballot: false,
        invalid_errors: [],
        invalid_alerts: [],
        choices: [],
        ...overrides,
    }
}
