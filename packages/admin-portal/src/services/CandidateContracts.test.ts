// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {filterCandidateByCheckableLists, isCandidateACheckableList} from "./CandidatesFilter"
import {
    getCheckableOptions,
    getImageUrl,
    getLinkUrl,
    checkIsWriteIn,
    checkIsInvalidVote,
} from "./ElectionConfigService"
import {EEnableCheckableLists} from "../../../ui-core/src/types/ContestPresentation"
import type {Sequent_Backend_Candidate} from "@/gql/graphql"
import type {ICandidate, IContest} from "@sequentech/ui-core"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("@sequentech/ui-core"),
    ...jest.requireActual("../../../ui-core/src/types/ContestPresentation"),
}))
const candidate = (presentation = {}) => ({presentation}) as Sequent_Backend_Candidate

it.each([
    [EEnableCheckableLists.CANDIDATES_AND_LISTS, true, true],
    [EEnableCheckableLists.CANDIDATES_ONLY, true, false],
    [EEnableCheckableLists.LISTS_ONLY, false, true],
    [EEnableCheckableLists.DISABLED, false, false],
])("agrees on candidate/list availability for %s", (mode, candidates, lists) => {
    expect(filterCandidateByCheckableLists(candidate(), mode)).toBe(candidates)
    expect(filterCandidateByCheckableLists(candidate({is_category_list: true}), mode)).toBe(lists)
    expect(getCheckableOptions({presentation: {enable_checkable_lists: mode}} as IContest)).toEqual(
        {checkableCandidates: candidates, checkableLists: lists}
    )
})
it("defaults absent presentation to an ordinary candidate with no list controls", () => {
    expect(isCandidateACheckableList({} as Sequent_Backend_Candidate)).toBe(false)
    expect(getCheckableOptions({} as IContest)).toEqual({
        checkableCandidates: false,
        checkableLists: false,
    })
    expect(checkIsWriteIn({} as ICandidate)).toBe(false)
    expect(checkIsInvalidVote({} as ICandidate)).toBe(false)
    expect(checkIsWriteIn({presentation: {is_write_in: true}} as ICandidate)).toBe(true)
    expect(checkIsInvalidVote({presentation: {is_explicit_invalid: true}} as ICandidate)).toBe(true)
})
it("selects the first image and the exact URL title, not an unrelated link", () => {
    const answer = {
        presentation: {
            urls: [
                {title: "url", url: "https://wrong.invalid"},
                {title: "photo", url: "https://image.invalid", is_image: true},
                {title: "URL", url: "https://link.invalid"},
                {title: "URL", url: "https://later.invalid", is_image: true},
            ],
        },
    } as ICandidate
    expect(getImageUrl(answer)).toBe("https://image.invalid")
    expect(getLinkUrl(answer)).toBe("https://link.invalid")
    expect(getImageUrl({} as ICandidate)).toBeUndefined()
    expect(getLinkUrl({} as ICandidate)).toBeUndefined()
})
