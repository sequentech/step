// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ECandidatesSelectionPolicy, EOverVotePolicy} from "@sequentech/ui-core"
import type {IBallotStyle as IElectionDTO, ICandidate, IContest} from "@sequentech/ui-core"
import type {IResolvedSlate} from "../services/Slates"
import type {IBallotStyle} from "../store/ballotStyles/ballotStylesSlice"

export const SLATES_ELECTION_ID = "slates-election"
export const PRESIDENT = "president"
export const SECRETARY = "secretary"
export const TRUSTEES = "trustees"

const candidate = (contestId: string, id: string, name: string): ICandidate =>
    ({
        id,
        tenant_id: "tenant-1",
        election_event_id: "event-1",
        election_id: SLATES_ELECTION_ID,
        contest_id: contestId,
        name,
        presentation: {},
    }) as ICandidate

const contest = (
    id: string,
    name: string,
    maxVotes: number,
    candidates: Array<[string, string]>
): IContest =>
    ({
        id,
        tenant_id: "tenant-1",
        election_event_id: "event-1",
        election_id: SLATES_ELECTION_ID,
        name,
        min_votes: 0,
        max_votes: maxVotes,
        winning_candidates_num: maxVotes,
        voting_type: "non-preferential",
        counting_algorithm: "plurality-at-large",
        is_encrypted: true,
        candidates: candidates.map(([candidateId, candidateName]) =>
            candidate(id, candidateId, candidateName)
        ),
        presentation: {},
    }) as IContest

const MEMBERS: Array<{id: string; name: string; members: Record<string, string[]>}> = [
    {
        id: "forward",
        name: "Forward Together",
        members: {
            [PRESIDENT]: ["p-forward"],
            [SECRETARY]: ["s-forward"],
            [TRUSTEES]: ["t-forward-1", "t-forward-2", "t-forward-3"],
        },
    },
    {
        id: "members-first",
        name: "Members First",
        members: {
            [PRESIDENT]: ["p-members"],
            [SECRETARY]: ["s-members"],
            [TRUSTEES]: ["t-members-1", "t-members-2", "t-members-3"],
        },
    },
    {
        id: "independent-voices",
        name: "Independent Voices",
        members: {[TRUSTEES]: ["t-voices-1", "t-voices-2"]},
    },
]

/**
 * Two single-seat contests and one three-seat contest, with two full slates, a
 * trustee-only slate and one independent candidate per contest. Names are
 * synthetic.
 */
export const buildSlatesBallot = (): IElectionDTO =>
    ({
        id: "slates-style",
        tenant_id: "tenant-1",
        election_event_id: "event-1",
        election_id: SLATES_ELECTION_ID,
        area_id: "area-1",
        contests: [
            contest(PRESIDENT, "President", 1, [
                ["p-forward", "Amara Lindqvist"],
                ["p-members", "Tobias Okafor"],
                ["p-independent", "Renata Halloran"],
            ]),
            contest(SECRETARY, "Secretary-Treasurer", 1, [
                ["s-forward", "Jonas Whitfield"],
                ["s-members", "Priya Castellanos"],
                ["s-independent", "Mirela Dvorak"],
            ]),
            contest(TRUSTEES, "Trustees", 3, [
                ["t-forward-1", "Elio Marchetti"],
                ["t-forward-2", "Saoirse Nakamura"],
                ["t-forward-3", "Dmitri Oyelaran"],
                ["t-members-1", "Hana Villeneuve"],
                ["t-members-2", "Corwin Abara"],
                ["t-members-3", "Ines Thorvald"],
                ["t-voices-1", "Malik Fenwick"],
                ["t-voices-2", "Odalys Brandt"],
                ["t-independent", "Kasper Yilmaz"],
            ]),
        ],
        election_event_presentation: {},
        election_presentation: {},
    }) as unknown as IElectionDTO

export const buildSlates = (ballotEml: IElectionDTO): IResolvedSlate[] =>
    MEMBERS.map((slate) => ({
        id: slate.id,
        name: {en: slate.name},
        contests: ballotEml.contests
            .filter((entry) => slate.members[entry.id])
            .map((entry) => ({
                contest: entry,
                candidates: slate.members[entry.id].flatMap((id) =>
                    entry.candidates.filter((member) => member.id === id)
                ),
            })),
    }))

export const buildSlate = (ballotEml: IElectionDTO, id: string): IResolvedSlate => {
    const slate = buildSlates(ballotEml).find((entry) => entry.id === id)
    if (!slate) {
        throw new Error(`missing slate ${id}`)
    }
    return slate
}

export const buildSlatesBallotStyle = (
    ballotEml: IElectionDTO = buildSlatesBallot()
): IBallotStyle => ({
    id: ballotEml.id,
    election_id: ballotEml.election_id,
    election_event_id: ballotEml.election_event_id,
    tenant_id: ballotEml.tenant_id,
    ballot_eml: ballotEml,
    created_at: "2026-01-01T00:00:00.000Z",
    last_updated_at: "2026-01-01T00:00:00.000Z",
})

/**
 * The same ballot with the usual limits of such an election: a single-seat
 * contest replaces its choice, and Trustees refuses a selection above three.
 */
export const buildLimitedSlatesBallot = (): IElectionDTO => {
    const ballot = buildSlatesBallot()
    return {
        ...ballot,
        contests: ballot.contests.map((entry) => ({
            ...entry,
            presentation:
                entry.max_votes === 1
                    ? {
                          candidates_selection_policy: ECandidatesSelectionPolicy.RADIO,
                          over_vote_policy: EOverVotePolicy.NOT_ALLOWED_WITH_MSG_AND_ALERT,
                      }
                    : {
                          candidates_selection_policy: ECandidatesSelectionPolicy.CUMULATIVE,
                          over_vote_policy: EOverVotePolicy.NOT_ALLOWED_WITH_MSG_AND_DISABLE,
                      },
        })),
    }
}
