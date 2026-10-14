// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {
    CandidatesOrder,
    ECandidatesSelectionPolicy,
    EEnableCheckableLists,
} from "@sequentech/ui-core"
import type {ICandidate, IContest} from "@sequentech/ui-core"
import {ELECTION_WITH_INVALID} from "../fixtures/election"
import * as config from "./ElectionConfigService"

function contest(presentation: IContest["presentation"] = {}): IContest {
    return {...structuredClone(ELECTION_WITH_INVALID.contests[0]), presentation}
}
function candidate(presentation: ICandidate["presentation"] = {}): ICandidate {
    return {...contest().candidates[0], presentation}
}

describe("candidate display and selection policy", () => {
    it("selects image and external-link roles independently from the URL list", () => {
        const answer = candidate({
            urls: [
                {title: "Biography", url: "https://example.test/bio", is_image: false},
                {title: "Portrait", url: "https://example.test/photo", is_image: true},
                {title: "URL", url: "https://example.test/candidate", is_image: false},
            ],
        })
        expect(config.getImageUrl(answer)).toBe("https://example.test/photo")
        expect(config.getLinkUrl(answer)).toBe("https://example.test/candidate")
        expect(config.findUrlByTitle(answer, "Biography")).toBe("https://example.test/bio")
        expect(config.findUrlByTitle(answer, "Missing")).toBeUndefined()
        expect(config.getImageUrl(candidate())).toBeUndefined()
        expect(config.getLinkUrl(candidate())).toBeUndefined()
    })

    it.each([true, false, undefined])(
        "keeps candidate flags independent when a flag is %s",
        (enabled) => {
            expect(config.checkIsWriteIn(candidate({is_write_in: enabled}))).toBe(enabled === true)
            expect(config.checkIsInvalidVote(candidate({is_explicit_invalid: enabled}))).toBe(
                enabled === true
            )
            expect(config.checkAllowWriteIns(contest({allow_writeins: enabled}))).toBe(
                enabled === true
            )
            expect(config.checkShuffleCategories(contest({shuffle_categories: enabled}))).toBe(
                enabled === true
            )
        }
    )

    it("uses the configured marker position and shuffle subset", () => {
        expect(config.checkPositionIsTop(candidate({invalid_vote_position: "top"}))).toBe(true)
        expect(config.checkPositionIsTop(candidate({invalid_vote_position: "bottom"}))).toBe(false)
        expect(config.checkPositionIsTop(candidate())).toBe(false)
        expect(config.checkShuffleCategoryList(contest())).toEqual([])
        expect(
            config.checkShuffleCategoryList(contest({shuffle_category_list: ["A", "B"]}))
        ).toEqual(["A", "B"])
        expect(
            config.checkCustomCandidatesOrder(contest({candidates_order: CandidatesOrder.CUSTOM}))
        ).toBe(true)
        expect(
            config.checkCustomCandidatesOrder(contest({candidates_order: CandidatesOrder.RANDOM}))
        ).toBe(false)
    })

    it.each([
        [EEnableCheckableLists.DISABLED, false, false],
        [EEnableCheckableLists.CANDIDATES_ONLY, false, true],
        [EEnableCheckableLists.LISTS_ONLY, true, false],
        [EEnableCheckableLists.CANDIDATES_AND_LISTS, true, true],
        [undefined, true, true],
    ])(
        "applies %s without enabling an unrelated selection control",
        (policy, checkableLists, checkableCandidates) => {
            expect(config.getCheckableOptions(contest({enable_checkable_lists: policy}))).toEqual({
                checkableLists,
                checkableCandidates,
            })
        }
    )

    it.each([
        [1, ECandidatesSelectionPolicy.RADIO, true],
        [2, ECandidatesSelectionPolicy.RADIO, false],
        [0, ECandidatesSelectionPolicy.RADIO, false],
        [1, undefined, false],
    ])(
        "uses radio selection only for a single-choice contest (%s, %s)",
        (maxVotes, policy, expected) => {
            const question = {
                ...contest({candidates_selection_policy: policy}),
                max_votes: maxVotes,
            }
            expect(config.checkIsRadioSelection(question)).toBe(expected)
        }
    )
})
