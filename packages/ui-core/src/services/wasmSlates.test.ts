// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    apply_slate_js,
    get_ballot_style_slates_coverage_js,
    get_ballot_style_slates_js,
} from "sequent-core"
import {IBallotStyle} from "../types/CoreTypes"
import {SLATES_ANNOTATION} from "../types/Slates"
import {applySlate, getBallotStyleSlates, getBallotStyleSlatesCoverage} from "./wasm"

jest.mock("sequent-core", () => ({
    get_ballot_style_slates_js: jest.fn(),
    get_ballot_style_slates_coverage_js: jest.fn(),
    apply_slate_js: jest.fn(),
}))

const ballotStyle = {id: "style-1", contests: []} as unknown as IBallotStyle
const slatesBallotStyle: IBallotStyle = {
    ...ballotStyle,
    election_annotations: {[SLATES_ANNOTATION]: "{}"},
}

beforeEach(() => {
    jest.mocked(get_ballot_style_slates_js).mockReset()
    jest.mocked(get_ballot_style_slates_coverage_js).mockReset()
    jest.mocked(apply_slate_js).mockReset()
})

it("reads the slates and their coverage through sequent-core", () => {
    const slates = {version: 1, mobile_candidate_lists: "collapsed", slates: []}
    const coverage = [
        {
            slate_id: "voices",
            kind: "partial",
            covered: [{contest_id: "trustees", candidate_ids: ["v-t1"], seats: 3}],
            uncovered_contest_ids: ["president"],
            members: 1,
            seats: 4,
        },
    ]
    jest.mocked(get_ballot_style_slates_js).mockReturnValue(slates)
    jest.mocked(get_ballot_style_slates_coverage_js).mockReturnValue(coverage)

    expect(getBallotStyleSlates(slatesBallotStyle)).toEqual(slates)
    expect(getBallotStyleSlatesCoverage(slatesBallotStyle)).toEqual(coverage)
    expect(get_ballot_style_slates_js).toHaveBeenCalledWith(slatesBallotStyle)
    expect(get_ballot_style_slates_coverage_js).toHaveBeenCalledWith(slatesBallotStyle)
})

it("applies a slate through sequent-core", () => {
    const slate = {id: "slate-1", name: {en: "Slate"}, members: {"contest-1": ["candidate-1"]}}
    const choices = {selection: [], changes: []}
    jest.mocked(apply_slate_js).mockReturnValue(choices)

    expect(applySlate(slate, [], [])).toEqual(choices)
    expect(apply_slate_js).toHaveBeenCalledWith(slate, [], [])
})

it("does not invoke WASM for a ballot style without slates", () => {
    expect(getBallotStyleSlates(ballotStyle)).toBeNull()
    expect(getBallotStyleSlatesCoverage(ballotStyle)).toBeNull()
    expect(
        getBallotStyleSlates({...ballotStyle, election_annotations: {other: "value"}})
    ).toBeNull()
    expect(get_ballot_style_slates_js).not.toHaveBeenCalled()
    expect(get_ballot_style_slates_coverage_js).not.toHaveBeenCalled()
})

it("reports no slates when the configured annotation resolves to none", () => {
    jest.mocked(get_ballot_style_slates_js).mockReturnValue(undefined)
    jest.mocked(get_ballot_style_slates_coverage_js).mockReturnValue(undefined)

    expect(getBallotStyleSlates(slatesBallotStyle)).toBeNull()
    expect(getBallotStyleSlatesCoverage(slatesBallotStyle)).toBeNull()
})

it("passes on the problems of an invalid configuration", () => {
    const problems = [{severity: "error", code: "invalid_value", path: "p", message: "m"}]
    jest.mocked(get_ballot_style_slates_js).mockImplementation(() => {
        throw problems
    })
    jest.spyOn(console, "log").mockImplementation(() => undefined)

    expect(() => getBallotStyleSlates(slatesBallotStyle)).toThrow()
})
