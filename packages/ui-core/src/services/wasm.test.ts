// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {afterEach, beforeEach, describe, expect, it, jest} from "@jest/globals"
import type {IContest, IDecodedVoteChoice, IDecodedVoteContest} from "./wasm"

// sequent-core is WASM, which this suite cannot load — see i18n.test.ts for
// the same treatment. What is under test here is the wrapper, not the
// boundary: that each one hands its arguments to the right function in the
// right order, returns what comes back, and lets a parse failure through
// rather than swallowing it. Whether sequent-core then deserializes those
// arguments correctly is not observable from here.
const mockFilterVisibleMessagesJs = jest.fn()
const mockSelectionCappedJs = jest.fn()
const mockApplySelectionJs = jest.fn()

jest.mock("sequent-core", () => ({
    __esModule: true,
    default: jest.fn(),
    filter_visible_messages_js: (...args: Array<unknown>) => mockFilterVisibleMessagesJs(...args),
    selection_capped_js: (...args: Array<unknown>) => mockSelectionCappedJs(...args),
    apply_selection_js: (...args: Array<unknown>) => mockApplySelectionJs(...args),
}))

import {applySelection, filterVisibleMessages, selectionCapped} from "./wasm"

// The wrappers pass these through untouched, so a sentinel the test can
// compare by identity says more than a realistic object would.
const contest = {id: "contest-1"} as unknown as IContest
const selection = {contest_id: "contest-1"} as unknown as IDecodedVoteContest
const choice = {id: "candidate-1", selected: 0} as unknown as IDecodedVoteChoice

describe("the sequent-core service wrappers", () => {
    beforeEach(() => {
        mockFilterVisibleMessagesJs.mockReset()
        mockSelectionCappedJs.mockReset()
        mockApplySelectionJs.mockReset()
        // The wrappers log before rethrowing; keep that out of the report.
        jest.spyOn(console, "log").mockImplementation(() => undefined)
    })
    afterEach(() => {
        jest.restoreAllMocks()
    })

    it("filterVisibleMessages keeps isReview and isTouched in their own positions", () => {
        const filtered = {contest_id: "filtered"} as unknown as IDecodedVoteContest
        mockFilterVisibleMessagesJs.mockReturnValue(filtered)

        // Deliberately different, so transposing them fails here rather than
        // showing the review screen's messages on the voting screen.
        const result = filterVisibleMessages(contest, selection, true, false)

        expect(mockFilterVisibleMessagesJs).toHaveBeenCalledWith(contest, selection, true, false)
        expect(result).toBe(filtered)
    })

    it("selectionCapped passes the contest and the selection, and returns the verdict", () => {
        mockSelectionCappedJs.mockReturnValue(true)

        expect(selectionCapped(contest, selection)).toBe(true)
        expect(mockSelectionCappedJs).toHaveBeenCalledWith(contest, selection)
    })

    it("applySelection passes the edit, whether it is a choice or the invalid flag", () => {
        const edited = {contest_id: "edited"} as unknown as IDecodedVoteContest
        mockApplySelectionJs.mockReturnValue(edited)

        expect(applySelection(contest, selection, choice, false)).toBe(edited)
        expect(mockApplySelectionJs).toHaveBeenCalledWith(contest, selection, choice, false)

        // A null choice means the edit is the explicit-invalid flag itself.
        applySelection(contest, selection, null, true)
        expect(mockApplySelectionJs).toHaveBeenLastCalledWith(contest, selection, null, true)
    })

    it.each([
        [
            "filterVisibleMessages",
            () => filterVisibleMessages(contest, selection, false, true),
            mockFilterVisibleMessagesJs,
        ],
        ["selectionCapped", () => selectionCapped(contest, selection), mockSelectionCappedJs],
        [
            "applySelection",
            () => applySelection(contest, selection, choice, false),
            mockApplySelectionJs,
        ],
    ])("%s lets a parse failure through rather than swallowing it", (_name, call, mocked) => {
        const parseError = new Error("Error parsing contest: missing field `id`")
        mocked.mockImplementation(() => {
            throw parseError
        })

        expect(call).toThrow(parseError)
    })
})
