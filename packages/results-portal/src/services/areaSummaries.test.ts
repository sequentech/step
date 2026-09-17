// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {expect, it} from "@jest/globals"
import {buildAreaElectionSummaries} from "./areaSummaries"
import {manifestCustomCss} from "./customCss"
import {cssClassToken, entityClassName} from "./cssClassNames"
import type {ResultsManifest, ResultsSqliteDataset} from "@/types/results"

it("chooses the largest census, then votes, without summing overlapping contests", () => {
    const dataset = {
        results_election_area: [{election_id: "e", area_id: "1", name: "North"}, {election_id: "e", area_id: "missing"}],
        results_area_contest: [
            {election_id: "e", area_id: 1, elegible_census: "100", total_votes: "70"},
            {election_id: "e", area_id: "1", elegible_census: 100, total_votes: 80},
            {election_id: "e", area_id: "1", elegible_census: 99, total_votes: 99},
            {election_id: "other", area_id: "1", elegible_census: 900, total_votes: 900},
            {election_id: "e", area_id: "other", elegible_census: 900, total_votes: 900},
        ],
    } as unknown as ResultsSqliteDataset
    expect(buildAreaElectionSummaries(dataset)).toEqual([
        {election_id: "e", area_id: "1", name: "North", elegible_census: 100, total_voters: 80, total_voters_percent: 0.8},
        {election_id: "e", area_id: "missing", elegible_census: 0, total_voters: 0, total_voters_percent: 0},
    ])
    expect(dataset.results_election_area[0]).not.toHaveProperty("total_voters")
})
it("normalizes invalid numeric values and avoids division by zero", () => {
    const dataset = {results_election_area: [{election_id: 1, area_id: 2}], results_area_contest: [
        {election_id: "1", area_id: "2", elegible_census: "invalid", total_votes: Infinity},
        {election_id: 1, area_id: 2, elegible_census: 0, total_votes: null},
    ]} as unknown as ResultsSqliteDataset
    expect(buildAreaElectionSummaries(dataset)[0]).toMatchObject({elegible_census: 0, total_voters: 0, total_voters_percent: 0})
})
it("combines event and selected-election CSS while preserving meaningful whitespace", () => {
    const manifest = {custom_css: {election_event: "  .event {}  ", elections: {e: ".election {}", blank: " \n "}}} as ResultsManifest
    expect(manifestCustomCss(manifest, "e")).toBe("  .event {}  \n.election {}")
    expect(manifestCustomCss(manifest, "blank")).toBe("  .event {}  ")
    expect(manifestCustomCss(manifest, "other")).toBe("  .event {}  ")
    expect(manifestCustomCss(undefined)).toBe("")
})
it("produces stable CSS tokens for global, numeric and punctuation-bearing identities", () => {
    for (const value of [null, undefined, ""]) expect(cssClassToken(value)).toBe("global")
    expect(cssClassToken(0)).toBe("0")
    expect(cssClassToken("North / A_2")).toBe("North---A_2")
    expect(entityClassName("area", "a-b")).toBe("seq-results-area--a-b")
})
