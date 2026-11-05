// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * What a contest's presentation settings mean. These moved here from the portal's
 * `ElectionConfigService`, which now re-exports them; its tests keep checking the
 * old path, these check the module the ballot imports.
 */

import {CandidatesOrder, EEnableCheckableLists} from "@sequentech/ui-core"
import type {ICandidate, IContest} from "@sequentech/ui-core"

import * as presentation from "./presentation"

const contest = (settings: IContest["presentation"] = {}, maxVotes = 1): IContest =>
    ({id: "c", max_votes: maxVotes, candidates: [], presentation: settings}) as unknown as IContest
const candidate = (settings: ICandidate["presentation"] = {}): ICandidate =>
    ({id: "a", presentation: settings}) as unknown as ICandidate

describe("candidate presentation", () => {
    it("finds the image and the external link independently in the URL list", () => {
        const answer = candidate({
            urls: [
                {title: "Biography", url: "https://example.test/bio", is_image: false},
                {title: "Portrait", url: "https://example.test/photo", is_image: true},
                {title: "URL", url: "https://example.test/candidate", is_image: false},
            ],
        })
        expect(presentation.getImageUrl(answer)).toBe("https://example.test/photo")
        expect(presentation.getLinkUrl(answer)).toBe("https://example.test/candidate")
        expect(presentation.findUrlByTitle(answer, "Biography")).toBe("https://example.test/bio")
        expect(presentation.findUrlByTitle(answer, "Missing")).toBeUndefined()
        expect(presentation.getImageUrl(candidate())).toBeUndefined()
        expect(presentation.getLinkUrl({id: "x"} as unknown as ICandidate)).toBeUndefined()
    })

    it.each([true, false, undefined])("reads each flag on its own when it is %s", (flag) => {
        expect(presentation.checkIsWriteIn(candidate({is_write_in: flag}))).toBe(flag === true)
        expect(presentation.checkIsInvalidVote(candidate({is_explicit_invalid: flag}))).toBe(
            flag === true
        )
        expect(presentation.checkIsExplicitBlankVote(candidate({is_explicit_blank: flag}))).toBe(
            flag === true
        )
        expect(presentation.checkIsCategoryList(candidate({is_category_list: flag}))).toBe(
            flag === true
        )
        expect(presentation.checkAllowWriteIns(contest({allow_writeins: flag}))).toBe(flag === true)
        expect(presentation.checkShuffleCategories(contest({shuffle_categories: flag}))).toBe(
            flag === true
        )
    })

    it("places a marker on top only when it says so", () => {
        expect(presentation.checkPositionIsTop(candidate({invalid_vote_position: "top"}))).toBe(
            true
        )
        expect(presentation.checkPositionIsTop(candidate({invalid_vote_position: "bottom"}))).toBe(
            false
        )
        expect(presentation.checkPositionIsTop(candidate())).toBe(false)
    })
})

describe("contest presentation", () => {
    it("reads the shuffle subset and the custom order", () => {
        expect(presentation.checkShuffleCategoryList(contest())).toEqual([])
        expect(
            presentation.checkShuffleCategoryList(contest({shuffle_category_list: ["A", "B"]}))
        ).toEqual(["A", "B"])
        expect(
            presentation.checkCustomCandidatesOrder(
                contest({candidates_order: CandidatesOrder.CUSTOM})
            )
        ).toBe(true)
        expect(
            presentation.checkCustomCandidatesOrder(
                contest({candidates_order: CandidatesOrder.RANDOM})
            )
        ).toBe(false)
    })

    it.each([
        [EEnableCheckableLists.DISABLED, false, false],
        [EEnableCheckableLists.CANDIDATES_ONLY, false, true],
        [EEnableCheckableLists.LISTS_ONLY, true, false],
        [EEnableCheckableLists.CANDIDATES_AND_LISTS, true, true],
        [undefined, true, true],
    ])("makes %s checkable as lists=%s, candidates=%s", (policy, lists, candidates) => {
        expect(presentation.getCheckableOptions(contest({enable_checkable_lists: policy}))).toEqual(
            {checkableLists: lists, checkableCandidates: candidates}
        )
    })

    it("uses radio selection only for a single choice under the radio policy", () => {
        const radio = {candidates_selection_policy: "radio"} as IContest["presentation"]
        expect(presentation.checkIsRadioSelection(contest(radio, 1))).toBe(true)
        expect(presentation.checkIsRadioSelection(contest(radio, 2))).toBe(false)
        expect(presentation.checkIsRadioSelection(contest({}, 1))).toBe(false)
    })
})
