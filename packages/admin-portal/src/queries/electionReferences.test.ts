// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {electionReferenceIds, matchElectionReferences} from "./electionReferences"

const NORTH = "6d1c2a43-3c3b-4b47-9c39-0f4f0d7a9a01"
const SOUTH = "8e2d3b54-4d4c-4c58-8d4a-1a5a1e8bab12"

const north = {id: NORTH, external_id: "north", presentation: {i18n: {en: {name: "North"}}}}
const south = {id: SOUTH, external_id: null, presentation: {i18n: {en: {name: "South"}}}}

describe("the elections a voter is authorized to", () => {
    it("asks for ids only the values that are ids", () => {
        expect(electionReferenceIds(["north", NORTH, NORTH.toUpperCase(), "", "1234"])).toEqual([
            NORTH,
            NORTH.toUpperCase(),
        ])
    })

    it("finds an election by its external id or its id", () => {
        expect(matchElectionReferences(["north", SOUTH], [north, south])).toEqual([
            {...north, id: "north", election_id: NORTH},
            {...south, id: SOUTH, election_id: SOUTH},
        ])
    })

    it("compares ids whatever their case", () => {
        expect(matchElectionReferences([SOUTH.toUpperCase()], [south])).toEqual([
            {...south, id: SOUTH.toUpperCase(), election_id: SOUTH},
        ])
    })

    it("keeps a value that names no election, to show it as it is", () => {
        expect(matchElectionReferences(["east"], [north])).toEqual([
            {id: "east", election_id: null, external_id: "east", presentation: null},
        ])
    })

    it("lists each value once", () => {
        expect(matchElectionReferences(["north", "north"], [north])).toHaveLength(1)
    })
})
