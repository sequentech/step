// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {create_tree_js, get_contest_matches_js} from "sequent-core"
import {createTree, getContestMatches, IAreaContest, ITreeNodeArea} from "./AreaService"

jest.mock("sequent-core", () => ({
    create_tree_js: jest.fn(),
    get_contest_matches_js: jest.fn(),
}))

const createTreeJs = jest.mocked(create_tree_js)
const getContestMatchesJs = jest.mocked(get_contest_matches_js)

const areas: Array<ITreeNodeArea> = [
    {id: "parent", tenant_id: "tenant", election_event_id: "event"},
    {id: "child", tenant_id: "tenant", election_event_id: "event", parent_id: "parent"},
]
const areaContests: Array<IAreaContest> = [{id: "ac-1", area_id: "child", contest_id: "contest"}]
const tree = {
    area: areas[0],
    children: [{area: areas[1], children: [], data: {contest_ids: ["contest"]}}],
    data: {contest_ids: []},
}

beforeEach(() => {
    jest.resetAllMocks()
    jest.spyOn(console, "log").mockImplementation(() => undefined)
})

describe("area tree", () => {
    it("builds the tree in sequent-core", () => {
        createTreeJs.mockReturnValue(tree)

        expect(createTree(areas, areaContests)).toBe(tree)
        expect(createTreeJs).toHaveBeenCalledWith(areas, areaContests)
    })

    it("rethrows the error sequent-core raises for areas it cannot arrange", () => {
        const error = new Error("area child has an unknown parent")
        createTreeJs.mockImplementation(() => {
            throw error
        })

        expect(() => createTree(areas, areaContests)).toThrow(error)
    })

    it("reads the matches of a contest from sequent-core", () => {
        getContestMatchesJs.mockReturnValue(areaContests)

        expect(getContestMatches(tree, "contest")).toBe(areaContests)
        expect(getContestMatchesJs).toHaveBeenCalledWith(tree, "contest")
    })

    it("rethrows the error sequent-core raises for a tree it cannot read", () => {
        const error = new Error("invalid tree")
        getContestMatchesJs.mockImplementation(() => {
            throw error
        })

        expect(() => getContestMatches(tree, "contest")).toThrow(error)
    })
})
