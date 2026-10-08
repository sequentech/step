// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {EUnderVotePolicy, IContest, IDecodedVoteContest} from "@sequentech/ui-core"
import {getUnfilledContests} from "./UnfilledContests"

const UNDER_VOTE = "errors.implicit.underVote"

const contest = (
    id: string,
    maxVotes: number,
    policy: EUnderVotePolicy | null = EUnderVotePolicy.WARN_AND_CONFIRM_IN_REVIEW,
    over: Partial<IContest> = {}
): IContest =>
    ({
        id,
        name: id,
        min_votes: 0,
        max_votes: maxVotes,
        candidates: [],
        presentation: policy ? {under_vote_policy: policy} : {},
        ...over,
    }) as unknown as IContest

const decoded = (
    contestId: string,
    underVote?: {numSelected: string; max: string},
    over: Partial<IDecodedVoteContest> = {}
): IDecodedVoteContest =>
    ({
        contest_id: contestId,
        is_explicit_invalid: false,
        choices: [],
        invalid_errors: [],
        invalid_alerts: underVote
            ? [{error_type: "Implicit", message: UNDER_VOTE, message_map: underVote}]
            : [],
        ...over,
    }) as unknown as IDecodedVoteContest

describe("getUnfilledContests", () => {
    it("lists an empty office and a contest with unused choices, in ballot order", () => {
        const contests = [
            contest("president", 1),
            contest("vice-president", 1),
            contest("trustees", 3),
        ]
        const unfilled = getUnfilledContests(contests, [
            decoded("trustees", {numSelected: "2", max: "3"}),
            decoded("president"),
            decoded("vice-president", {numSelected: "0", max: "1"}),
        ])
        expect(unfilled).toEqual([
            {contest: contests[1], selected: 0, max: 1},
            {contest: contests[2], selected: 2, max: 3},
        ])
    })

    it("lists every contest of an entirely blank ballot", () => {
        const contests = [contest("president", 1), contest("trustees", 3)]
        const unfilled = getUnfilledContests(contests, [
            decoded("president", {numSelected: "0", max: "1"}),
            decoded("trustees", {numSelected: "0", max: "3"}),
        ])
        expect(unfilled.map((entry) => entry.contest.id)).toEqual(["president", "trustees"])
    })

    it("returns nothing when every position is filled", () => {
        const contests = [contest("president", 1), contest("trustees", 3)]
        expect(getUnfilledContests(contests, [decoded("president"), decoded("trustees")])).toEqual(
            []
        )
    })

    it.each([
        EUnderVotePolicy.ALLOWED,
        EUnderVotePolicy.WARN,
        EUnderVotePolicy.WARN_ONLY_IN_REVIEW,
        EUnderVotePolicy.WARN_AND_ALERT,
        null,
    ])("ignores an undervote in a contest whose policy is %s", (policy) => {
        expect(
            getUnfilledContests(
                [contest("president", 1, policy)],
                [decoded("president", {numSelected: "0", max: "1"})]
            )
        ).toEqual([])
    })

    it("ignores acclaimed contests, explicitly invalid votes and contests not decoded", () => {
        const contests = [
            contest("acclaimed", 1, EUnderVotePolicy.WARN_AND_CONFIRM_IN_REVIEW, {
                is_acclaimed: true,
            }),
            contest("spoiled", 1),
            contest("missing", 1),
        ]
        expect(
            getUnfilledContests(contests, [
                decoded("acclaimed", {numSelected: "0", max: "1"}),
                decoded(
                    "spoiled",
                    {numSelected: "0", max: "1"},
                    {
                        is_explicit_invalid: true,
                    }
                ),
            ])
        ).toEqual([])
    })

    it("falls back to the contest's own maximum when the alert carries no numbers", () => {
        const contests = [contest("trustees", 3)]
        const alert = decoded("trustees", {numSelected: "x", max: ""})
        expect(getUnfilledContests(contests, [alert])).toEqual([
            {contest: contests[0], selected: 0, max: 3},
        ])
    })
})
