// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ICountingAlgorithm} from "@sequentech/ui-core"
import type {Sequent_Backend_Contest} from "@/gql/graphql"
import {
    convertContestsArray,
    convertSequentContestToIContest,
    orderItemsByIds,
    parseProcessResults,
    parseResultAnnotations,
} from "./utils"

jest.mock("@sequentech/ui-core", () =>
    jest.requireActual("../../../../ui-core/src/types/CoreTypes")
)

describe("orderItemsByIds", () => {
    const items = [
        {id: "election-1", name: "Election 1"},
        {id: "election-2", name: "Election 2"},
    ]

    it("uses the supplied snapshot order without mutating the source", () => {
        expect(orderItemsByIds(items, ["election-2", "election-1"]).map((item) => item.id)).toEqual(
            ["election-2", "election-1"]
        )
        expect(items.map((item) => item.id)).toEqual(["election-1", "election-2"])
    })

    it("returns an empty selection when snapshot IDs are unavailable", () => {
        expect(orderItemsByIds(items, [])).toEqual([])
    })

    it("ignores unknown and duplicate snapshot IDs", () => {
        expect(
            orderItemsByIds(items, [
                "election-2",
                "unknown-election",
                "election-2",
                "election-1",
            ]).map((item) => item.id)
        ).toEqual(["election-2", "election-1"])
    })
})

describe("tally contest conversion", () => {
    const contest: Sequent_Backend_Contest = {
        id: "contest-1",
        tenant_id: "tenant-1",
        election_event_id: "event-1",
        election_id: "election-1",
        candidates: [],
        candidates_aggregate: {nodes: []},
        min_votes: 0,
        max_votes: 2,
        winning_candidates_num: 1,
        is_encrypted: true,
        description: "Choose two representatives",
        voting_type: "non-preferential",
        counting_algorithm: "plurality-at-large",
        created_at: "2026-01-01T00:00:00Z",
        presentation: '{"i18n":{"en":{"name":"Council"}}}',
    }

    it("preserves contest identity, limits and metadata while decoding presentation JSON", () => {
        expect(convertSequentContestToIContest(contest)).toEqual({
            id: "contest-1",
            tenant_id: "tenant-1",
            election_event_id: "event-1",
            election_id: "election-1",
            candidates: [],
            min_votes: 0,
            max_votes: 2,
            winning_candidates_num: 1,
            is_encrypted: true,
            description: "Choose two representatives",
            voting_type: "non-preferential",
            counting_algorithm: "plurality-at-large",
            created_at: "2026-01-01T00:00:00Z",
            presentation: {i18n: {en: {name: "Council"}}},
        })
        expect(contest.presentation).toBe('{"i18n":{"en":{"name":"Council"}}}')
    })

    it.each([null, undefined])("normalizes absent nullable fields (%s)", (absent) => {
        expect(
            convertSequentContestToIContest({
                ...contest,
                min_votes: absent,
                max_votes: absent,
                winning_candidates_num: absent,
                is_encrypted: absent,
                description: absent,
                voting_type: absent,
                counting_algorithm: absent,
                created_at: absent,
                presentation: absent,
            })
        ).toEqual({
            id: "contest-1",
            tenant_id: "tenant-1",
            election_event_id: "event-1",
            election_id: "election-1",
            candidates: [],
            min_votes: 0,
            max_votes: 0,
            winning_candidates_num: 0,
            is_encrypted: false,
            description: undefined,
            voting_type: undefined,
            counting_algorithm: undefined,
            created_at: undefined,
            presentation: undefined,
        })
    })

    it("preserves the supplied contest order and accepts an empty collection", () => {
        expect(
            convertContestsArray([{...contest, id: "contest-2"}, contest]).map(({id}) => id)
        ).toEqual(["contest-2", "contest-1"])
        expect(convertContestsArray([])).toEqual([])
    })

    it("rejects malformed presentation JSON instead of silently dropping it", () => {
        expect(convertSequentContestToIContest(contest).presentation).toEqual({
            i18n: {en: {name: "Council"}},
        })
        expect(() =>
            convertSequentContestToIContest({...contest, presentation: '{"i18n":'})
        ).toThrow(SyntaxError)
    })
})

describe("tally result annotations", () => {
    const annotations = {process_results: {round_count: 1, rounds: []}}

    it("accepts both JSON text and decoded annotations", () => {
        expect(parseResultAnnotations('{"process_results":{"round_count":1,"rounds":[]}}')).toEqual(
            annotations
        )
        expect(parseResultAnnotations(annotations)).toEqual(annotations)
    })

    it.each([undefined, null, "", "{", "null", '"text"', "42", "false"])(
        "returns no annotations for absent, malformed or scalar input (%s)",
        (input) => {
            expect(parseResultAnnotations(input)).toBeNull()
        }
    )

    it("exposes the instant-runoff process results", () => {
        expect(
            parseProcessResults(
                '{"process_results":{"round_count":1,"rounds":[]}}',
                ICountingAlgorithm.INSTANT_RUNOFF
            )
        ).toEqual({round_count: 1, rounds: []})
    })

    it.each([undefined, null, "{", "{}", '{"process_results":null}'])(
        "returns no process results for missing or invalid annotations (%s)",
        (input) => {
            expect(parseProcessResults(input, ICountingAlgorithm.INSTANT_RUNOFF)).toBeNull()
        }
    )

    it("preserves process results for another counting algorithm", () => {
        const log = jest.spyOn(console, "log").mockImplementation(() => undefined)
        try {
            expect(
                parseProcessResults(
                    '{"process_results":{"ties":["alice","bob"]}}',
                    ICountingAlgorithm.PLURALITY_AT_LARGE
                )
            ).toEqual({ties: ["alice", "bob"]})
        } finally {
            log.mockRestore()
        }
    })
})
